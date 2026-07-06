use semtree_core::SyntaxKind;
use semtree_red::{SyntaxNode, SyntaxElement};
use semtree_semantic::SemanticModel;
use text_size::TextSize;

use crate::rename::TextEdit;

/// Inline a variable: replace all references with its initializer expression,
/// and remove the let binding.
pub fn inline_variable(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Option<Vec<TextEdit>> {
    let offset = TextSize::new(offset);
    let tok = root.token_at_offset(offset)?;
    if tok.kind() != SyntaxKind::IDENT {
        return None;
    }
    let name = tok.text();

    // Find the variable's definition
    let symbols = model.symbols.find_by_name(name);
    let sym = symbols.first()?;
    if sym.kind != semtree_semantic::SymbolKind::Variable {
        return None;
    }

    // Find the initializer text by looking at the let statement node
    let init_text = find_initializer(root, sym.range)?;

    let mut edits = Vec::new();

    // Replace all references with the initializer
    for reference in model.find_references(name) {
        edits.push(TextEdit {
            range: reference.range,
            new_text: init_text.clone(),
        });
    }

    // Remove the let statement
    edits.push(TextEdit {
        range: sym.range,
        new_text: String::new(),
    });

    edits.sort_by_key(|e| e.range.start());
    Some(edits)
}

fn find_initializer(root: &SyntaxNode, let_range: text_size::TextRange) -> Option<String> {
    for desc in root.descendants() {
        if desc.kind() == SyntaxKind::LET_STMT && desc.text_range() == let_range {
            // The initializer is everything after the '=' and before the ';'
            let mut found_eq = false;
            let mut init_parts = Vec::new();
            for elem in desc.children_with_tokens() {
                if found_eq {
                    match &elem {
                        SyntaxElement::Token(t) if t.kind() == SyntaxKind::SEMICOLON => break,
                        _ => {
                            let text = match &elem {
                                SyntaxElement::Token(t) => t.text().to_string(),
                                SyntaxElement::Node(n) => n.text(),
                            };
                            init_parts.push(text);
                        }
                    }
                } else if let SyntaxElement::Token(t) = &elem {
                    if t.kind() == SyntaxKind::EQ {
                        found_eq = true;
                    }
                }
            }
            let init = init_parts.join("").trim().to_string();
            if !init.is_empty() {
                return Some(init);
            }
        }
    }
    None
}
