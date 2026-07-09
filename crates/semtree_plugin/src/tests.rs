use crate::registry::PluginRegistry;
use crate::traits::*;
use semtree_grammar::Grammar;
use semtree_red::SyntaxNode;

struct MockLanguage;

impl LanguagePlugin for MockLanguage {
    fn name(&self) -> &str {
        "mock"
    }

    fn grammar(&self) -> Grammar {
        Grammar::new("mock")
    }

    fn file_extensions(&self) -> &[&str] {
        &["mock", "mk"]
    }
}

struct MockLintRule;

impl LintRulePlugin for MockLintRule {
    fn id(&self) -> &str {
        "mock-rule-1"
    }

    fn check(&self, _node: &SyntaxNode) -> Vec<PluginDiagnostic> {
        vec![]
    }
}

struct MockLinter;

impl LinterPlugin for MockLinter {
    fn name(&self) -> &str {
        "mock-linter"
    }

    fn rules(&self) -> Vec<Box<dyn LintRulePlugin>> {
        vec![Box::new(MockLintRule)]
    }
}

struct MockFormatter;

impl FormatterPlugin for MockFormatter {
    fn name(&self) -> &str {
        "mock-formatter"
    }

    fn format(&self, source: &str, _root: &SyntaxNode) -> String {
        source.trim().to_string()
    }
}

struct MockQueryPlugin;

impl QueryPlugin for MockQueryPlugin {
    fn name(&self) -> &str {
        "mock-queries"
    }

    fn queries(&self) -> Vec<NamedQuery> {
        vec![NamedQuery {
            name: "functions".to_string(),
            pattern: "(function_definition)".to_string(),
        }]
    }
}

#[test]
fn test_register_language_and_lookup() {
    let mut registry = PluginRegistry::new();
    registry.register_language(Box::new(MockLanguage));

    let found = registry.get_language_for_extension("mock");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name(), "mock");

    let found2 = registry.get_language_for_extension("mk");
    assert!(found2.is_some());

    let not_found = registry.get_language_for_extension("rs");
    assert!(not_found.is_none());
}

#[test]
fn test_register_linter() {
    let mut registry = PluginRegistry::new();
    registry.register_linter(Box::new(MockLinter));
    assert_eq!(registry.linters().len(), 1);
    assert_eq!(registry.linters()[0].name(), "mock-linter");
}

#[test]
fn test_register_formatter() {
    let mut registry = PluginRegistry::new();
    registry.register_formatter(Box::new(MockFormatter));
    assert_eq!(registry.formatters().len(), 1);
    assert_eq!(registry.formatters()[0].name(), "mock-formatter");
}

#[test]
fn test_register_query_plugin() {
    let mut registry = PluginRegistry::new();
    registry.register_query(Box::new(MockQueryPlugin));
    assert_eq!(registry.query_plugins().len(), 1);
    let plugin = &registry.query_plugins()[0];
    assert_eq!(plugin.name(), "mock-queries");
    let queries = plugin.queries();
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].name, "functions");
}

#[test]
fn test_plugin_diagnostic() {
    use text_size::{TextRange, TextSize};
    let diag = PluginDiagnostic {
        message: "test error".to_string(),
        range: TextRange::new(TextSize::from(0), TextSize::from(5)),
        severity: Severity::Error,
    };
    assert_eq!(diag.message, "test error");
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn test_named_query() {
    let q = NamedQuery {
        name: "test".to_string(),
        pattern: "(identifier)".to_string(),
    };
    assert_eq!(q.name, "test");
    assert_eq!(q.pattern, "(identifier)");
}
