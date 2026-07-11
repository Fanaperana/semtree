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
    fn matches_token(&self, tok: &RawToken, source: &str) -> bool {
        if self.is_universal {
            return true;
        }
        let text = tok.text(source);
        if self.literals.contains(text) {
            return true;
        }
        match tok.kind {
            RuntimeTokenKind::Ident | RuntimeTokenKind::Keyword(_) => self.can_match_ident,
            RuntimeTokenKind::Integer => self.can_match_int,
            RuntimeTokenKind::Float => self.can_match_float,
            RuntimeTokenKind::StringLit => self.can_match_string,
            RuntimeTokenKind::Indent => self.literals.contains("INDENT"),
            RuntimeTokenKind::Dedent => self.literals.contains("DEDENT"),
            _ => false,
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
                    return FirstSet {
                        can_match_ident: true,
                        ..Default::default()
                    };
                }
                "Integer" | "integer" | "number" => {
                    return FirstSet {
                        can_match_int: true,
                        ..Default::default()
                    };
                }
                "Float" | "float" => {
                    return FirstSet {
                        can_match_float: true,
                        ..Default::default()
                    };
                }
                "String" | "string" => {
                    return FirstSet {
                        can_match_string: true,
                        ..Default::default()
                    };
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
            if grammar
                .tokens
                .iter()
                .any(|t| t.name.as_str() == name.as_str())
            {
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
        RuleExpr::Prec(_, inner) | RuleExpr::PrecLeft(_, inner) | RuleExpr::PrecRight(_, inner) => {
            first_of_expr(inner, grammar, sets, visiting)
        }
        RuleExpr::Blank => FirstSet {
            can_be_empty: true,
            ..Default::default()
        },
    }
}

/// Get a stable identity for a RuleExpr based on its pointer address.
#[inline]
fn expr_id(expr: &RuleExpr) -> ExprId {
    ExprId(expr as *const RuleExpr as usize)
}

/// Check if a rule expression is a "pure dispatch" — a Choice where every
/// alternative is a single RuleRef. Such rules are just routing and don't
/// need their own node in the tree.
fn is_pure_dispatch(expr: &RuleExpr) -> bool {
    match expr {
        RuleExpr::Choice(alts) => {
            alts.len() >= 2 && alts.iter().all(|a| matches!(a, RuleExpr::RuleRef(_)))
        }
        _ => false,
    }
}

/// Check if a rule should collapse when it produces a single child node.
///
/// Targets precedence-chain links written as `Head Tail*` or `Head Suffix?`:
/// the expression is a `Seq` whose only mandatory element is a single
/// `RuleRef`, with every other element being `Optional`/`Repeat`. When no tail
/// matches at runtime the wrapper node holds exactly one child and adds nothing,
/// so it is spliced away (see `GreenNodeBuilder::finish_node_collapse_single`).
/// Rules with field bindings or a non-`RuleRef` mandatory core are NOT
/// collapsible — they anchor typed-AST accessors, queries, and field access.
fn is_collapsible_shape(expr: &RuleExpr) -> bool {
    match expr {
        RuleExpr::Seq(parts) => {
            let mut mandatory = parts.iter().filter(|p| {
                !matches!(
                    p,
                    RuleExpr::Optional(_) | RuleExpr::Repeat(_) | RuleExpr::Blank
                )
            });
            matches!(
                (mandatory.next(), mandatory.next()),
                (Some(RuleExpr::RuleRef(_)), None)
            )
        }
        _ => false,
    }
}

/// Whether an expression can match the empty string. Conservative: when the
/// nullability of a sub-rule is unknown it is treated as non-nullable only for
/// consuming constructs, which keeps `collect_left_corner` an over-approximation
/// (so any genuinely left-recursive rule is still detected).
fn expr_nullable(expr: &RuleExpr, first_sets: &FxHashMap<SmolStr, FirstSet>) -> bool {
    match expr {
        RuleExpr::Blank | RuleExpr::Optional(_) | RuleExpr::Repeat(_) => true,
        RuleExpr::Literal(_) => false,
        RuleExpr::RuleRef(name) => first_sets.get(name).map(|f| f.can_be_empty).unwrap_or(false),
        RuleExpr::Seq(parts) => parts.iter().all(|p| expr_nullable(p, first_sets)),
        RuleExpr::Choice(alts) => alts.iter().any(|a| expr_nullable(a, first_sets)),
        RuleExpr::Repeat1(inner)
        | RuleExpr::Field(_, inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => expr_nullable(inner, first_sets),
    }
}

/// Collect the "left corner" of an expression: rules that can appear as the
/// first matched symbol (before consuming any token). Used for left-recursion
/// detection.
fn collect_left_corner(
    expr: &RuleExpr,
    first_sets: &FxHashMap<SmolStr, FirstSet>,
    out: &mut FxHashSet<SmolStr>,
) {
    match expr {
        RuleExpr::RuleRef(name) => {
            out.insert(name.clone());
        }
        RuleExpr::Seq(parts) => {
            for p in parts {
                collect_left_corner(p, first_sets, out);
                if !expr_nullable(p, first_sets) {
                    break;
                }
            }
        }
        RuleExpr::Choice(alts) => {
            for a in alts {
                collect_left_corner(a, first_sets, out);
            }
        }
        RuleExpr::Optional(inner)
        | RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Field(_, inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => collect_left_corner(inner, first_sets, out),
        RuleExpr::Literal(_) | RuleExpr::Blank => {}
    }
}

/// Compute the set of rules that can (directly or indirectly) left-recurse.
/// Only these need the `in_progress` guard during parsing; skipping it for the
/// rest avoids hash-set churn on the common right-recursive / precedence-chain
/// grammars. Over-approximates (never misses a real left-recursion), and the
/// MAX_DEPTH guard remains a backstop.
fn compute_left_recursive(
    grammar: &Grammar,
    first_sets: &FxHashMap<SmolStr, FirstSet>,
) -> FxHashSet<SmolStr> {
    let mut left_corner: FxHashMap<SmolStr, FxHashSet<SmolStr>> = FxHashMap::default();
    for (name, rule) in &grammar.rules {
        let mut lc = FxHashSet::default();
        collect_left_corner(&rule.expr, first_sets, &mut lc);
        left_corner.insert(name.clone(), lc);
    }

    let mut result = FxHashSet::default();
    for name in grammar.rules.keys() {
        let mut visited: FxHashSet<SmolStr> = FxHashSet::default();
        let mut stack: Vec<SmolStr> = left_corner
            .get(name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        while let Some(cur) = stack.pop() {
            if &cur == name {
                result.insert(name.clone());
                break;
            }
            if !visited.insert(cur.clone()) {
                continue;
            }
            if let Some(next) = left_corner.get(&cur) {
                stack.extend(next.iter().cloned());
            }
        }
    }
    result
}

/// Per-rule data indexed by rule_idx (u16) for O(1) access.
struct RuleData {
    name: SmolStr,
    first_set: FirstSet,
    is_transparent: bool,
    /// Collapse this rule's node when it produced a single child node (elides
    /// empty precedence-chain links). See `is_collapsible_shape`.
    collapse_single_child: bool,
    /// Whether this rule can left-recurse. Only these need the `in_progress`
    /// guard; skipping it elsewhere avoids per-rule hash-set churn.
    may_left_recur: bool,
    syntax_kind: SyntaxKind,
}

/// Pre-resolved target of a `RuleExpr::RuleRef`.
/// Eliminates per-call string matching, linear scans, and HashMap lookups.
#[derive(Debug, Clone, Copy)]
enum ResolvedRef {
    /// A builtin like Identifier, Integer, Float, String, INDENT, DEDENT.
    Builtin(BuiltinKind),
    /// A custom token definition (index into grammar.tokens).
    CustomToken(u16),
    /// A grammar rule (index into rule_data / rule_expr_ptrs).
    Rule(u16),
}

#[derive(Debug, Clone, Copy)]
enum BuiltinKind {
    Ident,
    Integer,
    Float,
    String,
    Indent,
    Dedent,
}

/// Recursively walk all expressions in a rule, computing and caching FIRST sets.
fn precompute_expr_first(
    expr: &RuleExpr,
    first_sets: &FxHashMap<SmolStr, FirstSet>,
    cache: &mut FxHashMap<ExprId, FirstSet>,
) {
    let eid = expr_id(expr);
    if cache.contains_key(&eid) {
        return;
    }
    let fs = first_of_expr_static(expr, first_sets);
    cache.insert(eid, fs);

    // Recurse into children.
    match expr {
        RuleExpr::Seq(parts) | RuleExpr::Choice(parts) => {
            for part in parts {
                precompute_expr_first(part, first_sets, cache);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Field(_, inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => {
            precompute_expr_first(inner, first_sets, cache);
        }
        RuleExpr::Literal(_) | RuleExpr::RuleRef(_) | RuleExpr::Blank => {}
    }
}

/// Resolve a rule name to a `ResolvedRef` target.
fn resolve_name(
    name: &str,
    rule_indices: &FxHashMap<SmolStr, u16>,
    custom_token_indices: &FxHashMap<SmolStr, u16>,
) -> ResolvedRef {
    match name {
        "Identifier" | "identifier" | "_identifier" => ResolvedRef::Builtin(BuiltinKind::Ident),
        "Integer" | "integer" | "number" => ResolvedRef::Builtin(BuiltinKind::Integer),
        "Float" | "float" => ResolvedRef::Builtin(BuiltinKind::Float),
        "String" | "string" => ResolvedRef::Builtin(BuiltinKind::String),
        "INDENT" | "Indent" => ResolvedRef::Builtin(BuiltinKind::Indent),
        "DEDENT" | "Dedent" => ResolvedRef::Builtin(BuiltinKind::Dedent),
        _ => {
            if let Some(&idx) = custom_token_indices.get(name) {
                ResolvedRef::CustomToken(idx)
            } else if let Some(&idx) = rule_indices.get(name) {
                ResolvedRef::Rule(idx)
            } else {
                // Will produce an error at parse time.
                ResolvedRef::Rule(u16::MAX)
            }
        }
    }
}

/// Recursively walk all expressions in a rule, resolving RuleRef targets.
fn resolve_refs_in_expr(
    expr: &RuleExpr,
    rule_indices: &FxHashMap<SmolStr, u16>,
    custom_token_indices: &FxHashMap<SmolStr, u16>,
    cache: &mut FxHashMap<ExprId, ResolvedRef>,
) {
    match expr {
        RuleExpr::RuleRef(name) => {
            let eid = expr_id(expr);
            cache
                .entry(eid)
                .or_insert_with(|| resolve_name(name, rule_indices, custom_token_indices));
        }
        RuleExpr::Seq(parts) | RuleExpr::Choice(parts) => {
            for part in parts {
                resolve_refs_in_expr(part, rule_indices, custom_token_indices, cache);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Field(_, inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => {
            resolve_refs_in_expr(inner, rule_indices, custom_token_indices, cache);
        }
        RuleExpr::Literal(_) | RuleExpr::Blank => {}
    }
}

/// Compute the FIRST set for an expression using only the precomputed per-rule sets.
fn first_of_expr_static(expr: &RuleExpr, first_sets: &FxHashMap<SmolStr, FirstSet>) -> FirstSet {
    match expr {
        RuleExpr::Literal(s) => {
            let mut fs = FirstSet::default();
            fs.literals.insert(s.clone());
            fs
        }
        RuleExpr::RuleRef(name) => {
            if let Some(fs) = first_sets.get(name) {
                fs.clone()
            } else {
                match name.as_str() {
                    "Identifier" | "identifier" | "_identifier" => FirstSet {
                        can_match_ident: true,
                        ..Default::default()
                    },
                    "Integer" | "integer" | "number" => FirstSet {
                        can_match_int: true,
                        ..Default::default()
                    },
                    "Float" | "float" => FirstSet {
                        can_match_float: true,
                        ..Default::default()
                    },
                    "String" | "string" => FirstSet {
                        can_match_string: true,
                        ..Default::default()
                    },
                    _ => FirstSet::universal(),
                }
            }
        }
        RuleExpr::Seq(parts) if !parts.is_empty() => {
            let mut fs = FirstSet::default();
            for part in parts {
                let pf = first_of_expr_static(part, first_sets);
                let can_be_empty = pf.can_be_empty;
                fs.merge(&pf);
                fs.can_be_empty = false;
                if !can_be_empty {
                    return fs;
                }
            }
            fs.can_be_empty = true;
            fs
        }
        RuleExpr::Choice(alts) => {
            let mut fs = FirstSet::default();
            for alt in alts {
                fs.merge(&first_of_expr_static(alt, first_sets));
            }
            fs
        }
        RuleExpr::Repeat(inner) => {
            let mut fs = first_of_expr_static(inner, first_sets);
            fs.can_be_empty = true;
            fs
        }
        RuleExpr::Repeat1(inner) => first_of_expr_static(inner, first_sets),
        RuleExpr::Optional(inner) => {
            let mut fs = first_of_expr_static(inner, first_sets);
            fs.can_be_empty = true;
            fs
        }
        RuleExpr::Field(_, inner) | RuleExpr::Token(inner) => {
            first_of_expr_static(inner, first_sets)
        }
        RuleExpr::Prec(_, inner) | RuleExpr::PrecLeft(_, inner) | RuleExpr::PrecRight(_, inner) => {
            first_of_expr_static(inner, first_sets)
        }
        _ => FirstSet::universal(),
    }
}

/// Pre-computed dispatch table for a Choice expression.
/// Groups alternative indices by which token type they accept,
/// enabling O(1) dispatch instead of O(N) FIRST-set scanning.
struct ChoiceDispatch {
    /// Literal text → list of alternative indices that accept it.
    literal_map: FxHashMap<SmolStr, Vec<u8>>,
    /// Alternative indices that accept Ident tokens.
    ident_alts: Vec<u8>,
    /// Alternative indices that accept Integer tokens.
    int_alts: Vec<u8>,
    /// Alternative indices that accept Float tokens.
    float_alts: Vec<u8>,
    /// Alternative indices that accept String tokens.
    string_alts: Vec<u8>,
    /// Alternative indices that are "universal" (accept anything or can be empty).
    universal_alts: Vec<u8>,
}

impl ChoiceDispatch {
    /// Get the list of alternative indices to try for the given token.
    #[inline]
    fn candidates_for(&self, tok: &RawToken, source: &str) -> (&[u8], &[u8]) {
        let specific = match tok.kind {
            RuntimeTokenKind::Ident | RuntimeTokenKind::Keyword(_) => {
                // Check both ident_alts and literal_map for the specific text.
                let text = tok.text(source);
                if let Some(lit_alts) = self.literal_map.get(text) {
                    return (lit_alts.as_slice(), &self.ident_alts);
                }
                self.ident_alts.as_slice()
            }
            RuntimeTokenKind::Literal(_) => {
                let text = tok.text(source);
                if let Some(lit_alts) = self.literal_map.get(text) {
                    return (lit_alts.as_slice(), &[]);
                }
                &[]
            }
            RuntimeTokenKind::Integer => self.int_alts.as_slice(),
            RuntimeTokenKind::Float => self.float_alts.as_slice(),
            RuntimeTokenKind::StringLit => self.string_alts.as_slice(),
            RuntimeTokenKind::Indent => {
                if let Some(alts) = self.literal_map.get("INDENT") {
                    return (alts.as_slice(), &[]);
                }
                &[]
            }
            RuntimeTokenKind::Dedent => {
                if let Some(alts) = self.literal_map.get("DEDENT") {
                    return (alts.as_slice(), &[]);
                }
                &[]
            }
            _ => &[],
        };
        (specific, &[])
    }
}

/// Build a ChoiceDispatch for a Choice expression.
fn build_choice_dispatch(
    alts: &[RuleExpr],
    expr_first_cache: &FxHashMap<ExprId, FirstSet>,
) -> ChoiceDispatch {
    let mut literal_map: FxHashMap<SmolStr, Vec<u8>> = FxHashMap::default();
    let mut ident_alts = Vec::new();
    let mut int_alts = Vec::new();
    let mut float_alts = Vec::new();
    let mut string_alts = Vec::new();
    let mut universal_alts = Vec::new();

    for (i, alt) in alts.iter().enumerate() {
        if i >= 255 {
            break;
        } // u8 index limit
        let idx = i as u8;
        let eid = expr_id(alt);
        if let Some(fs) = expr_first_cache.get(&eid) {
            if fs.is_universal || fs.can_be_empty {
                universal_alts.push(idx);
                continue;
            }
            for lit in &fs.literals {
                literal_map.entry(lit.clone()).or_default().push(idx);
            }
            if fs.can_match_ident {
                ident_alts.push(idx);
            }
            if fs.can_match_int {
                int_alts.push(idx);
            }
            if fs.can_match_float {
                float_alts.push(idx);
            }
            if fs.can_match_string {
                string_alts.push(idx);
            }
        } else {
            universal_alts.push(idx);
        }
    }

    ChoiceDispatch {
        literal_map,
        ident_alts,
        int_alts,
        float_alts,
        string_alts,
        universal_alts,
    }
}

/// Precompute ChoiceDispatch tables for all Choice expressions in the grammar.
fn precompute_choice_dispatch(
    grammar: &Grammar,
    expr_first_cache: &FxHashMap<ExprId, FirstSet>,
) -> FxHashMap<ExprId, ChoiceDispatch> {
    let mut map = FxHashMap::default();
    for rule in grammar.rules.values() {
        precompute_choice_dispatch_expr(&rule.expr, expr_first_cache, &mut map);
    }
    map
}

fn precompute_choice_dispatch_expr(
    expr: &RuleExpr,
    expr_first_cache: &FxHashMap<ExprId, FirstSet>,
    map: &mut FxHashMap<ExprId, ChoiceDispatch>,
) {
    match expr {
        RuleExpr::Choice(alts) => {
            let eid = expr_id(expr);
            if !map.contains_key(&eid) && alts.len() >= 3 {
                map.insert(eid, build_choice_dispatch(alts, expr_first_cache));
            }
            for alt in alts {
                precompute_choice_dispatch_expr(alt, expr_first_cache, map);
            }
        }
        RuleExpr::Seq(parts) => {
            for part in parts {
                precompute_choice_dispatch_expr(part, expr_first_cache, map);
            }
        }
        RuleExpr::Repeat(inner)
        | RuleExpr::Repeat1(inner)
        | RuleExpr::Optional(inner)
        | RuleExpr::Field(_, inner)
        | RuleExpr::Token(inner)
        | RuleExpr::Prec(_, inner)
        | RuleExpr::PrecLeft(_, inner)
        | RuleExpr::PrecRight(_, inner) => {
            precompute_choice_dispatch_expr(inner, expr_first_cache, map);
        }
        _ => {}
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
    /// Rule name → compact u16 index for fast set operations.
    rule_indices: FxHashMap<SmolStr, u16>,
    /// Precomputed FIRST sets keyed by expression identity (pointer-based).
    /// Built once in `new()` by walking every expression in the grammar.
    expr_first_cache: FxHashMap<ExprId, FirstSet>,
    /// Per-rule data indexed by rule_idx (u16).
    rule_data: Vec<RuleData>,
    /// Raw pointers to rule expressions, indexed by rule_idx.
    /// Safe because Grammar is owned by RuntimeParser and outlives all uses.
    rule_expr_ptrs: Vec<*const RuleExpr>,
    /// Pre-resolved RuleRef targets keyed by ExprId (pointer identity).
    /// Eliminates per-call string matching, linear token scans, and HashMap lookups.
    resolved_refs: FxHashMap<ExprId, ResolvedRef>,
    /// Custom token name → index for O(1) lookup (replaces linear scan).
    #[allow(dead_code)]
    custom_token_indices: FxHashMap<SmolStr, u16>,
    /// Pre-computed dispatch tables for Choice expressions (≥3 alternatives).
    choice_dispatch: FxHashMap<ExprId, ChoiceDispatch>,
}

/// Identity key for a RuleExpr based on its address in the Grammar IR.
/// Safe because the Grammar (and therefore its RuleExpr nodes) lives as long
/// as the RuntimeParser that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ExprId(usize);

impl RuntimeParser {
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        let first_sets = compute_first_sets(&grammar);

        // Build rule name → u16 index mapping.
        let mut rule_indices = FxHashMap::default();
        for (i, name) in grammar.rules.keys().enumerate() {
            rule_indices.insert(name.clone(), i as u16);
        }

        // Precompute FIRST sets for every expression node in the grammar.
        let mut expr_first_cache = FxHashMap::default();
        for rule in grammar.rules.values() {
            precompute_expr_first(&rule.expr, &first_sets, &mut expr_first_cache);
        }

        // Identify transparent rules: pure dispatch rules (Choice of RuleRefs),
        // rules starting with '_', or single-RuleRef aliases (e.g. `Foo := Bar`).
        let mut transparent_set = FxHashSet::default();
        for (name, rule) in &grammar.rules {
            if name.starts_with('_')
                || is_pure_dispatch(&rule.expr)
                || matches!(&rule.expr, RuleExpr::RuleRef(_))
            {
                transparent_set.insert(name.clone());
            }
        }
        // Never make the entry rule transparent.
        if let Some(entry) = &grammar.entry_rule {
            transparent_set.remove(entry);
        }

        // Identify collapsible rules: single-child precedence-chain links whose
        // wrapper node is redundant when no tail/suffix matched. Excludes rules
        // with fields (AST/query anchors) and the entry rule.
        let mut collapsible_set = FxHashSet::default();
        for (name, rule) in &grammar.rules {
            if rule.fields.is_empty() && is_collapsible_shape(&rule.expr) {
                collapsible_set.insert(name.clone());
            }
        }
        if let Some(entry) = &grammar.entry_rule {
            collapsible_set.remove(entry);
        }

        // Rules that can left-recurse need the in_progress guard; others skip it.
        let left_recursive = compute_left_recursive(&grammar, &first_sets);

        // Build indexed rule data and expression pointers.
        let num_rules = rule_indices.len();
        let mut rule_data = Vec::with_capacity(num_rules);
        let mut rule_expr_ptrs: Vec<*const RuleExpr> = vec![std::ptr::null(); num_rules];
        // Initialize rule_data with defaults.
        rule_data.resize_with(num_rules, || RuleData {
            name: SmolStr::default(),
            first_set: FirstSet::default(),
            is_transparent: false,
            collapse_single_child: false,
            may_left_recur: true,
            syntax_kind: SyntaxKind::ERROR,
        });
        for (name, &idx) in &rule_indices {
            let i = idx as usize;
            rule_data[i].name = name.clone();
            rule_data[i].first_set = first_sets
                .get(name)
                .cloned()
                .unwrap_or_else(FirstSet::universal);
            rule_data[i].is_transparent = transparent_set.contains(name);
            rule_data[i].collapse_single_child =
                collapsible_set.contains(name) && !transparent_set.contains(name);
            rule_data[i].may_left_recur = left_recursive.contains(name);
            rule_data[i].syntax_kind = rule_name_to_kind(name);
            if let Some(rule) = grammar.rules.get(name) {
                rule_expr_ptrs[i] = &rule.expr as *const RuleExpr;
            }
        }

        // Build custom token name → index map for O(1) lookup.
        let mut custom_token_indices = FxHashMap::default();
        for (i, tok) in grammar.tokens.iter().enumerate() {
            custom_token_indices.insert(tok.name.clone(), i as u16);
        }

        // Pre-resolve all RuleRef targets in the grammar.
        let mut resolved_refs = FxHashMap::default();
        for rule in grammar.rules.values() {
            resolve_refs_in_expr(
                &rule.expr,
                &rule_indices,
                &custom_token_indices,
                &mut resolved_refs,
            );
        }

        // Pre-compute choice dispatch tables for all Choice expressions.
        let choice_dispatch = precompute_choice_dispatch(&grammar, &expr_first_cache);

        Self {
            grammar,
            lexer,
            rule_indices,
            expr_first_cache,
            rule_data,
            rule_expr_ptrs,
            resolved_refs,
            custom_token_indices,
            choice_dispatch,
        }
    }

    pub fn parse(&self, source: &str) -> RuntimeParseResult {
        let tokens = self.lexer.tokenize(source);
        let mut ctx = ParseContext::new(
            &tokens,
            source,
            &self.rule_indices,
            &self.expr_first_cache,
            &self.rule_data,
            &self.rule_expr_ptrs,
            &self.resolved_refs,
            &self.choice_dispatch,
        );

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
                    .rule_indices
                    .get(child.as_str())
                    .and_then(|&idx| self.rule_data.get(idx as usize))
                    .map(|rd| rd.first_set.clone())
                    .unwrap_or_else(FirstSet::universal);

                while !ctx.at_eof() {
                    // Quick rejection: if the current token can't start the
                    // child rule, skip directly without attempting a parse.
                    let peek = ctx.skip_trivia_pos();
                    if !child_first.is_universal
                        && peek < ctx.tokens.len()
                        && ctx.tokens[peek].kind != RuntimeTokenKind::Eof
                        && !child_first.matches_token(&ctx.tokens[peek], source)
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
                if let RuleExpr::Repeat(inner) | RuleExpr::Repeat1(inner) = &parts[0]
                    && let RuleExpr::RuleRef(name) = inner.as_ref()
                {
                    return Some(name.to_string());
                }
                None
            }
            _ => None,
        }
    }
}

struct ParseContext<'a> {
    tokens: &'a [RawToken],
    source: &'a str,
    pos: usize,
    builder: GreenNodeBuilder,
    errors: Vec<RuntimeParseError>,
    /// Nesting depth guard to prevent stack overflow on deep recursion.
    depth: u32,
    /// Rule name → compact u16 index.
    rule_indices: &'a FxHashMap<SmolStr, u16>,
    /// Precomputed FIRST sets per expression (by pointer identity).
    expr_first_cache: &'a FxHashMap<ExprId, FirstSet>,
    /// Per-rule data indexed by rule_idx.
    rule_data: &'a [RuleData],
    /// Raw pointers to rule expressions, indexed by rule_idx.
    rule_expr_ptrs: &'a [*const RuleExpr],
    /// Pre-resolved RuleRef targets.
    resolved_refs: &'a FxHashMap<ExprId, ResolvedRef>,
    /// Pre-computed choice dispatch tables.
    choice_dispatch: &'a FxHashMap<ExprId, ChoiceDispatch>,
    /// Sparse memo of (rule_idx, pos) pairs currently on the parse stack
    /// (left-recursion guard). Packed key: `(rule as u64) << 32 | pos`.
    in_progress: FxHashSet<u64>,
    /// Sparse memo of (rule_idx, pos) pairs known to fail at that position.
    fail_cache: FxHashSet<u64>,
    /// Cached skip_trivia result: (from_pos, to_pos).
    /// Avoids re-scanning trivia from the same position.
    trivia_cache_from: usize,
    trivia_cache_to: usize,
}

const MAX_DEPTH: u32 = 512;

impl<'a> ParseContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        tokens: &'a [RawToken],
        source: &'a str,
        rule_indices: &'a FxHashMap<SmolStr, u16>,
        expr_first_cache: &'a FxHashMap<ExprId, FirstSet>,
        rule_data: &'a [RuleData],
        rule_expr_ptrs: &'a [*const RuleExpr],
        resolved_refs: &'a FxHashMap<ExprId, ResolvedRef>,
        choice_dispatch: &'a FxHashMap<ExprId, ChoiceDispatch>,
    ) -> Self {
        Self {
            tokens,
            source,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
            depth: 0,
            rule_indices,
            expr_first_cache,
            rule_data,
            rule_expr_ptrs,
            resolved_refs,
            choice_dispatch,
            in_progress: FxHashSet::default(),
            fail_cache: FxHashSet::default(),
            trivia_cache_from: usize::MAX,
            trivia_cache_to: 0,
        }
    }

    #[inline]
    fn memo_key(rule_idx: u16, pos: u32) -> u64 {
        ((rule_idx as u64) << 32) | pos as u64
    }

    #[inline]
    fn in_progress_contains(&self, rule_idx: u16, pos: u32) -> bool {
        self.in_progress.contains(&Self::memo_key(rule_idx, pos))
    }

    #[inline]
    fn in_progress_insert(&mut self, rule_idx: u16, pos: u32) {
        self.in_progress.insert(Self::memo_key(rule_idx, pos));
    }

    #[inline]
    fn in_progress_remove(&mut self, rule_idx: u16, pos: u32) {
        self.in_progress.remove(&Self::memo_key(rule_idx, pos));
    }

    #[inline]
    fn fail_cache_contains(&self, rule_idx: u16, pos: u32) -> bool {
        self.fail_cache.contains(&Self::memo_key(rule_idx, pos))
    }

    #[inline]
    fn fail_cache_insert(&mut self, rule_idx: u16, pos: u32) {
        self.fail_cache.insert(Self::memo_key(rule_idx, pos));
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == RuntimeTokenKind::Eof
    }

    #[allow(dead_code)]
    fn current_text(&self) -> &str {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].text(self.source)
        } else {
            ""
        }
    }

    #[inline]
    fn skip_trivia_pos(&mut self) -> usize {
        let from = self.pos;
        if from == self.trivia_cache_from {
            return self.trivia_cache_to;
        }
        let mut i = from;
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        self.trivia_cache_from = from;
        self.trivia_cache_to = i;
        i
    }

    #[allow(dead_code)]
    fn peek_text(&mut self) -> &str {
        let i = self.skip_trivia_pos();
        if i < self.tokens.len() {
            self.tokens[i].text(self.source)
        } else {
            ""
        }
    }

    #[allow(dead_code)]
    fn peek_kind(&mut self) -> RuntimeTokenKind {
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
            self.builder.token(kind, tok.text(self.source));
            self.pos += 1;
        }
    }

    fn bump(&mut self) {
        self.eat_trivia();
        if !self.at_eof() {
            let tok = &self.tokens[self.pos];
            let kind = self.token_to_syntax_kind(tok);
            self.builder.token(kind, tok.text(self.source));
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
            // Inline trivia skip + text access to avoid borrow conflicts.
            let mut ti = self.pos;
            while ti < self.tokens.len() && self.tokens[ti].kind.is_trivia() {
                ti += 1;
            }
            let text: &str = if ti < self.tokens.len() {
                self.tokens[ti].text(self.source)
            } else {
                ""
            };

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
        // After skipping tokens, clear the entire fail cache on recovery.
        self.fail_cache.clear();
    }

    /// Check if a token text is a recovery point (statement/item start).
    fn is_recovery_token(&mut self, text: &str) -> bool {
        // Check hardcoded recovery tokens first.
        if RECOVERY_TOKENS.contains(&text) {
            return true;
        }
        // Also recover at any grammar keyword that typically starts statements.
        let trivia_pos = self.skip_trivia_pos();
        if let Some(kw_text) = self.tokens.get(trivia_pos) {
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

    /// Parse a rule by name. Used for top-level entry and error messages.
    /// For RuleRef dispatch, prefer `parse_resolved_ref` which avoids lookups.
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

        // Single HashMap lookup to get rule_idx.
        let rule_idx = match self.rule_indices.get(name) {
            Some(&idx) => idx,
            None => {
                self.error_here(&format!("undefined rule: {name}"));
                return false;
            }
        };

        self.parse_rule_by_idx(rule_idx)
    }

    /// Fast-path: parse a rule by its pre-resolved index.
    /// All access is O(1) — no HashMap lookups, no string matching.
    #[inline]
    fn parse_rule_by_idx(&mut self, rule_idx: u16) -> bool {
        if self.depth > MAX_DEPTH {
            return false;
        }

        let rd = &self.rule_data[rule_idx as usize];

        // FIRST-set early rejection using indexed data.
        if !rd.first_set.is_universal && !rd.first_set.can_be_empty {
            let peek = self.skip_trivia_pos();
            if peek < self.tokens.len()
                && self.tokens[peek].kind != RuntimeTokenKind::Eof
                && !rd.first_set.matches_token(&self.tokens[peek], self.source)
            {
                return false;
            }
        }

        let pos = self.pos as u32;
        let may_left_recur = rd.may_left_recur;

        // fail_cache is a cheap memo of known failures; in_progress is the
        // left-recursion guard, needed only for rules that can left-recurse.
        if self.fail_cache_contains(rule_idx, pos) {
            return false;
        }
        if may_left_recur {
            if self.in_progress_contains(rule_idx, pos) {
                return false;
            }
            self.in_progress_insert(rule_idx, pos);
        }
        self.depth += 1;

        let save_pos = self.pos;
        let save_builder = self.builder.checkpoint();

        let is_transparent = rd.is_transparent;
        let collapse_single = rd.collapse_single_child;

        if !is_transparent {
            // Use pre-computed SyntaxKind — no hashing.
            self.builder.start_node(rd.syntax_kind);
        }

        // Use precomputed pointer to rule expression — avoids BTreeMap lookup.
        let expr = unsafe { &*self.rule_expr_ptrs[rule_idx as usize] };
        let matched = self.parse_expr(expr);

        if !matched {
            if self.pos > save_pos {
                if !is_transparent {
                    if collapse_single {
                        self.builder.finish_node_collapse_single();
                    } else {
                        self.builder.finish_node();
                    }
                }
            } else {
                self.builder.rollback(save_builder);
                self.pos = save_pos;
                self.fail_cache_insert(rule_idx, pos);
            }
            self.depth -= 1;
            if may_left_recur {
                self.in_progress_remove(rule_idx, pos);
            }
            return false;
        }

        if !is_transparent {
            if collapse_single {
                self.builder.finish_node_collapse_single();
            } else {
                self.builder.finish_node();
            }
        }
        self.depth -= 1;
        if may_left_recur {
            self.in_progress_remove(rule_idx, pos);
        }
        true
    }

    /// Dispatch a pre-resolved RuleRef target directly.
    #[inline]
    fn parse_resolved_ref(&mut self, resolved: ResolvedRef) -> bool {
        match resolved {
            ResolvedRef::Builtin(BuiltinKind::Ident) => self.parse_builtin_ident(),
            ResolvedRef::Builtin(BuiltinKind::Integer) => self.parse_builtin_integer(),
            ResolvedRef::Builtin(BuiltinKind::Float) => self.parse_builtin_float(),
            ResolvedRef::Builtin(BuiltinKind::String) => self.parse_builtin_string(),
            ResolvedRef::Builtin(BuiltinKind::Indent) => self.parse_builtin_indent(),
            ResolvedRef::Builtin(BuiltinKind::Dedent) => self.parse_builtin_dedent(),
            ResolvedRef::CustomToken(idx) => self.parse_custom_token(idx),
            ResolvedRef::Rule(idx) => {
                if idx == u16::MAX {
                    self.error_here("undefined rule");
                    false
                } else {
                    self.parse_rule_by_idx(idx)
                }
            }
        }
    }

    fn parse_expr(&mut self, expr: &RuleExpr) -> bool {
        match expr {
            RuleExpr::Literal(s) => self.parse_literal(s),
            RuleExpr::RuleRef(_) => {
                // Use pre-resolved target for O(1) dispatch.
                let eid = expr_id(expr);
                if let Some(&resolved) = self.resolved_refs.get(&eid) {
                    self.parse_resolved_ref(resolved)
                } else {
                    // Fallback (should not happen for well-formed grammars).
                    if let RuleExpr::RuleRef(name) = expr {
                        self.parse_rule(name)
                    } else {
                        false
                    }
                }
            }
            RuleExpr::Seq(exprs) => self.parse_seq(exprs),
            RuleExpr::Choice(exprs) => self.parse_choice(expr, exprs),
            RuleExpr::Repeat(inner) => self.parse_repeat(inner),
            RuleExpr::Repeat1(inner) => self.parse_repeat1(inner),
            RuleExpr::Optional(inner) => {
                let save_pos = self.pos;
                let save_builder = self.builder.checkpoint();
                let save_errors = self.errors.len();
                if !self.parse_expr(inner) {
                    // Rollback any partial progress.
                    self.pos = save_pos;
                    self.errors.truncate(save_errors);
                    self.builder.rollback(save_builder);
                }
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
        if self.tokens[peek_pos].text(self.source) == expected {
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
                    if let RuleExpr::Literal(lit) = expr
                        && Self::is_recoverable_literal(lit)
                    {
                        self.error_missing(&format!("'{lit}'"));
                        continue;
                    }

                    // Case 2: an optional-like position — if the remaining
                    // elements are all Optional, we can succeed here.
                    let all_remaining_optional = exprs[i..]
                        .iter()
                        .all(|e| matches!(e, RuleExpr::Optional(_) | RuleExpr::Repeat(_)));
                    if all_remaining_optional {
                        return true;
                    }
                }
                return false;
            }
        }
        true
    }

    fn parse_choice(&mut self, choice_expr: &RuleExpr, exprs: &[RuleExpr]) -> bool {
        let save_pos = self.pos;
        let save_errors = self.errors.len();
        let save_builder = self.builder.checkpoint();

        // Peek at the next non-trivia token for FIRST-set filtering.
        let peek_pos = self.skip_trivia_pos();
        let have_lookahead =
            peek_pos < self.tokens.len() && self.tokens[peek_pos].kind != RuntimeTokenKind::Eof;

        // Optimisation: detect the common "LongerAlt | ShorterAlt" pattern
        // where alternatives share a leading prefix (e.g.
        //   A B C | A   — parse A once then optionally try B C).
        // This avoids re-parsing the common prefix on backtrack.
        if exprs.len() == 2
            && let (RuleExpr::Seq(long), short) = (&exprs[0], &exprs[1])
            && long.len() >= 2
            && *short == long[0]
        {
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

        // Fast path: use pre-computed dispatch table for choices with ≥3 alternatives.
        if have_lookahead {
            let choice_eid = expr_id(choice_expr);
            if let Some(dispatch) = self.choice_dispatch.get(&choice_eid) {
                let tok = &self.tokens[peek_pos];
                let (specific, extra) = dispatch.candidates_for(tok, self.source);

                // Try specific matches first, then extra (ident fallbacks), then universals.
                for &alt_idx in specific
                    .iter()
                    .chain(extra.iter())
                    .chain(dispatch.universal_alts.iter())
                {
                    let expr = &exprs[alt_idx as usize];
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
                return false;
            }
        }

        // Fallback: linear scan with FIRST-set filtering.
        if have_lookahead {
            let tok = &self.tokens[peek_pos];
            for expr in exprs {
                let eid = expr_id(expr);
                let skip = if let Some(fs) = self.expr_first_cache.get(&eid) {
                    !fs.is_universal && !fs.can_be_empty && !fs.matches_token(tok, self.source)
                } else {
                    false
                };
                if skip {
                    continue;
                }

                self.pos = save_pos;
                self.errors.truncate(save_errors);
                self.builder.rollback(save_builder);
                if self.try_parse_expr(expr) {
                    return true;
                }
            }
        } else {
            for expr in exprs {
                self.pos = save_pos;
                self.errors.truncate(save_errors);
                self.builder.rollback(save_builder);
                if self.try_parse_expr(expr) {
                    return true;
                }
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
