use gpui::*;

fn find_table(data: &[u8], tag: &[u8; 4]) -> Option<usize> {
    if data.len() < 12 {
        return None;
    }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    for i in 0..num_tables {
        let offset = 12 + i * 16;
        if data.len() < offset + 16 {
            break;
        }
        if &data[offset..offset + 4] == tag {
            return Some(u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]) as usize);
        }
    }
    None
}

fn patch_font_family(data: &mut [u8], new_name: &str) {
    if let Some(name_offset) = find_table(data, b"name") {
        let storage_offset =
            u16::from_be_bytes([data[name_offset + 4], data[name_offset + 5]]) as usize;
        let num_records =
            u16::from_be_bytes([data[name_offset + 2], data[name_offset + 3]]) as usize;
        for i in 0..num_records {
            let rec_offset = name_offset + 6 + i * 12;
            let platform_id = u16::from_be_bytes([data[rec_offset + 0], data[rec_offset + 1]]);
            let name_id = u16::from_be_bytes([data[rec_offset + 6], data[rec_offset + 7]]);
            if [1, 4, 16, 21].contains(&name_id) {
                let length =
                    u16::from_be_bytes([data[rec_offset + 8], data[rec_offset + 9]]) as usize;
                let string_offset =
                    u16::from_be_bytes([data[rec_offset + 10], data[rec_offset + 11]]) as usize;
                let absolute_offset = name_offset + storage_offset + string_offset;
                if platform_id == 3 {
                    let utf16_name: Vec<u16> = new_name.encode_utf16().collect();
                    if length >= utf16_name.len() * 2 {
                        for (j, &u) in utf16_name.iter().enumerate() {
                            let b = u.to_be_bytes();
                            data[absolute_offset + j * 2] = b[0];
                            data[absolute_offset + j * 2 + 1] = b[1];
                        }
                        for j in (utf16_name.len() * 2)..length {
                            data[absolute_offset + j] = 0;
                        }
                    }
                } else if platform_id == 1 {
                    let bytes = new_name.as_bytes();
                    if length >= bytes.len() {
                        for (j, &b) in bytes.iter().enumerate() {
                            data[absolute_offset + j] = b;
                        }
                        for j in bytes.len()..length {
                            data[absolute_offset + j] = 0;
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    Application::new().run(|cx| {
        let font_bytes = typst_assets::fonts().nth(0).unwrap();
        let mut data = font_bytes.to_vec();
        let unique_name = "MyUniqueTypstFont";
        patch_font_family(&mut data, unique_name);

        cx.text_system()
            .add_fonts(vec![std::borrow::Cow::Owned(data)])
            .unwrap();

        println!("Available font families after addition:");
        let mut names = cx.text_system().all_font_names();
        names.sort();
        for name in &names {
            println!("  - {}", name);
        }

        let id = cx.text_system().resolve_font(&Font {
            family: unique_name.into(),
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            features: Default::default(),
            fallbacks: None,
        });

        println!("Resolved unique name '{}' -> {:?}", unique_name, id);
        cx.quit();
    });
}
