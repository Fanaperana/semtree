use semtree_core::SyntaxKind;

use crate::cache::NodeCache;
use crate::node::{GreenElement, GreenNode, NodeOrToken};

/// A builder for constructing green trees in a stack-based manner.
///
/// Usage:
/// ```ignore
/// let mut builder = GreenNodeBuilder::new();
/// builder.start_node(SyntaxKind::FUNCTION);
/// builder.token(SyntaxKind::KW_FN, "fn");
/// builder.token(SyntaxKind::IDENT, "main");
/// builder.finish_node();
/// let root = builder.finish();
/// ```
pub struct GreenNodeBuilder {
    cache: NodeCache,
    stack: Vec<(SyntaxKind, Vec<GreenElement>)>,
}

impl GreenNodeBuilder {
    pub fn new() -> Self {
        Self {
            cache: NodeCache::new(),
            stack: Vec::new(),
        }
    }

    pub fn with_cache(cache: NodeCache) -> Self {
        Self {
            cache,
            stack: Vec::new(),
        }
    }

    /// Begin a new node with the given kind.
    pub fn start_node(&mut self, kind: SyntaxKind) {
        self.stack.push((kind, Vec::new()));
    }

    /// Add a token leaf to the current node.
    pub fn token(&mut self, kind: SyntaxKind, text: &str) {
        let token = self.cache.token(kind, text);
        self.current_children().push(NodeOrToken::Token(token));
    }

    /// Finish the current node and attach it as a child of the parent.
    pub fn finish_node(&mut self) {
        let (kind, children) = self.stack.pop().expect("unbalanced start_node/finish_node");
        let node = self.cache.node(kind, children);
        if let Some((_, parent_children)) = self.stack.last_mut() {
            parent_children.push(NodeOrToken::Node(node));
        } else {
            self.stack.push((kind, vec![NodeOrToken::Node(node)]));
        }
    }

    /// Consume the builder and return the root green node.
    pub fn finish(mut self) -> GreenNode {
        assert_eq!(self.stack.len(), 1, "unbalanced tree construction");
        let (_, mut children) = self.stack.pop().unwrap();
        assert_eq!(children.len(), 1);
        match children.pop().unwrap() {
            NodeOrToken::Node(n) => n,
            NodeOrToken::Token(_) => panic!("root must be a node"),
        }
    }

    /// Return the node cache for reuse across incremental parses.
    pub fn into_cache(self) -> NodeCache {
        self.cache
    }

    /// Save a checkpoint of the current builder state for rollback.
    pub fn checkpoint(&self) -> BuilderCheckpoint {
        BuilderCheckpoint {
            stack_len: self.stack.len(),
            children_len: self.stack.last().map(|(_, c)| c.len()).unwrap_or(0),
        }
    }

    /// Rollback to a previously saved checkpoint, discarding any tokens/nodes
    /// added since the checkpoint.
    pub fn rollback(&mut self, checkpoint: BuilderCheckpoint) {
        self.stack.truncate(checkpoint.stack_len);
        if let Some((_, children)) = self.stack.last_mut() {
            children.truncate(checkpoint.children_len);
        }
    }

    fn current_children(&mut self) -> &mut Vec<GreenElement> {
        &mut self.stack.last_mut().expect("no node started").1
    }
}

/// A checkpoint into the builder state, used for speculative parsing.
#[derive(Debug, Clone, Copy)]
pub struct BuilderCheckpoint {
    stack_len: usize,
    children_len: usize,
}

impl Default for GreenNodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}
