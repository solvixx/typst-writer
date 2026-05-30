//! Project-level state and file management.
//!
//! Handles the current working directory, tracks the main source file,
//! and provides file system watching capabilities.

pub mod model;
pub mod manager;

pub use model::Project;
pub use manager::ProjectManager;
