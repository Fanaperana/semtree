use semtree_parser::Parser;
use semtree_red::SyntaxNode;

use crate::builtins::*;
use crate::codegen::generate_ast;
use crate::typed::AstNode;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

#[test]
fn cast_source_file() {
    let root = parse("fn main() {}");
    let sf = SourceFile::cast(root).unwrap();
    let fns: Vec<_> = sf.functions().collect();
    assert_eq!(fns.len(), 1);
}

#[test]
fn function_name() {
    let root = parse("fn hello() {}");
    let sf = SourceFile::cast(root).unwrap();
    let func = sf.functions().next().unwrap();
    assert_eq!(func.name_text().unwrap(), "hello");
}

#[test]
fn function_body() {
    let root = parse("fn main() { let x = 1; }");
    let sf = SourceFile::cast(root).unwrap();
    let func = sf.functions().next().unwrap();
    let body = func.body().unwrap();
    let stmts = body.statements();
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::Let(_)));
}

#[test]
fn let_statement_details() {
    let root = parse("fn main() { let x = 42; }");
    let sf = SourceFile::cast(root).unwrap();
    let func = sf.functions().next().unwrap();
    let body = func.body().unwrap();
    let stmts = body.statements();
    if let Statement::Let(let_stmt) = &stmts[0] {
        assert_eq!(let_stmt.name_text().unwrap(), "x");
    } else {
        panic!("expected let statement");
    }
}

#[test]
fn multiple_functions() {
    let root = parse("fn foo() {} fn bar() {} fn baz() {}");
    let sf = SourceFile::cast(root).unwrap();
    let names: Vec<_> = sf.functions().filter_map(|f| f.name_text()).collect();
    assert_eq!(names, vec!["foo", "bar", "baz"]);
}

#[test]
fn struct_definition() {
    let root = parse("struct Point { x: f64, y: f64 }");
    let sf = SourceFile::cast(root).unwrap();
    let structs: Vec<_> = sf.structs().collect();
    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].name_text().unwrap(), "Point");
}

#[test]
fn enum_definition() {
    let root = parse("enum Color { Red, Green, Blue }");
    let sf = SourceFile::cast(root).unwrap();
    let enums: Vec<_> = sf.enums().collect();
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0].name_text().unwrap(), "Color");
}

#[test]
fn return_statement() {
    let root = parse("fn main() { return 42; }");
    let sf = SourceFile::cast(root).unwrap();
    let func = sf.functions().next().unwrap();
    let body = func.body().unwrap();
    let stmts = body.statements();
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Statement::Return(_)));
}

#[test]
fn if_expression() {
    let root = parse("fn main() { if x { let a = 1; } }");
    let sf = SourceFile::cast(root).unwrap();
    let func = sf.functions().next().unwrap();
    let body = func.body().unwrap();
    // The if expression is present in the tree.
    let descendants = body.syntax().descendants();
    let has_if = descendants
        .iter()
        .any(|n| n.kind() == semtree_core::SyntaxKind::IF_EXPR);
    assert!(has_if);
}

#[test]
fn codegen_produces_output() {
    let grammar_src = r#"
language test

keyword fn

Function :=
    "fn" name: Identifier "(" ")" "{" "}"
"#;
    let grammar = semtree_grammar::parse_semtree_dsl(grammar_src).unwrap();
    let code = generate_ast(&grammar);
    assert!(code.contains("pub struct Function"));
    assert!(code.contains("fn kind()"));
    assert!(code.contains("fn cast("));
    assert!(code.contains("fn name("));
}
