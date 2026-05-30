use crate::core::compiler::SimpleWorld;
use gpui::{AsyncApp, Task, WeakEntity};
use std::ops::Range;
use std::sync::Arc;
use typst::layout::PagedDocument;

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

pub struct CompilationMetadata {
    pub doc: Arc<PagedDocument>,
    pub warnings: usize,
    pub structural_ranges: Vec<Range<usize>>,
    pub structural_hash: u64,
}

impl Default for CompilerManager {
    fn default() -> Self {
        Self::new()
    }
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
        on_success: impl FnOnce(&mut V, CompilationMetadata, &mut gpui::Context<V>)
        + 'static
        + Send,
    ) {
        if self.is_compiling {
            self.needs_recompile = true;
            return;
        }

        self.is_compiling = true;
        self.needs_recompile = false;

        let world_clone = world.clone();
        let old_structural_ranges = self.structural_ranges.clone();
        let old_structural_hash = self.structural_hash;

        self.compilation_task = Some(cx.spawn(move |_, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = typst::compile::<PagedDocument>(&world_clone);
                
                // BACKGROUND: Extract structural metadata while still on the background thread
                let (structural_ranges, structural_hash) = if result.output.is_ok() {
                    let text = world_clone.main_source.text();
                    let mut ranges = old_structural_ranges;
                    let mut hash = old_structural_hash;
                    
                    let new_hash = hash_text_regions(text, &ranges);
                    if new_hash != hash {
                        ranges = extract_structural_ranges(text);
                        hash = hash_text_regions(text, &ranges);
                    }
                    (ranges, hash)
                } else {
                    (old_structural_ranges, old_structural_hash)
                };

                view_weak
                    .update(&mut cx, |view, cx| {
                        match result.output {
                            Ok(doc) => {
                                let metadata = CompilationMetadata {
                                    doc: Arc::new(doc),
                                    warnings: result.warnings.len(),
                                    structural_ranges,
                                    structural_hash,
                                };
                                on_success(view, metadata, cx);
                            }
                            Err(_diags) => {
                                // Diagnostics are handled by the caller or specialized callback
                            }
                        }
                    })
                    .ok();
            }
        }));
    }

    /// Updates the structural hash and re-extracts ranges if needed.
    /// Returns true if updated.
    pub fn apply_structural_metadata(&mut self, ranges: Vec<Range<usize>>, hash: u64) -> bool {
        if hash != self.structural_hash {
            self.structural_ranges = ranges;
            self.structural_hash = hash;
            return true;
        }
        false
    }

    /// Updates the structural hash and re-extracts ranges if needed from the provided text.
    pub fn update_structural_metadata(&mut self, text: &str) {
        let current_hash = hash_text_regions(text, &self.structural_ranges);
        if current_hash != self.structural_hash {
            self.structural_ranges = extract_structural_ranges(text);
            self.structural_hash = hash_text_regions(text, &self.structural_ranges);
        }
    }
}

/// Walk the top-level CST of `text` and collect byte ranges of every structural
/// node: set rules, show rules, imports, includes, top-level let bindings, and
/// `context` expressions.
pub fn extract_structural_ranges(text: &str) -> Vec<Range<usize>> {
    use typst::syntax::{LinkedNode, SyntaxKind, parse};
    let root = parse(text);
    let linked = LinkedNode::new(&root);
    let mut ranges = Vec::new();

    let mut stack = vec![linked];
    while let Some(node) = stack.pop() {
        match node.kind() {
            SyntaxKind::Text
            | SyntaxKind::Space
            | SyntaxKind::Parbreak
            | SyntaxKind::Linebreak
            | SyntaxKind::Escape
            | SyntaxKind::Shorthand
            | SyntaxKind::SmartQuote
            | SyntaxKind::Math
            | SyntaxKind::Equation
            | SyntaxKind::LineComment
            | SyntaxKind::BlockComment
            | SyntaxKind::Raw => {
                // Prune non-structural text, math, raw, space, and comment subtrees
                continue;
            }
            SyntaxKind::SetRule
            | SyntaxKind::ShowRule
            | SyntaxKind::ModuleImport
            | SyntaxKind::ModuleInclude
            | SyntaxKind::LetBinding
            | SyntaxKind::Contextual
            | SyntaxKind::Heading
            | SyntaxKind::Label => {
                ranges.push(node.range());
                // Do not traverse children of structural nodes either
                continue;
            }
            SyntaxKind::FuncCall => {
                if let Some(ident_node) = node.children().find(|c| c.kind() == SyntaxKind::Ident) {
                    let name = &text[ident_node.range()];
                    if name == "bibliography" || name == "figure" {
                        ranges.push(node.range());
                        continue;
                    }
                }
            }
            _ => {}
        }

        for child in node.children() {
            stack.push(child);
        }
    }
    ranges
}

/// Simple FNV-1a hash over the bytes of all structural regions.
pub fn hash_text_regions(text: &str, ranges: &[Range<usize>]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
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
