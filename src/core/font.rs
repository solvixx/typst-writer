use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use std::process::Command;
use typst::text::{Font as TypstFont, FontBook};
use typst::foundations::Bytes;
use typst::utils::LazyHash;

#[derive(Clone)]
pub enum FontSource {
    Embedded(&'static [u8]),
    Path(PathBuf),
}

#[derive(Clone)]
pub struct SystemFont {
    pub source: FontSource,
    pub index: u32,
}

pub struct FontManager {
    pub book: LazyHash<FontBook>,
    pub system_fonts: Vec<SystemFont>,
    loaded_fonts: std::sync::RwLock<std::collections::HashMap<usize, TypstFont>>,
}

impl FontManager {
    /// Gets the global singleton instance of the FontManager.
    pub fn get() -> &'static Self {
        static MANAGER: std::sync::OnceLock<FontManager> = std::sync::OnceLock::new();
        MANAGER.get_or_init(Self::build)
    }

    fn build() -> Self {
        let mut system_fonts = Vec::new();
        let mut book = FontBook::new();
        
        // Index ONLY embedded fonts from typst-assets.
        // The user explicitly requested not to use system defaults.
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

        // Also load from config's custom paths if available
        let config = crate::core::config::ConfigManager::load();
        for path in config.custom_font_paths {
            Self::load_fonts_from_dir(Path::new(&path), &mut book, &mut system_fonts);
        }

        // Load standard system fonts via fontdb so Typst has access to CJK fallback fonts
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        for face in db.faces() {
            if let (fontdb::Source::File(path), Ok(data)) = (&face.source, fs::read(match &face.source { fontdb::Source::File(p) => p, _ => unreachable!() })) {
                let bytes = Bytes::new(data.clone());
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

        Self {
            book: LazyHash::new(book),
            system_fonts,
            loaded_fonts: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    fn load_fonts_from_dir(
        dir: &Path,
        book: &mut FontBook,
        system_fonts: &mut Vec<SystemFont>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    Self::load_fonts_from_dir(&path, book, system_fonts);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if (ext_lower == "ttf" || ext_lower == "otf" || ext_lower == "ttc" || ext_lower == "otc") && fs::read(&path).is_ok() {
                        let data = fs::read(&path).unwrap();
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

    pub fn font(&self, id: usize) -> Option<TypstFont> {
        // Check loaded cache first
        {
            let read_guard = self.loaded_fonts.read().unwrap();
            if let Some(font) = read_guard.get(&id) {
                return Some(font.clone());
            }
        }

        // Font needs loading from index
        let slot = self.system_fonts.get(id)?;
        
        let font = match &slot.source {
            FontSource::Embedded(data) => {
                let bytes = Bytes::new(data.to_vec());
                TypstFont::new(bytes, slot.index)?
            }
            FontSource::Path(path) => {
                let data = fs::read(path).ok()?;
                let bytes = Bytes::new(data);
                TypstFont::new(bytes, slot.index)?
            }
        };

        // Cache loaded font for subsequent requests
        let mut write_guard = self.loaded_fonts.write().unwrap();
        write_guard.insert(id, font.clone());
        
        Some(font)
    }
}

pub fn provision_fonts() {
    let mut provision_dirs = Vec::new();

    if let Some(proj_dirs) = ProjectDirs::from("com", "TypstWriter", "TypstWriter") {
        let data_dir = proj_dirs.data_local_dir().join("fonts");
        provision_dirs.push(data_dir);
    }
    
    // Add standard OS specific fallback paths just in case GPUI font loader looks there
    #[cfg(target_os = "linux")]
    if let Ok(home) = std::env::var("HOME") {
        provision_dirs.push(PathBuf::from(&home).join(".local/share/fonts/typst-writer"));
    }

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        provision_dirs.push(PathBuf::from(&home).join("Library/Fonts/TypstWriter"));
    }

    #[cfg(target_os = "windows")]
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        provision_dirs.push(PathBuf::from(&local_app_data).join("Microsoft\\Windows\\Fonts\\TypstWriter"));
    }

    let mut provisioned_any = false;
    
    for dir in &provision_dirs {
        let _ = fs::create_dir_all(dir);
        for (idx, font_bytes) in typst_assets::fonts().enumerate() {
            let file_name = format!("embedded_font_{}.otf", idx);
            let path = dir.join(&file_name);
            
            if !path.exists() && fs::write(&path, font_bytes).is_ok() {
                provisioned_any = true;
            }
        }
    }

    // Rebuild user's fontconfig cache instantly if new fonts were written
    #[cfg(target_os = "linux")]
    if let (true, Ok(home)) = (provisioned_any, std::env::var("HOME")) {
        let _ = Command::new("fc-cache")
            .arg("-f")
            .arg(PathBuf::from(&home).join(".local/share/fonts"))
            .output();
    }
}
