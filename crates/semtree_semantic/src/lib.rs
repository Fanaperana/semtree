mod symbols;
mod scope;
mod resolver;
mod diagnostics;

pub use symbols::{Symbol, SymbolKind, SymbolTable};
pub use scope::{Scope, ScopeId, ScopeTree};
pub use resolver::SemanticModel;
pub use diagnostics::{Diagnostic, DiagnosticSeverity};

#[cfg(test)]
mod tests;
