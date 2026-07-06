use semtree_parser::Parser;
use semtree_red::SyntaxNode;

use crate::resolver::SemanticModel;
use crate::symbols::SymbolKind;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

#[test]
fn finds_function_symbols() {
    let root = parse("fn main() {} fn helper() {}");
    let model = SemanticModel::analyze(&root);

    let fns = model.symbols.find_by_kind(SymbolKind::Function);
    assert_eq!(fns.len(), 2);

    let names: Vec<_> = fns.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"main"));
    assert!(names.contains(&"helper"));
}

#[test]
fn finds_variable_symbols() {
    let root = parse("fn main() { let x = 1; let y = 2; }");
    let model = SemanticModel::analyze(&root);

    let vars = model.symbols.find_by_kind(SymbolKind::Variable);
    assert_eq!(vars.len(), 2);

    let names: Vec<_> = vars.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
}

#[test]
fn finds_struct_and_fields() {
    let root = parse("struct Point { x: f64, y: f64 }");
    let model = SemanticModel::analyze(&root);

    let structs = model.symbols.find_by_kind(SymbolKind::Struct);
    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].name.as_str(), "Point");

    let fields = model.symbols.find_by_kind(SymbolKind::Field);
    assert_eq!(fields.len(), 2);
}

#[test]
fn finds_enum_and_variants() {
    let root = parse("enum Color { Red, Green, Blue }");
    let model = SemanticModel::analyze(&root);

    let enums = model.symbols.find_by_kind(SymbolKind::Enum);
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0].name.as_str(), "Color");

    let variants = model.symbols.find_by_kind(SymbolKind::Variant);
    assert_eq!(variants.len(), 3);
}

#[test]
fn scope_tree_created() {
    let root = parse("fn main() { let x = 1; { let y = 2; } }");
    let model = SemanticModel::analyze(&root);

    // Root scope + function scope + block scope + inner block scope
    assert!(model.scopes.len() >= 3);
}

#[test]
fn function_parameters_registered() {
    let root = parse("fn add(x: i32, y: i32) { return x; }");
    let model = SemanticModel::analyze(&root);

    let params = model.symbols.find_by_kind(SymbolKind::Parameter);
    assert_eq!(params.len(), 2);

    let names: Vec<_> = params.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"x"));
    assert!(names.contains(&"y"));
}

#[test]
fn find_symbol_by_name() {
    let root = parse("fn main() { let foo = 42; }");
    let model = SemanticModel::analyze(&root);

    let found = model.symbols.find_by_name("foo");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, SymbolKind::Variable);
}

#[test]
fn symbol_table_len() {
    let root = parse("fn a() {} fn b() {} struct C {}");
    let model = SemanticModel::analyze(&root);
    assert_eq!(model.symbols.len(), 3);
}
