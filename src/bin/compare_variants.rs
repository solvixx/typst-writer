use typst::foundations::Bytes;
use typst::text::Font;

fn main() {
    let font_bytes_book = typst_assets::fonts().nth(7).unwrap();
    let font_bytes_reg = typst_assets::fonts().nth(8).unwrap();

    let font_book = Font::new(Bytes::new(font_bytes_book.to_vec()), 0).unwrap();
    let font_reg = Font::new(Bytes::new(font_bytes_reg.to_vec()), 0).unwrap();

    let ids = [2824, 2825, 2826, 4732, 12, 30];
    println!("Comparing Book (Asset 7) vs Regular (Asset 8):");
    for &id in &ids {
        let gid = ttf_parser::GlyphId(id);
        let bb_book = font_book.ttf().glyph_bounding_box(gid);
        let bb_reg = font_reg.ttf().glyph_bounding_box(gid);

        println!("  ID {}:", id);
        println!("    Book: {:?}", bb_book);
        println!("    Reg : {:?}", bb_reg);
        if bb_book != bb_reg {
            println!("    DIFF DETECTED!");
        }
    }
}
