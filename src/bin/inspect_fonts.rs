fn main() {
    println!("--- TYPST ASSETS FONTS ---");
    // typst_assets::fonts() returns a list/slice of static font bytes
    for (i, font_bytes) in typst_assets::fonts().enumerate() {
        if let Some(font) = typst::text::Font::new(typst::foundations::Bytes::new(font_bytes.to_vec()), 0) {
            println!("Font {}: {:?}", i, font.info().family);
        } else {
            println!("Font {}: Failed to parse", i);
        }
    }
}
