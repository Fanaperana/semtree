use semtree_grammar::parse_semtree_dsl;

use crate::{EditRegion, IncrementalParser, RuntimeParser, apply_edits};
use crate::runtime_lexer::{RuntimeLexer, RuntimeTokenKind};

fn simple_grammar() -> semtree_grammar::Grammar {
    let src = r#"
language simple

keyword fn
keyword let
keyword return

Function :=
    "fn" name: Identifier "(" ")" "{" Statement* "}"

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

// ── Runtime Lexer Tests ─────────────────────────────────────

#[test]
fn runtime_lexer_keywords() {
    let grammar = simple_grammar();
    let lexer = RuntimeLexer::new(&grammar);
    let tokens = lexer.tokenize("fn let return");

    let non_trivia: Vec<_> = tokens.iter().filter(|t| !t.kind.is_trivia()).collect();
    assert!(matches!(non_trivia[0].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[0].text.as_str(), "fn");
    assert!(matches!(non_trivia[1].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[1].text.as_str(), "let");
    assert!(matches!(non_trivia[2].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[2].text.as_str(), "return");
}

#[test]
fn runtime_lexer_literals_and_idents() {
    let grammar = simple_grammar();
    let lexer = RuntimeLexer::new(&grammar);
    let tokens = lexer.tokenize("fn main() { let x = 42; }");

    let non_trivia: Vec<_> = tokens
        .iter()
        .filter(|t| !t.kind.is_trivia() && t.kind != RuntimeTokenKind::Eof)
        .collect();

    assert_eq!(non_trivia.len(), 11);
    assert_eq!(non_trivia[1].text.as_str(), "main");
    assert_eq!(non_trivia[1].kind, RuntimeTokenKind::Ident);
    assert_eq!(non_trivia[8].kind, RuntimeTokenKind::Integer);
}

#[test]
fn runtime_lexer_strings_and_comments() {
    let grammar = simple_grammar();
    let lexer = RuntimeLexer::new(&grammar);
    let tokens = lexer.tokenize(r#"let x = "hello"; // comment"#);

    let strings: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::StringLit)
        .collect();
    assert_eq!(strings.len(), 1);

    let comments: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::LineComment)
        .collect();
    assert_eq!(comments.len(), 1);
}

// ── Runtime Parser Tests ────────────────────────────────────

#[test]
fn parse_simple_function() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { let x = 42; }");

    let root = result.syntax();
    assert_eq!(root.text(), "fn main() { let x = 42; }");
    assert!(!root.children().is_empty());
}

#[test]
fn parse_multiple_statements() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { let x = 1; let y = 2; return x; }");

    let root = result.syntax();
    assert_eq!(root.text(), "fn main() { let x = 1; let y = 2; return x; }");
}

#[test]
fn parse_with_string_literal() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(r#"fn greet() { let msg = "hello"; return msg; }"#);

    let root = result.syntax();
    assert!(root.text().contains("\"hello\""));
}

#[test]
fn parse_empty_function() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn empty() {}");

    let root = result.syntax();
    assert_eq!(root.text(), "fn empty() {}");
}

#[test]
fn parse_error_recovery() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn broken() { ??? }");

    let root = result.syntax();
    assert!(!root.text().is_empty());
    // Tree always produced even with errors.
    assert!(result.has_errors());
}

#[test]
fn lossless_roundtrip() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = "fn main() { let x = 42; return x; }";
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

// ── Error Recovery Tests ────────────────────────────────────

#[test]
fn error_recovery_missing_semicolon() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    // Missing semicolons — should still produce a tree with errors.
    let result = parser.parse("fn main() { let x = 1 let y = 2 }");
    let root = result.syntax();
    assert!(!root.text().is_empty());
}

#[test]
fn error_recovery_garbage_between_functions() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn a() {} @@@ fn b() {}");
    let root = result.syntax();
    assert!(root.text().contains("fn a"));
    assert!(root.text().contains("fn b"));
}

#[test]
fn error_recovery_unclosed_brace() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { let x = 1;");
    let root = result.syntax();
    assert!(root.text().contains("fn main"));
}

#[test]
fn error_recovery_deeply_nested() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    // Many nested issues — parser should not panic.
    let result = parser.parse("fn a() { fn b() { fn c() { } } }");
    let root = result.syntax();
    assert!(!root.text().is_empty());
}

// ── Incremental Parser Tests ────────────────────────────────

#[test]
fn incremental_parse_basic() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let r1 = inc.parse("fn main() { let x = 1; }");
    assert_eq!(r1.syntax().text(), "fn main() { let x = 1; }");

    let r2 = inc.parse("fn main() { let x = 2; }");
    assert_eq!(r2.syntax().text(), "fn main() { let x = 2; }");
}

#[test]
fn incremental_update_with_edit() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let _r1 = inc.parse("fn main() { let x = 1; }");

    let r2 = inc.update(
        "fn main() { let x = 42; }",
        &[EditRegion::new(21, 22, "42")],
    );

    assert_eq!(r2.syntax().text(), "fn main() { let x = 42; }");
}

#[test]
fn incremental_insert_at_end() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let _r1 = inc.parse("fn a() {}");
    let r2 = inc.update(
        "fn a() {} fn b() {}",
        &[EditRegion::new(9, 9, " fn b() {}")],
    );
    assert_eq!(r2.syntax().text(), "fn a() {} fn b() {}");
}

#[test]
fn incremental_delete() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let _r1 = inc.parse("fn a() { let x = 1; let y = 2; }");
    let r2 = inc.update(
        "fn a() { let y = 2; }",
        &[EditRegion::new(9, 20, "")],
    );
    assert_eq!(r2.syntax().text(), "fn a() { let y = 2; }");
}

#[test]
fn apply_edits_helper() {
    let source = "hello world";
    let result = apply_edits(source, &[EditRegion::new(5, 5, ",")]);
    assert_eq!(result, "hello, world");

    let result2 = apply_edits(source, &[EditRegion::new(0, 5, "goodbye")]);
    assert_eq!(result2, "goodbye world");
}

// ── Fuzz-like Robustness Tests ──────────────────────────────

#[test]
fn fuzz_empty_input() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("");
    let _root = result.syntax();
}

#[test]
fn fuzz_only_whitespace() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("   \n\t  \n  ");
    let root = result.syntax();
    assert_eq!(root.text(), "   \n\t  \n  ");
}

#[test]
fn fuzz_only_comments() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("// just a comment\n/* block */");
    let root = result.syntax();
    assert!(root.text().contains("// just a comment"));
}

#[test]
fn fuzz_random_punctuation() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("@#$%^&*!~`<>?,./");
    let _root = result.syntax();
    // No panic is success.
}

#[test]
fn fuzz_repeated_keywords() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn fn fn fn fn fn fn fn fn fn");
    let _root = result.syntax();
}

#[test]
fn fuzz_deeply_nested_parens() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let deep = "(".repeat(100) + &")".repeat(100);
    let result = parser.parse(&deep);
    let _root = result.syntax();
}

#[test]
fn fuzz_very_long_identifier() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let long_ident = "x".repeat(10_000);
    let source = format!("fn {}() {{}}", long_ident);
    let result = parser.parse(&source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

#[test]
fn fuzz_mixed_unicode() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn привет() { let 变量 = 42; }");
    let root = result.syntax();
    assert!(root.text().contains("привет"));
    assert!(root.text().contains("变量"));
}

#[test]
fn fuzz_null_bytes() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn a\0b() {}");
    let _root = result.syntax();
}

// ── Tree-sitter Import Integration ──────────────────────────

#[test]
fn parse_with_imported_grammar() {
    let json = r#"{
        "name": "calc",
        "rules": {
            "program": {
                "type": "REPEAT",
                "content": { "type": "SYMBOL", "name": "expression" }
            },
            "expression": {
                "type": "CHOICE",
                "members": [
                    { "type": "SYMBOL", "name": "number" },
                    { "type": "SYMBOL", "name": "identifier" },
                    {
                        "type": "SEQ",
                        "members": [
                            { "type": "STRING", "value": "(" },
                            { "type": "SYMBOL", "name": "expression" },
                            { "type": "STRING", "value": ")" }
                        ]
                    }
                ]
            },
            "number": {
                "type": "PATTERN",
                "value": "[0-9]+"
            },
            "identifier": {
                "type": "PATTERN",
                "value": "[a-zA-Z_]+"
            }
        }
    }"#;

    let grammar = semtree_ts_import::import_tree_sitter_grammar(json).unwrap();
    let parser = RuntimeParser::new(grammar);

    let result = parser.parse("42 x (y)");
    let root = result.syntax();
    assert_eq!(root.text(), "42 x (y)");
}
