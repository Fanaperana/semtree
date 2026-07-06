use super::*;
use semtree_core::SyntaxKind;

#[test]
fn lex_simple_function() {
    let tokens = Lexer::tokenize("fn main() {}");
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::KW_FN,
            SyntaxKind::IDENT,
            SyntaxKind::LPAREN,
            SyntaxKind::RPAREN,
            SyntaxKind::LBRACE,
            SyntaxKind::RBRACE,
            SyntaxKind::EOF,
        ]
    );
}

#[test]
fn lex_preserves_trivia() {
    let tokens = Lexer::tokenize("  fn  ");
    assert_eq!(tokens[0].kind, SyntaxKind::KW_FN);
    assert_eq!(tokens[0].leading_trivia.len(), 1);
    assert_eq!(tokens[0].trailing_trivia.len(), 1);
}

#[test]
fn lex_string_literal() {
    let tokens = Lexer::tokenize(r#""hello world""#);
    assert_eq!(tokens[0].kind, SyntaxKind::STRING_LIT);
    assert_eq!(tokens[0].text.as_str(), r#""hello world""#);
}

#[test]
fn lex_numbers() {
    let tokens = Lexer::tokenize("42 3.14 1_000");
    assert_eq!(tokens[0].kind, SyntaxKind::INT_LIT);
    assert_eq!(tokens[1].kind, SyntaxKind::FLOAT_LIT);
    assert_eq!(tokens[2].kind, SyntaxKind::INT_LIT);
}

#[test]
fn lex_operators() {
    let tokens = Lexer::tokenize("+ - * / == != <= >= && || -> =>");
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::PLUS,
            SyntaxKind::MINUS,
            SyntaxKind::STAR,
            SyntaxKind::SLASH,
            SyntaxKind::EQEQ,
            SyntaxKind::NEQ,
            SyntaxKind::LTEQ,
            SyntaxKind::GTEQ,
            SyntaxKind::AMPAMP,
            SyntaxKind::PIPEPIPE,
            SyntaxKind::ARROW,
            SyntaxKind::FAT_ARROW,
            SyntaxKind::EOF,
        ]
    );
}

#[test]
fn lex_line_comment() {
    let tokens = Lexer::tokenize("fn // comment\nmain");
    assert_eq!(tokens[0].kind, SyntaxKind::KW_FN);
    assert_eq!(tokens[0].trailing_trivia.len(), 2); // space + comment
    assert_eq!(tokens[1].kind, SyntaxKind::IDENT);
}

#[test]
fn lex_block_comment() {
    let tokens = Lexer::tokenize("/* nested /* comment */ */ fn");
    assert_eq!(tokens[0].kind, SyntaxKind::KW_FN);
    assert!(!tokens[0].leading_trivia.is_empty());
}

#[test]
fn lex_keywords() {
    let tokens = Lexer::tokenize("let mut struct enum impl trait pub use mod match");
    let kinds: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind != SyntaxKind::EOF)
        .map(|t| t.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::KW_LET,
            SyntaxKind::KW_MUT,
            SyntaxKind::KW_STRUCT,
            SyntaxKind::KW_ENUM,
            SyntaxKind::KW_IMPL,
            SyntaxKind::KW_TRAIT,
            SyntaxKind::KW_PUB,
            SyntaxKind::KW_USE,
            SyntaxKind::KW_MOD,
            SyntaxKind::KW_MATCH,
        ]
    );
}
