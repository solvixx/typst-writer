use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use std::sync::Once;

static INIT: Once = Once::new();

pub fn register_typst() {
    INIT.call_once(|| {
        let typst_language: tree_sitter::Language = codebook_tree_sitter_typst::LANGUAGE.into();
        
        let typst_config = LanguageConfig::new(
            "typst",
            typst_language,
            vec![],
            include_str!("typst_highlights.scm"),
            include_str!("typst_injections.scm"),
            "",
        );

        LanguageRegistry::singleton().register("typst", &typst_config);
    });
}
