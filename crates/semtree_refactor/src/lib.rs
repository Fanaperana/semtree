pub mod extract;
pub mod inline;
pub mod rename;
pub mod tree_edit;

pub use extract::{Extraction, extract_variable};
pub use inline::inline_variable;
pub use rename::{TextEdit, rename_symbol};
pub use tree_edit::TreeEditor;

#[cfg(test)]
mod tests;
