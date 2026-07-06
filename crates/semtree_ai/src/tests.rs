use semtree_parser::Parser;
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;
use text_size::TextRange;

use crate::api;
use crate::json_api;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

#[test]
fn find_symbol_by_name() {
    let root = parse("fn main() { let x = 1; }");
    let model = SemanticModel::analyze(&root);

    let results = api::find_symbol(&root, &model, "x");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "x");
    assert_eq!(results[0].kind, "variable");
}

#[test]
fn find_symbol_multiple() {
    let root = parse("fn foo() {} fn bar() {}");
    let model = SemanticModel::analyze(&root);

    let results = api::find_symbol(&root, &model, "foo");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, "function");
}

#[test]
fn find_symbol_not_found() {
    let root = parse("fn main() {}");
    let model = SemanticModel::analyze(&root);

    let results = api::find_symbol(&root, &model, "nonexistent");
    assert!(results.is_empty());
}

#[test]
fn rename_symbol_basic() {
    let source = "fn main() { let x = 1; return x; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let result = api::rename_symbol(source, &root, &model, "x", "y");
    assert!(result.contains("let y"));
    assert!(!result.contains("let x"));
}

#[test]
fn find_references_basic() {
    let source = "fn main() { let x = 1; return x; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let refs = api::find_references(&root, &model, "x");
    assert!(!refs.is_empty());
    assert!(refs.iter().all(|r| r.symbol_name == "x"));
}

#[test]
fn nearest_scope_at_offset() {
    let source = "fn main() { let x = 1; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let scope = api::nearest_scope(&root, &model, 15);
    assert!(scope.is_some());
}

#[test]
fn nearest_scope_out_of_range() {
    let source = "fn main() {}";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let scope = api::nearest_scope(&root, &model, 1000);
    assert!(scope.is_none());
}

#[test]
fn current_function_inside() {
    let source = "fn main() { let x = 1; }";
    let root = parse(source);

    let func = api::current_function(&root, 15);
    assert!(func.is_some());
    assert_eq!(func.unwrap().name, "main");
}

#[test]
fn current_function_outside() {
    let source = "fn main() {}";
    let root = parse(source);

    let func = api::current_function(&root, 1000);
    assert!(func.is_none());
}

#[test]
fn affected_nodes_basic() {
    let source = "fn main() { let x = 1; let y = 2; }";
    let root = parse(source);

    let range = TextRange::new(12.into(), 22.into());
    let affected = api::affected_nodes(&root, range);
    assert!(!affected.is_empty());
}

#[test]
fn diff_tree_identical() {
    let source = "fn main() {}";
    let root1 = parse(source);
    let root2 = parse(source);

    let diffs = api::diff_tree(&root1, &root2);
    assert!(diffs.is_empty());
}

#[test]
fn diff_tree_different() {
    let root1 = parse("fn main() {}");
    let root2 = parse("fn other() {}");

    let diffs = api::diff_tree(&root1, &root2);
    assert!(!diffs.is_empty());
}

#[test]
fn suggest_completion_basic() {
    let source = "fn main() { let x = 1; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let suggestions = api::suggest_completion(&root, &model, 15);
    assert!(!suggestions.is_empty());
    let labels: Vec<&str> = suggestions.iter().map(|s| s.label.as_str()).collect();
    assert!(labels.contains(&"main") || labels.contains(&"x"));
}

#[test]
fn json_execute_find_symbol() {
    let root = parse("fn main() { let x = 1; }");
    let model = SemanticModel::analyze(&root);

    let result = json_api::execute_command(&root, &model, "find_symbol {\"name\": \"x\"}");
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "x");
}

#[test]
fn json_execute_current_function() {
    let root = parse("fn main() { let x = 1; }");
    let model = SemanticModel::analyze(&root);

    let result = json_api::execute_command(&root, &model, "current_function {\"offset\": 15}");
    assert!(!result.is_null());
    assert_eq!(result["name"], "main");
}

#[test]
fn json_execute_unknown_command() {
    let root = parse("fn main() {}");
    let model = SemanticModel::analyze(&root);

    let result = json_api::execute_command(&root, &model, "bogus_command");
    assert!(result.get("error").is_some());
}

#[test]
fn json_execute_suggest_completion() {
    let root = parse("fn main() { let x = 1; }");
    let model = SemanticModel::analyze(&root);

    let result = json_api::execute_command(&root, &model, "suggest_completion {\"offset\": 20}");
    assert!(result.is_array());
}

#[test]
fn json_execute_nearest_scope() {
    let root = parse("fn main() { let x = 1; }");
    let model = SemanticModel::analyze(&root);

    let result = json_api::execute_command(&root, &model, "nearest_scope {\"offset\": 15}");
    assert!(!result.is_null());
}
