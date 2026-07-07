use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::BTreeMap;

/// The Grammar IR: a language-agnostic intermediate representation that all
/// grammar frontends (SemTree DSL, Rust DSL, Tree-sitter import) compile into.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammar {
    pub name: SmolStr,
    pub rules: BTreeMap<SmolStr, Rule>,
    pub keywords: Vec<SmolStr>,
    pub extras: Vec<SmolStr>,
    /// Formatting hints for the formatter generator.
    pub format_hints: Vec<FormatHint>,
    /// The name of the entry/root rule (first rule defined in the grammar).
    pub entry_rule: Option<SmolStr>,
}

impl Grammar {
    pub fn new(name: impl Into<SmolStr>) -> Self {
        Self {
            name: name.into(),
            rules: BTreeMap::new(),
            keywords: Vec::new(),
            extras: Vec::new(),
            format_hints: Vec::new(),
            entry_rule: None,
        }
    }

    pub fn add_rule(&mut self, name: impl Into<SmolStr>, rule: Rule) {
        self.rules.insert(name.into(), rule);
    }

    pub fn add_keyword(&mut self, kw: impl Into<SmolStr>) {
        self.keywords.push(kw.into());
    }
}

/// A named production rule in the grammar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub name: SmolStr,
    pub expr: RuleExpr,
    pub fields: Vec<FieldDef>,
}

/// A field binding within a rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: SmolStr,
    pub rule: SmolStr,
}

/// The expression types that make up grammar rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleExpr {
    /// A literal string: `"fn"`
    Literal(SmolStr),
    /// A reference to another rule: `Identifier`
    RuleRef(SmolStr),
    /// A sequence of expressions: `A B C`
    Seq(Vec<RuleExpr>),
    /// An ordered choice: `A | B`
    Choice(Vec<RuleExpr>),
    /// Zero or more: `A*`
    Repeat(Box<RuleExpr>),
    /// One or more: `A+`
    Repeat1(Box<RuleExpr>),
    /// Optional: `A?`
    Optional(Box<RuleExpr>),
    /// A named field: `name: Identifier`
    Field(SmolStr, Box<RuleExpr>),
    /// Token-level rule (no whitespace skipping inside).
    Token(Box<RuleExpr>),
    /// Precedence wrapper.
    Prec(i32, Box<RuleExpr>),
    /// Left-associative precedence.
    PrecLeft(i32, Box<RuleExpr>),
    /// Right-associative precedence.
    PrecRight(i32, Box<RuleExpr>),
    /// A blank/placeholder (matches nothing).
    Blank,
}

/// Formatting hints attached to the grammar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormatHint {
    Indent(SmolStr),
    Linebreak(SmolStr),
    SpaceAround(SmolStr),
    SpaceBefore(SmolStr),
    SpaceAfter(SmolStr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrammarError {
    UndefinedRule(SmolStr),
    DuplicateRule(SmolStr),
    EmptyRule(SmolStr),
    ParseError(String),
    CycleDetected(Vec<SmolStr>),
    UnreachableRule(SmolStr),
    EmptyAlternative(SmolStr),
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrammarError::UndefinedRule(name) => write!(f, "undefined rule: {name}"),
            GrammarError::DuplicateRule(name) => write!(f, "duplicate rule: {name}"),
            GrammarError::EmptyRule(name) => write!(f, "empty rule: {name}"),
            GrammarError::ParseError(msg) => write!(f, "parse error: {msg}"),
            GrammarError::CycleDetected(cycle) => {
                let path: Vec<&str> = cycle.iter().map(|s| s.as_str()).collect();
                write!(f, "cycle detected: {}", path.join(" -> "))
            }
            GrammarError::UnreachableRule(name) => write!(f, "unreachable rule: {name}"),
            GrammarError::EmptyAlternative(name) => {
                write!(f, "empty alternative (Blank in Choice) in rule: {name}")
            }
        }
    }
}

impl std::error::Error for GrammarError {}
