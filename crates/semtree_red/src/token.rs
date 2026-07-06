use std::sync::Arc;

use semtree_core::SyntaxKind;
use semtree_green::GreenToken;
use text_size::{TextRange, TextSize};

use crate::node::SyntaxNode;

/// A red tree token: a leaf with parent and offset information.
#[derive(Debug, Clone)]
pub struct SyntaxToken {
    green: GreenToken,
    parent: Option<Arc<SyntaxNode>>,
    index_in_parent: usize,
    offset: TextSize,
}

impl SyntaxToken {
    pub(crate) fn new(
        green: GreenToken,
        parent: Option<Arc<SyntaxNode>>,
        index_in_parent: usize,
        offset: TextSize,
    ) -> Self {
        Self {
            green,
            parent,
            index_in_parent,
            offset,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    pub fn text(&self) -> &str {
        self.green.text()
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::at(self.offset, self.green.text_len())
    }

    pub fn parent(&self) -> Option<&SyntaxNode> {
        self.parent.as_deref()
    }

    pub fn index_in_parent(&self) -> usize {
        self.index_in_parent
    }
}
