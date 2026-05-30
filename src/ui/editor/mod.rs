//! Source code editor integration.
//!
//! Provides the generic `SourceEditorView` which wraps a GPUI text input with
//! Typst-specific syntax highlighting and Language Server Protocol (LSP) features.

pub mod lsp;
pub mod typst_highlighter;
pub mod view;

pub use view::SourceEditorView;
