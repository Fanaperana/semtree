use regex::Regex;
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
    Keyword(u16),
    Literal(u16),
    /// Custom token from `token Name := /regex/` in grammar.
    Custom(u16),
    Ident,
    Integer,
    Float,
    StringLit,
    Indent,
    Dedent,
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    Error,
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

struct CompiledToken {
    regex: Option<Regex>,
    literal: Option<SmolStr>,
    id: u16,
}

struct CompiledExtra {
    regex: Regex,
    kind: RuntimeTokenKind,
}

/// A grammar-driven lexer. Tokenizes using keywords, literals, custom `token`
/// patterns, and optional `extra` regexes from the grammar DSL.
pub struct RuntimeLexer {
    keywords: FxHashMap<SmolStr, u16>,
    literals: Vec<(SmolStr, u16)>,
    custom_tokens: Vec<CompiledToken>,
    token_names: Vec<SmolStr>,
    extra_patterns: Vec<CompiledExtra>,
    indent_sensitive: bool,
    /// Whether the grammar uses `#` as a comment prefix (e.g. Python, Ruby).
    hash_comments: bool,
}

impl RuntimeLexer {
    pub fn new(grammar: &Grammar) -> Self {
        let mut keywords = FxHashMap::default();
        for (i, kw) in grammar.keywords.iter().enumerate() {
            keywords.insert(kw.clone(), i as u16);
        }

        let mut literal_set = FxHashMap::default();
        let mut literal_counter = 0u16;
        Self::collect_literals_from_grammar(grammar, &mut literal_set, &mut literal_counter);

        let mut literals: Vec<_> = literal_set.into_iter().collect();
        literals.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

        let mut custom_tokens = Vec::new();
        let mut token_names = Vec::new();
        for (i, def) in grammar.tokens.iter().enumerate() {
            token_names.push(def.name.clone());
            let compiled = if def.is_regex {
                CompiledToken {
                    regex: Regex::new(def.pattern.as_str()).ok(),
                    literal: None,
                    id: i as u16,
                }
            } else {
                CompiledToken {
                    regex: None,
                    literal: Some(def.pattern.clone()),
                    id: i as u16,
                }
            };
            custom_tokens.push(compiled);
        }
        custom_tokens.sort_by(|a, b| {
            let alen = a.literal.as_ref().map(|s| s.len()).unwrap_or(0);
            let blen = b.literal.as_ref().map(|s| s.len()).unwrap_or(0);
            blen.cmp(&alen)
        });

        let extra_patterns = Self::compile_extras(grammar);

        // Detect whether the grammar uses `#` for comments.
        // Check extras patterns, or infer from language name / indent-sensitive flag.
        let has_hash_extra = extra_patterns.iter().any(|e| {
            matches!(e.kind, RuntimeTokenKind::LineComment)
        });
        let hash_comments = has_hash_extra
            || grammar.indent_sensitive
            || grammar.name.as_str() == "python"
            || grammar.name.as_str() == "ruby"
            || grammar.name.as_str() == "bash"
            || grammar.name.as_str() == "shell"
            || grammar.name.as_str() == "toml"
            || grammar.name.as_str() == "yaml";

        // If grammar uses `#` as comments but no extras pattern is defined,
        // ensure `#` is not treated as a literal error token.

        Self {
            keywords,
            literals,
            custom_tokens,
            token_names,
            extra_patterns,
            indent_sensitive: grammar.indent_sensitive,
            hash_comments,
        }
    }

    pub fn token_name(&self, id: u16) -> Option<&str> {
        self.token_names.get(id as usize).map(|s| s.as_str())
    }

    fn compile_extras(grammar: &Grammar) -> Vec<CompiledExtra> {
        let mut extras = Vec::new();
        for extra in &grammar.extras {
            let pattern = extra.as_str();
            if let (Some(p), Some(_)) = (pattern.strip_prefix('/'), pattern.strip_suffix('/'))
                && let Ok(re) = Regex::new(p)
            {
                let kind = if p.contains('\n') || p == r"\s+" || p == r"[ \t]+" {
                    RuntimeTokenKind::Whitespace
                } else if p.contains("//") || p.contains('#') {
                    RuntimeTokenKind::LineComment
                } else {
                    RuntimeTokenKind::Whitespace
                };
                extras.push(CompiledExtra { regex: re, kind });
            }
        }
        extras
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

    pub fn tokenize(&self, source: &str) -> Vec<RawToken> {
        let mut tokens = self.tokenize_raw(source);
        if self.indent_sensitive {
            tokens = inject_indent_tokens(tokens, source);
        }
        tokens
    }

    fn tokenize_raw(&self, source: &str) -> Vec<RawToken> {
        let mut tokens = Vec::new();
        let mut pos = 0usize;
        let bytes = source.as_bytes();

        while pos < source.len() {
            let start = pos;

            // Grammar-defined extra/trivia patterns
            if self.try_match_extra(source, pos, &mut tokens, &mut pos) {
                continue;
            }

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

            // Default whitespace
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

            // Default comments
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
                        pos += source[pos..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                    }
                }
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::BlockComment,
                    text: source[start..pos].into(),
                    range: Self::make_range(start, pos),
                });
                continue;
            }

            // Hash comments (#... until newline) for Python, Ruby, etc.
            if self.hash_comments && bytes[pos] == b'#' {
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

            // String literal (also f"/F" prefix via custom token first)
            if self.try_custom_tokens(source, pos, &mut tokens, &mut pos) {
                continue;
            }

            if bytes[pos] == b'"' || bytes[pos] == b'\'' {
                let quote = bytes[pos];

                // Triple-quoted strings ("""...""" or '''...''')
                if pos + 2 < source.len() && bytes[pos + 1] == quote && bytes[pos + 2] == quote {
                    pos += 3;
                    while pos + 2 < source.len() {
                        if bytes[pos] == b'\\' {
                            pos += 2;
                        } else if bytes[pos] == quote
                            && bytes[pos + 1] == quote
                            && bytes[pos + 2] == quote
                        {
                            pos += 3;
                            break;
                        } else {
                            pos += 1;
                        }
                    }
                    // Handle case where we ran out of input
                    if pos > source.len() {
                        pos = source.len();
                    }
                    tokens.push(RawToken {
                        kind: RuntimeTokenKind::StringLit,
                        text: source[start..pos].into(),
                        range: Self::make_range(start, pos),
                    });
                    continue;
                }

                // Single-quoted string
                pos += 1;
                while pos < source.len() {
                    if bytes[pos] == b'\\' {
                        pos += 2;
                    } else if bytes[pos] == quote {
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

            // Number (including 0x, 0o, 0b prefixes)
            if bytes[pos].is_ascii_digit() {
                if bytes[pos] == b'0' && pos + 1 < source.len() {
                    match bytes[pos + 1] {
                        b'x' | b'X' => {
                            pos += 2;
                            while pos < source.len()
                                && (bytes[pos].is_ascii_hexdigit() || bytes[pos] == b'_')
                            {
                                pos += 1;
                            }
                            tokens.push(RawToken {
                                kind: RuntimeTokenKind::Integer,
                                text: source[start..pos].into(),
                                range: Self::make_range(start, pos),
                            });
                            continue;
                        }
                        b'o' | b'O' => {
                            pos += 2;
                            while pos < source.len()
                                && ((bytes[pos] >= b'0' && bytes[pos] <= b'7') || bytes[pos] == b'_')
                            {
                                pos += 1;
                            }
                            tokens.push(RawToken {
                                kind: RuntimeTokenKind::Integer,
                                text: source[start..pos].into(),
                                range: Self::make_range(start, pos),
                            });
                            continue;
                        }
                        b'b' | b'B' if pos + 2 < source.len()
                            && (bytes[pos + 2] == b'0' || bytes[pos + 2] == b'1') =>
                        {
                            pos += 2;
                            while pos < source.len()
                                && (bytes[pos] == b'0' || bytes[pos] == b'1' || bytes[pos] == b'_')
                            {
                                pos += 1;
                            }
                            tokens.push(RawToken {
                                kind: RuntimeTokenKind::Integer,
                                text: source[start..pos].into(),
                                range: Self::make_range(start, pos),
                            });
                            continue;
                        }
                        _ => {}
                    }
                }
                while pos < source.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_') {
                    pos += 1;
                }
                if pos < source.len()
                    && bytes[pos] == b'.'
                    && pos + 1 < source.len()
                    && bytes[pos + 1].is_ascii_digit()
                {
                    pos += 1;
                    while pos < source.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_')
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

            // Grammar literals
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

    fn try_match_extra(
        &self,
        source: &str,
        pos: usize,
        tokens: &mut Vec<RawToken>,
        cursor: &mut usize,
    ) -> bool {
        for extra in &self.extra_patterns {
            if let Some(m) = extra.regex.find_at(source, pos)
                && m.start() == pos
            {
                *cursor = m.end();
                tokens.push(RawToken {
                    kind: extra.kind,
                    text: source[m.start()..m.end()].into(),
                    range: Self::make_range(m.start(), m.end()),
                });
                return true;
            }
        }
        false
    }

    fn try_custom_tokens(
        &self,
        source: &str,
        pos: usize,
        tokens: &mut Vec<RawToken>,
        cursor: &mut usize,
    ) -> bool {
        for ct in &self.custom_tokens {
            if let Some(lit) = &ct.literal {
                if source[pos..].starts_with(lit.as_str()) {
                    *cursor = pos + lit.len();
                    tokens.push(RawToken {
                        kind: RuntimeTokenKind::Custom(ct.id),
                        text: lit.clone(),
                        range: Self::make_range(pos, pos + lit.len()),
                    });
                    return true;
                }
            } else if let Some(re) = &ct.regex
                && let Some(m) = re.find_at(source, pos)
                && m.start() == pos
            {
                *cursor = m.end();
                tokens.push(RawToken {
                    kind: RuntimeTokenKind::Custom(ct.id),
                    text: source[m.start()..m.end()].into(),
                    range: Self::make_range(m.start(), m.end()),
                });
                return true;
            }
        }
        false
    }

    fn make_range(start: usize, end: usize) -> TextRange {
        TextRange::new(TextSize::new(start as u32), TextSize::new(end as u32))
    }
}

/// Insert INDENT/DEDENT tokens for indentation-sensitive grammars (Python-style).
fn inject_indent_tokens(mut tokens: Vec<RawToken>, source: &str) -> Vec<RawToken> {
    let mut out = Vec::with_capacity(tokens.len() + 16);
    let mut indent_stack = vec![0usize];
    let mut at_line_start = true;

    let column_at = |offset: usize| -> usize {
        source[..offset]
            .rfind('\n')
            .map(|p| offset - p - 1)
            .unwrap_or(offset)
    };

    let push_indent_dedent = |out: &mut Vec<RawToken>, stack: &mut Vec<usize>, col: usize| {
        if col > *stack.last().unwrap() {
            stack.push(col);
            out.push(RawToken {
                kind: RuntimeTokenKind::Indent,
                text: "".into(),
                range: TextRange::new(TextSize::new(0), TextSize::new(0)),
            });
        } else {
            while stack.len() > 1 && *stack.last().unwrap() > col {
                stack.pop();
                out.push(RawToken {
                    kind: RuntimeTokenKind::Dedent,
                    text: "".into(),
                    range: TextRange::new(TextSize::new(0), TextSize::new(0)),
                });
            }
        }
    };

    for tok in tokens.drain(..) {
        if tok.kind == RuntimeTokenKind::Eof {
            while indent_stack.len() > 1 {
                indent_stack.pop();
                out.push(RawToken {
                    kind: RuntimeTokenKind::Dedent,
                    text: "".into(),
                    range: TextRange::new(TextSize::new(0), TextSize::new(0)),
                });
            }
            out.push(tok);
            break;
        }

        if tok.kind == RuntimeTokenKind::Newline {
            out.push(tok);
            at_line_start = true;
            continue;
        }

        if tok.kind.is_trivia() {
            out.push(tok);
            continue;
        }

        if at_line_start {
            let col = column_at(u32::from(tok.range.start()) as usize);
            if col > 0 || indent_stack.len() > 1 {
                push_indent_dedent(&mut out, &mut indent_stack, col);
            }
            at_line_start = false;
        }

        out.push(tok);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use semtree_grammar::parse_semtree_dsl;

    #[test]
    fn custom_token_regex() {
        let g = parse_semtree_dsl(
            r#"
language x
token Arrow := /->/
Rule := Arrow
"#,
        )
        .unwrap();
        let lexer = RuntimeLexer::new(&g);
        let toks = lexer.tokenize("->");
        assert!(
            toks.iter()
                .any(|t| matches!(t.kind, RuntimeTokenKind::Custom(0)))
        );
    }

    #[test]
    fn indent_tokens_basic() {
        let g = parse_semtree_dsl(
            r#"
language py
indent-sensitive
keyword def
Rule := "def" Identifier
"#,
        )
        .unwrap();
        let lexer = RuntimeLexer::new(&g);
        let src = "def a:\n    pass\n";
        let toks = lexer.tokenize(src);
        assert!(toks.iter().any(|t| t.kind == RuntimeTokenKind::Indent));
        assert!(toks.iter().any(|t| t.kind == RuntimeTokenKind::Dedent));
    }
}
