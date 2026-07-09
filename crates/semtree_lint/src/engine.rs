use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;

use crate::rules::{LintDiagnostic, LintRule};

/// The lint engine: runs a collection of rules against a syntax tree.
pub struct LintEngine {
    rules: Vec<Box<dyn LintRule>>,
}

/// The result of running all lint rules.
pub struct LintResult {
    pub diagnostics: Vec<LintDiagnostic>,
}

impl LintResult {
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::rules::LintSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::rules::LintSeverity::Warning)
            .count()
    }
}

impl LintEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a lint engine with all built-in rules.
    pub fn with_defaults() -> Self {
        let mut engine = Self::new();
        engine.add_rule(Box::new(crate::rules::builtins::EmptyFunction));
        engine.add_rule(Box::new(crate::rules::builtins::NamingConvention));
        engine.add_rule(Box::new(crate::rules::builtins::UnusedVariable));
        engine.add_rule(Box::new(crate::rules::builtins::MissingDocumentation));
        engine
    }

    pub fn add_rule(&mut self, rule: Box<dyn LintRule>) {
        self.rules.push(rule);
    }

    /// Run all rules against a syntax tree (syntax-only, no semantic model).
    pub fn lint_syntax(&self, root: &SyntaxNode) -> LintResult {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            diagnostics.extend(rule.check(root, None));
        }
        diagnostics.sort_by_key(|d| d.range.start());
        LintResult { diagnostics }
    }

    /// Run all rules with both syntax tree and semantic model.
    pub fn lint(&self, root: &SyntaxNode, model: &SemanticModel) -> LintResult {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            diagnostics.extend(rule.check(root, Some(model)));
        }
        diagnostics.sort_by_key(|d| d.range.start());
        LintResult { diagnostics }
    }
}

impl Default for LintEngine {
    fn default() -> Self {
        Self::new()
    }
}
