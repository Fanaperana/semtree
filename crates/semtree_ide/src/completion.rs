use semtree_core::SyntaxKind;
use semtree_red::SyntaxNode;
use semtree_semantic::{SemanticModel, SymbolKind};
use text_size::TextSize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Variable,
    Snippet,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: String,
}

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "while", "for", "return", "struct", "enum", "impl", "trait",
    "pub", "use", "mod", "match", "const", "static", "type", "loop", "break", "continue",
];

/// Provide completion suggestions at the given offset.
pub fn complete_at(root: &SyntaxNode, model: &SemanticModel, offset: u32) -> Vec<CompletionItem> {
    let offset = TextSize::new(offset);
    let mut items = Vec::new();

    let prefix = find_prefix(root, offset);

    let scope_id = find_scope_at(root, model, offset);

    if let Some(scope_id) = scope_id {
        for sym in model.visible_symbols(scope_id) {
            let label = sym.name.to_string();
            if !prefix.is_empty() && !label.starts_with(prefix.as_str()) {
                continue;
            }
            let kind = match sym.kind {
                SymbolKind::Function => CompletionKind::Function,
                _ => CompletionKind::Variable,
            };
            let detail = Some(format!("{}", sym.kind));
            items.push(CompletionItem {
                label: label.clone(),
                kind,
                detail,
                insert_text: label,
            });
        }
    }

    for &kw in KEYWORDS {
        if prefix.is_empty() || kw.starts_with(prefix.as_str()) {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some("keyword".to_string()),
                insert_text: kw.to_string(),
            });
        }
    }

    items
}

fn find_prefix(root: &SyntaxNode, offset: TextSize) -> String {
    if offset == TextSize::new(0) {
        return String::new();
    }
    if let Some(tok) = root.token_at_offset(offset - TextSize::new(1))
        && tok.kind() == SyntaxKind::IDENT
    {
        return tok.text().to_string();
    }
    String::new()
}

fn find_scope_at(root: &SyntaxNode, model: &SemanticModel, offset: TextSize) -> Option<usize> {
    let mut best: Option<(usize, u32)> = None;
    for i in 0..model.scopes.len() {
        if let Some(scope) = model.scopes.get(i) {
            let range = scope.range;
            if range.contains(offset) || range.end() == offset {
                let size = u32::from(range.len());
                match best {
                    None => best = Some((i, size)),
                    Some((_, best_size)) if size < best_size => best = Some((i, size)),
                    _ => {}
                }
            }
        }
    }

    // If nothing found, check if we're at the root
    if best.is_none() && root.text_range().contains(offset) {
        return Some(0);
    }

    best.map(|(id, _)| id)
}
