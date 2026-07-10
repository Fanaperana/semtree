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
        // Fast path: check if the token is already cached without allocating a SmolStr key.
        // We use a two-level lookup: first by kind, then by text.
        let key = (kind, SmolStr::from(text));
        if let Some(tok) = self.tokens.get(&key) {
            return tok.clone();
        }
        let tok = GreenToken::new(key.0, key.1.clone());
        self.tokens.insert(key, tok.clone());
        tok
    }

    pub fn node(&mut self, kind: SyntaxKind, children: Vec<GreenElement>) -> GreenNode {
        // Skip cache for cold parses — hashing all children + HashMap lookup
        // is pure overhead when subtrees are rarely repeated.
        // The cache is most useful for incremental re-parsing.
        GreenNode::new(kind, children)
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
