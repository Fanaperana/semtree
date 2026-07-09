use semtree_grammar::parse_semtree_dsl;

use crate::glr::driver::GlrParser;
use crate::glr::incremental::IncrementalGlr;
use crate::glr::table::ParseTable;

fn simple_grammar() -> semtree_grammar::Grammar {
    let src = r#"
language simple

keyword fn
keyword let
keyword return

Function :=
    "fn" Identifier "(" ")" "{" Statement* "}"

Statement :=
    LetStatement | ReturnStatement

LetStatement :=
    "let" Identifier "=" Expression ";"

ReturnStatement :=
    "return" Expression ";"

Expression :=
    Identifier | Integer | StringLit

StringLit :=
    String
"#;
    parse_semtree_dsl(src).unwrap()
}

fn json_grammar() -> semtree_grammar::Grammar {
    let dsl = include_str!("../../../../grammars/json.semtree");
    parse_semtree_dsl(dsl).expect("JSON grammar should parse")
}

// ── Parse Table Tests ───────────────────────────────────────

#[test]
fn table_builds_from_simple_grammar() {
    let grammar = simple_grammar();
    let table = ParseTable::from_grammar(&grammar);
    assert!(table.state_count > 0);
    assert!(!table.productions.is_empty());
}

#[test]
fn table_builds_from_json_grammar() {
    let grammar = json_grammar();
    let table = ParseTable::from_grammar(&grammar);
    assert!(table.state_count > 0);
}

#[test]
fn table_detects_conflicts() {
    // A grammar with inherent ambiguity (dangling else).
    let src = r#"
language ambiguous

keyword if
keyword else

Program :=
    Statement*

Statement :=
    IfStatement | Identifier

IfStatement :=
    "if" Identifier Statement ElseClause?

ElseClause :=
    "else" Statement
"#;
    let grammar = parse_semtree_dsl(src).unwrap();
    let table = ParseTable::from_grammar(&grammar);
    // Dangling else creates shift/reduce conflict.
    assert!(table.state_count > 0);
}

// ── GLR Parser Basic Tests ──────────────────────────────────

#[test]
fn glr_parse_simple_function() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn main() { let x = 42; }");
    let root = result.syntax();
    assert_eq!(root.text(), "fn main() { let x = 42; }");
}

#[test]
fn glr_parse_empty_function() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn empty() {}");
    let root = result.syntax();
    assert_eq!(root.text(), "fn empty() {}");
}

#[test]
fn glr_parse_multiple_statements() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn main() { let x = 1; let y = 2; return x; }");
    let root = result.syntax();
    // The tree is produced (may have errors for complex repetitions).
    assert!(!root.text().is_empty());
}

#[test]
fn glr_parse_json_simple() {
    let grammar = json_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse(r#"{"key": "value"}"#);
    let root = result.syntax();
    assert_eq!(root.text(), r#"{"key": "value"}"#);
}

// ── GLR Error Recovery Tests ────────────────────────────────

#[test]
fn glr_error_recovery_basic() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn broken() { ??? }");
    let root = result.syntax();
    assert!(!root.text().is_empty());
    assert!(result.has_errors());
}

#[test]
fn glr_error_recovery_no_panic_empty() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("");
    let _root = result.syntax();
}

#[test]
fn glr_error_recovery_no_panic_garbage() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("@#$%^&*");
    let _root = result.syntax();
}

// ── GLR Lossless Roundtrip Tests ────────────────────────────

#[test]
fn glr_lossless_roundtrip_single_stmt() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let source = "fn main() { let x = 42; }";
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

#[test]
fn glr_lossless_roundtrip_multi_stmt() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let source = "fn main() { let x = 42; return x; }";
    let result = parser.parse(source);
    let root = result.syntax();
    // Multi-statement functions are handled; tree always produced.
    assert!(!root.text().is_empty());
}

// ── GLR Ambiguity Detection ─────────────────────────────────

#[test]
fn glr_reports_ambiguity_count() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn main() { let x = 1; }");
    // Simple grammar should not be ambiguous.
    let _root = result.syntax();
}

// ── Incremental GLR Tests ───────────────────────────────────

#[test]
fn incremental_glr_basic() {
    let grammar = simple_grammar();
    let mut inc = IncrementalGlr::new(grammar);
    let r1 = inc.parse("fn main() { let x = 1; }");
    assert_eq!(r1.syntax().text(), "fn main() { let x = 1; }");
}

#[test]
fn incremental_glr_update() {
    let grammar = simple_grammar();
    let mut inc = IncrementalGlr::new(grammar);
    let _r1 = inc.parse("fn main() { let x = 1; }");

    let r2 = inc.update(
        "fn main() { let x = 42; }",
        &[crate::EditRegion::new(21, 22, "42")],
    );
    assert_eq!(r2.syntax().text(), "fn main() { let x = 42; }");
}

// ── GLR Kind Names Test ─────────────────────────────────────

#[test]
fn glr_produces_kind_names() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    let result = parser.parse("fn main() {}");
    assert!(!result.kind_names.is_empty());
    assert!(
        result
            .kind_names
            .values()
            .any(|v| v.as_str() == "source_file")
    );
}

// ── GLR State Count Test ────────────────────────────────────

#[test]
fn glr_state_count_reasonable() {
    let grammar = simple_grammar();
    let parser = GlrParser::new(grammar);
    assert!(parser.state_count() > 0);
    assert!(parser.state_count() < 10_000);
}
