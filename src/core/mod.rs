//! Core headless logic for the Typst Writer application.
//!
//! This module contains the business logic, compiler management, project state,
//! and the language server client. It is entirely decoupled from the GPUI views.

pub mod font;
pub mod compiler;
pub mod config;
pub mod editor;
pub mod renderer;
pub mod project;
pub mod lsp;
