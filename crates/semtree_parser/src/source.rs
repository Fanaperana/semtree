use semtree_core::{SyntaxKind, Token};

/// A token source that feeds tokens to the parser, skipping trivia.
pub struct TokenSource {
    tokens: Vec<Token>,
    /// Index into non-trivia tokens.
    cursor: usize,
}

impl TokenSource {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    pub fn current_kind(&self) -> SyntaxKind {
        self.tokens[self.cursor].kind
    }

    pub fn peek_kind(&self) -> Option<SyntaxKind> {
        self.tokens.get(self.cursor + 1).map(|t| t.kind)
    }

    pub fn bump(&mut self) {
        if self.cursor < self.tokens.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn is_eof(&self) -> bool {
        self.current_kind() == SyntaxKind::EOF
    }

    pub fn all_tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn cursor_pos(&self) -> usize {
        self.cursor
    }
}
