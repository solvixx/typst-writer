use std::path::Path;
use typst::syntax::{FileId, Source, VirtualPath};
use typst_writer::core::compiler::SimpleWorld;
use typst::layout::FrameItem;
use typst::compile;

fn main() -> anyhow::Result<()> {
    let text = "- Item A\n- Item B\n\n+ One\n+ Two";
    let id = FileId::new(None, VirtualPath::new(Path::new("lists.typ")));
    let source = Source::new(id, text.to_string());
    let world = SimpleWorld::new(source);
    
    let result = compile(&world);
    let doc: typst::layout::PagedDocument = match result.output {
        Ok(doc) => doc,
        Err(diags) => {
            for diag in diags { println!("DIAG: {:?}", diag.message); }
            return Err(anyhow::anyhow!("Compile failed"));
        }
    };
    
    for (i, page) in doc.pages.iter().enumerate() {
        println!("--- Page {} ---", i + 1);
        inspect_frame(&page.frame, 0);
    }
    Ok(())
}

fn inspect_frame(frame: &typst::layout::Frame, indent: usize) {
    let space = "  ".repeat(indent);
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Text(text_item) => {
                let info = text_item.font.info();
                let mut ps = String::from("None");
                for n in text_item.font.ttf().names() {
                    if n.name_id == 6 {
                        if let Some(s) = n.to_string() { ps = s; }
                    }
                }
                println!("{}Text at {:?}: Font='{}' PS='{}', Glyphs:", space, pos, info.family, ps);
                for glyph in &text_item.glyphs {
                    println!("{}  - ID={}, span={:?}", space, glyph.id, glyph.span);
                }
            }
            FrameItem::Group(group) => {
                inspect_frame(&group.frame, indent + 1);
            }
            _ => {}
        }
    }
}
