use typst::syntax::Source;

fn main() {
    let source_code = "= Introduction\nThis is *bold* and _italic_ text.";
    let mut source = Source::detached(source_code);
    
    println!("--- BEFORE EDIT ---");
    println!("Text:\n{}", source.text());
    
    // Find where the word "bold" is and replace it with "super bold"
    let bold_text = "bold";
    if let Some(offset) = source.text().find(bold_text) {
        let range = offset..(offset + bold_text.len());
        println!("\nEditing range {:?} ('{}')", range, &source.text()[range.clone()]);
        
        // Incremental reparse via Source::edit
        source.edit(range, "super bold");
        
        println!("\n--- AFTER EDIT ---");
        println!("Text:\n{}", source.text());
        
        // Let's print the root tree after editing to verify incremental parsing worked
        println!("\n--- REPARSED SYNTAX TREE ---");
        print_tree(source.root(), 0);
    }
}

fn print_tree(node: &typst::syntax::SyntaxNode, indent: usize) {
    let indent_str = "  ".repeat(indent);
    println!(
        "{}{:?}: {:?}",
        indent_str,
        node.kind(),
        if node.text().len() > 30 {
            format!("{}...", &node.text()[..30])
        } else {
            node.text().to_string()
        }
    );
    for child in node.children() {
        print_tree(child, indent + 1);
    }
}
