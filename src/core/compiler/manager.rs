use std::sync::Arc;
use std::ops::Range;
use gpui::{AsyncApp, Task, WeakEntity};
use typst::layout::PagedDocument;
use crate::core::compiler::SimpleWorld;

/// Manages background compilation of Typst documents.
pub struct CompilerManager {
    /// Whether a compilation is currently in progress.
    pub is_compiling: bool,
    /// Whether the document needs recompilation due to source changes during a build.
    pub needs_recompile: bool,
    /// Current version of the document (increments on every successful compile).
    pub document_version: usize,
    /// The last successfully compiled document.
    pub compiled_document: Option<Arc<PagedDocument>>,
    /// Error message from the last compilation, if any.
    pub error_message: Option<String>,
    /// Byte ranges of top-level structural Typst nodes.
    pub structural_ranges: Vec<Range<usize>>,
    /// Hash of the concatenated structural region bytes.
    pub structural_hash: u64,
    
    /// Handle to the active background compilation task.
    compilation_task: Option<Task<()>>,
}

impl CompilerManager {
    pub fn new() -> Self {
        Self {
            is_compiling: false,
            needs_recompile: false,
            document_version: 0,
            compiled_document: None,
            error_message: None,
            structural_ranges: Vec::new(),
            structural_hash: 0,
            compilation_task: None,
        }
    }

    /// Triggers a background compilation.
    pub fn compile<V: 'static>(
        &mut self, 
        world: SimpleWorld, 
        view_weak: WeakEntity<V>,
        cx: &mut gpui::Context<V>,
        on_success: impl FnOnce(&mut V, Arc<PagedDocument>, usize, &mut gpui::Context<V>) + 'static + Send,
    ) {
        if self.is_compiling {
            self.needs_recompile = true;
            return;
        }

        self.is_compiling = true;
        self.needs_recompile = false;

        let world_clone = world.clone();
        
        self.compilation_task = Some(cx.spawn(move |_, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let start = std::time::Instant::now();
                let result = typst::compile::<PagedDocument>(&world_clone);
                let _duration = start.elapsed();

                view_weak.update(&mut cx, |view, cx| {
                    match result.output {
                        Ok(doc) => {
                            let doc_arc = Arc::new(doc);
                            on_success(view, doc_arc, result.warnings.len(), cx);
                        }
                        Err(_diags) => {
                            // Diagnostics are handled by the caller or specialized callback
                        }
                    }
                }).ok();
            }
        }));
    }

    /// Updates the structural hash and re-extracts ranges if needed.
    pub fn update_structural_metadata(&mut self, text: &str) -> bool {
        let new_hash = hash_text_regions(text, &self.structural_ranges);
        if new_hash != self.structural_hash {
            self.structural_ranges = extract_structural_ranges(text);
            self.structural_hash = hash_text_regions(text, &self.structural_ranges);
            return true;
        }
        false
    }
}

/// Walk the top-level CST of `text` and collect byte ranges of every structural
/// node: set rules, show rules, imports, includes, top-level let bindings, and
/// `context` expressions.
pub fn extract_structural_ranges(text: &str) -> Vec<Range<usize>> {
    use typst::syntax::{parse, LinkedNode, SyntaxKind};
    let root = parse(text);
    let linked = LinkedNode::new(&root);
    let mut ranges = Vec::new();

    let mut stack = vec![linked];
    while let Some(node) = stack.pop() {
        for child in node.children() {
            stack.push(child);
        }

        match node.kind() {
            SyntaxKind::SetRule
            | SyntaxKind::ShowRule
            | SyntaxKind::ModuleImport
            | SyntaxKind::ModuleInclude
            | SyntaxKind::LetBinding
            | SyntaxKind::Contextual
            | SyntaxKind::Heading
            | SyntaxKind::Label => {
                ranges.push(node.range());
            }
            SyntaxKind::FuncCall => {
                if let Some(ident_node) = node.children().find(|c| c.kind() == SyntaxKind::Ident) {
                    let name = &text[ident_node.range()];
                    if name == "bibliography" || name == "figure" {
                        ranges.push(node.range());
                    }
                }
            }
            _ => {}
        }
    }
    ranges
}

/// Simple FNV-1a hash over the bytes of all structural regions.
pub fn hash_text_regions(text: &str, ranges: &[Range<usize>]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64  = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for range in ranges {
        if let Some(region) = text.get(range.clone()) {
            for byte in region.bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
    }
    hash
}
