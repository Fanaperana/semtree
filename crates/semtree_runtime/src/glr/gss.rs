use crate::glr::sppf::SppfNodeId;

/// A node in the Graph-Structured Stack.
#[derive(Debug, Clone)]
pub struct GssNode {
    pub state: usize,
    /// Links to predecessor nodes (multiple predecessors = merged stacks).
    pub links: Vec<GssLink>,
    pub generation: u32,
}

/// A link between GSS nodes, carrying an SPPF node as the "label" (the parse
/// tree fragment produced by the shift or reduce that created this link).
#[derive(Debug, Clone)]
pub struct GssLink {
    pub predecessor: GssNodeId,
    pub sppf_node: SppfNodeId,
}

/// Identifies a GSS node in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GssNodeId(pub u32);

/// Arena-allocated Graph-Structured Stack.
///
/// The GSS maintains all active parse stacks simultaneously. When the parser
/// encounters a shift/reduce or reduce/reduce conflict, stacks split. When
/// multiple stacks reach the same state, they merge, sharing the GSS node.
pub struct Gss {
    nodes: Vec<GssNode>,
    generation: u32,
}

impl Gss {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(1024),
            generation: 0,
        }
    }

    /// Create a new GSS node with the given parser state.
    pub fn create_node(&mut self, state: usize) -> GssNodeId {
        let id = GssNodeId(self.nodes.len() as u32);
        self.nodes.push(GssNode {
            state,
            links: Vec::new(),
            generation: self.generation,
        });
        id
    }

    /// Add a link from `from` to `to` with the given SPPF node.
    /// Returns true if this is a new link (not a duplicate).
    pub fn add_link(&mut self, from: GssNodeId, to: GssNodeId, sppf_node: SppfNodeId) -> bool {
        let node = &mut self.nodes[from.0 as usize];
        for link in &node.links {
            if link.predecessor == to {
                return false;
            }
        }
        node.links.push(GssLink {
            predecessor: to,
            sppf_node,
        });
        true
    }

    pub fn get(&self, id: GssNodeId) -> &GssNode {
        &self.nodes[id.0 as usize]
    }

    pub fn state_of(&self, id: GssNodeId) -> usize {
        self.nodes[id.0 as usize].state
    }

    pub fn links(&self, id: GssNodeId) -> &[GssLink] {
        &self.nodes[id.0 as usize].links
    }

    /// Find an existing GSS node with the given state in the current active set,
    /// or None if no such node exists.
    pub fn find_node_with_state(&self, state: usize, active: &[GssNodeId]) -> Option<GssNodeId> {
        for &id in active {
            if self.nodes[id.0 as usize].state == state {
                return Some(id);
            }
        }
        None
    }

    /// Advance generation counter (for incremental parsing).
    pub fn advance_generation(&mut self) {
        self.generation += 1;
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Collect all paths of length `depth` from `node`, returning sequences of
    /// (GssNodeId, SppfNodeId) pairs along each path. Used during reduction.
    pub fn paths(&self, node: GssNodeId, depth: usize) -> Vec<Vec<(GssNodeId, SppfNodeId)>> {
        if depth == 0 {
            return vec![vec![(node, SppfNodeId(u32::MAX))]];
        }
        let mut result = Vec::new();
        let links = self.nodes[node.0 as usize].links.clone();
        for link in &links {
            let sub_paths = self.paths(link.predecessor, depth - 1);
            for mut path in sub_paths {
                path.push((node, link.sppf_node));
                result.push(path);
            }
        }
        result
    }
}
