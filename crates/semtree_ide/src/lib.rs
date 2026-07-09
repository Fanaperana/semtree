pub mod completion;
pub mod folding;
pub mod navigation;
pub mod semantic_tokens;

pub use completion::{CompletionItem, CompletionKind, complete_at};
pub use folding::{FoldingKind, FoldingRange, folding_ranges};
pub use navigation::{
    Breadcrumb, DocumentSymbol, HoverInfo, breadcrumbs, document_symbols, find_references,
    goto_definition, hover_info,
};
pub use semantic_tokens::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, classify_tokens,
};

#[cfg(test)]
mod tests;
