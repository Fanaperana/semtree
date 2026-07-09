mod diagnostics;
mod resolver;
mod scope;
mod symbols;

pub use diagnostics::{Diagnostic, DiagnosticSeverity};
pub use resolver::SemanticModel;
pub use scope::{Scope, ScopeId, ScopeTree};
pub use symbols::{Symbol, SymbolKind, SymbolTable};

#[cfg(test)]
mod tests;
