use semtree_core::SyntaxKind;
use semtree_red::SyntaxNode;
use text_size::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingKind {
    Block,
    Function,
    Struct,
    Enum,
    Impl,
    Trait,
    Comment,
}

#[derive(Debug, Clone)]
pub struct FoldingRange {
    pub range: TextRange,
    pub kind: FoldingKind,
}

/// Identify all foldable regions in the syntax tree.
pub fn folding_ranges(root: &SyntaxNode) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    collect_folding(root, &mut ranges);
    ranges
}

fn collect_folding(node: &SyntaxNode, out: &mut Vec<FoldingRange>) {
    let kind = node.kind();
    let folding_kind = match kind {
        SyntaxKind::FUNCTION => Some(FoldingKind::Function),
        SyntaxKind::STRUCT_DEF => Some(FoldingKind::Struct),
        SyntaxKind::ENUM_DEF => Some(FoldingKind::Enum),
        SyntaxKind::IMPL_DEF => Some(FoldingKind::Impl),
        SyntaxKind::TRAIT_DEF => Some(FoldingKind::Trait),
        SyntaxKind::BLOCK => Some(FoldingKind::Block),
        _ => None,
    };

    if let Some(fk) = folding_kind {
        let range = node.text_range();
        if u32::from(range.len()) > 0 {
            out.push(FoldingRange { range, kind: fk });
        }
    }

    for child in node.children() {
        collect_folding(&child, out);
    }
}
