use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font as TypstFont, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst::LibraryExt;

// --- LAZY FONT TYPES ---

#[derive(Clone)]
enum FontSource {
    Embedded(&'static [u8]),
    Path(std::path::PathBuf),
}

#[derive(Clone)]
pub struct SystemFont {
    source: FontSource,
    index: u32,
}

// --- RECURSIVE MEMORY-SAFE FONT SCANNER (INDEX ONLY) ---

fn load_fonts_from_dir(
    dir: &std::path::Path,
    book: &mut FontBook,
    system_fonts: &mut Vec<SystemFont>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                // Skip massive non-standard/cjk/emoji directories to avoid scanning overhead
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                if dir_name.contains("cjk") || dir_name.contains("emoji") || dir_name.contains("noto") || dir_name.contains("han") {
                    continue;
                }
                load_fonts_from_dir(&path, book, system_fonts);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "ttf" || ext_lower == "otf" || ext_lower == "ttc" || ext_lower == "otc" {
                    // 1. Skip massive CJK and Emoji font files by path name
                    let path_str = path.to_string_lossy().to_lowercase();
                    if path_str.contains("cjk") || path_str.contains("emoji") || path_str.contains("noto") || path_str.contains("han") {
                        continue;
                    }
                    
                    // 2. Skip files larger than 5MB to avoid memory-pooling of massive fonts
                    if let Ok(meta) = std::fs::metadata(&path) {
                        if meta.len() > 5 * 1024 * 1024 {
                            continue;
                        }
                    }
                    
                    if let Ok(data) = std::fs::read(&path) {
                        let bytes = Bytes::new(data);
                        let mut face_idx = 0;
                        while let Some(font) = TypstFont::new(bytes.clone(), face_idx) {
                            book.push(font.info().clone());
                            system_fonts.push(SystemFont {
                                source: FontSource::Path(path.clone()),
                                index: face_idx,
                            });
                            face_idx += 1;
                        }
                    }
                }
            }
        }
    }
}

// --- STATIC STATIC MEMORY THREAD-SAFE LAZY FONT CACHE ---

static LAZY_FONTS: std::sync::OnceLock<(LazyHash<FontBook>, Vec<SystemFont>)> = std::sync::OnceLock::new();
static LOADED_FONTS: std::sync::RwLock<Option<std::collections::HashMap<usize, TypstFont>>> = std::sync::RwLock::new(None);

pub fn get_shared_fonts() -> &'static (LazyHash<FontBook>, Vec<SystemFont>) {
    LAZY_FONTS.get_or_init(|| {
        let mut system_fonts = Vec::new();
        let mut book = FontBook::new();
        
        // 1. Index standard system fonts first for 100% mathematical and layout compatibility with GPUI/system resolver.
        // Memory-safe scanning excludes CJK, Emoji, and massive font packs to keep RSS memory exceptionally low.
        load_fonts_from_dir(std::path::Path::new("/usr/share/fonts"), &mut book, &mut system_fonts);
        load_fonts_from_dir(std::path::Path::new("/usr/local/share/fonts"), &mut book, &mut system_fonts);
        if let Ok(home) = std::env::var("HOME") {
            let home_fonts = std::path::Path::new(&home).join(".local/share/fonts");
            load_fonts_from_dir(&home_fonts, &mut book, &mut system_fonts);
            let classic_fonts = std::path::Path::new(&home).join(".fonts");
            load_fonts_from_dir(&classic_fonts, &mut book, &mut system_fonts);
        }

        // 2. Index embedded fallback fonts from typst-assets last, only as a fallback.
        for font_bytes in typst_assets::fonts() {
            let bytes = Bytes::new(font_bytes.to_vec());
            let mut face_idx = 0;
            while let Some(font) = TypstFont::new(bytes.clone(), face_idx) {
                book.push(font.info().clone());
                system_fonts.push(SystemFont {
                    source: FontSource::Embedded(font_bytes),
                    index: face_idx,
                });
                face_idx += 1;
            }
        }

        (LazyHash::new(book), system_fonts)
    })
}

// --- TYPST COMPILER WORLD ---
#[derive(Clone)]
pub struct SimpleWorld {
    library: LazyHash<Library>,
    book: &'static LazyHash<FontBook>,
    main_id: FileId,
    sources: std::collections::HashMap<FileId, Source>,
    files: std::collections::HashMap<FileId, Bytes>,
}

impl SimpleWorld {
    pub fn new(source: Source) -> Self {
        let (shared_book, _) = get_shared_fonts();
        let main_id = source.id();
        let mut sources = std::collections::HashMap::new();
        sources.insert(main_id, source);
        Self {
            library: LazyHash::new(Library::default()),
            book: shared_book,
            main_id,
            sources,
            files: std::collections::HashMap::new(),
        }
    }

    pub fn source_ref(&self) -> &Source {
        self.sources.get(&self.main_id).unwrap()
    }

    pub fn source_mut(&mut self) -> &mut Source {
        self.sources.get_mut(&self.main_id).unwrap()
    }

    pub fn insert_source(&mut self, source: Source) {
        self.sources.insert(source.id(), source);
    }

    pub fn insert_file(&mut self, id: FileId, bytes: Bytes) {
        self.files.insert(id, bytes);
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
        self.sources.get(&id).cloned().ok_or_else(|| {
            typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
        })
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| {
            typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
        })
    }

    fn font(&self, id: usize) -> Option<TypstFont> {
        // Initialize loaded cache on first use
        {
            let read_guard = LOADED_FONTS.read().unwrap();
            if let Some(cache) = &*read_guard {
                if let Some(font) = cache.get(&id) {
                    return Some(font.clone());
                }
            }
        }

        // Font needs loading from index
        let (_, system_fonts) = get_shared_fonts();
        let slot = system_fonts.get(id)?;
        
        let font = match &slot.source {
            FontSource::Embedded(data) => {
                let bytes = Bytes::new(data.to_vec());
                TypstFont::new(bytes, slot.index)?
            }
            FontSource::Path(path) => {
                let data = std::fs::read(path).ok()?;
                let bytes = Bytes::new(data);
                TypstFont::new(bytes, slot.index)?
            }
        };

        // Cache loaded font for subsequent requests
        let mut write_guard = LOADED_FONTS.write().unwrap();
        let cache = write_guard.get_or_insert_with(std::collections::HashMap::new);
        cache.insert(id, font.clone());
        
        Some(font)
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
