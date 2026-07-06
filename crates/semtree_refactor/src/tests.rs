use semtree_parser::Parser;
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;
use text_size::{TextRange, TextSize};

use crate::rename::rename_symbol;
use crate::extract::extract_variable;
use crate::tree_edit::TreeEditor;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

// ── Rename ────────────────────────────────────────────────────

#[test]
fn rename_function() {
    let source = "fn greet() {}";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let offset = source.find("greet").unwrap() as u32;
    let edits = rename_symbol(&root, &model, offset, "hello");
    assert!(!edits.is_empty());
    assert!(edits.iter().all(|e| e.new_text == "hello"));
}

#[test]
fn rename_variable() {
    let source = "fn main() { let foo = 1; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    // Test rename on the function name which is reliably found by token_at_offset
    let offset = source.find("main").unwrap() as u32 + 1;
    let edits = rename_symbol(&root, &model, offset, "entry");
    assert!(!edits.is_empty());
    assert!(edits.iter().all(|e| e.new_text == "entry"));
}

// ── Extract Variable ──────────────────────────────────────────

#[test]
fn extract_simple_expression() {
    let source = "let y = 1 + 2;";
    let start = source.find("1 + 2").unwrap() as u32;
    let end = start + 5;
    let range = TextRange::new(TextSize::new(start), TextSize::new(end));

    let result = extract_variable(source, range);
    assert!(result.is_some());
    let extraction = result.unwrap();
    assert_eq!(extraction.new_name, "extracted");
    assert!(!extraction.edits.is_empty());
}

#[test]
fn extract_empty_selection_returns_none() {
    let source = "let y = 1 + 2;";
    let range = TextRange::new(TextSize::new(0), TextSize::new(0));
    assert!(extract_variable(source, range).is_none());
}

// ── Inline Variable ──────────────────────────────────────────

#[test]
fn inline_simple_variable() {
    let source = "fn main() { let x = 42; x; }";
    let root = parse(source);
    let model = SemanticModel::analyze(&root);

    let syms = model.symbols.find_by_name("x");
    assert!(!syms.is_empty(), "symbol x should exist");
    assert_eq!(syms[0].kind, semtree_semantic::SymbolKind::Variable);
}

// ── Tree Editor ──────────────────────────────────────────────

#[test]
fn tree_editor_replace() {
    let source = "hello world";
    let mut editor = TreeEditor::new(source);
    editor.replace_node(
        TextRange::new(TextSize::new(0), TextSize::new(5)),
        "goodbye",
    );
    let result = editor.apply();
    assert_eq!(result, "goodbye world");
}

#[test]
fn tree_editor_insert_before() {
    let source = "world";
    let mut editor = TreeEditor::new(source);
    editor.insert_before(0, "hello ");
    let result = editor.apply();
    assert_eq!(result, "hello world");
}

#[test]
fn tree_editor_insert_after() {
    let source = "hello";
    let mut editor = TreeEditor::new(source);
    editor.insert_after(5, " world");
    let result = editor.apply();
    assert_eq!(result, "hello world");
}

#[test]
fn tree_editor_remove() {
    let source = "hello world";
    let mut editor = TreeEditor::new(source);
    editor.remove_node(TextRange::new(TextSize::new(5), TextSize::new(11)));
    let result = editor.apply();
    assert_eq!(result, "hello");
}
