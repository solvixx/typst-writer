use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn default_max_log_lines() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub theme: String,
    pub font_size: f32,
    pub mono_font: String,
    pub ui_font: String,
    pub auto_compile: bool,
    pub custom_font_paths: Vec<String>,
    #[serde(default = "default_max_log_lines")]
    pub max_log_lines: usize,
    #[serde(default = "default_true")]
    pub sidebar_visible: bool,
    #[serde(default = "default_true")]
    pub log_panel_visible: bool,
    #[serde(default = "default_true")]
    pub source_code_visible: bool,
    #[serde(default = "default_true")]
    pub preview_panel_visible: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14.0,
            mono_font: {
                #[cfg(target_os = "macos")]
                {
                    "Menlo".to_string()
                }
                #[cfg(target_os = "windows")]
                {
                    "Consolas".to_string()
                }
                #[cfg(target_os = "linux")]
                {
                    "DejaVu Sans Mono".to_string()
                }
            },
            ui_font: {
                #[cfg(target_os = "macos")]
                {
                    "SF Pro".to_string()
                }
                #[cfg(target_os = "windows")]
                {
                    "Segoe UI".to_string()
                }
                #[cfg(target_os = "linux")]
                {
                    "Noto Sans".to_string()
                }
            },
            auto_compile: true,
            custom_font_paths: Vec::new(),
            max_log_lines: 1000,
            sidebar_visible: true,
            log_panel_visible: true,
            source_code_visible: true,
            preview_panel_visible: true,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_path() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "TypstWriter", "TypstWriter") {
            let config_dir = proj_dirs.config_dir();
            if !config_dir.exists() {
                let _ = fs::create_dir_all(config_dir);
            }
            Some(config_dir.join("config.json"))
        } else {
            None
        }
    }

    pub fn load() -> AppConfig {
        if let Some(path) = Self::config_path()
            && path.exists()
            && let Ok(contents) = fs::read_to_string(&path)
            && let Ok(config) = serde_json::from_str(&contents)
        {
            return config;
        }
        AppConfig::default()
    }

    pub fn save(config: &AppConfig) {
        if let Some(path) = Self::config_path()
            && let Ok(json) = serde_json::to_string_pretty(config)
        {
            let _ = fs::write(path, json);
        }
    }
}
