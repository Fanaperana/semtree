use semtree_core::{SyntaxKind, Token};
use semtree_green::GreenNode;
use semtree_red::SyntaxNode;

use crate::event::Event;
use crate::grammar;
use crate::sink::Sink;

/// Result of a parse operation.
pub struct ParseResult {
    pub green_tree: GreenNode,
    pub errors: Vec<ParseError>,
}

impl ParseResult {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_tree.clone())
    }
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub range: text_size::TextRange,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error at {}..{}: {}",
            u32::from(self.range.start()),
            u32::from(self.range.end()),
            self.message
        )
    }
}

/// The main parser. Takes a token stream and produces a green tree via events.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    events: Vec<Event>,
    errors: Vec<ParseError>,
}

impl Parser {
    /// Parse a source string into a syntax tree (Rust-like language for MVP).
    pub fn parse(source: &str) -> ParseResult {
        let tokens = semtree_lexer::Lexer::tokenize(source);
        let mut parser = Self {
            tokens,
            pos: 0,
            events: Vec::new(),
            errors: Vec::new(),
        };

        grammar::source_file(&mut parser);

        let errors = parser.errors.clone();
        let green = Sink::new(&parser.tokens, parser.events).finish();

        ParseResult {
            green_tree: green,
            errors,
        }
    }

    // ── Token access ──────────────────────────────────────────

    pub(crate) fn current(&self) -> SyntaxKind {
        self.nth(0)
    }

    pub(crate) fn nth(&self, n: usize) -> SyntaxKind {
        self.nth_token(n)
            .map(|t| t.kind)
            .unwrap_or(SyntaxKind::EOF)
    }

    fn nth_token(&self, n: usize) -> Option<&Token> {
        let mut idx = self.pos;
        let mut count = 0;
        while idx < self.tokens.len() {
            if self.tokens[idx].kind == SyntaxKind::EOF || !self.tokens[idx].kind.is_trivia() {
                if count == n {
                    return Some(&self.tokens[idx]);
                }
                count += 1;
            }
            idx += 1;
        }
        None
    }

    pub(crate) fn current_text(&self) -> &str {
        self.nth_token(0).map(|t| t.text.as_str()).unwrap_or("")
    }

    pub(crate) fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    pub(crate) fn at_end(&self) -> bool {
        self.current() == SyntaxKind::EOF
    }

    // ── Event emission ────────────────────────────────────────

    pub(crate) fn start_node(&mut self, kind: SyntaxKind) -> usize {
        let pos = self.events.len();
        self.events.push(Event::StartNode {
            kind,
            forward_parent: None,
        });
        pos
    }

    pub(crate) fn start_node_before(&mut self, pos: usize, kind: SyntaxKind) {
        let new_pos = self.events.len();
        self.events.push(Event::StartNode {
            kind,
            forward_parent: None,
        });
        match &mut self.events[pos] {
            Event::StartNode { forward_parent, .. } => {
                *forward_parent = Some(new_pos - pos);
            }
            _ => panic!("expected StartNode at position {pos}"),
        }
    }

    pub(crate) fn finish_node(&mut self) {
        self.events.push(Event::FinishNode);
    }

    /// Consume the current token and add it to the tree.
    pub(crate) fn bump(&mut self) {
        self.events.push(Event::AddToken);
        self.pos += 1;
    }

    /// Expect the current token to be `kind`, consuming it. Emit an error if not.
    pub(crate) fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            self.error_here(&format!("expected {:?}", kind));
            false
        }
    }

    pub(crate) fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    // ── Error recovery ────────────────────────────────────────

    pub(crate) fn error_here(&mut self, message: &str) {
        let range = self
            .nth_token(0)
            .map(|t| t.range)
            .unwrap_or_else(|| {
                let pos = self.tokens.last().map(|t| u32::from(t.range.end())).unwrap_or(0);
                text_size::TextRange::new(
                    text_size::TextSize::new(pos),
                    text_size::TextSize::new(pos),
                )
            });
        self.errors.push(ParseError {
            message: message.to_string(),
            range,
        });
    }

    /// Error recovery: wrap the current token in an ERROR node and skip it.
    pub(crate) fn error_recover(&mut self, message: &str) {
        if self.at_end() {
            self.error_here(message);
            return;
        }
        self.error_here(message);
        let _m = self.start_node(SyntaxKind::ERROR);
        self.bump();
        self.finish_node();
    }

    /// Skip tokens until we find one in the recovery set or EOF.
    pub(crate) fn recover_to(&mut self, recovery: &[SyntaxKind]) {
        while !self.at_end() && !recovery.contains(&self.current()) {
            self.error_recover("unexpected token");
        }
    }
}
