pub mod registry;
pub mod traits;

pub use registry::PluginRegistry;
pub use traits::{
    FormatterPlugin, LanguagePlugin, LintRulePlugin, LinterPlugin, NamedQuery, PluginDiagnostic,
    QueryPlugin, Severity,
};

#[cfg(test)]
mod tests;
