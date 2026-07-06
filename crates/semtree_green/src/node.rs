use std::sync::Arc;

use semtree_core::SyntaxKind;
use smol_str::SmolStr;
use text_size::TextSize;

/// An element in the green tree is either a node (branch) or a token (leaf).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

/// A green tree element: either a node or a token.
pub type GreenElement = NodeOrToken<GreenNode, GreenToken>;

/// An immutable, atomically reference-counted syntax tree node.
///
/// Green nodes are purely structural: they store their kind, children, and
/// the total text length spanned. They have no parent pointers, enabling
/// structural sharing across incremental edits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreenNode {
    inner: Arc<GreenNodeData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GreenNodeData {
    kind: SyntaxKind,
    text_len: TextSize,
    children: Vec<GreenElement>,
}

impl GreenNode {
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Self {
        let text_len = children.iter().map(|c| c.text_len()).sum();
        Self {
            inner: Arc::new(GreenNodeData {
                kind,
                text_len,
                children,
            }),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.inner.kind
    }

    pub fn text_len(&self) -> TextSize {
        self.inner.text_len
    }

    pub fn children(&self) -> &[GreenElement] {
        &self.inner.children
    }

    pub fn children_count(&self) -> usize {
        self.inner.children.len()
    }

    /// Replace a child at the given index, returning a new green node.
    /// This enables structural sharing: unchanged subtrees share Arc pointers.
    pub fn replace_child(&self, index: usize, new_child: GreenElement) -> GreenNode {
        let mut children = self.inner.children.clone();
        children[index] = new_child;
        GreenNode::new(self.inner.kind, children)
    }

    /// Collect all text in the subtree.
    pub fn text(&self) -> String {
        let mut buf = String::new();
        self.collect_text(&mut buf);
        buf
    }

    fn collect_text(&self, buf: &mut String) {
        for child in &self.inner.children {
            match child {
                NodeOrToken::Token(t) => buf.push_str(t.text()),
                NodeOrToken::Node(n) => n.collect_text(buf),
            }
        }
    }
}

/// An immutable leaf token in the green tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GreenToken {
    kind: SyntaxKind,
    text: SmolStr,
}

impl GreenToken {
    pub fn new(kind: SyntaxKind, text: SmolStr) -> Self {
        Self { kind, text }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_len(&self) -> TextSize {
        TextSize::of(self.text.as_str())
    }
}

impl GreenElement {
    pub fn text_len(&self) -> TextSize {
        match self {
            NodeOrToken::Node(n) => n.text_len(),
            NodeOrToken::Token(t) => t.text_len(),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        match self {
            NodeOrToken::Node(n) => n.kind(),
            NodeOrToken::Token(t) => t.kind(),
        }
    }
}
