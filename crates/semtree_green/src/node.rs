use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rustc_hash::FxHasher;
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
/// Green nodes are purely structural: they store their kind, children, the
/// total text length spanned, and a precomputed structural hash. They have no
/// parent pointers, enabling structural sharing across incremental edits and
/// deduplication of identical subtrees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreenNode {
    inner: Arc<GreenNodeData>,
}

/// Hashing a `GreenNode` is O(1): it returns the precomputed structural hash
/// rather than re-hashing the whole subtree.
impl Hash for GreenNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.inner.hash);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GreenNodeData {
    kind: SyntaxKind,
    text_len: TextSize,
    children: Vec<GreenElement>,
    /// Precomputed structural hash (kind + children hashes). Lets the node
    /// cache dedup identical subtrees in O(children) instead of O(subtree).
    hash: u64,
}

/// Compute the structural hash of a node from its kind and children, using each
/// child's precomputed hash (O(children), not O(subtree)).
pub(crate) fn structural_hash(kind: SyntaxKind, children: &[GreenElement]) -> u64 {
    let mut h = FxHasher::default();
    kind.hash(&mut h);
    for child in children {
        match child {
            NodeOrToken::Node(n) => h.write_u64(n.inner.hash),
            NodeOrToken::Token(t) => t.hash(&mut h),
        }
    }
    h.finish()
}

impl GreenNode {
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Self {
        let hash = structural_hash(kind, &children);
        Self::with_hash(kind, children, hash)
    }

    /// Construct a node with an already-computed structural hash (used by the
    /// node cache, which computes the hash before deciding whether to allocate).
    pub(crate) fn with_hash(kind: SyntaxKind, children: Vec<GreenElement>, hash: u64) -> Self {
        let text_len = children.iter().map(|c| c.text_len()).sum();
        Self {
            inner: Arc::new(GreenNodeData {
                kind,
                text_len,
                children,
                hash,
            }),
        }
    }

    /// True if two nodes are the *same allocation* (cheap identity check used to
    /// verify cache hits without a deep structural comparison).
    pub(crate) fn ptr_eq(&self, other: &GreenNode) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// A stable identity for this node's allocation. Two `GreenNode`s that share
    /// the same interned subtree return the same `id`, which lets callers count
    /// *distinct* heap allocations (as opposed to structural node occurrences).
    pub fn id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
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
