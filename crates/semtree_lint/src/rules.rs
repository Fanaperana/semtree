use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;
use smol_str::SmolStr;
use text_size::TextRange;

/// A lint diagnostic produced by a rule.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub rule: SmolStr,
    pub message: String,
    pub range: TextRange,
    pub severity: LintSeverity,
    pub fix: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintSeverity::Error => write!(f, "error"),
            LintSeverity::Warning => write!(f, "warning"),
            LintSeverity::Info => write!(f, "info"),
        }
    }
}

/// Trait for implementing lint rules.
///
/// Rules can inspect syntax trees, semantic models, or both.
pub trait LintRule: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn default_severity(&self) -> LintSeverity;
    fn check(&self, root: &SyntaxNode, model: Option<&SemanticModel>) -> Vec<LintDiagnostic>;
}

/// Built-in lint rules.
pub mod builtins {
    use super::*;
    use semtree_core::SyntaxKind;
    use semtree_semantic::SymbolKind;

    /// Warns about empty function bodies.
    pub struct EmptyFunction;

    impl LintRule for EmptyFunction {
        fn name(&self) -> &str {
            "empty-function"
        }
        fn description(&self) -> &str {
            "warns about functions with empty bodies"
        }
        fn default_severity(&self) -> LintSeverity {
            LintSeverity::Warning
        }

        fn check(&self, root: &SyntaxNode, _model: Option<&SemanticModel>) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            self.visit(root, &mut diags);
            diags
        }
    }

    impl EmptyFunction {
        fn visit(&self, node: &SyntaxNode, diags: &mut Vec<LintDiagnostic>) {
            if node.kind() == SyntaxKind::FUNCTION
                && let Some(block) = node.child_node(SyntaxKind::BLOCK)
            {
                let non_trivia_children: Vec<_> = block
                    .children()
                    .into_iter()
                    .filter(|c| !c.kind().is_trivia())
                    .collect();
                if non_trivia_children.is_empty() {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        message: "function body is empty".to_string(),
                        range: node.text_range(),
                        severity: self.default_severity(),
                        fix: None,
                    });
                }
            }
            for child in node.children() {
                self.visit(&child, diags);
            }
        }
    }

    /// Warns when variable names don't follow snake_case.
    pub struct NamingConvention;

    impl LintRule for NamingConvention {
        fn name(&self) -> &str {
            "naming-convention"
        }
        fn description(&self) -> &str {
            "checks that variable names use snake_case"
        }
        fn default_severity(&self) -> LintSeverity {
            LintSeverity::Warning
        }

        fn check(&self, _root: &SyntaxNode, model: Option<&SemanticModel>) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            if let Some(model) = model {
                for sym in model.symbols.all() {
                    if matches!(sym.kind, SymbolKind::Variable | SymbolKind::Parameter)
                        && !is_snake_case(&sym.name)
                    {
                        diags.push(LintDiagnostic {
                            rule: self.name().into(),
                            message: format!("{} '{}' should use snake_case", sym.kind, sym.name),
                            range: sym.range,
                            severity: self.default_severity(),
                            fix: Some(to_snake_case(&sym.name)),
                        });
                    }
                }
            }
            diags
        }
    }

    /// Warns about unused variables.
    pub struct UnusedVariable;

    impl LintRule for UnusedVariable {
        fn name(&self) -> &str {
            "unused-variable"
        }
        fn description(&self) -> &str {
            "warns about variables that are never referenced"
        }
        fn default_severity(&self) -> LintSeverity {
            LintSeverity::Warning
        }

        fn check(&self, _root: &SyntaxNode, model: Option<&SemanticModel>) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            if let Some(model) = model {
                for (i, sym) in model.symbols.all().iter().enumerate() {
                    if sym.kind != SymbolKind::Variable {
                        continue;
                    }
                    if sym.name.starts_with('_') {
                        continue;
                    }
                    let has_refs = model.references.iter().any(|r| r.target_symbol == i);
                    if !has_refs {
                        diags.push(LintDiagnostic {
                            rule: self.name().into(),
                            message: format!("variable '{}' is never used", sym.name),
                            range: sym.range,
                            severity: self.default_severity(),
                            fix: Some(format!("_{}", sym.name)),
                        });
                    }
                }
            }
            diags
        }
    }

    /// Warns about functions missing documentation (no comment before them).
    pub struct MissingDocumentation;

    impl LintRule for MissingDocumentation {
        fn name(&self) -> &str {
            "missing-docs"
        }
        fn description(&self) -> &str {
            "warns about public functions without documentation"
        }
        fn default_severity(&self) -> LintSeverity {
            LintSeverity::Info
        }

        fn check(&self, root: &SyntaxNode, _model: Option<&SemanticModel>) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            self.visit(root, &mut diags);
            diags
        }
    }

    impl MissingDocumentation {
        fn visit(&self, node: &SyntaxNode, diags: &mut Vec<LintDiagnostic>) {
            if node.kind() == SyntaxKind::FUNCTION {
                // Check if there's a line comment immediately before.
                let has_doc = node.prev_sibling().is_some_and(|prev| {
                    prev.kind() == SyntaxKind::LINE_COMMENT
                        || prev.kind() == SyntaxKind::BLOCK_COMMENT
                });

                if !has_doc && let Some(name) = node.child_token(SyntaxKind::IDENT) {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        message: format!("function '{}' has no documentation", name.text()),
                        range: node.text_range(),
                        severity: self.default_severity(),
                        fix: None,
                    });
                }
            }
            for child in node.children() {
                self.visit(&child, diags);
            }
        }
    }

    fn is_snake_case(s: &str) -> bool {
        s.chars()
            .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
    }

    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    }
}
