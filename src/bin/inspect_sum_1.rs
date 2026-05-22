use typst::syntax::{Source, SyntaxNode};
fn print_node(node: &SyntaxNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{:?} '{}'", indent, node.kind(), node.text());
    for child in node.children() {
        print_node(child, depth + 1);
    }
}
fn main() {
    let source = Source::detached("$sum^1$");
    let root = source.root();
    print_node(root, 0);
}
