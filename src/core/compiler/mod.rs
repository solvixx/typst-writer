//! Typst compilation integration and management.
//!
//! Provides the `SimpleWorld` implementation required by the Typst compiler
//! and the `CompilerManager` which handles background compilation tasks and
//! diagnostic synchronization.

pub mod manager;
pub mod world;

pub use manager::CompilerManager;
pub use world::SimpleWorld;
