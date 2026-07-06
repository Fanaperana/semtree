use semtree_core::SyntaxKind;

/// Events produced by the parser, later consumed by the tree builder (sink).
#[derive(Debug, Clone)]
pub enum Event {
    /// Begin a new node.
    StartNode {
        kind: SyntaxKind,
        /// If set, this node will be placed as a parent of the node that
        /// started at `forward_parent` positions ago.
        forward_parent: Option<usize>,
    },
    /// Add the current token and advance.
    AddToken,
    /// Finish the current node.
    FinishNode,
    /// A placeholder event that can be overwritten.
    Placeholder,
}
