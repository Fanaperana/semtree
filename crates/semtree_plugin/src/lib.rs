pub mod traits;
pub mod registry;

pub use traits::{
    FormatterPlugin, LanguagePlugin, LintRulePlugin, LinterPlugin, NamedQuery, PluginDiagnostic,
    QueryPlugin, Severity,
};
pub use registry::PluginRegistry;

#[cfg(test)]
mod tests;
