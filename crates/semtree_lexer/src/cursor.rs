/// A zero-copy cursor over a source string that tracks byte offset.
#[derive(Debug, Clone)]
pub struct Cursor<'src> {
    source: &'src str,
    pos: usize,
}

impl<'src> Cursor<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    pub fn remaining(&self) -> &'src str {
        &self.source[self.pos..]
    }

    /// Peek at the current character without advancing.
    pub fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Peek at the nth character ahead (0-indexed from current position).
    pub fn peek_at(&self, n: usize) -> Option<char> {
        self.remaining().chars().nth(n)
    }

    /// Advance by one character and return it.
    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Advance while the predicate holds, return the consumed slice.
    pub fn eat_while(&mut self, pred: impl Fn(char) -> bool) -> &'src str {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if !pred(c) {
                break;
            }
            self.pos += c.len_utf8();
        }
        &self.source[start..self.pos]
    }

    /// Check if the remaining input starts with the given string.
    pub fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    /// Advance by `n` bytes. Caller must ensure this lands on a char boundary.
    pub fn advance_by(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.source.len());
    }

    /// Extract a slice from `start` to the current position.
    pub fn slice_from(&self, start: usize) -> &'src str {
        &self.source[start..self.pos]
    }
}
