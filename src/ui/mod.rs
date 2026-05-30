//! User Interface components and views powered by GPUI.
//!
//! This module contains the main workspace, the renderer panel, the code editor,
//! and various reusable UI widgets like the ribbon and file tree.

pub mod components;
pub mod editor;
pub mod renderer;
pub mod workspace;

use gpui::FontId;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static FONT_REGISTRY: OnceLock<Mutex<HashMap<String, FontId>>> = OnceLock::new();
static FONT_COUNTER: OnceLock<Mutex<u32>> = OnceLock::new();

pub fn get_registry() -> &'static Mutex<HashMap<String, FontId>> {
    FONT_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn get_counter() -> &'static Mutex<u32> {
    FONT_COUNTER.get_or_init(|| Mutex::new(0))
}

pub fn gpui_font_id_by_postscript_name(ps_name: &str) -> Option<FontId> {
    get_registry().lock().unwrap().get(ps_name).copied()
}

fn read_postscript_name(face: &ttf_parser::Face<'_>) -> String {
    let mut family = String::new();
    let mut subfamily = String::new();
    let mut ps_name = String::new();
    for record in face.names() {
        match record.name_id {
            1 => {
                if let Some(s) = record.to_string() {
                    family = s;
                }
            }
            2 => {
                if let Some(s) = record.to_string() {
                    subfamily = s;
                }
            }
            6 => {
                if let Some(s) = record.to_string() {
                    ps_name = s;
                }
            }
            _ => {}
        }
    }
    if !ps_name.is_empty() {
        return ps_name;
    }
    if subfamily.is_empty() {
        family
    } else {
        format!("{}-{}", family, subfamily)
    }
}

pub fn patch_font_family_safe(data: &mut [u8], font_id: u32, face_idx: u32) {
    let mut face_offset = 0;
    if data.starts_with(b"ttcf") {
        if data.len() < 12 {
            return;
        }
        let num_fonts = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        if face_idx < num_fonts {
            let offset_pos = 12 + (face_idx as usize) * 4;
            if data.len() < offset_pos + 4 {
                return;
            }
            face_offset = u32::from_be_bytes([
                data[offset_pos],
                data[offset_pos + 1],
                data[offset_pos + 2],
                data[offset_pos + 3],
            ]) as usize;

            if face_idx != 0 {
                let offset0_pos = 12;
                let offset0 = u32::from_be_bytes([
                    data[offset0_pos],
                    data[offset0_pos + 1],
                    data[offset0_pos + 2],
                    data[offset0_pos + 3],
                ]);

                let face_bytes = (face_offset as u32).to_be_bytes();
                data[offset0_pos..offset0_pos + 4].copy_from_slice(&face_bytes);

                let offset0_bytes = offset0.to_be_bytes();
                data[offset_pos..offset_pos + 4].copy_from_slice(&offset0_bytes);
            }
        } else {
            return;
        }
    }

    fn find_table(data: &[u8], face_offset: usize, tag: &[u8; 4]) -> Option<(usize, usize, usize)> {
        if data.len() < face_offset + 12 {
            return None;
        }
        let num_tables =
            u16::from_be_bytes([data[face_offset + 4], data[face_offset + 5]]) as usize;
        for i in 0..num_tables {
            let offset = face_offset + 12 + i * 16;
            if data.len() < offset + 16 {
                return None;
            }
            if &data[offset..offset + 4] == tag {
                let table_offset = u32::from_be_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]) as usize;
                let table_length = u32::from_be_bytes([
                    data[offset + 12],
                    data[offset + 13],
                    data[offset + 14],
                    data[offset + 15],
                ]) as usize;
                return Some((offset, table_offset, table_length));
            }
        }
        None
    }

    if let Some((record_offset, name_offset, table_len)) = find_table(data, face_offset, b"name") {
        if data.len() < name_offset + 6 {
            return;
        }
        let num_records =
            u16::from_be_bytes([data[name_offset + 2], data[name_offset + 3]]) as usize;
        let storage_offset =
            u16::from_be_bytes([data[name_offset + 4], data[name_offset + 5]]) as usize;

        for i in 0..num_records {
            let rec_offset = name_offset + 6 + i * 12;
            if data.len() < rec_offset + 12 {
                break;
            }

            let platform_id = u16::from_be_bytes([data[rec_offset + 0], data[rec_offset + 1]]);
            let name_id = u16::from_be_bytes([data[rec_offset + 6], data[rec_offset + 7]]);

            if [1, 4, 16, 21].contains(&name_id) {
                let length =
                    u16::from_be_bytes([data[rec_offset + 8], data[rec_offset + 9]]) as usize;
                let string_offset =
                    u16::from_be_bytes([data[rec_offset + 10], data[rec_offset + 11]]) as usize;
                let absolute_offset = name_offset + storage_offset + string_offset;

                if data.len() < absolute_offset + length {
                    continue;
                }

                let id_str = format!("TF{:04}", font_id);

                if platform_id == 3 || platform_id == 0 {
                    // Windows / Unicode (UTF-16BE)
                    let chars_count = length / 2;
                    if chars_count >= id_str.len() {
                        let mut replacement = id_str.clone();
                        while replacement.len() < chars_count {
                            replacement.push('X');
                        }

                        let utf16_name: Vec<u16> = replacement.encode_utf16().collect();
                        for (j, &u) in utf16_name.iter().enumerate() {
                            let bytes = u.to_be_bytes();
                            data[absolute_offset + j * 2] = bytes[0];
                            data[absolute_offset + j * 2 + 1] = bytes[1];
                        }
                    } else {
                        // Too short to patch! Invalidate this record so GPUI ignores it and uses the English one.
                        data[rec_offset + 6] = 0xFF;
                        data[rec_offset + 7] = 0xFF;
                    }
                } else if platform_id == 1 {
                    // Mac Roman
                    if length >= id_str.len() {
                        let mut replacement = id_str.clone();
                        while replacement.len() < length {
                            replacement.push('X');
                        }

                        let bytes = replacement.as_bytes();
                        for (j, &b) in bytes.iter().enumerate() {
                            data[absolute_offset + j] = b;
                        }
                    } else {
                        // Too short to patch! Invalidate this record.
                        data[rec_offset + 6] = 0xFF;
                        data[rec_offset + 7] = 0xFF;
                    }
                }
            }
        }

        // Recalculate name table checksum
        let mut sum = 0u32;
        let mut i = 0;
        while i < table_len {
            let b0 = data.get(name_offset + i).copied().unwrap_or(0);
            let b1 = data.get(name_offset + i + 1).copied().unwrap_or(0);
            let b2 = data.get(name_offset + i + 2).copied().unwrap_or(0);
            let b3 = data.get(name_offset + i + 3).copied().unwrap_or(0);
            sum = sum.wrapping_add(u32::from_be_bytes([b0, b1, b2, b3]));
            i += 4;
        }
        let checksum_bytes = sum.to_be_bytes();
        data[record_offset + 4..record_offset + 8].copy_from_slice(&checksum_bytes);
    }
}

pub fn load_ui_fonts(_cx: &mut gpui::App) {
    // Dynamic font loading handles everything lazily now.
}

pub fn resolve_typst_font(cx: &mut gpui::App, font: &typst::text::Font) -> Option<FontId> {
    let ps = read_postscript_name(&font.ttf());

    if let Some(id) = gpui_font_id_by_postscript_name(&ps) {
        return Some(id);
    }

    // Dynamic load
    let mut data = font.data().to_vec();
    let face_idx = font.index();

    let mut counter = get_counter().lock().unwrap();
    let font_idx = *counter;
    *counter += 1;
    drop(counter);

    patch_font_family_safe(&mut data, font_idx, face_idx);
    let prefix = format!("TF{:04}", font_idx);

    let res = cx.text_system().add_fonts(vec![Cow::Owned(data)]);
    if let Err(_e) = res {
        // Silently ignore or rely on internal GPUI logs
    }

    let mut resolved_family = prefix.clone();
    for name in cx.text_system().all_font_names() {
        if name.starts_with(&prefix) {
            resolved_family = name;
            break;
        }
    }

    // Suppressed terminal output

    let font_id = cx.text_system().resolve_font(&gpui::Font {
        family: resolved_family.into(),
        weight: gpui::FontWeight::NORMAL,
        style: gpui::FontStyle::Normal,
        features: Default::default(),
        fallbacks: None,
    });

    get_registry().lock().unwrap().insert(ps.clone(), font_id);
    Some(font_id)
}
