use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use typst::Library;
use typst::LibraryExt;
use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font as TypstFont, FontBook};
use typst::utils::LazyHash;

/// SimpleWorld implements the `typst::World` trait, providing the compiler with access
/// to sources, files, fonts, and other environment information.
#[derive(Clone)]
pub struct SimpleWorld {
    /// The Typst standard library.
    pub library: LazyHash<Library>,
    /// The global font book.
    pub book: &'static LazyHash<FontBook>,
    /// The ID of the main source file.
    pub main_id: FileId,
    /// The root directory of the project, if any.
    pub root_path: Option<std::path::PathBuf>,
    /// The main source file content.
    pub main_source: Source,
    /// Cached secondary source files.
    pub sources: Arc<Mutex<HashMap<FileId, Source>>>,
    /// Cached binary files (images, data, etc.).
    pub files: Arc<Mutex<HashMap<FileId, Bytes>>>,
}

impl SimpleWorld {
    /// Creates a new SimpleWorld with a single main source.
    pub fn new(source: Source) -> Self {
        let main_id = source.id();
        Self {
            library: LazyHash::new(Library::default()),
            book: &crate::core::font::FontManager::get().book,
            main_id,
            root_path: None,
            main_source: source,
            sources: Arc::new(Mutex::new(HashMap::new())),
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Creates a new SimpleWorld rooted at a specific directory.
    pub fn with_root(root: std::path::PathBuf, main_path: std::path::PathBuf) -> Self {
        let vpath =
            typst::syntax::VirtualPath::within_root(&main_path, &root).unwrap_or_else(|| {
                // Fallback: if main_path is outside root, use its filename as a root-relative path
                let filename = main_path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("main.typ"));
                typst::syntax::VirtualPath::new(filename)
            });
        let main_id = FileId::new(None, vpath);
        let text = std::fs::read_to_string(&main_path).unwrap_or_default();
        let source = Source::new(main_id, text);

        Self {
            library: LazyHash::new(Library::default()),
            book: &crate::core::font::FontManager::get().book,
            main_id,
            root_path: Some(root),
            main_source: source,
            sources: Arc::new(Mutex::new(HashMap::new())),
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns a reference to the main source.
    pub fn source_ref(&self) -> &Source {
        &self.main_source
    }

    /// Returns a mutable reference to the main source.
    pub fn source_mut(&mut self) -> &mut Source {
        &mut self.main_source
    }

    /// Inserts or updates a source file in the world.
    pub fn insert_source(&mut self, source: Source) {
        if source.id() == self.main_id {
            self.main_source = source;
        } else {
            self.sources.lock().unwrap().insert(source.id(), source);
        }
    }

    /// Inserts or updates a binary file in the world.
    pub fn insert_file(&mut self, id: FileId, bytes: Bytes) {
        self.files.lock().unwrap().insert(id, bytes);
    }

    /// Resolve a package-scoped `FileId` to a local filesystem path.
    ///
    /// Supports both `@local` and `@preview` namespaces by looking into standard
    /// Typst package directories.
    fn resolve_package_file(id: FileId) -> Option<std::path::PathBuf> {
        use directories::BaseDirs;
        let spec = id.package()?;
        let rel = id.vpath().as_rootless_path();
        let base_dirs = BaseDirs::new()?;

        let base = match spec.namespace.as_str() {
            "local" => base_dirs
                .data_local_dir()
                .join("typst")
                .join("packages")
                .join("local"),
            "preview" => base_dirs
                .cache_dir()
                .join("typst")
                .join("packages")
                .join("preview"),
            _ => return None,
        };

        Some(
            base.join(spec.name.as_str())
                .join(spec.version.to_string())
                .join(rel),
        )
    }
}

impl typst::World for SimpleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        // 1. Is it the main source?
        if id == self.main_id {
            return Ok(self.main_source.clone());
        }

        // 2. Already cached in memory?
        if let Some(source) = self.sources.lock().unwrap().get(&id) {
            return Ok(source.clone());
        }

        // 3. Project-local file (no package namespace)?
        if id.package().is_none() {
            if let Some(root) = &self.root_path {
                let path = id.vpath().resolve(root).ok_or_else(|| {
                    typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
                })?;
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let src = Source::new(id, text);
                    self.sources.lock().unwrap().insert(id, src.clone());
                    return Ok(src);
                }
            }
            return Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ));
        }

        // 4. Package-scoped file (@local or @preview)?
        if let Some(pkg_path) = Self::resolve_package_file(id) {
            match std::fs::read_to_string(&pkg_path) {
                Ok(text) => {
                    let src = Source::new(id, text);
                    self.sources.lock().unwrap().insert(id, src.clone());
                    return Ok(src);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            }
        }

        Err(typst::diag::FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        // 1. Already cached in memory?
        if let Some(bytes) = self.files.lock().unwrap().get(&id) {
            return Ok(bytes.clone());
        }

        // 2. Project-local binary asset?
        if id.package().is_none() {
            if let Some(root) = &self.root_path {
                let path = id.vpath().resolve(root).ok_or_else(|| {
                    typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
                })?;
                if let Ok(data) = std::fs::read(&path) {
                    let b = Bytes::new(data);
                    self.files.lock().unwrap().insert(id, b.clone());
                    return Ok(b);
                }
            }
            return Err(typst::diag::FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ));
        }

        // 3. Package-scoped binary asset (@local or @preview)?
        if let Some(pkg_path) = Self::resolve_package_file(id) {
            match std::fs::read(&pkg_path) {
                Ok(data) => {
                    let b = Bytes::new(data);
                    self.files.lock().unwrap().insert(id, b.clone());
                    return Ok(b);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            }
        }

        Err(typst::diag::FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn font(&self, id: usize) -> Option<TypstFont> {
        crate::core::font::FontManager::get().font(id)
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        use chrono::{Datelike, Local, Utc};

        if let Some(offset) = offset {
            let naive = Utc::now().naive_utc() + chrono::Duration::hours(offset);
            Datetime::from_ymd(naive.year(), naive.month() as u8, naive.day() as u8)
        } else {
            let local = Local::now();
            Datetime::from_ymd(local.year(), local.month() as u8, local.day() as u8)
        }
    }
}
