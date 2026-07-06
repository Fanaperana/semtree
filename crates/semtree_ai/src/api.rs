use semtree_core::{SyntaxKind, TextRange};
use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;
use text_size::TextSize;

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub start: u32,
    pub end: u32,
    pub is_public: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub symbol_name: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct ScopeInfo {
    pub scope_id: usize,
    pub parent_scope: Option<usize>,
    pub start: u32,
    pub end: u32,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub kind: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum TreeDiffKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone)]
pub struct TreeDiff {
    pub kind: TreeDiffKind,
    pub node_kind: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
}

/// Find all symbols matching the given name.
pub fn find_symbol(root: &SyntaxNode, model: &SemanticModel, name: &str) -> Vec<SymbolInfo> {
    let _ = root;
    model
        .symbols
        .find_by_name(name)
        .into_iter()
        .map(|s| SymbolInfo {
            name: s.name.to_string(),
            kind: s.kind.to_string(),
            start: u32::from(s.range.start()),
            end: u32::from(s.range.end()),
            is_public: s.is_public,
            is_mutable: s.is_mutable,
        })
        .collect()
}

/// Apply a rename of `old` to `new` across the source text and return the updated source.
pub fn rename_symbol(
    source: &str,
    root: &SyntaxNode,
    model: &SemanticModel,
    old: &str,
    new: &str,
) -> String {
    let _ = (root, model);
    let mut result = String::new();
    let chars: Vec<char> = source.chars().collect();
    let old_chars: Vec<char> = old.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + old_chars.len() <= chars.len()
            && &chars[i..i + old_chars.len()] == old_chars.as_slice()
        {
            let before_ok =
                i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
            let after_idx = i + old_chars.len();
            let after_ok = after_idx >= chars.len()
                || !(chars[after_idx].is_alphanumeric() || chars[after_idx] == '_');
            if before_ok && after_ok {
                result.push_str(new);
                i += old_chars.len();
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Find all references to a symbol by name.
pub fn find_references(
    root: &SyntaxNode,
    model: &SemanticModel,
    name: &str,
) -> Vec<ReferenceInfo> {
    let _ = root;
    let symbols = model.symbols.find_by_name(name);
    let symbol_name = symbols
        .first()
        .map(|s| s.name.to_string())
        .unwrap_or_else(|| name.to_string());

    model
        .find_references(name)
        .into_iter()
        .map(|r| ReferenceInfo {
            symbol_name: symbol_name.clone(),
            start: u32::from(r.range.start()),
            end: u32::from(r.range.end()),
        })
        .collect()
}

/// Get the scope context at the given byte offset.
pub fn nearest_scope(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Option<ScopeInfo> {
    let _ = root;
    let offset_ts = TextSize::from(offset);
    let mut best: Option<(usize, TextRange)> = None;

    for scope_id in 0..model.scopes.len() {
        if let Some(scope) = model.scopes.get(scope_id) {
            if scope.range.contains(offset_ts) {
                let is_tighter = best
                    .as_ref()
                    .map(|(_, r)| scope.range.len() < r.len())
                    .unwrap_or(true);
                if is_tighter {
                    best = Some((scope_id, scope.range));
                }
            }
        }
    }

    best.and_then(|(scope_id, _)| {
        let scope = model.scopes.get(scope_id)?;
        let symbols: Vec<String> = scope
            .bindings
            .keys()
            .map(|k| k.to_string())
            .collect();
        Some(ScopeInfo {
            scope_id,
            parent_scope: scope.parent,
            start: u32::from(scope.range.start()),
            end: u32::from(scope.range.end()),
            symbols,
        })
    })
}

/// Find which function contains the given byte offset.
pub fn current_function(root: &SyntaxNode, offset: u32) -> Option<FunctionInfo> {
    let offset_ts = TextSize::from(offset);
    let mut best: Option<FunctionInfo> = None;
    let mut best_len = u32::MAX;

    for node in root.descendants() {
        if node.kind() == SyntaxKind::FUNCTION && node.text_range().contains(offset_ts) {
            let range = node.text_range();
            let len = u32::from(range.len());
            if len < best_len {
                let name = node
                    .child_token(SyntaxKind::IDENT)
                    .map(|t| t.text().to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string());
                best = Some(FunctionInfo {
                    name,
                    start: u32::from(range.start()),
                    end: u32::from(range.end()),
                });
                best_len = len;
            }
        }
    }
    best
}

/// Find all nodes whose ranges overlap with the given edit range.
pub fn affected_nodes(root: &SyntaxNode, edit_range: TextRange) -> Vec<NodeInfo> {
    let mut results = Vec::new();
    collect_affected(root, edit_range, &mut results);
    results
}

fn collect_affected(node: &SyntaxNode, edit_range: TextRange, out: &mut Vec<NodeInfo>) {
    let range = node.text_range();
    if range.end() <= edit_range.start() || range.start() >= edit_range.end() {
        return;
    }

    let mut has_affected_child = false;
    for child in node.children() {
        let cr = child.text_range();
        if cr.end() > edit_range.start() && cr.start() < edit_range.end() {
            has_affected_child = true;
            collect_affected(&child, edit_range, out);
        }
    }

    if !has_affected_child {
        out.push(NodeInfo {
            kind: format!("{}", node.kind()),
            start: u32::from(range.start()),
            end: u32::from(range.end()),
            text: node.text(),
        });
    }
}

/// Compute a structural diff between two syntax trees.
pub fn diff_tree(old_root: &SyntaxNode, new_root: &SyntaxNode) -> Vec<TreeDiff> {
    let mut diffs = Vec::new();
    diff_nodes(old_root, new_root, &mut diffs);
    diffs
}

fn diff_nodes(old: &SyntaxNode, new: &SyntaxNode, diffs: &mut Vec<TreeDiff>) {
    use semtree_red::SyntaxElement;

    if old.text() == new.text() {
        return;
    }

    let old_children = old.children_with_tokens();
    let new_children = new.children_with_tokens();

    let max_len = old_children.len().max(new_children.len());

    for i in 0..max_len {
        match (old_children.get(i), new_children.get(i)) {
            (Some(old_elem), Some(new_elem)) => {
                if old_elem.kind() != new_elem.kind()
                    || elem_text(old_elem) != elem_text(new_elem)
                {
                    match (old_elem, new_elem) {
                        (SyntaxElement::Node(on), SyntaxElement::Node(nn))
                            if on.kind() == nn.kind() =>
                        {
                            diff_nodes(on, nn, diffs);
                        }
                        _ => {
                            diffs.push(TreeDiff {
                                kind: TreeDiffKind::Changed,
                                node_kind: format!("{}", new_elem.kind()),
                                start: u32::from(new_elem.text_range().start()),
                                end: u32::from(new_elem.text_range().end()),
                                text: elem_text(new_elem),
                            });
                        }
                    }
                }
            }
            (Some(old_elem), None) => {
                diffs.push(TreeDiff {
                    kind: TreeDiffKind::Removed,
                    node_kind: format!("{}", old_elem.kind()),
                    start: u32::from(old_elem.text_range().start()),
                    end: u32::from(old_elem.text_range().end()),
                    text: elem_text(old_elem),
                });
            }
            (None, Some(new_elem)) => {
                diffs.push(TreeDiff {
                    kind: TreeDiffKind::Added,
                    node_kind: format!("{}", new_elem.kind()),
                    start: u32::from(new_elem.text_range().start()),
                    end: u32::from(new_elem.text_range().end()),
                    text: elem_text(new_elem),
                });
            }
            (None, None) => unreachable!(),
        }
    }
}

fn elem_text(elem: &semtree_red::SyntaxElement) -> String {
    use semtree_red::SyntaxElement;
    match elem {
        SyntaxElement::Node(n) => n.text(),
        SyntaxElement::Token(t) => t.text().to_string(),
    }
}

/// Suggest completion candidates at the given offset based on visible symbols in scope.
pub fn suggest_completion(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Vec<CompletionSuggestion> {
    let scope_info = nearest_scope(root, model, offset);
    let scope_id = match &scope_info {
        Some(s) => s.scope_id,
        None => return Vec::new(),
    };

    let visible = model.visible_symbols(scope_id);
    visible
        .into_iter()
        .map(|sym| CompletionSuggestion {
            label: sym.name.to_string(),
            kind: sym.kind.to_string(),
            detail: if sym.is_public {
                Some("public".to_string())
            } else {
                None
            },
        })
        .collect()
}

