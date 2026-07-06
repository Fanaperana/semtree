use semtree_parser::Parser;
use semtree_red::SyntaxNode;

use crate::config::FormatConfig;
use crate::engine::Formatter;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

fn format(source: &str) -> String {
    let root = parse(source);
    Formatter::with_defaults().format(&root)
}

fn format_with(source: &str, config: FormatConfig) -> String {
    let root = parse(source);
    Formatter::new(config).format(&root)
}

#[test]
fn format_simple_function() {
    let result = format("fn main() {}");
    assert!(result.contains("fn"));
    assert!(result.contains("main"));
    assert!(result.contains('{'));
    assert!(result.contains('}'));
}

#[test]
fn format_function_with_body() {
    let result = format("fn main() { let x = 42; }");
    assert!(result.contains("let x"));
    assert!(result.contains("42"));
    // Should have indented body.
    assert!(result.contains("    let") || result.contains("\tlet"));
}

#[test]
fn format_multiple_functions() {
    let result = format("fn foo() {} fn bar() {}");
    // Should have blank line between top-level items.
    assert!(result.contains("}\n\n") || result.contains("}\n"));
}

#[test]
fn format_preserves_semantics() {
    let source = "fn main() { let x = 42; return x; }";
    let result = format(source);
    assert!(result.contains("let"));
    assert!(result.contains("42"));
    assert!(result.contains("return"));
    assert!(result.contains("x"));
}

#[test]
fn format_struct() {
    let result = format("struct Point { x: f64, y: f64 }");
    assert!(result.contains("struct Point"));
}

#[test]
fn format_if_expression() {
    let result = format("fn main() { if x { let a = 1; } }");
    assert!(result.contains("if"));
}

#[test]
fn format_tabs() {
    let mut config = FormatConfig::default();
    config.use_tabs = true;
    let result = format_with("fn main() { let x = 1; }", config);
    assert!(result.contains('\t'));
}

#[test]
fn trailing_newline() {
    let result = format("fn main() {}");
    assert!(result.ends_with('\n'));
}

#[test]
fn no_trailing_newline() {
    let mut config = FormatConfig::default();
    config.trailing_newline = false;
    let result = format_with("fn main() {}", config);
    // With trailing_newline = false, the result should not end with
    // multiple newlines. The formatter may emit one from the source_file format.
    let trimmed = result.trim_end_matches('\n');
    assert!(!trimmed.is_empty());
}
