use semtree_core::SyntaxKind;
use crate::Parser;

#[test]
fn parse_empty_function() {
    let result = Parser::parse("fn main() {}");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    let root = result.syntax();
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);

    let func = root.first_child().expect("should have function");
    assert_eq!(func.kind(), SyntaxKind::FUNCTION);
}

#[test]
fn parse_function_with_params() {
    let result = Parser::parse("fn add(x: i32, y: i32) {}");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    let root = result.syntax();
    let func = root.first_child().unwrap();
    let params = func.child_node(SyntaxKind::PARAM_LIST).unwrap();
    let param_nodes = params.children();
    assert_eq!(param_nodes.len(), 2);
}

#[test]
fn parse_let_statement() {
    let result = Parser::parse("fn main() { let x = 42; }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn parse_binary_expression() {
    let result = Parser::parse("fn main() { let x = 1 + 2 * 3; }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn parse_struct() {
    let result = Parser::parse("struct Point { x: f64, y: f64 }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    let root = result.syntax();
    let st = root.first_child().unwrap();
    assert_eq!(st.kind(), SyntaxKind::STRUCT_DEF);
}

#[test]
fn parse_enum() {
    let result = Parser::parse("enum Color { Red, Green, Blue }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

    let root = result.syntax();
    let en = root.first_child().unwrap();
    assert_eq!(en.kind(), SyntaxKind::ENUM_DEF);
}

#[test]
fn error_recovery_produces_tree() {
    let result = Parser::parse("fn () {}");
    assert!(!result.errors.is_empty());
    // Tree should still be produced
    let root = result.syntax();
    assert_eq!(root.kind(), SyntaxKind::SOURCE_FILE);
}

#[test]
fn parse_if_expression() {
    let result = Parser::parse("fn main() { if x { y; } else { z; } }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn parse_while_loop() {
    let result = Parser::parse("fn main() { while x { y; } }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn parse_return_statement() {
    let result = Parser::parse("fn main() { return 42; }");
    assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
}

#[test]
fn roundtrip_text() {
    let source = "fn main() { let x = 1 + 2; }";
    let result = Parser::parse(source);
    let root = result.syntax();
    assert_eq!(root.text().replace(" ", "").len(), source.replace(" ", "").len());
}
