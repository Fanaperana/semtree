mod rules;
mod engine;

pub use engine::{LintEngine, LintResult};
pub use rules::{LintRule, LintDiagnostic, LintSeverity};

pub use rules::builtins;

#[cfg(test)]
mod tests;
