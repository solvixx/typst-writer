use std::path::{Path, PathBuf};
use std::str::FromStr;
use url::Url;

/// Manages project-level state like working directory and main file.
pub struct ProjectManager {
    /// The root directory of the project.
    pub root_path: PathBuf,
    /// The path to the main Typst file.
    pub main_file_path: Option<PathBuf>,
}

impl ProjectManager {
    pub fn new(root_path: PathBuf) -> Self {
        let root_path = std::fs::canonicalize(&root_path).unwrap_or(root_path);
        Self {
            root_path,
            main_file_path: None,
        }
    }

    /// Sets the main file path and returns it canonicalized.
    pub fn set_main_file(&mut self, path: PathBuf) -> PathBuf {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        self.main_file_path = Some(path.clone());
        path
    }

    /// Returns the URL for a given path relative to the root.
    pub fn get_url(&self, path: &Path) -> Option<Url> {
        Url::from_str(&format!("file://{}", path.to_string_lossy())).ok()
    }
}
