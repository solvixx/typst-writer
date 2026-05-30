use typst::text::Font;
use typst::foundations::Bytes;

fn main() {
    println!("Extracting PostScript names for all typst-assets:");
    for (idx, font_bytes) in typst_assets::fonts().enumerate() {
        let bytes = Bytes::new(font_bytes.to_vec());
        let mut face_idx = 0;
        while let Some(font) = Font::new(bytes.clone(), face_idx) {
            let mut ps_name = String::new();
            for n in font.ttf().names() {
                if n.name_id == 6 {
                    if let Some(s) = n.to_string() { ps_name = s; }
                }
            }
            println!("Asset {} Face {}: PS='{}' Family='{}' W={:?} S={:?}", 
                     idx, face_idx, ps_name, font.info().family, font.info().variant.weight, font.info().variant.style);
            face_idx += 1;
        }
    }
}
