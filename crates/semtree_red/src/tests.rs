use semtree_core::SyntaxKind;
use semtree_green::GreenNodeBuilder;
use crate::SyntaxNode;
use text_size::TextSize;

fn build_sample_tree() -> SyntaxNode {
    let mut b = GreenNodeBuilder::new();
    b.start_node(SyntaxKind::SOURCE_FILE);

    b.start_node(SyntaxKind::FUNCTION);
    b.token(SyntaxKind::KW_FN, "fn");
    b.token(SyntaxKind::WHITESPACE, " ");
    b.token(SyntaxKind::IDENT, "main");
    b.start_node(SyntaxKind::PARAM_LIST);
    b.token(SyntaxKind::LPAREN, "(");
    b.token(SyntaxKind::RPAREN, ")");
    b.finish_node();
    b.token(SyntaxKind::WHITESPACE, " ");
    b.start_node(SyntaxKind::BLOCK);
    b.token(SyntaxKind::LBRACE, "{");
    b.token(SyntaxKind::RBRACE, "}");
    b.finish_node();
    b.finish_node();

    b.finish_node();
    SyntaxNode::new_root(b.finish())
}

#[test]
fn red_tree_navigation() {
    let root = build_sample_tree();
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
    assert_eq!(root.text(), "fn main() {}");

    let func = root.first_child().expect("should have function");
    assert_eq!(func.kind(), SyntaxKind::FUNCTION);

    let fn_token = func.child_token(SyntaxKind::KW_FN).unwrap();
    assert_eq!(fn_token.text(), "fn");

    let ident = func.child_token(SyntaxKind::IDENT).unwrap();
    assert_eq!(ident.text(), "main");
}

#[test]
fn red_tree_parent_navigation() {
    let root = build_sample_tree();
    let func = root.first_child().unwrap();
    let params = func.child_node(SyntaxKind::PARAM_LIST).unwrap();

    let parent = params.parent().unwrap();
    assert_eq!(parent.kind(), SyntaxKind::FUNCTION);
}

#[test]
fn token_at_offset() {
    let root = build_sample_tree();
    let tok = root.token_at_offset(TextSize::new(0)).unwrap();
    assert_eq!(tok.text(), "fn");

    let tok = root.token_at_offset(TextSize::new(3)).unwrap();
    assert_eq!(tok.text(), "main");
}

#[test]
fn descendants_walk() {
    let root = build_sample_tree();
    let descs = root.descendants();
    assert!(descs.len() >= 3); // function, param_list, block
}
