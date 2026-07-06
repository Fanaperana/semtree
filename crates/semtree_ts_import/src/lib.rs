mod importer;

pub use importer::{import_tree_sitter_grammar, TsImportError};

#[cfg(test)]
mod tests;
