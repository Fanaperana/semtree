use semtree_core::{SmolStr, SyntaxKind, Token, Trivia, TriviaKind};
use text_size::{TextRange, TextSize};

use crate::cursor::Cursor;

/// A Unicode-aware lexer that preserves trivia (whitespace, comments) attached
/// to each token.
pub struct Lexer<'src> {
    cursor: Cursor<'src>,
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            cursor: Cursor::new(source),
            source,
        }
    }

    /// Tokenize the entire source into a vector of tokens.
    pub fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == SyntaxKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Produce the next token, consuming leading and trailing trivia.
    pub fn next_token(&mut self) -> Token {
        let leading = self.eat_trivia();

        if self.cursor.is_eof() {
            let pos = self.cursor.pos() as u32;
            let range = TextRange::new(TextSize::new(pos), TextSize::new(pos));
            return Token::new(SyntaxKind::EOF, SmolStr::default(), range)
                .with_leading_trivia(leading);
        }

        let start = self.cursor.pos();
        let kind = self.scan_token();
        let end = self.cursor.pos();
        let text: SmolStr = self.source[start..end].into();
        let range = TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32));

        let trailing = self.eat_trailing_trivia();

        Token::new(kind, text, range)
            .with_leading_trivia(leading)
            .with_trailing_trivia(trailing)
    }

    fn eat_trivia(&mut self) -> Vec<Trivia> {
        let mut trivia = Vec::new();
        loop {
            match self.cursor.peek() {
                Some('\n') | Some('\r') => {
                    let start = self.cursor.pos();
                    self.cursor.bump();
                    if self.source.as_bytes().get(start) == Some(&b'\r')
                        && self.cursor.peek() == Some('\n')
                    {
                        self.cursor.bump();
                    }
                    let end = self.cursor.pos();
                    trivia.push(Trivia {
                        kind: TriviaKind::Newline,
                        text: self.source[start..end].into(),
                        range: TextRange::new(
                            TextSize::new(start as u32),
                            TextSize::new(end as u32),
                        ),
                    });
                }
                Some(c) if c.is_whitespace() => {
                    let start = self.cursor.pos();
                    self.cursor
                        .eat_while(|c| c.is_whitespace() && c != '\n' && c != '\r');
                    let end = self.cursor.pos();
                    trivia.push(Trivia {
                        kind: TriviaKind::Whitespace,
                        text: self.source[start..end].into(),
                        range: TextRange::new(
                            TextSize::new(start as u32),
                            TextSize::new(end as u32),
                        ),
                    });
                }
                Some('/') if self.cursor.peek_at(1) == Some('/') => {
                    let start = self.cursor.pos();
                    self.cursor.eat_while(|c| c != '\n');
                    let end = self.cursor.pos();
                    trivia.push(Trivia {
                        kind: TriviaKind::LineComment,
                        text: self.source[start..end].into(),
                        range: TextRange::new(
                            TextSize::new(start as u32),
                            TextSize::new(end as u32),
                        ),
                    });
                }
                Some('/') if self.cursor.peek_at(1) == Some('*') => {
                    let start = self.cursor.pos();
                    self.cursor.advance_by(2);
                    let mut depth = 1u32;
                    while depth > 0 && !self.cursor.is_eof() {
                        if self.cursor.starts_with("/*") {
                            depth += 1;
                            self.cursor.advance_by(2);
                        } else if self.cursor.starts_with("*/") {
                            depth -= 1;
                            self.cursor.advance_by(2);
                        } else {
                            self.cursor.bump();
                        }
                    }
                    let end = self.cursor.pos();
                    trivia.push(Trivia {
                        kind: TriviaKind::BlockComment,
                        text: self.source[start..end].into(),
                        range: TextRange::new(
                            TextSize::new(start as u32),
                            TextSize::new(end as u32),
                        ),
                    });
                }
                _ => break,
            }
        }
        trivia
    }

    /// Trailing trivia: whitespace on the same line after a token, up to (but not
    /// including) a newline.
    fn eat_trailing_trivia(&mut self) -> Vec<Trivia> {
        let mut trivia = Vec::new();
        if let Some(c) = self.cursor.peek()
            && c.is_whitespace()
            && c != '\n'
            && c != '\r'
        {
            let start = self.cursor.pos();
            self.cursor
                .eat_while(|c| c.is_whitespace() && c != '\n' && c != '\r');
            let end = self.cursor.pos();
            trivia.push(Trivia {
                kind: TriviaKind::Whitespace,
                text: self.source[start..end].into(),
                range: TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32)),
            });
        }
        if self.cursor.starts_with("//") {
            let start = self.cursor.pos();
            self.cursor.eat_while(|c| c != '\n');
            let end = self.cursor.pos();
            trivia.push(Trivia {
                kind: TriviaKind::LineComment,
                text: self.source[start..end].into(),
                range: TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32)),
            });
        }
        trivia
    }

    fn scan_token(&mut self) -> SyntaxKind {
        let c = match self.cursor.bump() {
            Some(c) => c,
            None => return SyntaxKind::EOF,
        };

        match c {
            '(' => SyntaxKind::LPAREN,
            ')' => SyntaxKind::RPAREN,
            '{' => SyntaxKind::LBRACE,
            '}' => SyntaxKind::RBRACE,
            '[' => SyntaxKind::LBRACKET,
            ']' => SyntaxKind::RBRACKET,
            ';' => SyntaxKind::SEMICOLON,
            ',' => SyntaxKind::COMMA,
            '#' => SyntaxKind::HASH,
            '@' => SyntaxKind::AT,
            '?' => SyntaxKind::QUESTION,
            '~' => SyntaxKind::TILDE,
            ':' => {
                if self.cursor.peek() == Some(':') {
                    self.cursor.bump();
                    SyntaxKind::COLONCOLON
                } else {
                    SyntaxKind::COLON
                }
            }
            '.' => {
                if self.cursor.peek() == Some('.') {
                    self.cursor.bump();
                    SyntaxKind::DOTDOT
                } else {
                    SyntaxKind::DOT
                }
            }
            '+' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::PLUSEQ
                } else {
                    SyntaxKind::PLUS
                }
            }
            '-' => {
                if self.cursor.peek() == Some('>') {
                    self.cursor.bump();
                    SyntaxKind::ARROW
                } else if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::MINUSEQ
                } else {
                    SyntaxKind::MINUS
                }
            }
            '*' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::STAREQ
                } else {
                    SyntaxKind::STAR
                }
            }
            '/' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::SLASHEQ
                } else {
                    SyntaxKind::SLASH
                }
            }
            '%' => SyntaxKind::PERCENT,
            '&' => {
                if self.cursor.peek() == Some('&') {
                    self.cursor.bump();
                    SyntaxKind::AMPAMP
                } else {
                    SyntaxKind::AMP
                }
            }
            '|' => {
                if self.cursor.peek() == Some('|') {
                    self.cursor.bump();
                    SyntaxKind::PIPEPIPE
                } else {
                    SyntaxKind::PIPE
                }
            }
            '^' => SyntaxKind::CARET,
            '!' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::NEQ
                } else {
                    SyntaxKind::BANG
                }
            }
            '=' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::EQEQ
                } else if self.cursor.peek() == Some('>') {
                    self.cursor.bump();
                    SyntaxKind::FAT_ARROW
                } else {
                    SyntaxKind::EQ
                }
            }
            '<' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::LTEQ
                } else if self.cursor.peek() == Some('<') {
                    self.cursor.bump();
                    SyntaxKind::SHL
                } else {
                    SyntaxKind::LT
                }
            }
            '>' => {
                if self.cursor.peek() == Some('=') {
                    self.cursor.bump();
                    SyntaxKind::GTEQ
                } else if self.cursor.peek() == Some('>') {
                    self.cursor.bump();
                    SyntaxKind::SHR
                } else {
                    SyntaxKind::GT
                }
            }
            '"' => self.scan_string(),
            '\'' => self.scan_char(),
            c if c.is_ascii_digit() => self.scan_number(),
            c if is_ident_start(c) => self.scan_ident_or_keyword(),
            _ => SyntaxKind::ERROR,
        }
    }

    fn scan_string(&mut self) -> SyntaxKind {
        loop {
            match self.cursor.bump() {
                Some('"') => return SyntaxKind::STRING_LIT,
                Some('\\') => {
                    self.cursor.bump(); // skip escaped char
                }
                None => return SyntaxKind::ERROR,
                _ => {}
            }
        }
    }

    fn scan_char(&mut self) -> SyntaxKind {
        match self.cursor.bump() {
            Some('\\') => {
                self.cursor.bump();
            }
            Some(_) => {}
            None => return SyntaxKind::ERROR,
        }
        if self.cursor.peek() == Some('\'') {
            self.cursor.bump();
            SyntaxKind::CHAR_LIT
        } else {
            SyntaxKind::ERROR
        }
    }

    fn scan_number(&mut self) -> SyntaxKind {
        self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
        if self.cursor.peek() == Some('.')
            && self.cursor.peek_at(1).is_some_and(|c| c.is_ascii_digit())
        {
            self.cursor.bump(); // consume '.'
            self.cursor.eat_while(|c| c.is_ascii_digit() || c == '_');
            if self.cursor.peek() == Some('e') || self.cursor.peek() == Some('E') {
                self.cursor.bump();
                if self.cursor.peek() == Some('+') || self.cursor.peek() == Some('-') {
                    self.cursor.bump();
                }
                self.cursor.eat_while(|c| c.is_ascii_digit());
            }
            SyntaxKind::FLOAT_LIT
        } else {
            SyntaxKind::INT_LIT
        }
    }

    fn scan_ident_or_keyword(&mut self) -> SyntaxKind {
        self.cursor.eat_while(is_ident_continue);

        let start_of_token = self.find_token_start();
        let text = &self.source[start_of_token..self.cursor.pos()];

        match text {
            "fn" => SyntaxKind::KW_FN,
            "let" => SyntaxKind::KW_LET,
            "mut" => SyntaxKind::KW_MUT,
            "if" => SyntaxKind::KW_IF,
            "else" => SyntaxKind::KW_ELSE,
            "while" => SyntaxKind::KW_WHILE,
            "for" => SyntaxKind::KW_FOR,
            "return" => SyntaxKind::KW_RETURN,
            "struct" => SyntaxKind::KW_STRUCT,
            "enum" => SyntaxKind::KW_ENUM,
            "impl" => SyntaxKind::KW_IMPL,
            "trait" => SyntaxKind::KW_TRAIT,
            "pub" => SyntaxKind::KW_PUB,
            "use" => SyntaxKind::KW_USE,
            "mod" => SyntaxKind::KW_MOD,
            "match" => SyntaxKind::KW_MATCH,
            "true" => SyntaxKind::KW_TRUE,
            "false" => SyntaxKind::KW_FALSE,
            "self" => SyntaxKind::KW_SELF,
            "super" => SyntaxKind::KW_SUPER,
            "as" => SyntaxKind::KW_AS,
            "in" => SyntaxKind::KW_IN,
            "const" => SyntaxKind::KW_CONST,
            "static" => SyntaxKind::KW_STATIC,
            "type" => SyntaxKind::KW_TYPE,
            "where" => SyntaxKind::KW_WHERE,
            "loop" => SyntaxKind::KW_LOOP,
            "break" => SyntaxKind::KW_BREAK,
            "continue" => SyntaxKind::KW_CONTINUE,
            _ => SyntaxKind::IDENT,
        }
    }

    /// Walk backwards from cursor to find the start of the current token text.
    fn find_token_start(&self) -> usize {
        let pos = self.cursor.pos();
        let bytes = self.source.as_bytes();
        let mut i = pos;
        while i > 0 {
            let prev = i - 1;
            if bytes[prev].is_ascii() {
                let c = bytes[prev] as char;
                if is_ident_continue(c) {
                    i = prev;
                } else {
                    break;
                }
            } else {
                let s = &self.source[..i];
                let c = s.chars().next_back().unwrap();
                if is_ident_continue(c) {
                    i -= c.len_utf8();
                } else {
                    break;
                }
            }
        }
        i
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}
