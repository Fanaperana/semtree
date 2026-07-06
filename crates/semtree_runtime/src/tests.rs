use semtree_grammar::{parse_semtree_dsl, Grammar, Rule, RuleExpr};
use semtree_red::SyntaxNode;
use smol_str::SmolStr;

use crate::{IncrementalParser, RuntimeParser};
use crate::runtime_lexer::{RuntimeLexer, RuntimeTokenKind};

// ── Helper: build a simple expression language grammar ──────

fn expr_grammar() -> Grammar {
    let src = r#"
language expr

keyword let
keyword if
keyword else

Program :=
    Statement*

Statement :=
    LetStatement | ExpressionStatement

LetStatement :=
    "let" name: Identifier "=" Expression ";"

ExpressionStatement :=
    Expression ";"

Expression :=
    Identifier | Integer | BinaryExpr | ParenExpr

BinaryExpr :=
    Expression "+" Expression

ParenExpr :=
    "(" Expression ")"
"#;
    parse_semtree_dsl(src).unwrap()
}

fn simple_grammar() -> Grammar {
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

    // fn main ( ) { let x = 42 ; }
    assert_eq!(non_trivia.len(), 11);
    assert_eq!(non_trivia[1].text.as_str(), "main");
    assert_eq!(non_trivia[1].kind, RuntimeTokenKind::Ident);
    assert_eq!(non_trivia[8].kind, RuntimeTokenKind::Integer);
    assert_eq!(non_trivia[8].text.as_str(), "42");
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
    assert_eq!(strings[0].text.as_str(), r#""hello""#);

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
    // Missing semicolons and wrong tokens — should still produce a tree.
    let result = parser.parse("fn broken() { ??? }");

    let root = result.syntax();
    // Tree is always produced, even with errors.
    assert!(!root.text().is_empty());
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
    use crate::EditRegion;

    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let _r1 = inc.parse("fn main() { let x = 1; }");

    let r2 = inc.update(
        "fn main() { let x = 42; }",
        &[EditRegion::new(21, 22, "42")],
    );

    assert_eq!(r2.syntax().text(), "fn main() { let x = 42; }");
}

// ── Grammar from Tree-sitter Import ─────────────────────────

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
