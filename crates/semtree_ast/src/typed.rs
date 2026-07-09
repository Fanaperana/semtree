use semtree_core::SyntaxKind;
use semtree_red::{SyntaxNode, SyntaxToken};

/// Trait for typed AST nodes that wrap a `SyntaxNode`.
///
/// Every strongly-typed AST type implements this trait. It provides
/// safe casting between the untyped syntax tree and typed wrappers.
pub trait AstNode: Sized {
    /// The `SyntaxKind` this node wraps.
    fn kind() -> SyntaxKind;

    /// Try to cast an untyped `SyntaxNode` to this typed node.
    /// Returns `None` if the kind doesn't match.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// Get the underlying untyped syntax node.
    fn syntax(&self) -> &SyntaxNode;

    /// Can this kind be cast to Self?
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == Self::kind()
    }
}

/// Trait for typed AST tokens.
pub trait AstToken: Sized {
    fn kind() -> SyntaxKind;
    fn cast(token: SyntaxToken) -> Option<Self>;
    fn syntax(&self) -> &SyntaxToken;
    fn text(&self) -> &str {
        self.syntax().text()
    }
}

/// An iterator adapter that yields only children of a specific typed AST node.
pub struct AstChildren<N: AstNode> {
    inner: std::vec::IntoIter<SyntaxNode>,
    _marker: std::marker::PhantomData<N>,
}

impl<N: AstNode> AstChildren<N> {
    pub fn new(parent: &SyntaxNode) -> Self {
        Self {
            inner: parent.children().into_iter(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<N: AstNode> Iterator for AstChildren<N> {
    type Item = N;

    fn next(&mut self) -> Option<N> {
        loop {
            let node = self.inner.next()?;
            if let Some(typed) = N::cast(node) {
                return Some(typed);
            }
        }
    }
}
