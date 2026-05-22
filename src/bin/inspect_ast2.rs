use typst::syntax::{Source, LinkedNode};
fn main() {
    let source = Source::detached("$sum_(a)^(b) c$");
    let root = source.root();
    let linked = LinkedNode::new(root);
    let offset = 7; // after 'a'
    let leaf = linked.leaf_at(offset, typst::syntax::Side::Before).unwrap();
    println!("Leaf at {}: {:?} '{}'", offset, leaf.kind(), leaf.text());
    let mut current = Some(leaf.clone());
    while let Some(node) = current {
        println!("Node: {:?} '{}'", node.kind(), node.text());
        current = node.parent().cloned();
    }
}
