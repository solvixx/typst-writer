use std::fs;
use std::path::Path;
use std::process::Command;

pub fn provision_fonts() {
    if let Ok(home) = std::env::var("HOME") {
        let local_dir = Path::new(&home).join(".local/share/fonts/typst-writer");
        let classic_dir = Path::new(&home).join(".fonts/typst-writer");
        
        let _ = fs::create_dir_all(&local_dir);
        let _ = fs::create_dir_all(&classic_dir);

        let mut provisioned_any = false;
        // Save each embedded typst-assets font dynamically to both locations
        for (idx, font_bytes) in typst_assets::fonts().enumerate() {
            let file_name = format!("embedded_font_{}.otf", idx);
            let local_path = local_dir.join(&file_name);
            let classic_path = classic_dir.join(&file_name);

            if !local_path.exists() {
                if fs::write(&local_path, font_bytes).is_ok() {
                    provisioned_any = true;
                }
            }
            if !classic_path.exists() {
                if fs::write(&classic_path, font_bytes).is_ok() {
                    provisioned_any = true;
                }
            }
        }

        // Rebuild user's fontconfig cache instantly if new fonts were written
        if provisioned_any {
            let _ = Command::new("fc-cache")
                .arg("-f")
                .arg(Path::new(&home).join(".local/share/fonts"))
                .output();
            let _ = Command::new("fc-cache")
                .arg("-f")
                .arg(Path::new(&home).join(".fonts"))
                .output();
        }
    }
}
