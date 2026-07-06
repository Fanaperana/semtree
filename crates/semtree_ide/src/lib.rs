pub mod semantic_tokens;
pub mod completion;
pub mod navigation;
pub mod folding;

pub use semantic_tokens::{SemanticToken, SemanticTokenType, SemanticTokenModifier, classify_tokens};
pub use completion::{CompletionItem, CompletionKind, complete_at};
pub use navigation::{goto_definition, find_references, document_symbols, hover_info, breadcrumbs, DocumentSymbol, HoverInfo, Breadcrumb};
pub use folding::{FoldingRange, FoldingKind, folding_ranges};

#[cfg(test)]
mod tests;
