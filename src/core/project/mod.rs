//! Project-level state and file management.
//!
//! Handles the current working directory, tracks the main source file,
//! and provides file system watching capabilities.

pub mod manager;
pub mod model;

pub use manager::ProjectManager;
pub use model::Project;
