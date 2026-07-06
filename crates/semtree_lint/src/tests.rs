use semtree_parser::Parser;
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;

use crate::engine::LintEngine;
use crate::rules::LintSeverity;

fn parse(source: &str) -> SyntaxNode {
    Parser::parse(source).syntax()
}

#[test]
fn empty_function_warning() {
    let root = parse("fn empty() {}");
    let engine = LintEngine::with_defaults();
    let result = engine.lint_syntax(&root);

    let empty_fn: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "empty-function")
        .collect();
    assert_eq!(empty_fn.len(), 1);
    assert_eq!(empty_fn[0].severity, LintSeverity::Warning);
}

#[test]
fn non_empty_function_no_warning() {
    let root = parse("fn main() { let x = 1; }");
    let engine = LintEngine::with_defaults();
    let result = engine.lint_syntax(&root);

    let empty_fn: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "empty-function")
        .collect();
    assert_eq!(empty_fn.len(), 0);
}

#[test]
fn naming_convention_warning() {
    let root = parse("fn main() { let myVar = 42; }");
    let model = SemanticModel::analyze(&root);
    let engine = LintEngine::with_defaults();
    let result = engine.lint(&root, &model);

    let naming: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "naming-convention")
        .collect();
    assert_eq!(naming.len(), 1);
    assert!(naming[0].message.contains("myVar"));
    assert!(naming[0].fix.is_some());
}

#[test]
fn snake_case_no_warning() {
    let root = parse("fn main() { let my_var = 42; }");
    let model = SemanticModel::analyze(&root);
    let engine = LintEngine::with_defaults();
    let result = engine.lint(&root, &model);

    let naming: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "naming-convention")
        .collect();
    assert_eq!(naming.len(), 0);
}

#[test]
fn missing_docs_info() {
    let root = parse("fn undocumented() { let x = 1; }");
    let engine = LintEngine::with_defaults();
    let result = engine.lint_syntax(&root);

    let docs: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "missing-docs")
        .collect();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].severity, LintSeverity::Info);
}

#[test]
fn lint_result_counts() {
    let root = parse("fn empty() {}");
    let engine = LintEngine::with_defaults();
    let result = engine.lint_syntax(&root);

    assert!(result.warning_count() > 0);
    assert_eq!(result.error_count(), 0);
    assert!(!result.is_clean());
}

#[test]
fn multiple_functions_linted() {
    let root = parse("fn a() {} fn b() {} fn c() { let x = 1; }");
    let engine = LintEngine::with_defaults();
    let result = engine.lint_syntax(&root);

    let empty_fn: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.rule == "empty-function")
        .collect();
    assert_eq!(empty_fn.len(), 2);
}

#[test]
fn custom_rule() {
    use crate::rules::{LintRule, LintDiagnostic};
    use semtree_core::SyntaxKind;

    struct NoStructs;
    impl LintRule for NoStructs {
        fn name(&self) -> &str { "no-structs" }
        fn description(&self) -> &str { "disallows struct definitions" }
        fn default_severity(&self) -> LintSeverity { LintSeverity::Error }
        fn check(&self, root: &SyntaxNode, _model: Option<&SemanticModel>) -> Vec<LintDiagnostic> {
            let mut diags = Vec::new();
            for node in root.descendants() {
                if node.kind() == SyntaxKind::STRUCT_DEF {
                    diags.push(LintDiagnostic {
                        rule: self.name().into(),
                        message: "structs are not allowed".to_string(),
                        range: node.text_range(),
                        severity: self.default_severity(),
                        fix: None,
                    });
                }
            }
            diags
        }
    }

    let root = parse("struct Foo {} fn main() {}");
    let mut engine = LintEngine::new();
    engine.add_rule(Box::new(NoStructs));
    let result = engine.lint_syntax(&root);

    assert_eq!(result.error_count(), 1);
}
