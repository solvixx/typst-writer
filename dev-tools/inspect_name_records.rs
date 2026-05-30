use typst::foundations::Bytes;
use typst::text::Font;

fn main() {
    println!("Inspecting Name Table for Asset 0 (Libertinus Serif Regular):");
    let font_bytes = typst_assets::fonts().nth(0).unwrap();
    let bytes = Bytes::new(font_bytes.to_vec());
    let font = Font::new(bytes, 0).unwrap();
    let face = font.ttf();

    for n in face.names() {
        if let Some(s) = n.to_string() {
            println!(
                "  Plat={:?} Enc={:?} Lang={:?} ID={} : '{}'",
                n.platform_id, n.encoding_id, n.language_id, n.name_id, s
            );
        }
    }
}
