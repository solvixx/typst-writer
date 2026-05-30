//! Typst compilation integration and management.
//!
//! Provides the `SimpleWorld` implementation required by the Typst compiler
//! and the `CompilerManager` which handles background compilation tasks and 
//! diagnostic synchronization.

pub mod world;
pub mod manager;

pub use world::SimpleWorld;
pub use manager::CompilerManager;
