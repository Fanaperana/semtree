use crate::{GreenNodeBuilder, NodeOrToken};
use semtree_core::SyntaxKind;

#[test]
fn build_simple_tree() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::SOURCE_FILE);
    builder.start_node(SyntaxKind::FUNCTION);
    builder.token(SyntaxKind::KW_FN, "fn");
    builder.token(SyntaxKind::WHITESPACE, " ");
    builder.token(SyntaxKind::IDENT, "main");
    builder.finish_node();
    builder.finish_node();

    let root = builder.finish();
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
    assert_eq!(root.text(), "fn main");
    assert_eq!(root.children_count(), 1);

    match &root.children()[0] {
        NodeOrToken::Node(func) => {
            assert_eq!(func.kind(), SyntaxKind::FUNCTION);
            assert_eq!(func.children_count(), 3);
        }
        _ => panic!("expected node"),
    }
}

#[test]
fn structural_sharing() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::SOURCE_FILE);
    builder.token(SyntaxKind::IDENT, "x");
    builder.finish_node();
    let tree1 = builder.finish();

    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::SOURCE_FILE);
    builder.token(SyntaxKind::IDENT, "x");
    builder.finish_node();
    let tree2 = builder.finish();

    assert_eq!(tree1, tree2);
}

#[test]
fn replace_child_creates_new_node() {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::SOURCE_FILE);
    builder.token(SyntaxKind::IDENT, "hello");
    builder.token(SyntaxKind::IDENT, "world");
    builder.finish_node();
    let root = builder.finish();

    use crate::GreenToken;
    let new_tok = NodeOrToken::Token(GreenToken::new(SyntaxKind::IDENT, "semtree".into()));
    let new_root = root.replace_child(1, new_tok);
    assert_eq!(new_root.text(), "hellosemtree");
    assert_eq!(root.text(), "helloworld");
}
