use typst::syntax::{parse, LinkedNode, SyntaxKind};

fn main() {
    let text = "#figure(caption: [Hi])[Content] <my-fig>\n#bibliography(\"refs.bib\")";
    let root = parse(text);
    let linked = LinkedNode::new(&root);
    
    let mut stack = vec![linked];
    while let Some(node) = stack.pop() {
        for child in node.children() {
            stack.push(child);
        }
        
        if node.kind() == SyntaxKind::FuncCall {
            if let Some(ident) = node.children().find(|c| c.kind() == SyntaxKind::Ident) {
                let name = &text[ident.range()];
                println!("FuncCall ident: {:?}", name);
            }
        }
    }
}
