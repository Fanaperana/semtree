use semtree_grammar::parse_semtree_dsl;

use crate::runtime_lexer::{RuntimeLexer, RuntimeTokenKind};
use crate::{EditRegion, IncrementalParser, RuntimeParser, apply_edits};

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
    let r2 = inc.update("fn a() { let y = 2; }", &[EditRegion::new(9, 20, "")]);
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

// ── JSON Grammar Integration ────────────────────────────────

fn json_grammar() -> semtree_grammar::Grammar {
    let dsl = include_str!("../../../grammars/json.semtree");
    parse_semtree_dsl(dsl).expect("JSON grammar should parse")
}

#[test]
fn json_grammar_loads() {
    let grammar = json_grammar();
    assert_eq!(grammar.name.as_str(), "json");
    assert!(grammar.rules.contains_key("Document"));
    assert!(grammar.rules.contains_key("Value"));
    assert!(grammar.rules.contains_key("Object"));
    assert!(grammar.rules.contains_key("Array"));
    assert!(grammar.rules.contains_key("Pair"));
}

#[test]
fn json_parse_roundtrip() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/test.json");
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

#[test]
fn json_parse_simple_object() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = r#"{"key": "value"}"#;
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

#[test]
fn json_parse_array() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = r#"[1, 2, 3]"#;
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

// ── TOML Grammar Integration ────────────────────────────────

fn toml_grammar() -> semtree_grammar::Grammar {
    let dsl = include_str!("../../../grammars/toml.semtree");
    parse_semtree_dsl(dsl).expect("TOML grammar should parse")
}

#[test]
fn toml_grammar_loads() {
    let grammar = toml_grammar();
    assert_eq!(grammar.name.as_str(), "toml");
    assert!(grammar.rules.contains_key("Document"));
    assert!(grammar.rules.contains_key("Table"));
    assert!(grammar.rules.contains_key("KeyValue"));
    assert!(grammar.rules.contains_key("Key"));
    assert!(grammar.rules.contains_key("Value"));
}

#[test]
fn toml_parse_roundtrip() {
    let grammar = toml_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/test.toml");
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
}

#[test]
fn toml_parse_simple_kv() {
    let grammar = toml_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = "name = \"test\"";
    let result = parser.parse(source);
    let root = result.syntax();
    assert_eq!(root.text(), source);
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

// ── Edge Case Tests ──────────────────────────────────────────────────────

#[test]
fn edge_custom_token_regex_matches_before_ident() {
    let g = parse_semtree_dsl(
        r#"
language test
token Arrow := /->/
token FatArrow := /=>/
Rule := Arrow | FatArrow | Identifier
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("-> => foo");
    let custom_count = toks
        .iter()
        .filter(|t| matches!(t.kind, RuntimeTokenKind::Custom(_)))
        .count();
    assert_eq!(custom_count, 2, "both -> and => should be Custom tokens");
}

#[test]
fn edge_custom_token_literal() {
    let g = parse_semtree_dsl(
        r#"
language test
token Ellipsis := "..."
Rule := Ellipsis | Identifier
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("...");
    assert!(
        toks.iter()
            .any(|t| matches!(t.kind, RuntimeTokenKind::Custom(_))),
        "... should match as Custom token"
    );
}

#[test]
fn edge_indent_empty_source() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
Rule := Identifier
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("");
    assert_eq!(toks.len(), 1);
    assert_eq!(toks[0].kind, RuntimeTokenKind::Eof);
}

#[test]
fn edge_indent_flat_no_indent() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
keyword x
Rule := "x"
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("x\nx\n");
    let indent_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Indent)
        .count();
    let dedent_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Dedent)
        .count();
    assert_eq!(indent_count, 0, "no indentation changes expected");
    assert_eq!(dedent_count, 0, "no dedents expected");
}

#[test]
fn edge_indent_nested_three_levels() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
keyword a
keyword b
keyword c
Rule := "a" | "b" | "c"
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("a\n  b\n    c\n");
    let indent_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Indent)
        .count();
    let dedent_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Dedent)
        .count();
    assert_eq!(indent_count, 2, "two levels of indent expected");
    assert_eq!(dedent_count, 2, "should close all indent levels at EOF");
}

#[test]
fn edge_indent_dedent_multiple_at_once() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
keyword a
keyword b
keyword c
keyword d
Rule := "a" | "b" | "c" | "d"
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("a\n  b\n    c\nd\n");
    let dedent_before_d = toks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == RuntimeTokenKind::Dedent)
        .count();
    assert!(
        dedent_before_d >= 2,
        "should emit 2 dedents when jumping from indent 4 to indent 0"
    );
}

#[test]
fn edge_incremental_insert_at_start() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let source = "fn foo() { return 1; }";
    let _ = inc.parse(source);

    let new_source = "// comment\nfn foo() { return 1; }";
    let edits = vec![EditRegion::new(0, 0, "// comment\n")];
    let result = inc.update(new_source, &edits);
    assert_eq!(result.syntax().text(), new_source);
}

#[test]
fn edge_incremental_delete_middle() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let source = "fn foo() { let x = 42; return x; }";
    let _ = inc.parse(source);

    let new_source = "fn foo() { return x; }";
    let edits = vec![EditRegion::new(11, 23, "")];
    let result = inc.update(&new_source, &edits);
    assert_eq!(result.syntax().text(), new_source);
}

#[test]
fn edge_incremental_replace_token() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);

    let source = "fn foo() { return 42; }";
    let _ = inc.parse(source);

    let new_source = "fn foo() { return 99; }";
    let edits = vec![EditRegion::new(18, 20, "99")];
    let result = inc.update(&new_source, &edits);
    assert_eq!(result.syntax().text(), new_source);
}

#[test]
fn edge_glr_simple_parse() {
    let grammar = simple_grammar();
    let parser = crate::GlrParser::new(grammar);
    let result = parser.parse("fn foo() { return 1; }");
    assert!(
        !result.syntax().text().is_empty(),
        "GLR should produce non-empty tree"
    );
}

#[test]
fn edge_glr_empty_input() {
    let grammar = simple_grammar();
    let parser = crate::GlrParser::new(grammar);
    let result = parser.parse("");
    assert_eq!(result.errors.len(), 0);
}

#[test]
fn edge_dsl_token_def_parsing() {
    let g = parse_semtree_dsl(
        r#"
language test
token HexNumber := /0x[0-9a-fA-F]+/
token DotDot := ".."
keyword let
Rule := "let" Identifier "=" HexNumber
"#,
    )
    .unwrap();
    assert_eq!(g.tokens.len(), 2);
    assert_eq!(g.tokens[0].name.as_str(), "HexNumber");
    assert!(g.tokens[0].is_regex);
    assert_eq!(g.tokens[1].name.as_str(), "DotDot");
    assert!(!g.tokens[1].is_regex);
}

#[test]
fn edge_dsl_indent_sensitive_flag() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
Rule := Identifier
"#,
    )
    .unwrap();
    assert!(g.indent_sensitive);
}

#[test]
fn edge_dsl_no_indent_sensitive_by_default() {
    let g = parse_semtree_dsl(
        r#"
language test
Rule := Identifier
"#,
    )
    .unwrap();
    assert!(!g.indent_sensitive);
}

#[test]
fn edge_parse_python_style_indent() {
    let g = parse_semtree_dsl(
        r#"
language mini_py
indent-sensitive
keyword def
keyword return
Function := "def" name: Identifier "(" ")" ":" Body
Body := INDENT Statement+ DEDENT
Statement := ReturnStmt
ReturnStmt := "return" Identifier
"#,
    )
    .unwrap();
    let parser = RuntimeParser::new(g);
    let result = parser.parse("def foo():\n    return x\n");
    let root = result.syntax();
    assert!(!root.text().is_empty());
    assert!(
        result.errors.len() <= 3,
        "simple indented function should parse with minimal errors (got {})",
        result.errors.len()
    );
}

#[test]
fn edge_lexer_single_quote_strings() {
    let g = parse_semtree_dsl(
        r#"
language test
Rule := String
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("'hello'");
    assert!(
        toks.iter().any(|t| t.kind == RuntimeTokenKind::StringLit),
        "single-quoted string should be recognized"
    );
}

#[test]
fn edge_lexer_escaped_string() {
    let g = parse_semtree_dsl(
        r#"
language test
Rule := String
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize(r#""hello \"world\"""#);
    let string_toks: Vec<_> = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::StringLit)
        .collect();
    assert_eq!(string_toks.len(), 1, "escaped string should be one token");
}

#[test]
fn edge_lexer_unicode_identifiers() {
    let g = parse_semtree_dsl(
        r#"
language test
Rule := Identifier
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("café naïve über");
    let ident_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Ident)
        .count();
    assert_eq!(ident_count, 3, "unicode identifiers should be recognized");
}

#[test]
fn edge_incremental_no_edits_returns_same_tree() {
    let grammar = simple_grammar();
    let mut inc = IncrementalParser::new(grammar);
    let source = "fn foo() { return 1; }";
    let first = inc.parse(source);
    let second = inc.update(source, &[]);
    assert_eq!(first.syntax().text(), second.syntax().text());
}

#[test]
fn edge_error_recovery_only_garbage() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("@@@@####$$$$");
    let root = result.syntax();
    assert!(!root.text().is_empty(), "should still produce a tree");
    assert!(!result.errors.is_empty(), "should have parse errors");
}

#[test]
fn edge_error_recovery_truncated_function() {
    let grammar = simple_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn foo(");
    let root = result.syntax();
    assert!(!root.text().is_empty(), "partial input should still parse");
}

#[test]
fn edge_apply_edits_ordering() {
    let source = "abcdef";
    let edits = vec![EditRegion::new(4, 6, "XY"), EditRegion::new(0, 2, "ZZ")];
    let result = apply_edits(source, &edits);
    assert_eq!(result, "ZZcdXY");
}

#[test]
fn edge_lexer_hash_comment_python() {
    let g = parse_semtree_dsl(
        r#"
language test
indent-sensitive
Rule := Identifier
"#,
    )
    .unwrap();
    let lexer = RuntimeLexer::new(&g);
    let toks = lexer.tokenize("# this is a comment\nx");
    let ident_count = toks
        .iter()
        .filter(|t| t.kind == RuntimeTokenKind::Ident)
        .count();
    assert!(ident_count >= 1, "x should still be recognized as ident");
}
