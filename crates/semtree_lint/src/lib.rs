mod engine;
mod rules;

pub use engine::{LintEngine, LintResult};
pub use rules::{LintDiagnostic, LintRule, LintSeverity};

pub use rules::builtins;

#[cfg(test)]
mod tests;
