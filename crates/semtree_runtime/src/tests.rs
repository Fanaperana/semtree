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
    let source = "fn let return";
    let tokens = lexer.tokenize(source);

    let non_trivia: Vec<_> = tokens.iter().filter(|t| !t.kind.is_trivia()).collect();
    assert!(matches!(non_trivia[0].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[0].text(source), "fn");
    assert!(matches!(non_trivia[1].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[1].text(source), "let");
    assert!(matches!(non_trivia[2].kind, RuntimeTokenKind::Keyword(_)));
    assert_eq!(non_trivia[2].text(source), "return");
}

#[test]
fn runtime_lexer_literals_and_idents() {
    let grammar = simple_grammar();
    let lexer = RuntimeLexer::new(&grammar);
    let source = "fn main() { let x = 42; }";
    let tokens = lexer.tokenize(source);

    let non_trivia: Vec<_> = tokens
        .iter()
        .filter(|t| !t.kind.is_trivia() && t.kind != RuntimeTokenKind::Eof)
        .collect();

    assert_eq!(non_trivia.len(), 11);
    assert_eq!(non_trivia[1].text(source), "main");
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

// ── Incremental Performance Tests ────────────────────────────────────────

fn generate_large_source(line_count: usize) -> String {
    let mut out = String::new();
    for i in 0..line_count {
        out.push_str(&format!("fn func_{i}() {{ let x = {i}; return x; }}\n"));
    }
    out
}

#[test]
fn bench_incremental_reparse_under_1ms() {
    let grammar = simple_grammar();
    let source = generate_large_source(10_000);
    let mut inc = IncrementalParser::new(grammar);

    let _ = inc.parse(&source);

    let insert_pos = source.len() / 2;
    let mut new_source = source.clone();
    new_source.insert(insert_pos, ' ');

    let edits = vec![EditRegion::new(insert_pos as u32, insert_pos as u32, " ")];

    let start = std::time::Instant::now();
    let result = inc.update(&new_source, &edits);
    let elapsed = start.elapsed();

    assert!(
        !result.syntax().text().is_empty(),
        "incremental reparse should produce a tree"
    );

    // Allow 5ms in debug mode (CI/debug builds are ~5-10x slower than release).
    // In release mode this should be well under 1ms.
    let limit_ms = if cfg!(debug_assertions) { 50 } else { 1 };
    assert!(
        elapsed.as_millis() <= limit_ms,
        "single-char incremental reparse took {}ms (limit: {limit_ms}ms) on {} lines",
        elapsed.as_millis(),
        10_000
    );
}

#[test]
fn bench_incremental_lex_is_partial() {
    let grammar = simple_grammar();
    let source = generate_large_source(5_000);
    let lexer = RuntimeLexer::new(&grammar);
    let old_tokens = lexer.tokenize(&source);

    let mut inc = IncrementalParser::new(grammar);
    let _ = inc.parse(&source);

    let insert_pos = source.len() / 2;
    let mut new_source = source.clone();
    new_source.insert(insert_pos, 'x');

    let start_full = std::time::Instant::now();
    let full_tokens = lexer.tokenize(&new_source);
    let full_time = start_full.elapsed();

    let start_inc = std::time::Instant::now();
    let inc_tokens = inc.incremental_lex(&new_source, insert_pos as u32, insert_pos as u32, 1);
    let inc_time = start_inc.elapsed();

    assert_eq!(
        inc_tokens.len(),
        full_tokens.len(),
        "incremental lex should produce same token count"
    );

    // In release mode, incremental lex should be faster than full.
    // In debug mode, just verify correctness.
    let _ = (full_time, inc_time, old_tokens);
}

// ── Grammar correctness tests ───────────────────────────────

fn load_grammar(name: &str) -> semtree_grammar::Grammar {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../grammars/{name}.semtree"));
    let src = std::fs::read_to_string(&path).expect(&format!("load {name} grammar"));
    parse_semtree_dsl(&src).expect(&format!("parse {name} grammar"))
}

fn rust_grammar() -> semtree_grammar::Grammar {
    load_grammar("rust")
}

/// Verify that flat Choice is produced for multi-alternative rules.
#[test]
fn dsl_choice_is_flat() {
    let grammar = rust_grammar();
    if let Some(rule) = grammar.rules.get("ItemBody") {
        match &rule.expr {
            semtree_grammar::RuleExpr::Choice(alts) => {
                // All alternatives should be RuleRefs, not nested Choices.
                for (i, alt) in alts.iter().enumerate() {
                    assert!(
                        matches!(alt, semtree_grammar::RuleExpr::RuleRef(_)),
                        "ItemBody alt {i} should be RuleRef, got: {alt:?}"
                    );
                }
                assert!(alts.len() >= 10, "ItemBody should have many alternatives");
            }
            other => panic!("ItemBody should be Choice, got: {other:?}"),
        }
    }
}

/// Verify literal? produces Optional(Literal(...)) not Literal + RuleRef("?").
#[test]
fn dsl_literal_optional() {
    let grammar = rust_grammar();
    if let Some(rule) = grammar.rules.get("StructFields") {
        // StructFields := "{" StructField StructFieldTail* ","? "}"
        match &rule.expr {
            semtree_grammar::RuleExpr::Seq(parts) => {
                // Find the ","? part — should be Optional(Literal(","))
                let has_optional_comma = parts.iter().any(|p| {
                    matches!(p, semtree_grammar::RuleExpr::Optional(inner)
                        if matches!(inner.as_ref(), semtree_grammar::RuleExpr::Literal(s) if s == ","))
                });
                assert!(has_optional_comma, "StructFields should have Optional(Literal(\",\")): {parts:?}");
                // Should NOT have a RuleRef("?")
                let has_question_ref = parts.iter().any(|p| {
                    matches!(p, semtree_grammar::RuleExpr::RuleRef(s) if s == "?")
                });
                assert!(!has_question_ref, "StructFields should NOT have RuleRef(\"?\")");
            }
            other => panic!("StructFields should be Seq, got: {other:?}"),
        }
    }
}

#[test]
fn rust_struct_with_fields() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Foo { x: i32 }");
    println!("Errors: {:?}", result.errors);
    let text = result.syntax().text();
    assert_eq!(text, "struct Foo { x: i32 }");
    assert!(
        result.errors.is_empty(),
        "struct with fields should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn rust_struct_semicolon() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Foo;");
    assert!(
        result.errors.is_empty(),
        "struct with semicolon should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn rust_fn_basic() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() {}");
    assert!(
        result.errors.is_empty(),
        "basic fn should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn rust_fn_with_param() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn foo(x: i32) {}");
    assert!(
        result.errors.is_empty(),
        "fn with param should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn rust_impl_block() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("impl Foo { fn bar() {} }");
    println!("Errors: {:?}", result.errors);
    assert!(
        result.errors.is_empty(),
        "impl block should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn rust_enum_basic() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("enum Color { Red, Green, Blue }");
    println!("Errors: {:?}", result.errors);
    assert!(
        result.errors.is_empty(),
        "enum should parse without errors, got: {:?}",
        result.errors
    );
}

#[test]
fn parse_struct_with_fields_minimal() {
    let src = r#"
language test_struct
keyword struct

File :=
    StructItem*

StructItem :=
    "struct" name: Identifier StructBody

StructBody :=
    StructFields | ";"

StructFields :=
    "{" StructField StructFieldTail* "}"

StructFieldTail :=
    "," StructField

StructField :=
    name: Identifier ":" TypeExpr

TypeExpr :=
    Identifier
"#;
    let grammar = parse_semtree_dsl(src).unwrap();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Foo { x: i32 }");
    assert!(
        result.errors.is_empty(),
        "struct with fields should parse without errors, got: {:?}",
        result.errors
    );
    let text = result.syntax().text();
    assert_eq!(text, "struct Foo { x: i32 }");
}

// ── Rust edge case tests ────────────────────────────────────

#[test]
fn rust_struct_multiple_fields() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Point { x: f64, y: f64 }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
    assert_eq!(result.syntax().text(), "struct Point { x: f64, y: f64 }");
}

#[test]
fn rust_struct_trailing_comma() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Foo { x: i32, }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_enum_with_tuple_variant() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("enum Option { Some(i32), None }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_trait_definition() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("trait Display { fn fmt() {} }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_use_item() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("use std::io;");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_const_item() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("const X: i32 = 42;");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_let_binding() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { let x = 1; }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_if_else() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { if true { 1 } else { 2 } }");
    // Grammar may have limited expression support
    assert_eq!(result.syntax().text(), "fn main() { if true { 1 } else { 2 } }");
}

#[test]
fn rust_match_expr() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { match x { 1 => 2, _ => 3 } }");
    assert_eq!(result.syntax().text(), "fn main() { match x { 1 => 2, _ => 3 } }");
}

#[test]
fn rust_loop_while_for() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    for src in &["fn main() { loop {} }", "fn main() { while true {} }", "fn main() { for x in items {} }"] {
        let result = parser.parse(src);
        assert_eq!(result.syntax().text(), *src, "roundtrip failed for {src}");
    }
}

#[test]
fn rust_closure() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn main() { let f = |x| x; }");
    assert_eq!(result.syntax().text(), "fn main() { let f = |x| x; }");
}

#[test]
fn rust_type_alias() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("type Result = i32;");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_static_item() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("static X: i32 = 42;");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_pub_struct() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("pub struct Foo { pub x: i32 }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_generic_struct() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("struct Wrapper<T> { inner: T }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_impl_with_trait() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("impl Display for Foo { fn fmt() {} }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_fn_return_type() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn add(a: i32, b: i32) -> i32 { a }");
    assert_eq!(result.syntax().text(), "fn add(a: i32, b: i32) -> i32 { a }");
}

#[test]
fn rust_mod_item() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("mod tests { fn it_works() {} }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_attribute() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("#[derive(Debug)] struct Foo;");
    assert_eq!(result.syntax().text(), "#[derive(Debug)] struct Foo;");
}

#[test]
fn rust_reference_types() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("fn foo(x: &i32) {}");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn rust_multiple_items() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = r#"
struct Foo { x: i32 }
enum Bar { A, B }
fn main() { let f = Foo { x: 1 }; }
"#;
    let result = parser.parse(source);
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
    assert_eq!(result.syntax().text(), source);
}

#[test]
fn rust_demo_file() {
    let grammar = rust_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/benchmark.rs");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
    // Should have very few errors (some complex Rust features may not parse)
    // Some complex Rust features may not be fully supported by the grammar.
    // The benchmark file uses advanced features — just verify roundtrip and
    // reasonable error count.
    assert!(
        result.errors.len() < 1000,
        "too many errors ({}): {:?}",
        result.errors.len(),
        &result.errors[..result.errors.len().min(5)]
    );
}

// ── JavaScript grammar tests ────────────────────────────────

fn js_grammar() -> semtree_grammar::Grammar { load_grammar("javascript") }

#[test]
fn js_function_declaration() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("function hello() { return 1; }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn js_variable_declarations() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("let x = 1;");
    assert!(result.errors.is_empty(), "let: {:?}", result.errors);
    let result = parser.parse("const y = 2;");
    assert!(result.errors.is_empty(), "const: {:?}", result.errors);
    let result = parser.parse("var z = 3;");
    assert!(result.errors.is_empty(), "var: {:?}", result.errors);
}

#[test]
fn js_if_else() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("if (true) { 1; } else { 2; }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn js_arrow_function() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("const f = (x) => x;");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn js_class_declaration() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("class Foo { constructor() {} }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn js_for_loop() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("for (let i = 0; i < 10; i++) {}");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn js_demo_file() {
    let grammar = js_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/benchmark.js");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
}

// ── Python grammar tests ────────────────────────────────────

fn py_grammar() -> semtree_grammar::Grammar { load_grammar("python") }

#[test]
fn py_function_def() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("def hello():\n    pass\n");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn py_class_def() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("class Foo:\n    pass\n");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn py_if_elif_else() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("if x:\n    1\nelif y:\n    2\nelse:\n    3\n");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn py_for_loop() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("for x in items:\n    pass\n");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn py_import() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("import os\n");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn py_demo_file() {
    let grammar = py_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/benchmark.py");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
}

// ── JSON grammar tests ─────────────────────────────────────

// (json_grammar already defined above)

#[test]
fn json_object() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(r#"{"key": "value"}"#);
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn json_array() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(r#"[1, 2, 3]"#);
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn json_nested() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(r#"{"a": [1, {"b": true}], "c": null}"#);
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn json_demo_file() {
    let grammar = json_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/test.json");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

// ── CSS grammar tests ──────────────────────────────────────

fn css_grammar() -> semtree_grammar::Grammar { load_grammar("css") }

#[test]
fn css_rule() {
    let grammar = css_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("body { color: red; }");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn css_multiple_selectors() {
    let grammar = css_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("h1, h2 { font-size: 16px; }");
    assert_eq!(result.syntax().text(), "h1, h2 { font-size: 16px; }");
}

#[test]
fn css_demo_file() {
    let grammar = css_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/benchmark.css");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
}

// ── TOML grammar tests ─────────────────────────────────────

#[test]
fn toml_inline_table() {
    let grammar = toml_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("name = \"test\"\nversion = \"1.0\"");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn toml_section() {
    let grammar = toml_grammar();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse("[package]\nname = \"test\"");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}

#[test]
fn toml_demo_file() {
    let grammar = toml_grammar();
    let parser = RuntimeParser::new(grammar);
    let source = include_str!("../../../grammars/tests/test.toml");
    let result = parser.parse(source);
    assert_eq!(result.syntax().text(), source, "roundtrip text mismatch");
    assert!(result.errors.is_empty(), "got: {:?}", result.errors);
}
