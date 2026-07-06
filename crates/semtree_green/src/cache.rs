use rustc_hash::FxHashMap;
use smol_str::SmolStr;

use semtree_core::SyntaxKind;

use crate::node::{GreenElement, GreenNode, GreenToken};

/// Caches green nodes and tokens to enable structural sharing.
///
/// When the same subtree appears multiple times (or is unchanged across edits),
/// the cache returns the same `Arc`-backed instance.
#[derive(Debug)]
pub struct NodeCache {
    tokens: FxHashMap<(SyntaxKind, SmolStr), GreenToken>,
    nodes: FxHashMap<(SyntaxKind, u64), GreenNode>,
}

impl NodeCache {
    pub fn new() -> Self {
        Self {
            tokens: FxHashMap::default(),
            nodes: FxHashMap::default(),
        }
    }

    pub fn token(&mut self, kind: SyntaxKind, text: &str) -> GreenToken {
        let key = (kind, SmolStr::from(text));
        self.tokens
            .entry(key.clone())
            .or_insert_with(|| GreenToken::new(key.0, key.1))
            .clone()
    }

    pub fn node(&mut self, kind: SyntaxKind, children: Vec<GreenElement>) -> GreenNode {
        let hash = self.hash_children(&children);
        let key = (kind, hash);
        self.nodes
            .entry(key)
            .or_insert_with(|| GreenNode::new(kind, children))
            .clone()
    }

    fn hash_children(&self, children: &[GreenElement]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        children.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for NodeCache {
    fn default() -> Self {
        Self::new()
    }
}
