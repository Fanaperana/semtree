use semtree_core::SyntaxKind;
use semtree_parser::Parser;
use semtree_red::SyntaxNode;

use crate::engine::QueryEngine;
use crate::pattern::{PatternNode, QueryPattern, parse_query};

fn parse_rust(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

#[test]
fn find_all_functions() {
    let root = parse_rust("fn foo() {} fn bar() {} fn baz() {}");
    let results = QueryEngine::find_by_kind(&root, SyntaxKind::FUNCTION);
    assert_eq!(results.len(), 3);
}

#[test]
fn find_all_let_statements() {
    let root = parse_rust("fn main() { let x = 1; let y = 2; let z = 3; }");
    let results = QueryEngine::find_by_kind(&root, SyntaxKind::LET_STMT);
    assert_eq!(results.len(), 3);
}

#[test]
fn find_by_text() {
    let root = parse_rust("fn hello() {} fn world() {} fn hello_world() {}");
    let results = QueryEngine::find_by_text(&root, "hello");
    assert!(results.len() >= 2); // "hello" and "hello_world"
}

#[test]
fn find_identifiers() {
    let root = parse_rust("fn main() { let x = 42; }");
    let idents = QueryEngine::find_identifiers(&root);
    let names: Vec<_> = idents.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"x"));
}

#[test]
fn node_at_offset() {
    let root = parse_rust("fn main() { let x = 42; }");
    // Offset 3 should be inside "main"
    let node = QueryEngine::node_at_offset(&root, 3).unwrap();
    assert_eq!(node.kind(), SyntaxKind::FUNCTION);
}

#[test]
fn pattern_query_functions() {
    let root = parse_rust("fn alpha() {} fn beta() {}");

    let pattern = QueryPattern {
        nodes: vec![PatternNode::with_kind(SyntaxKind::FUNCTION).capture("func")],
    };

    let matches = QueryEngine::query(&root, &pattern);
    assert_eq!(matches.len(), 2);

    for m in &matches {
        assert!(m.get_capture("func").is_some());
    }
}

#[test]
fn pattern_query_by_kind_name() {
    let root = parse_rust("fn foo() {} struct Bar {} fn baz() {}");

    let pattern = QueryPattern {
        nodes: vec![PatternNode::with_kind_name("Function").capture("f")],
    };

    let matches = QueryEngine::query(&root, &pattern);
    assert_eq!(matches.len(), 2);

    let pattern2 = QueryPattern {
        nodes: vec![PatternNode::with_kind_name("StructDef").capture("s")],
    };
    let matches2 = QueryEngine::query(&root, &pattern2);
    assert_eq!(matches2.len(), 1);
}

#[test]
fn parse_sexp_query() {
    let query = parse_query("(Function @func)").unwrap();
    assert_eq!(query.nodes.len(), 1);
    assert_eq!(
        query.nodes[0].kind_name.as_deref(),
        Some("Function")
    );
    assert_eq!(query.nodes[0].capture.as_deref(), Some("func"));
}

#[test]
fn parse_nested_query() {
    let query = parse_query("(Function (Block) @body)").unwrap();
    assert_eq!(query.nodes.len(), 1);
    assert_eq!(query.nodes[0].children.len(), 1);
    assert_eq!(
        query.nodes[0].children[0].kind_name.as_deref(),
        Some("Block")
    );
}

#[test]
fn execute_parsed_query() {
    let root = parse_rust("fn main() { let x = 1; } fn other() {}");
    let query = parse_query("(Function @func)").unwrap();
    let matches = QueryEngine::query(&root, &query);
    assert_eq!(matches.len(), 2);

    for m in &matches {
        let text = m.capture_text("func").unwrap();
        assert!(text.contains("fn"));
    }
}

#[test]
fn find_structs_with_query() {
    let root = parse_rust("struct Point { x: f64, y: f64 } struct Color { r: u8 }");
    let query = parse_query("(StructDef @s)").unwrap();
    let matches = QueryEngine::query(&root, &query);
    assert_eq!(matches.len(), 2);
}
