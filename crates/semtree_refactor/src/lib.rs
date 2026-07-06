pub mod rename;
pub mod extract;
pub mod inline;
pub mod tree_edit;

pub use rename::{rename_symbol, TextEdit};
pub use extract::{extract_variable, Extraction};
pub use inline::inline_variable;
pub use tree_edit::TreeEditor;

#[cfg(test)]
mod tests;
