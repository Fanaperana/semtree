use rustc_hash::FxHashSet;
use semtree_core::SyntaxKind;
use semtree_grammar::{Grammar, RuleExpr};
use semtree_green::{GreenNode, GreenNodeBuilder};
use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::TextRange;

use crate::runtime_lexer::{RawToken, RuntimeLexer, RuntimeTokenKind};

/// Result of a grammar-driven parse.
pub struct RuntimeParseResult {
    pub green_tree: GreenNode,
    pub errors: Vec<RuntimeParseError>,
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

/// A grammar-driven parser. Given a Grammar IR and source text, it produces
/// a lossless green tree by interpreting the grammar rules at runtime.
///
/// This is the equivalent of what Tree-sitter does: you define a grammar,
/// and it can parse any source that follows that grammar.
pub struct RuntimeParser {
    grammar: Grammar,
    lexer: RuntimeLexer,
}

impl RuntimeParser {
    /// Create a new runtime parser from a grammar.
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        Self { grammar, lexer }
    }

    /// Parse source text according to the grammar, producing a syntax tree.
    pub fn parse(&self, source: &str) -> RuntimeParseResult {
        let tokens = self.lexer.tokenize(source);
        let mut ctx = ParseContext::new(&self.grammar, &tokens, source);

        // Find the entry rule (first rule in the grammar).
        let entry_rule = self
            .grammar
            .rules
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "source_file".into());

        ctx.builder.start_node(SyntaxKind::SOURCE_FILE);

        // Parse the entry rule repeatedly (top-level items).
        while !ctx.at_eof() {
            let before = ctx.pos;
            ctx.parse_rule(&entry_rule);
            if ctx.pos == before {
                // No progress — consume one token as error and move on.
                ctx.error_skip("unexpected token");
            }
        }

        // Eat any remaining trivia.
        ctx.eat_trivia();

        ctx.builder.finish_node();

        RuntimeParseResult {
            green_tree: ctx.builder.finish(),
            errors: ctx.errors,
        }
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
    /// Tracks rules currently being parsed to detect left recursion.
    in_progress: FxHashSet<SmolStr>,
}

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
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == RuntimeTokenKind::Eof
    }

    fn current(&self) -> &RawToken {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            self.tokens.last().unwrap()
        }
    }

    fn peek_non_trivia(&self) -> &RawToken {
        let mut i = self.pos;
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        if i < self.tokens.len() {
            &self.tokens[i]
        } else {
            self.tokens.last().unwrap()
        }
    }

    /// Return the position of the next non-trivia token without consuming anything.
    fn skip_trivia_pos(&self) -> usize {
        let mut i = self.pos;
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        i
    }

    /// Emit trivia tokens into the builder and advance `self.pos` past them.
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

    /// Consume trivia + one real token and emit them all into the builder.
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

    fn error_skip(&mut self, message: &str) {
        self.eat_trivia();
        if !self.at_eof() {
            let tok = &self.tokens[self.pos];
            self.errors.push(RuntimeParseError {
                message: message.to_string(),
                range: tok.range,
            });
            self.builder.start_node(SyntaxKind::ERROR);
            self.bump();
            self.builder.finish_node();
        }
    }

    fn error_here(&mut self, message: &str) {
        let range = if self.pos < self.tokens.len() {
            self.tokens[self.pos].range
        } else {
            let end = self.source.len() as u32;
            TextRange::new(text_size::TextSize::new(end), text_size::TextSize::new(end))
        };
        self.errors.push(RuntimeParseError {
            message: message.to_string(),
            range,
        });
    }

    /// Try to parse a named rule from the grammar.
    /// Returns true if the rule matched and consumed tokens.
    fn parse_rule(&mut self, name: &str) -> bool {
        // Handle built-in terminal rules.
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

        // Left recursion guard.
        let name_smol: SmolStr = name.into();
        if self.in_progress.contains(&name_smol) {
            return false;
        }

        self.in_progress.insert(name_smol.clone());

        let save_pos = self.pos;
        let save_builder = self.builder.checkpoint();

        let node_kind = self.rule_name_to_kind(name);
        self.builder.start_node(node_kind);

        let matched = self.parse_expr(&rule.expr);

        if !matched {
            self.builder.rollback(save_builder);
            self.pos = save_pos;
            self.in_progress.remove(&name_smol);
            return false;
        }

        self.builder.finish_node();
        self.in_progress.remove(&name_smol);
        true
    }

    /// Map rule names to SyntaxKind values deterministically.
    fn rule_name_to_kind(&self, name: &str) -> SyntaxKind {
        // Use a range starting at 4096 for user-defined rules.
        let mut hash: u16 = 4096;
        for (i, b) in name.bytes().enumerate() {
            hash = hash.wrapping_add(b as u16).wrapping_mul(31).wrapping_add(i as u16);
        }
        if hash < 4096 {
            hash += 4096;
        }
        SyntaxKind(hash)
    }

    /// Try to parse a grammar expression. Returns true if it matched.
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
                true // Optional always succeeds.
            }
            RuleExpr::Field(_name, inner) => self.parse_expr(inner),
            RuleExpr::Token(inner) => self.parse_expr(inner),
            RuleExpr::Prec(_, inner) | RuleExpr::PrecLeft(_, inner) | RuleExpr::PrecRight(_, inner) => {
                self.parse_expr(inner)
            }
            RuleExpr::Blank => true,
        }
    }

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
        let _save = self.pos;
        for expr in exprs {
            if !self.parse_expr(expr) {
                // Error recovery: report but continue trying.
                // For strict mode we'd restore `save` and return false.
                // For error-tolerant parsing, emit an error and skip.
                self.eat_trivia();
                if !self.at_eof() {
                    let expected = self.describe_expr(expr);
                    self.error_here(&format!("expected {expected}"));
                }
                return true; // Partial match with errors.
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
        }
        true
    }

    fn parse_repeat1(&mut self, inner: &RuleExpr) -> bool {
        if !self.parse_expr(inner) {
            return false;
        }
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
        }
        true
    }

    /// Try to parse an expression without emitting errors on failure.
    /// Used for backtracking in Choice and Repeat.
    fn try_parse_expr(&mut self, expr: &RuleExpr) -> bool {
        let save_errors = self.errors.len();
        let result = self.parse_expr(expr);
        if !result {
            self.errors.truncate(save_errors);
        }
        result
    }

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
