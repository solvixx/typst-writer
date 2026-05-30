fn main() {
    for font_bytes in typst_assets::fonts() {
        let mut face_idx = 0;
        loop {
            if let Ok(face) = ttf_parser::Face::parse(font_bytes, face_idx) {
                let mut is_libertine = false;
                for name in face.names() {
                    if name.name_id == 1 {
                        if let Some(s) = name.to_string() {
                            if s.contains("Libertine") || s.contains("Libertinus") {
                                is_libertine = true;
                                println!("Font: {}", s);
                            }
                        }
                    }
                }
                if is_libertine {
                    for n in face.names() {
                        if [1, 2, 6, 16].contains(&n.name_id) {
                            println!("  name_id={}: {:?}", n.name_id, n.to_string());
                        }
                    }
                }
                face_idx += 1;
            } else {
                break;
            }
        }
    }
}
