//! Corpus tests: parse real sample files with shipped grammars.

use semtree_grammar::parse_semtree_dsl;
use semtree_runtime::RuntimeParser;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_grammar(name: &str) -> semtree_grammar::Grammar {
    let path = repo_root().join("grammars").join(format!("{name}.semtree"));
    let src = std::fs::read_to_string(&path).expect("grammar file");
    parse_semtree_dsl(&src).expect("grammar parse")
}

fn parse_file(grammar_name: &str, rel_path: &str) -> usize {
    let grammar = load_grammar(grammar_name);
    let path = repo_root().join(rel_path);
    let source = std::fs::read_to_string(&path).expect("source file");
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(&source);
    let root = result.syntax();
    assert!(!root.text().is_empty(), "empty tree for {}", path.display());
    result.errors.len()
}

#[test]
fn corpus_json_test_file() {
    let errors = parse_file("json", "grammars/tests/test.json");
    assert_eq!(errors, 0, "JSON test file should parse cleanly");
}

#[test]
fn corpus_json_roundtrip_lossless() {
    let grammar = load_grammar("json");
    let path = repo_root().join("grammars/tests/test.json");
    let source = std::fs::read_to_string(&path).unwrap();
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(&source);
    assert_eq!(result.syntax().text(), source);
}

#[test]
fn corpus_toml_demo() {
    let errors = parse_file("toml", "examples/demo.toml");
    assert!(
        errors <= 3,
        "TOML demo should have at most 3 errors (got {errors})"
    );
}

#[test]
fn corpus_python_demo() {
    let errors = parse_file("python", "examples/demo.py");
    assert!(
        errors <= 5,
        "Python demo should have at most 5 errors (got {errors})"
    );
}

#[test]
fn corpus_rust_demo() {
    let _errors = parse_file("rust", "examples/demo.rs");
}

#[test]
fn corpus_javascript_sample() {
    let grammar = load_grammar("javascript");
    let source = r#"function hello() { return 42; }"#;
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(source);
    assert!(!result.syntax().text().is_empty());
}

#[test]
fn corpus_css_sample() {
    let grammar = load_grammar("css");
    let source = ".foo { color: red; }";
    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(source);
    assert!(!result.syntax().text().is_empty());
}
