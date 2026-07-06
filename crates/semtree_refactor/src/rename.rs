use semtree_core::SyntaxKind;
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;
use text_size::{TextRange, TextSize};

/// A text edit: replace a range with new text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

/// Compute all edits needed to rename the symbol at the given offset.
pub fn rename_symbol(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
    new_name: &str,
) -> Vec<TextEdit> {
    let offset = TextSize::new(offset);
    let tok = match root.token_at_offset(offset) {
        Some(t) => t,
        None => return Vec::new(),
    };
    if tok.kind() != SyntaxKind::IDENT {
        return Vec::new();
    }
    let old_name = tok.text();

    let mut edits = Vec::new();

    // Rename at definition sites
    for sym in model.symbols.find_by_name(old_name) {
        if let Some(name_range) = find_ident_in_node(root, sym.range, old_name) {
            edits.push(TextEdit {
                range: name_range,
                new_text: new_name.to_string(),
            });
        }
    }

    // Rename at reference sites
    for reference in model.find_references(old_name) {
        edits.push(TextEdit {
            range: reference.range,
            new_text: new_name.to_string(),
        });
    }

    edits.sort_by_key(|e| e.range.start());
    edits.dedup_by_key(|e| e.range);
    edits
}

fn find_ident_in_node(root: &SyntaxNode, node_range: TextRange, name: &str) -> Option<TextRange> {
    for desc in root.descendants() {
        if desc.text_range() == node_range {
            if let Some(tok) = desc.child_token(SyntaxKind::IDENT) {
                if tok.text() == name {
                    return Some(tok.text_range());
                }
            }
        }
    }
    None
}
