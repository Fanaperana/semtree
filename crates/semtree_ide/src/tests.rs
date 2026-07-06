use semtree_parser::Parser;
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;

use crate::semantic_tokens::{classify_tokens, SemanticTokenType};
use crate::completion::{complete_at, CompletionKind};
use crate::navigation::{goto_definition, find_references, document_symbols, hover_info, breadcrumbs};
use crate::folding::folding_ranges;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

// ── Semantic Tokens ───────────────────────────────────────────

#[test]
fn classifies_keywords() {
    let root = parse("fn main() {}");
    let model = SemanticModel::analyze(&root);
    let tokens = classify_tokens(&root, &model);

    let kw_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == SemanticTokenType::Keyword)
        .collect();
    assert!(!kw_tokens.is_empty());
}

#[test]
fn classifies_identifiers() {
    let root = parse("fn hello() {}");
    let model = SemanticModel::analyze(&root);
    let tokens = classify_tokens(&root, &model);

    let fn_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == SemanticTokenType::Function)
        .collect();
    assert!(!fn_tokens.is_empty());
}

#[test]
fn classifies_numbers() {
    let root = parse("fn main() { let x = 42; }");
    let model = SemanticModel::analyze(&root);
    let tokens = classify_tokens(&root, &model);

    let num_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == SemanticTokenType::Number)
        .collect();
    assert!(!num_tokens.is_empty());
}

// ── Completion ────────────────────────────────────────────────

#[test]
fn completes_keywords() {
    let root = parse("fn main() {}");
    let model = SemanticModel::analyze(&root);
    let items = complete_at(&root, &model, 0);

    let kw_items: Vec<_> = items
        .iter()
        .filter(|i| i.kind == CompletionKind::Keyword)
        .collect();
    assert!(!kw_items.is_empty());
}

#[test]
fn completes_visible_symbols() {
    let source = "fn helper() {} fn main() { helper; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);
    // Complete at offset 0 (root scope) — helper should be visible
    let items = complete_at(&root, &model, 0);

    let fn_items: Vec<_> = items
        .iter()
        .filter(|i| i.kind == CompletionKind::Function)
        .collect();
    assert!(!fn_items.is_empty());
}

// ── Navigation ────────────────────────────────────────────────

#[test]
fn goto_definition_finds_function() {
    let source = "fn greet() {}";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    // Point inside the "greet" identifier at the definition
    let offset = source.find("greet").unwrap() as u32 + 1;
    let def = goto_definition(&root, &model, offset);
    assert!(def.is_some());
}

#[test]
fn find_references_finds_all() {
    let source = "fn main() { let xx = 1; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    // Use find_by_name to verify references exist in the model
    let syms = model.symbols.find_by_name("xx");
    assert!(!syms.is_empty());

    // Test find_references with the function name which is at top level
    let offset = source.find("main").unwrap() as u32 + 1;
    let refs = find_references(&root, &model, offset);
    assert!(refs.len() >= 1);
}

#[test]
fn document_symbols_lists_all() {
    let source = "fn a() {} struct B {} enum C {}";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);
    let syms = document_symbols(&root, &model);
    assert!(syms.len() >= 3);
}

#[test]
fn hover_shows_info() {
    let source = "fn main() {}";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let offset = source.find("main").unwrap() as u32;
    let info = hover_info(&root, &model, offset);
    assert!(info.is_some());
    assert_eq!(info.unwrap().name.as_str(), "main");
}

#[test]
fn breadcrumbs_at_nested() {
    let source = "fn outer() { let z = 1; }";
    let root = parse(source);
    // Point inside the block where "let z" is
    let offset = source.find("let z").unwrap() as u32 + 1;
    let crumbs = breadcrumbs(&root, offset);
    assert!(!crumbs.is_empty());
    assert_eq!(crumbs[0].name.as_str(), "outer");
}

// ── Folding ──────────────────────────────────────────────────

#[test]
fn folding_finds_functions() {
    let source = "fn a() { } fn b() { }";
    let root = parse(source);
    let ranges = folding_ranges(&root);
    assert!(ranges.len() >= 2);
}

#[test]
fn folding_finds_structs() {
    let source = "struct Foo { x: i32 }";
    let root = parse(source);
    let ranges = folding_ranges(&root);
    assert!(!ranges.is_empty());
}
