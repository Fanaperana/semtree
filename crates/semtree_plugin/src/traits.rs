use semtree_grammar::Grammar;
use semtree_red::SyntaxNode;
use text_size::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone)]
pub struct PluginDiagnostic {
    pub message: String,
    pub range: TextRange,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub struct NamedQuery {
    pub name: String,
    pub pattern: String,
}

pub trait LanguagePlugin: Send + Sync {
    fn name(&self) -> &str;
    fn grammar(&self) -> Grammar;
    fn file_extensions(&self) -> &[&str];
}

pub trait LintRulePlugin: Send + Sync {
    fn id(&self) -> &str;
    fn check(&self, node: &SyntaxNode) -> Vec<PluginDiagnostic>;
}

pub trait LinterPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn rules(&self) -> Vec<Box<dyn LintRulePlugin>>;
}

pub trait FormatterPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn format(&self, source: &str, root: &SyntaxNode) -> String;
}

pub trait QueryPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn queries(&self) -> Vec<NamedQuery>;
}
