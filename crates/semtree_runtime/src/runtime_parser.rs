use rustc_hash::{FxHashMap, FxHashSet};
use semtree_core::SyntaxKind;
use semtree_grammar::{Grammar, RuleExpr};
use semtree_green::{GreenNode, GreenNodeBuilder};
use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::runtime_lexer::{RawToken, RuntimeLexer, RuntimeTokenKind};

/// Precomputed FIRST sets for predictive parsing.
/// Maps each rule name → set of literal tokens that can start it.
/// `can_match_ident` / `can_match_int` / etc. track whether the rule can
/// start with a built-in token class (identifiers, numbers, strings).
#[derive(Debug, Clone, Default)]
struct FirstSet {
    literals: FxHashSet<SmolStr>,
    can_match_ident: bool,
    can_match_int: bool,
    can_match_float: bool,
    can_match_string: bool,
    /// True if the rule can match the empty string (ε).
    can_be_empty: bool,
    /// True if we couldn't fully determine the set (fallback to trying everything).
    is_universal: bool,
}

impl FirstSet {
    fn universal() -> Self {
        Self {
            is_universal: true,
            ..Default::default()
        }
    }

    fn merge(&mut self, other: &FirstSet) {
        if other.is_universal {
            self.is_universal = true;
            return;
        }
        for lit in &other.literals {
            self.literals.insert(lit.clone());
        }
        self.can_match_ident |= other.can_match_ident;
        self.can_match_int |= other.can_match_int;
        self.can_match_float |= other.can_match_float;
        self.can_match_string |= other.can_match_string;
        self.can_be_empty |= other.can_be_empty;
    }

    /// Does this FIRST set predict a match for the given token?
    fn matches_token(&self, tok: &RawToken) -> bool {
        if self.is_universal {
            return true;
        }
        if self.literals.contains(&tok.text) {
            return true;
        }
        match tok.kind {
            RuntimeTokenKind::Ident | RuntimeTokenKind::Keyword(_) => self.can_match_ident,
            RuntimeTokenKind::Integer => self.can_match_int,
            RuntimeTokenKind::Float => self.can_match_float,
            RuntimeTokenKind::StringLit => self.can_match_string,
            RuntimeTokenKind::Indent => self.literals.contains("INDENT"),
            RuntimeTokenKind::Dedent => self.literals.contains("DEDENT"),
            _ => {
                // Punctuation / operators — check literals.
                false
            }
        }
    }
}

/// Compute FIRST sets for all rules in the grammar.
fn compute_first_sets(grammar: &Grammar) -> FxHashMap<SmolStr, FirstSet> {
    let mut sets: FxHashMap<SmolStr, FirstSet> = FxHashMap::default();

    // Initialize empty sets.
    for name in grammar.rules.keys() {
        sets.insert(name.clone(), FirstSet::default());
    }

    // Fixed-point iteration — keep merging until stable.
    let mut changed = true;
    let mut iterations = 0;
    while changed && iterations < 50 {
        changed = false;
        iterations += 1;

        for (name, rule) in &grammar.rules {
            let new_set = first_of_expr(&rule.expr, grammar, &sets, &mut FxHashSet::default());
            let entry = sets.get_mut(name).unwrap();
            let old_len = entry.literals.len();
            let old_ident = entry.can_match_ident;
            let old_int = entry.can_match_int;
            let old_float = entry.can_match_float;
            let old_string = entry.can_match_string;
            let old_empty = entry.can_be_empty;
            let old_univ = entry.is_universal;
            entry.merge(&new_set);
            if entry.literals.len() != old_len
                || entry.can_match_ident != old_ident
                || entry.can_match_int != old_int
                || entry.can_match_float != old_float
                || entry.can_match_string != old_string
                || entry.can_be_empty != old_empty
                || entry.is_universal != old_univ
            {
                changed = true;
            }
        }
    }

    sets
}

fn first_of_expr(
    expr: &RuleExpr,
    grammar: &Grammar,
    sets: &FxHashMap<SmolStr, FirstSet>,
    visiting: &mut FxHashSet<SmolStr>,
) -> FirstSet {
    match expr {
        RuleExpr::Literal(s) => {
            let mut fs = FirstSet::default();
            fs.literals.insert(s.clone());
            fs
        }
        RuleExpr::RuleRef(name) => {
            // Check builtins.
            match name.as_str() {
                "Identifier" | "identifier" | "_identifier" => {
                    let mut fs = FirstSet::default();
                    fs.can_match_ident = true;
                    return fs;
                }
                "Integer" | "integer" | "number" => {
                    let mut fs = FirstSet::default();
                    fs.can_match_int = true;
                    return fs;
                }
                "Float" | "float" => {
                    let mut fs = FirstSet::default();
                    fs.can_match_float = true;
                    return fs;
                }
                "String" | "string" => {
                    let mut fs = FirstSet::default();
                    fs.can_match_string = true;
                    return fs;
                }
                "INDENT" | "Indent" => {
                    let mut fs = FirstSet::default();
                    fs.literals.insert("INDENT".into());
                    return fs;
                }
                "DEDENT" | "Dedent" => {
                    let mut fs = FirstSet::default();
                    fs.literals.insert("DEDENT".into());
                    return fs;
                }
                _ => {}
            }

            // Check custom tokens — they can match ident-like things.
            if grammar.tokens.iter().any(|t| t.name.as_str() == name.as_str()) {
                return FirstSet::universal();
            }

            // Avoid infinite recursion on left-recursive rules.
            if visiting.contains(name) {
                return FirstSet::default();
            }
            visiting.insert(name.clone());

            let result = if let Some(fs) = sets.get(name) {
                fs.clone()
            } else {
                // Unknown rule — be conservative.
                FirstSet::universal()
            };
            visiting.remove(name);
            result
        }
        RuleExpr::Seq(parts) => {
            let mut fs = FirstSet::default();
            for part in parts {
                let pf = first_of_expr(part, grammar, sets, visiting);
                let can_be_empty = pf.can_be_empty;
                fs.merge(&pf);
                fs.can_be_empty = false; // seq is not empty unless ALL parts are
                if !can_be_empty {
                    return fs;
                }
            }
            fs.can_be_empty = true; // all parts can be empty
            fs
        }
        RuleExpr::Choice(alts) => {
            let mut fs = FirstSet::default();
            for alt in alts {
                fs.merge(&first_of_expr(alt, grammar, sets, visiting));
            }
            fs
        }
        RuleExpr::Repeat(inner) => {
            let mut fs = first_of_expr(inner, grammar, sets, visiting);
            fs.can_be_empty = true;
            fs
        }
        RuleExpr::Repeat1(inner) => first_of_expr(inner, grammar, sets, visiting),
        RuleExpr::Optional(inner) => {
            let mut fs = first_of_expr(inner, grammar, sets, visiting);
            fs.can_be_empty = true;
            fs
        }
        RuleExpr::Field(_, inner) | RuleExpr::Token(inner) => {
            first_of_expr(inner, grammar, sets, visiting)
        }
        RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => first_of_expr(inner, grammar, sets, visiting),
        RuleExpr::Blank => {
            let mut fs = FirstSet::default();
            fs.can_be_empty = true;
            fs
        }
    }
}

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
    ";", "}", ")", "]", "fn", "let", "if", "while", "for", "return", "struct", "enum", "impl",
    "trait", "use", "mod", "pub", "def", "class", "elif", "else", "import", "from", "try",
    "except", "finally", "raise", "with", "pass", "break", "continue", "assert", "yield", "async",
];

/// A grammar-driven parser. Given a Grammar IR and source text, it produces
/// a lossless green tree by interpreting the grammar rules at runtime.
pub struct RuntimeParser {
    grammar: Grammar,
    lexer: RuntimeLexer,
    /// Precomputed FIRST sets for predictive parsing.
    first_sets: FxHashMap<SmolStr, FirstSet>,
}

impl RuntimeParser {
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        let first_sets = compute_first_sets(&grammar);
        Self {
            grammar,
            lexer,
            first_sets,
        }
    }

    pub fn parse(&self, source: &str) -> RuntimeParseResult {
        let tokens = self.lexer.tokenize(source);
        let mut ctx = ParseContext::new(&self.grammar, &tokens, source, &self.first_sets);

        let entry_rule = self
            .grammar
            .entry_rule
            .clone()
            .or_else(|| self.grammar.rules.keys().next().cloned())
            .unwrap_or_else(|| "source_file".into());

        ctx.builder.start_node(SyntaxKind::SOURCE_FILE);

        // Determine the repeating child rule from the entry rule.
        // If entry is `Module := Statement*`, we want to loop over Statement.
        let child_rule = self.extract_repeat_child(&entry_rule);

        match child_rule {
            Some(child) => {
                // Parse the entry rule's children individually with per-item recovery.
                let node_kind = rule_name_to_kind(&entry_rule);
                ctx.builder.start_node(node_kind);

                // Precompute the FIRST set of the child rule for fast rejection.
                let child_first = self
                    .first_sets
                    .get(child.as_str())
                    .cloned()
                    .unwrap_or_else(FirstSet::universal);

                while !ctx.at_eof() {
                    // Quick rejection: if the current token can't start the
                    // child rule, skip directly without attempting a parse.
                    let peek = ctx.skip_trivia_pos();
                    if !child_first.is_universal
                        && peek < ctx.tokens.len()
                        && ctx.tokens[peek].kind != RuntimeTokenKind::Eof
                        && !child_first.matches_token(&ctx.tokens[peek])
                    {
                        ctx.error_recover("unexpected token");
                        continue;
                    }

                    let before = ctx.pos;
                    if ctx.parse_rule(&child) {
                        continue;
                    }
                    // Failed to parse a child item — skip to recovery point.
                    if ctx.pos == before {
                        ctx.error_recover("unexpected token");
                    }
                }
                ctx.builder.finish_node();
            }
            None => {
                // Fallback: parse entry rule as-is, with recovery loop.
                while !ctx.at_eof() {
                    let before = ctx.pos;
                    ctx.parse_rule(&entry_rule);
                    if ctx.pos == before {
                        ctx.error_recover("unexpected token");
                    }
                }
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

    /// If the entry rule is `Foo := Bar*` or `Foo := Bar+`, return "Bar".
    /// This lets the top-level loop parse each Bar individually with recovery.
    fn extract_repeat_child(&self, entry_rule: &str) -> Option<String> {
        let rule = self.grammar.rules.get(entry_rule)?;
        match &rule.expr {
            RuleExpr::Repeat(inner) | RuleExpr::Repeat1(inner) => {
                if let RuleExpr::RuleRef(name) = inner.as_ref() {
                    Some(name.to_string())
                } else if let RuleExpr::Choice(_) = inner.as_ref() {
                    // Entry rule is `Foo := (A | B | C)*` — we can't extract
                    // a single child, but we can still benefit from looping.
                    None
                } else {
                    None
                }
            }
            // Entry rule like `Foo := Statement*` at the end of a Seq
            RuleExpr::Seq(parts) if parts.len() == 1 => {
                if let RuleExpr::Repeat(inner) | RuleExpr::Repeat1(inner) = &parts[0] {
                    if let RuleExpr::RuleRef(name) = inner.as_ref() {
                        return Some(name.to_string());
                    }
                }
                None
            }
            _ => None,
        }
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
    /// Precomputed FIRST sets for predictive lookahead.
    first_sets: &'a FxHashMap<SmolStr, FirstSet>,
    /// Negative parse cache: (rule_name, token_position) → known to fail.
    /// Avoids re-attempting rules that already failed at a given position.
    fail_cache: FxHashSet<(SmolStr, usize)>,
}

const MAX_DEPTH: u32 = 512;

impl<'a> ParseContext<'a> {
    fn new(
        grammar: &'a Grammar,
        tokens: &'a [RawToken],
        source: &'a str,
        first_sets: &'a FxHashMap<SmolStr, FirstSet>,
    ) -> Self {
        Self {
            grammar,
            tokens,
            source,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            in_progress: FxHashSet::default(),
            depth: 0,
            first_sets,
            fail_cache: FxHashSet::default(),
        }
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == RuntimeTokenKind::Eof
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
            RuntimeTokenKind::Indent | RuntimeTokenKind::Dedent | RuntimeTokenKind::Custom(_) => {
                SyntaxKind::IDENT
            }
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
        let mut brace_depth: i32 = 0;
        while !self.at_eof() && count < 50 {
            let text = self.peek_text();

            // Track brace/bracket nesting so we don't stop inside blocks.
            if text == "{" || text == "(" || text == "[" {
                brace_depth += 1;
            } else if text == "}" || text == ")" || text == "]" {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else if count > 0 {
                    // Stop at unmatched closer.
                    break;
                }
            }

            // Stop at recovery tokens only at the outer level.
            if count > 0 && brace_depth == 0 && self.is_recovery_token(text) {
                break;
            }

            self.bump();
            count += 1;
        }

        self.builder.finish_node();
        // After skipping tokens, invalidate fail-cache entries at positions
        // we've moved past.  Keep entries for positions ahead of us — they're
        // still valid.
        let new_pos = self.pos;
        self.fail_cache.retain(|(_rule, pos)| *pos >= new_pos);
    }

    /// Check if a token text is a recovery point (statement/item start).
    fn is_recovery_token(&self, text: &str) -> bool {
        // Check hardcoded recovery tokens first.
        if RECOVERY_TOKENS.contains(&text) {
            return true;
        }
        // Also recover at any grammar keyword that typically starts statements.
        if let Some(kw_text) = self.tokens.get(self.skip_trivia_pos()) {
            matches!(kw_text.kind, RuntimeTokenKind::Keyword(_))
        } else {
            false
        }
    }

    /// A literal is "recoverable" if it's a closing delimiter or separator
    /// that commonly goes missing (e.g. ";", ")", "}", ",", ":").
    /// Operators like "=>", "==", "+=" are NOT recoverable because inserting
    /// them silently would mask structural misparses.
    fn is_recoverable_literal(lit: &str) -> bool {
        matches!(
            lit,
            ";" | ")" | "}" | "]" | "," | ":" | ">" | "(" | "{" | "["
        )
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
            "INDENT" | "Indent" => return self.parse_builtin_indent(),
            "DEDENT" | "Dedent" => return self.parse_builtin_dedent(),
            _ => {}
        }

        if let Some(idx) = self
            .grammar
            .tokens
            .iter()
            .position(|t| t.name.as_str() == name)
        {
            return self.parse_custom_token(idx as u16);
        }

        // FIRST-set early rejection: if the lookahead token cannot start this
        // rule, bail out immediately without recursing into the rule body.
        if let Some(fs) = self.first_sets.get(name) {
            if !fs.is_universal && !fs.can_be_empty {
                let peek = self.skip_trivia_pos();
                if peek < self.tokens.len()
                    && self.tokens[peek].kind != RuntimeTokenKind::Eof
                    && !fs.matches_token(&self.tokens[peek])
                {
                    return false;
                }
            }
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

        // Negative memoization: if we already failed this rule at this position, skip.
        if self.fail_cache.contains(&guard_key) {
            return false;
        }

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
            // If we consumed tokens but still failed, keep what we have as a
            // partial node rather than throwing it all away. This preserves
            // tree structure for error recovery.
            if self.pos > save_pos {
                // We made progress — finish the node with what we have.
                self.builder.finish_node();
            } else {
                // No progress at all — full rollback.
                self.builder.rollback(save_builder);
                self.pos = save_pos;
                // Cache the failure so we don't retry this rule at this position.
                self.fail_cache.insert(guard_key.clone());
            }
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
        if let RuleExpr::Seq(parts) = inner
            && parts.len() == 3
        {
            return self.parse_binary_left(parts);
        }
        self.parse_expr(inner)
    }

    fn parse_prec_right(&mut self, _prec: i32, inner: &RuleExpr) -> bool {
        if let RuleExpr::Seq(parts) = inner
            && parts.len() == 3
        {
            return self.parse_binary_right(parts);
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
        if peek_pos >= self.tokens.len() || self.tokens[peek_pos].kind == RuntimeTokenKind::Eof {
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
                    // We already matched some children. Try to recover rather
                    // than failing the entire sequence.

                    // Case 1: missing closing/separator literal (e.g. ";", ")",
                    // "}", ",", ":"). Only recover for known delimiters, not
                    // operators like "=>" that disambiguate rules.
                    if let RuleExpr::Literal(lit) = expr {
                        if Self::is_recoverable_literal(lit) {
                            self.error_missing(&format!("'{lit}'"));
                            continue;
                        }
                    }

                    // Case 2: an optional-like position — if the remaining
                    // elements are all Optional, we can succeed here.
                    let all_remaining_optional = exprs[i..].iter().all(|e| {
                        matches!(e, RuleExpr::Optional(_) | RuleExpr::Repeat(_))
                    });
                    if all_remaining_optional {
                        return true;
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

        // Peek at the next non-trivia token for FIRST-set filtering.
        let peek_pos = self.skip_trivia_pos();
        let have_lookahead = peek_pos < self.tokens.len()
            && self.tokens[peek_pos].kind != RuntimeTokenKind::Eof;

        // Optimisation: detect the common "LongerAlt | ShorterAlt" pattern
        // where alternatives share a leading prefix (e.g.
        //   A B C | A   — parse A once then optionally try B C).
        // This avoids re-parsing the common prefix on backtrack.
        if exprs.len() == 2 {
            if let (RuleExpr::Seq(long), short) = (&exprs[0], &exprs[1]) {
                if long.len() >= 2 && *short == long[0] {
                    // Try the common prefix.
                    if self.try_parse_expr(short) {
                        // Prefix matched. Now try the suffix of the longer alternative.
                        let suffix_save = self.pos;
                        let suffix_save_builder = self.builder.checkpoint();
                        let suffix_save_errors = self.errors.len();
                        let mut suffix_ok = true;
                        for part in &long[1..] {
                            if !self.parse_expr(part) {
                                suffix_ok = false;
                                break;
                            }
                        }
                        if !suffix_ok {
                            // Suffix failed — rollback to after the prefix.
                            self.pos = suffix_save;
                            self.errors.truncate(suffix_save_errors);
                            self.builder.rollback(suffix_save_builder);
                        }
                        // Either way, the prefix succeeded — return true.
                        return true;
                    }
                    // Prefix failed — fall through to normal logic.
                    self.pos = save_pos;
                    self.errors.truncate(save_errors);
                    self.builder.rollback(save_builder);
                    return false;
                }
            }
        }

        for expr in exprs {
            // Predictive lookahead: skip alternatives whose FIRST set
            // doesn't include the current token.
            if have_lookahead {
                let fs = self.first_of_expr_cached(expr);
                if !fs.is_universal && !fs.can_be_empty && !fs.matches_token(&self.tokens[peek_pos])
                {
                    continue;
                }
            }

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

    /// Get the FIRST set for an expression, using precomputed sets for RuleRefs.
    fn first_of_expr_cached(&self, expr: &RuleExpr) -> FirstSet {
        match expr {
            RuleExpr::Literal(s) => {
                let mut fs = FirstSet::default();
                fs.literals.insert(s.clone());
                fs
            }
            RuleExpr::RuleRef(name) => {
                if let Some(fs) = self.first_sets.get(name) {
                    fs.clone()
                } else {
                    // Builtins and unknowns.
                    match name.as_str() {
                        "Identifier" | "identifier" | "_identifier" => {
                            let mut fs = FirstSet::default();
                            fs.can_match_ident = true;
                            fs
                        }
                        "Integer" | "integer" | "number" => {
                            let mut fs = FirstSet::default();
                            fs.can_match_int = true;
                            fs
                        }
                        "Float" | "float" => {
                            let mut fs = FirstSet::default();
                            fs.can_match_float = true;
                            fs
                        }
                        "String" | "string" => {
                            let mut fs = FirstSet::default();
                            fs.can_match_string = true;
                            fs
                        }
                        _ => FirstSet::universal(),
                    }
                }
            }
            RuleExpr::Seq(parts) if !parts.is_empty() => self.first_of_expr_cached(&parts[0]),
            RuleExpr::Choice(alts) => {
                let mut fs = FirstSet::default();
                for alt in alts {
                    fs.merge(&self.first_of_expr_cached(alt));
                }
                fs
            }
            RuleExpr::Repeat(inner) => {
                let mut fs = self.first_of_expr_cached(inner);
                fs.can_be_empty = true;
                fs
            }
            RuleExpr::Repeat1(inner) => self.first_of_expr_cached(inner),
            RuleExpr::Optional(inner) => {
                let mut fs = self.first_of_expr_cached(inner);
                fs.can_be_empty = true;
                fs
            }
            RuleExpr::Field(_, inner) | RuleExpr::Token(inner) => {
                self.first_of_expr_cached(inner)
            }
            RuleExpr::Prec(_, inner)
            | RuleExpr::PrecLeft(_, inner)
            | RuleExpr::PrecRight(_, inner) => self.first_of_expr_cached(inner),
            _ => FirstSet::universal(),
        }
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

    fn parse_builtin_indent(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::Indent {
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_builtin_dedent(&mut self) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::Dedent {
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_custom_token(&mut self, token_id: u16) -> bool {
        let peek = self.skip_trivia_pos();
        if peek >= self.tokens.len() {
            return false;
        }
        if self.tokens[peek].kind == RuntimeTokenKind::Custom(token_id) {
            self.bump();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
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
