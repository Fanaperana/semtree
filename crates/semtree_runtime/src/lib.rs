mod runtime_lexer;
mod runtime_parser;
mod incremental;

pub use runtime_lexer::RuntimeLexer;
pub use runtime_parser::{RuntimeParser, RuntimeParseResult, rule_name_to_kind};
pub use incremental::{EditRegion, IncrementalParser, apply_edits};

#[cfg(test)]
mod tests;
