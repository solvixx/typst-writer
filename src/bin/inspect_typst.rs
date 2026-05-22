use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime};
use typst::layout::{PagedDocument, Frame, FrameItem};
use typst::syntax::{FileId, Source};
use typst::text::{Font as TypstFont, FontBook};
use typst::utils::LazyHash;
use typst::Library;
use typst::LibraryExt;

// --- DEDICATED INSPECTION WORLD ---

fn load_fonts_from_dir(dir: &std::path::Path, book: &mut FontBook, fonts: &mut Vec<TypstFont>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                load_fonts_from_dir(&path, book, fonts);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ext_lower == "ttf" || ext_lower == "otf" || ext_lower == "ttc" || ext_lower == "otc" {
                    if let Ok(data) = std::fs::read(&path) {
                        let bytes = Bytes::new(data);
                        let mut face_idx = 0;
                        while let Some(font) = TypstFont::new(bytes.clone(), face_idx) {
                            book.push(font.info().clone());
                            fonts.push(font);
                            face_idx += 1;
                        }
                    }
                }
            }
        }
    }
}

struct SimpleWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<TypstFont>,
    main_id: FileId,
    source: Source,
}

impl SimpleWorld {
    fn new(text: &str) -> Self {
        let mut fonts = Vec::new();
        let mut book = FontBook::new();
        
        // 1. Load embedded fallback fonts from typst-assets
        for font_bytes in typst_assets::fonts() {
            let bytes = Bytes::new(font_bytes.to_vec());
            let mut face_idx = 0;
            while let Some(font) = TypstFont::new(bytes.clone(), face_idx) {
                book.push(font.info().clone());
                fonts.push(font);
                face_idx += 1;
            }
        }

        // 2. Load system fonts for full math, bold, italic, and weights compatibility
        load_fonts_from_dir(std::path::Path::new("/usr/share/fonts"), &mut book, &mut fonts);
        load_fonts_from_dir(std::path::Path::new("/usr/local/share/fonts"), &mut book, &mut fonts);
        if let Ok(home) = std::env::var("HOME") {
            let home_fonts = std::path::Path::new(&home).join(".local/share/fonts");
            load_fonts_from_dir(&home_fonts, &mut book, &mut fonts);
        }

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main_id: FileId::new(None, typst::syntax::VirtualPath::new("main.typ")),
            source: Source::detached(text),
        }
    }
}

impl typst::World for SimpleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(typst::diag::FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }

    fn font(&self, id: usize) -> Option<TypstFont> {
        self.fonts.get(id).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

fn main() {
    println!("=== BUNDLED TYPST-ASSETS FONTS ===");
    let mut book = typst::text::FontBook::new();
    for font_bytes in typst_assets::fonts() {
        let bytes = typst::foundations::Bytes::new(font_bytes.to_vec());
        let mut face_idx = 0;
        while let Some(font) = typst::text::Font::new(bytes.clone(), face_idx) {
            println!("  - Font Family: {:?}, Weight: {:?}, Style: {:?}", 
                font.info().family, 
                font.info().variant.weight, 
                font.info().variant.style
            );
            face_idx += 1;
        }
    }
}

fn inspect_frame(frame: &Frame, indent: usize) {
    let indent_str = "  ".repeat(indent);
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Text(text_item) => {
                println!(
                    "{}* [Text] at ({:?}, {:?}) -> Font Family = {:?}, Size = {:?}, Weight = {:?}, Style = {:?}, String = {:?}",
                    indent_str,
                    pos.x,
                    pos.y,
                    text_item.font.info().family,
                    text_item.size,
                    text_item.font.info().variant.weight,
                    text_item.font.info().variant.style,
                    text_item.text.to_string()
                );
                
                // Inspect individual shaped glyphs within this text run
                let mut current_offset = 0.0;
                for (glyph_idx, glyph) in text_item.glyphs.iter().enumerate() {
                    let width_pt = (glyph.x_advance.get() as f32) * (text_item.size.to_pt() as f32);
                    println!(
                        "{}  └─ Glyphs[{}] -> Span = {:?}, Width = {:.2}pt, Local Offset = {:.2}pt",
                        indent_str,
                        glyph_idx,
                        glyph.span,
                        width_pt,
                        current_offset
                    );
                    current_offset += width_pt;
                }
            }
            FrameItem::Group(group) => {
                println!(
                    "{}* [Group] at ({:?}, {:?}) -> Transform = tx:{:?}, ty:{:?}",
                    indent_str,
                    pos.x,
                    pos.y,
                    group.transform.tx,
                    group.transform.ty
                );
                inspect_frame(&group.frame, indent + 1);
            }
            FrameItem::Shape(shape, _) => {
                let fill_desc = match &shape.fill {
                    Some(paint) => format!("{:?}", paint),
                    None => "None".to_string(),
                };
                let stroke_desc = match &shape.stroke {
                    Some(stroke) => format!("{:?} Thickness = {:?}", stroke.paint, stroke.thickness),
                    None => "None".to_string(),
                };
                println!(
                    "{}* [Shape] at ({:?}, {:?}) -> Geometry = {:?}, Fill = {}, Stroke = {}",
                    indent_str,
                    pos.x,
                    pos.y,
                    shape.geometry,
                    fill_desc,
                    stroke_desc
                );
            }
            _ => {
                println!("{}* [Other Item] at ({:?}, {:?})", indent_str, pos.x, pos.y);
            }
        }
    }
}
