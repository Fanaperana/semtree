use rustc_hash::FxHashMap;
use semtree_grammar::Grammar;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

/// A token produced by the runtime lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawToken {
    pub kind: RuntimeTokenKind,
    pub text: SmolStr,
    pub range: TextRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeTokenKind {
    /// A keyword defined in the grammar.
    Keyword(u16),
    /// A literal string from a grammar rule (e.g. "+", ";").
    Literal(u16),
    /// An identifier (not matching any keyword).
    Ident,
    /// An integer literal.
    Integer,
    /// A float literal.
    Float,
    /// A string literal.
    StringLit,
    /// Whitespace (non-newline).
    Whitespace,
    /// Newline.
    Newline,
    /// Line comment.
    LineComment,
    /// Block comment.
    BlockComment,
    /// Unknown/error character.
    Error,
    /// End of input.
    Eof,
}

impl RuntimeTokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            RuntimeTokenKind::Whitespace
                | RuntimeTokenKind::Newline
                | RuntimeTokenKind::LineComment
                | RuntimeTokenKind::BlockComment
        )
    }
}

/// A grammar-driven lexer. Given a Grammar IR, it tokenizes source text
/// using the keywords and literal strings defined in that grammar.
pub struct RuntimeLexer {
    keywords: FxHashMap<SmolStr, u16>,
    /// Literal strings from grammar rules, sorted longest-first for greedy matching.
    literals: Vec<(SmolStr, u16)>,
    /// Extra patterns (typically whitespace regex from tree-sitter grammars).
    _extras: Vec<SmolStr>,
}

impl RuntimeLexer {
    /// Build a runtime lexer from a Grammar IR.
    pub fn new(grammar: &Grammar) -> Self {
        let mut keywords = FxHashMap::default();
        for (i, kw) in grammar.keywords.iter().enumerate() {
            keywords.insert(kw.clone(), i as u16);
        }

        let mut literal_set = FxHashMap::default();
        let mut literal_counter = 0u16;
        Self::collect_literals_from_grammar(grammar, &mut literal_set, &mut literal_counter);

        let mut literals: Vec<_> = literal_set.into_iter().collect();
        // Sort longest first for greedy matching.
        literals.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Self {
            keywords,
            literals,
            _extras: grammar.extras.clone(),
        }
    }

    fn collect_literals_from_grammar(
        grammar: &Grammar,
        set: &mut FxHashMap<SmolStr, u16>,
        counter: &mut u16,
    ) {
        for rule in grammar.rules.values() {
            Self::collect_literals_from_expr(&rule.expr, set, counter);
        }
    }

    fn collect_literals_from_expr(
        expr: &semtree_grammar::RuleExpr,
        set: &mut FxHashMap<SmolStr, u16>,
        counter: &mut u16,
    ) {
        use semtree_grammar::RuleExpr;
        match expr {
            RuleExpr::Literal(s) => {
                if !set.contains_key(s) {
                    set.insert(s.clone(), *counter);
                    *counter += 1;
                }
            }
            RuleExpr::Seq(exprs) | RuleExpr::Choice(exprs) => {
                for e in exprs {
                    Self::collect_literals_from_expr(e, set, counter);
                }
            }
            RuleExpr::Repeat(inner)
            | RuleExpr::Repeat1(inner)
            | RuleExpr::Optional(inner)
            | RuleExpr::Token(inner)
            | RuleExpr::Prec(_, inner)
            | RuleExpr::PrecLeft(_, inner)
            | RuleExpr::PrecRight(_, inner)
            | RuleExpr::Field(_, inner) => {
                Self::collect_literals_from_expr(inner, set, counter);
            }
            RuleExpr::RuleRef(_) | RuleExpr::Blank => {}
        }
    }

    /// Tokenize the entire source.
    pub fn tokenize(&self, source: &str) -> Vec<RawToken> {
        let mut tokens = Vec::new();
        let mut pos = 0usize;
        let bytes = source.as_bytes();

        while pos < source.len() {
            let start = pos;

            // Newlines
            if bytes[pos] == b'\n' {
                pos += 1;
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::Newline,
                    text: "\n".into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }
            if bytes[pos] == b'\r' {
                pos += 1;
                if pos < bytes.len() && bytes[pos] == b'\n' {
                    pos += 1;
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::Newline,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Whitespace
            if source[pos..].starts_with(|c: char| c.is_whitespace()) {
                while pos < source.len() {
                    let c = source[pos..].chars().next().unwrap();
                    if c.is_whitespace() && c != '\n' && c != '\r' {
                        pos += c.len_utf8();
                    } else {
                        break;
                    }
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::Whitespace,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Line comment
            if source[pos..].starts_with("//") {
                while pos < source.len() && bytes[pos] != b'\n' {
                    pos += 1;
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::LineComment,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Block comment (nested)
            if source[pos..].starts_with("/*") {
                pos += 2;
                let mut depth = 1u32;
                while depth > 0 && pos < source.len() {
                    if source[pos..].starts_with("/*") {
                        depth += 1;
                        pos += 2;
                    } else if source[pos..].starts_with("*/") {
                        depth -= 1;
                        pos += 2;
                    } else {
                        pos += source[pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                    }
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::BlockComment,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // String literal
            if bytes[pos] == b'"' {
                pos += 1;
                while pos < source.len() {
                    if bytes[pos] == b'\\' {
                        pos += 2;
                    } else if bytes[pos] == b'"' {
                        pos += 1;
                        break;
                    } else {
                        pos += 1;
                    }
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::StringLit,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Number
            if bytes[pos].is_ascii_digit() {
                while pos < source.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_') {
                    pos += 1;
                }
                if pos < source.len()
                    && bytes[pos] == b'.'
                    && pos + 1 < source.len()
                    && bytes[pos + 1].is_ascii_digit()
                {
                    pos += 1;
                    while pos < source.len()
                        && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_')
                    {
                        pos += 1;
                    }
                    tokens.push(RawToken {
                        kind: RuntimeTokenKind::Float,
                        text: source[start..pos].into(),
                        range: Self::make_range(start, pos),
                    });
                } else {
                    tokens.push(RawToken {
                        kind: RuntimeTokenKind::Integer,
                        text: source[start..pos].into(),
                        range: Self::make_range(start, pos),
                    });
                }
                continue;
            }

            // Identifier / keyword
            let c = source[pos..].chars().next().unwrap();
            if c == '_' || c.is_alphabetic() {
                while pos < source.len() {
                    let ch = source[pos..].chars().next().unwrap();
                    if ch == '_' || ch.is_alphanumeric() {
                        pos += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let text: SmolStr = source[start..pos].into();
                let kind = if let Some(&idx) = self.keywords.get(&text) {
                    RuntimeTokenKind::Keyword(idx)
                } else {
                    RuntimeTokenKind::Ident
                };
                tokens.push(RawToken {
                    kind,
                    text,
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Try to match grammar literals (operators, punctuation)
            let remaining = &source[pos..];
            let mut matched = false;
            for (lit, idx) in &self.literals {
                if remaining.starts_with(lit.as_str()) {
                    pos += lit.len();
                    tokens.push(RawToken {
                        kind: RuntimeTokenKind::Literal(*idx),
                        text: lit.clone(),
                        range: Self::make_range(start, pos),
                    });
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }

            // Unknown character
            pos += c.len_utf8();
            tokens.push(RawToken {
                kind: RuntimeTokenKind::Error,
                text: source[start..pos].into(),
                range: Self::make_range(start, pos),
            });
        }

        tokens.push(RawToken {
            kind: RuntimeTokenKind::Eof,
            text: SmolStr::default(),
            range: Self::make_range(pos, pos),
        });

        tokens
    }

    fn make_range(start: usize, end: usize) -> TextRange {
        TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32))
    }
}
