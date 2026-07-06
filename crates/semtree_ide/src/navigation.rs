use semtree_core::SyntaxKind;
use semtree_red::SyntaxNode;
use semtree_semantic::{SemanticModel, SymbolKind};
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: SmolStr,
    pub kind: SymbolKind,
    pub range: TextRange,
    pub is_public: bool,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub name: SmolStr,
    pub kind: SymbolKind,
    pub range: TextRange,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct Breadcrumb {
    pub name: SmolStr,
    pub kind: SyntaxKind,
    pub range: TextRange,
}

/// Navigate to the definition of the symbol at the given offset.
pub fn goto_definition(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Option<TextRange> {
    let offset = TextSize::new(offset);
    let tok = root.token_at_offset(offset)?;
    if tok.kind() != SyntaxKind::IDENT {
        return None;
    }
    let name = tok.text();
    let symbols = model.symbols.find_by_name(name);
    symbols.first().map(|s| s.range)
}

/// Find all references to the symbol at the given offset.
pub fn find_references(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Vec<TextRange> {
    let offset = TextSize::new(offset);
    let tok = match root.token_at_offset(offset) {
        Some(t) => t,
        None => return Vec::new(),
    };
    if tok.kind() != SyntaxKind::IDENT {
        return Vec::new();
    }
    let name = tok.text();

    let mut ranges = Vec::new();

    // Include the definition itself
    for sym in model.symbols.find_by_name(name) {
        ranges.push(sym.range);
    }

    // Include all references
    for reference in model.find_references(name) {
        if !ranges.contains(&reference.range) {
            ranges.push(reference.range);
        }
    }

    ranges
}

/// Get an outline of all symbols in the document.
pub fn document_symbols(
    _root: &SyntaxNode,
    model: &SemanticModel,
) -> Vec<DocumentSymbol> {
    model
        .symbols
        .all()
        .iter()
        .map(|sym| DocumentSymbol {
            name: sym.name.clone(),
            kind: sym.kind,
            range: sym.range,
            is_public: sym.is_public,
        })
        .collect()
}

/// Get hover info for the symbol at the given offset.
pub fn hover_info(
    root: &SyntaxNode,
    model: &SemanticModel,
    offset: u32,
) -> Option<HoverInfo> {
    let offset = TextSize::new(offset);
    let tok = root.token_at_offset(offset)?;
    if tok.kind() != SyntaxKind::IDENT {
        return None;
    }
    let name = tok.text();
    let symbols = model.symbols.find_by_name(name);
    let sym = symbols.first()?;
    Some(HoverInfo {
        name: sym.name.clone(),
        kind: sym.kind,
        range: sym.range,
        detail: format!("{} `{}`", sym.kind, sym.name),
    })
}

/// Get the breadcrumb (scope chain) at the given offset.
pub fn breadcrumbs(root: &SyntaxNode, offset: u32) -> Vec<Breadcrumb> {
    let offset = TextSize::new(offset);
    let mut crumbs = Vec::new();
    collect_breadcrumbs(root, offset, &mut crumbs);
    crumbs
}

fn collect_breadcrumbs(node: &SyntaxNode, offset: TextSize, out: &mut Vec<Breadcrumb>) {
    if !node.text_range().contains(offset) {
        return;
    }

    let kind = node.kind();
    if is_named_scope(kind) {
        if let Some(name_tok) = node.child_token(SyntaxKind::IDENT) {
            out.push(Breadcrumb {
                name: name_tok.text().into(),
                kind,
                range: node.text_range(),
            });
        }
    }

    for child in node.children() {
        collect_breadcrumbs(&child, offset, out);
    }
}

fn is_named_scope(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::FUNCTION
            | SyntaxKind::STRUCT_DEF
            | SyntaxKind::ENUM_DEF
            | SyntaxKind::TRAIT_DEF
            | SyntaxKind::IMPL_DEF
    )
}
