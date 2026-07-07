use rustc_hash::{FxHashMap, FxHashSet};
use semtree_core::SyntaxKind;
use semtree_grammar::{Grammar, RuleExpr};
use semtree_green::{GreenNode, GreenNodeBuilder};
use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::runtime_lexer::{RawToken, RuntimeLexer, RuntimeTokenKind};

/// Result of a grammar-driven parse.
pub struct RuntimeParseResult {
    pub green_tree: GreenNode,
    pub errors: Vec<RuntimeParseError>,
    pub kind_names: FxHashMap<SyntaxKind, SmolStr>,
}

impl RuntimeParseResult {
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green_tree.clone())
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeParseError {
    pub message: String,
    pub range: TextRange,
}

impl std::fmt::Display for RuntimeParseError {
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

/// Recovery tokens that typically start new statements or close blocks.
const RECOVERY_TOKENS: &[&str] = &[
    ";", "}", ")", "]",
    "fn", "let", "if", "while", "for", "return", "struct", "enum", "impl", "trait", "use", "mod", "pub",
    "def", "class", "elif", "else", "import", "from", "try", "except", "finally", "raise",
    "with", "pass", "break", "continue", "assert", "yield", "async",
];

/// A grammar-driven parser. Given a Grammar IR and source text, it produces
/// a lossless green tree by interpreting the grammar rules at runtime.
pub struct RuntimeParser {
    grammar: Grammar,
    lexer: RuntimeLexer,
}

impl RuntimeParser {
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        Self { grammar, lexer }
    }

    pub fn parse(&self, source: &str) -> RuntimeParseResult {
        let tokens = self.lexer.tokenize(source);
        let mut ctx = ParseContext::new(&self.grammar, &tokens, source);

        let entry_rule = self
            .grammar
            .entry_rule
            .clone()
            .or_else(|| self.grammar.rules.keys().next().cloned())
            .unwrap_or_else(|| "source_file".into());

        ctx.builder.start_node(SyntaxKind::SOURCE_FILE);

        while !ctx.at_eof() {
            let before = ctx.pos;
            ctx.parse_rule(&entry_rule);
            if ctx.pos == before {
                ctx.error_recover("unexpected token");
            }
        }

        ctx.eat_trivia();
        ctx.builder.finish_node();

        RuntimeParseResult {
            green_tree: ctx.builder.finish(),
            errors: ctx.errors,
            kind_names: self.build_kind_names(),
        }
    }

    fn build_kind_names(&self) -> FxHashMap<SyntaxKind, SmolStr> {
        let mut map = FxHashMap::default();
        map.insert(SyntaxKind::SOURCE_FILE, "source_file".into());
        map.insert(SyntaxKind::ERROR, "ERROR".into());
        map.insert(SyntaxKind::WHITESPACE, "whitespace".into());
        map.insert(SyntaxKind::NEWLINE, "newline".into());
        map.insert(SyntaxKind::LINE_COMMENT, "comment".into());
        map.insert(SyntaxKind::BLOCK_COMMENT, "comment".into());
        map.insert(SyntaxKind::IDENT, "identifier".into());
        map.insert(SyntaxKind::INT_LIT, "integer".into());
        map.insert(SyntaxKind::FLOAT_LIT, "float".into());
        map.insert(SyntaxKind::STRING_LIT, "string".into());
        map.insert(SyntaxKind::CHAR_LIT, "char".into());
        map.insert(SyntaxKind::BOOL_LIT, "boolean".into());

        for name in self.grammar.rules.keys() {
            let kind = rule_name_to_kind(name);
            map.insert(kind, name.clone());
        }
        map
    }

    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }
}

struct ParseContext<'a> {
    grammar: &'a Grammar,
    tokens: &'a [RawToken],
    source: &'a str,
    pos: usize,
    builder: GreenNodeBuilder,
    errors: Vec<RuntimeParseError>,
    /// Track (rule_name, token_position) to prevent left-recursion only at the same position.
    in_progress: FxHashSet<(SmolStr, usize)>,
    /// Nesting depth guard to prevent stack overflow on deep recursion.
    depth: u32,
}

const MAX_DEPTH: u32 = 512;

impl<'a> ParseContext<'a> {
    fn new(grammar: &'a Grammar, tokens: &'a [RawToken], source: &'a str) -> Self {
        Self {
            grammar,
            tokens,
            source,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            in_progress: FxHashSet::default(),
            depth: 0,
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == RuntimeTokenKind::Eof
    }

    fn current_text(&self) -> &str {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].text.as_str()
        } else {
            ""
        }
    }

    fn skip_trivia_pos(&self) -> usize {
        let mut i = self.pos;
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        i
    }

    fn peek_text(&self) -> &str {
        let i = self.skip_trivia_pos();
        if i < self.tokens.len() {
            self.tokens[i].text.as_str()
        } else {
            ""
        }
    }

    fn peek_kind(&self) -> RuntimeTokenKind {
        let i = self.skip_trivia_pos();
        if i < self.tokens.len() {
            self.tokens[i].kind
        } else {
            RuntimeTokenKind::Eof
        }
    }

    fn eat_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind.is_trivia() {
            let tok = &self.tokens[self.pos];
            let kind = match tok.kind {
                RuntimeTokenKind::Whitespace => SyntaxKind::WHITESPACE,
                RuntimeTokenKind::Newline => SyntaxKind::NEWLINE,
                RuntimeTokenKind::LineComment => SyntaxKind::LINE_COMMENT,
                RuntimeTokenKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
                _ => SyntaxKind::WHITESPACE,
            };
            self.builder.token(kind, tok.text.as_str());
            self.pos += 1;
        }
    }

    fn bump(&mut self) {
        self.eat_trivia();
        if !self.at_eof() {
            let tok = &self.tokens[self.pos];
            let kind = self.token_to_syntax_kind(tok);
            self.builder.token(kind, tok.text.as_str());
            self.pos += 1;
        }
    }

    fn token_to_syntax_kind(&self, tok: &RawToken) -> SyntaxKind {
        match tok.kind {
            RuntimeTokenKind::Keyword(_) => SyntaxKind::IDENT,
            RuntimeTokenKind::Literal(_) => SyntaxKind::IDENT,
            RuntimeTokenKind::Ident => SyntaxKind::IDENT,
            RuntimeTokenKind::Integer => SyntaxKind::INT_LIT,
            RuntimeTokenKind::Float => SyntaxKind::FLOAT_LIT,
            RuntimeTokenKind::StringLit => SyntaxKind::STRING_LIT,
            RuntimeTokenKind::Whitespace => SyntaxKind::WHITESPACE,
            RuntimeTokenKind::Newline => SyntaxKind::NEWLINE,
            RuntimeTokenKind::LineComment => SyntaxKind::LINE_COMMENT,
            RuntimeTokenKind::BlockComment => SyntaxKind::BLOCK_COMMENT,
            RuntimeTokenKind::Error => SyntaxKind::ERROR,
            RuntimeTokenKind::Eof => SyntaxKind::EOF,
        }
    }

    // ── Error Recovery ──────────────────────────────────────

    /// Skip to a recovery point, wrapping skipped tokens in an ERROR node.
    fn error_recover(&mut self, message: &str) {
        self.eat_trivia();
        if self.at_eof() {
            return;
        }

        let tok = &self.tokens[self.pos];
        self.errors.push(RuntimeParseError {
            message: message.to_string(),
            range: tok.range,
        });

        self.builder.start_node(SyntaxKind::ERROR);

        // Consume tokens until we hit a recovery point.
        let mut count = 0;
        while !self.at_eof() && count < 50 {
            let text = self.peek_text();
            if count > 0 && RECOVERY_TOKENS.contains(&text) {
                break;
            }
            // Stop at block closers.
            if count > 0 && (text == "}" || text == ")") {
                break;
            }
            self.bump();
            count += 1;
        }

        self.builder.finish_node();
    }

    /// Emit an error and try to insert a missing token (phantom token).
    fn error_missing(&mut self, expected: &str) {
        let range = if self.pos < self.tokens.len() {
            self.tokens[self.pos].range
        } else {
            let end = self.source.len() as u32;
            TextRange::new(TextSize::new(end), TextSize::new(end))
        };
        self.errors.push(RuntimeParseError {
            message: format!("expected {expected}"),
            range,
        });
    }

    fn error_here(&mut self, message: &str) {
        let range = if self.pos < self.tokens.len() {
            self.tokens[self.pos].range
        } else {
            let end = self.source.len() as u32;
            TextRange::new(TextSize::new(end), TextSize::new(end))
        };
        self.errors.push(RuntimeParseError {
            message: message.to_string(),
            range,
        });
    }

    // ── Rule Parsing ────────────────────────────────────────

    fn parse_rule(&mut self, name: &str) -> bool {
        if self.depth > MAX_DEPTH {
            self.error_here("maximum parse depth exceeded");
            return false;
        }

        match name {
            "Identifier" | "identifier" | "_identifier" => return self.parse_builtin_ident(),
            "Integer" | "integer" | "number" => return self.parse_builtin_integer(),
            "Float" | "float" => return self.parse_builtin_float(),
            "String" | "string" => return self.parse_builtin_string(),
            _ => {}
        }

        let rule = match self.grammar.rules.get(name) {
            Some(r) => r.clone(),
            None => {
                self.error_here(&format!("undefined rule: {name}"));
                return false;
            }
        };

        let name_smol: SmolStr = name.into();
        let guard_key = (name_smol.clone(), self.pos);
        if self.in_progress.contains(&guard_key) {
            return false;
        }

        self.in_progress.insert(guard_key.clone());
        self.depth += 1;

        let save_pos = self.pos;
        let save_builder = self.builder.checkpoint();

        let node_kind = self.rule_name_to_kind(name);
        self.builder.start_node(node_kind);

        let matched = self.parse_expr(&rule.expr);

        if !matched {
            self.builder.rollback(save_builder);
            self.pos = save_pos;
            self.depth -= 1;
            self.in_progress.remove(&guard_key);
            return false;
        }

        self.builder.finish_node();
        self.depth -= 1;
        self.in_progress.remove(&guard_key);
        true
    }

    fn rule_name_to_kind(&self, name: &str) -> SyntaxKind {
        rule_name_to_kind(name)
    }

    fn parse_expr(&mut self, expr: &RuleExpr) -> bool {
        match expr {
            RuleExpr::Literal(s) => self.parse_literal(s),
            RuleExpr::RuleRef(name) => self.parse_rule(name),
            RuleExpr::Seq(exprs) => self.parse_seq(exprs),
            RuleExpr::Choice(exprs) => self.parse_choice(exprs),
            RuleExpr::Repeat(inner) => self.parse_repeat(inner),
            RuleExpr::Repeat1(inner) => self.parse_repeat1(inner),
            RuleExpr::Optional(inner) => {
                self.parse_expr(inner);
                true
            }
            RuleExpr::Field(_name, inner) => self.parse_expr(inner),
            RuleExpr::Token(inner) => self.parse_expr(inner),
            RuleExpr::Prec(prec, inner) => self.parse_prec(*prec, inner),
            RuleExpr::PrecLeft(prec, inner) => self.parse_prec_left(*prec, inner),
            RuleExpr::PrecRight(prec, inner) => self.parse_prec_right(*prec, inner),
            RuleExpr::Blank => true,
        }
    }

    // ── Precedence Climbing ─────────────────────────────────

    fn parse_prec(&mut self, _prec: i32, inner: &RuleExpr) -> bool {
        self.parse_expr(inner)
    }

    fn parse_prec_left(&mut self, _prec: i32, inner: &RuleExpr) -> bool {
        // For PrecLeft(p, Seq([lhs, op, rhs])), parse left-associatively.
        if let RuleExpr::Seq(parts) = inner {
            if parts.len() == 3 {
                return self.parse_binary_left(parts);
            }
        }
        self.parse_expr(inner)
    }

    fn parse_prec_right(&mut self, _prec: i32, inner: &RuleExpr) -> bool {
        if let RuleExpr::Seq(parts) = inner {
            if parts.len() == 3 {
                return self.parse_binary_right(parts);
            }
        }
        self.parse_expr(inner)
    }

    fn parse_binary_left(&mut self, parts: &[RuleExpr]) -> bool {
        if !self.parse_expr(&parts[0]) {
            return false;
        }
        loop {
            let save = self.pos;
            let save_builder = self.builder.checkpoint();
            if !self.try_parse_expr(&parts[1]) || !self.try_parse_expr(&parts[2]) {
                self.pos = save;
                self.builder.rollback(save_builder);
                break;
            }
        }
        true
    }

    fn parse_binary_right(&mut self, parts: &[RuleExpr]) -> bool {
        if !self.parse_expr(&parts[0]) {
            return false;
        }
        let save = self.pos;
        let save_builder = self.builder.checkpoint();
        if self.try_parse_expr(&parts[1]) {
            if !self.parse_binary_right(parts) {
                self.pos = save;
                self.builder.rollback(save_builder);
            }
        } else {
            self.pos = save;
            self.builder.rollback(save_builder);
        }
        true
    }

    // ── Core Parse Methods ──────────────────────────────────

    fn parse_literal(&mut self, expected: &str) -> bool {
        let peek_pos = self.skip_trivia_pos();
        if peek_pos >= self.tokens.len()
            || self.tokens[peek_pos].kind == RuntimeTokenKind::Eof
        {
            return false;
        }
        if self.tokens[peek_pos].text.as_str() == expected {
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_seq(&mut self, exprs: &[RuleExpr]) -> bool {
        for (i, expr) in exprs.iter().enumerate() {
            if !self.parse_expr(expr) {
                if i > 0 {
                    // Only recover for missing small closing literals near the
                    // end of a sequence (e.g. missing ";" or ")" after all
                    // meaningful content matched).
                    if let RuleExpr::Literal(lit) = expr {
                        let is_last = i == exprs.len() - 1;
                        if lit.len() <= 2 && is_last {
                            self.error_missing(&format!("'{lit}'"));
                            continue;
                        }
                    }
                }
                return false;
            }
        }
        true
    }

    fn parse_choice(&mut self, exprs: &[RuleExpr]) -> bool {
        let save_pos = self.pos;
        let save_errors = self.errors.len();
        let save_builder = self.builder.checkpoint();

        for expr in exprs {
            self.pos = save_pos;
            self.errors.truncate(save_errors);
            self.builder.rollback(save_builder);
            if self.try_parse_expr(expr) {
                return true;
            }
        }

        self.pos = save_pos;
        self.errors.truncate(save_errors);
        self.builder.rollback(save_builder);
        false
    }

    fn parse_repeat(&mut self, inner: &RuleExpr) -> bool {
        let mut iterations = 0u32;
        loop {
            let save = self.pos;
            let save_builder = self.builder.checkpoint();
            if !self.try_parse_expr(inner) {
                self.pos = save;
                self.builder.rollback(save_builder);
                break;
            }
            if self.pos == save {
                break;
            }
            iterations += 1;
            if iterations > 100_000 {
                self.error_here("repeat limit exceeded");
                break;
            }
        }
        true
    }

    fn parse_repeat1(&mut self, inner: &RuleExpr) -> bool {
        if !self.parse_expr(inner) {
            return false;
        }
        let mut iterations = 0u32;
        loop {
            let save = self.pos;
            let save_builder = self.builder.checkpoint();
            if !self.try_parse_expr(inner) {
                self.pos = save;
                self.builder.rollback(save_builder);
                break;
            }
            if self.pos == save {
                break;
            }
            iterations += 1;
            if iterations > 100_000 {
                self.error_here("repeat limit exceeded");
                break;
            }
        }
        true
    }

    fn try_parse_expr(&mut self, expr: &RuleExpr) -> bool {
        let save_errors = self.errors.len();
        let result = self.parse_expr(expr);
        if !result {
            self.errors.truncate(save_errors);
        }
        result
    }

    // ── Built-in terminals ──────────────────────────────────

    fn parse_builtin_ident(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() || self.tokens[peek].kind == RuntimeTokenKind::Eof {
            return false;
        }
        match self.tokens[peek].kind {
            RuntimeTokenKind::Ident | RuntimeTokenKind::Keyword(_) => {
                self.bump();
                true
            }
            _ => false,
        }
    }

    fn parse_builtin_integer(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() || self.tokens[peek].kind == RuntimeTokenKind::Eof {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::Integer {
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_builtin_float(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() || self.tokens[peek].kind == RuntimeTokenKind::Eof {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::Float {
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_builtin_string(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() || self.tokens[peek].kind == RuntimeTokenKind::Eof {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::StringLit {
            self.bump();
            true
        } else {
            false
        }
    }

    fn describe_expr(&self, expr: &RuleExpr) -> String {
        match expr {
            RuleExpr::Literal(s) => format!("'{s}'"),
            RuleExpr::RuleRef(name) => name.to_string(),
            RuleExpr::Seq(_) => "sequence".to_string(),
            RuleExpr::Choice(_) => "one of alternatives".to_string(),
            RuleExpr::Repeat(inner) | RuleExpr::Repeat1(inner) => {
                format!("repetition of {}", self.describe_expr(inner))
            }
            RuleExpr::Optional(inner) => format!("optional {}", self.describe_expr(inner)),
            RuleExpr::Field(name, _) => format!("field '{name}'"),
            _ => "expression".to_string(),
        }
    }
}

pub fn rule_name_to_kind(name: &str) -> SyntaxKind {
    let mut hash: u16 = 4096;
    for (i, b) in name.bytes().enumerate() {
        hash = hash
            .wrapping_add(b as u16)
            .wrapping_mul(31)
            .wrapping_add(i as u16);
    }
    if hash < 4096 {
        hash += 4096;
    }
    SyntaxKind(hash)
}
