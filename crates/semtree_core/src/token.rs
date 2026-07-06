use crate::SyntaxKind;
use smol_str::SmolStr;
use text_size::TextRange;

/// A single lexical token produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: SyntaxKind,
    pub text: SmolStr,
    pub range: TextRange,
    pub leading_trivia: Vec<Trivia>,
    pub trailing_trivia: Vec<Trivia>,
}

impl Token {
    pub fn new(kind: SyntaxKind, text: SmolStr, range: TextRange) -> Self {
        Self {
            kind,
            text,
            range,
            leading_trivia: Vec::new(),
            trailing_trivia: Vec::new(),
        }
    }

    pub fn with_leading_trivia(mut self, trivia: Vec<Trivia>) -> Self {
        self.leading_trivia = trivia;
        self
    }

    pub fn with_trailing_trivia(mut self, trivia: Vec<Trivia>) -> Self {
        self.trailing_trivia = trivia;
        self
    }

    pub fn text_len(&self) -> text_size::TextSize {
        self.range.len()
    }
}

/// Trivia: whitespace, comments, and other non-semantic tokens attached to real tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub text: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriviaKind {
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
}

impl TriviaKind {
    pub fn to_syntax_kind(self) -> SyntaxKind {
        match self {
            TriviaKind::Whitespace => SyntaxKind::WHITESPACE,
            TriviaKind::Newline => SyntaxKind::NEWLINE,
            TriviaKind::LineComment => SyntaxKind::LINE_COMMENT,
            TriviaKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
        }
    }
}
