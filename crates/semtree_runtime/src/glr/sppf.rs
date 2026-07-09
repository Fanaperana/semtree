use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use semtree_core::SyntaxKind;
use semtree_green::{GreenNode, GreenNodeBuilder};

use crate::glr::table::Symbol;
use crate::runtime_parser::rule_name_to_kind;

/// Identifies an SPPF node in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SppfNodeId(pub u32);

/// The kind of SPPF node.
#[derive(Debug, Clone)]
pub enum SppfNodeKind {
    /// A terminal symbol (leaf): holds the token text.
    Terminal {
        symbol: Symbol,
        text: SmolStr,
        range: TextRange,
        syntax_kind: SyntaxKind,
    },
    /// A non-terminal symbol node: produced by a reduction.
    Symbol {
        name: SmolStr,
        /// Children of this node (may include packed nodes for ambiguity).
        children: Vec<SppfNodeId>,
        range: TextRange,
    },
    /// A packed node: represents one alternative derivation in an ambiguity.
    /// Multiple packed nodes under the same Symbol node = ambiguity.
    Packed {
        production_id: usize,
        children: Vec<SppfNodeId>,
    },
    /// An error node: wraps tokens that couldn't be parsed.
    Error {
        children: Vec<SppfNodeId>,
        range: TextRange,
        message: String,
    },
    /// Epsilon node: represents an empty production.
    Epsilon,
}

/// Arena-allocated Shared Packed Parse Forest.
///
/// The SPPF compactly represents all possible parse trees for an ambiguous
/// grammar. Each Symbol node can have multiple Packed children, each
/// representing one way to derive that symbol.
pub struct Sppf {
    nodes: Vec<SppfNodeKind>,
}

impl Default for Sppf {
    fn default() -> Self {
        Self::new()
    }
}

impl Sppf {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(4096),
        }
    }

    pub fn create_terminal(
        &mut self,
        symbol: Symbol,
        text: SmolStr,
        range: TextRange,
        syntax_kind: SyntaxKind,
    ) -> SppfNodeId {
        let id = SppfNodeId(self.nodes.len() as u32);
        self.nodes.push(SppfNodeKind::Terminal {
            symbol,
            text,
            range,
            syntax_kind,
        });
        id
    }

    pub fn create_symbol(
        &mut self,
        name: SmolStr,
        children: Vec<SppfNodeId>,
        range: TextRange,
    ) -> SppfNodeId {
        let id = SppfNodeId(self.nodes.len() as u32);
        self.nodes.push(SppfNodeKind::Symbol {
            name,
            children,
            range,
        });
        id
    }

    pub fn create_packed(&mut self, production_id: usize, children: Vec<SppfNodeId>) -> SppfNodeId {
        let id = SppfNodeId(self.nodes.len() as u32);
        self.nodes.push(SppfNodeKind::Packed {
            production_id,
            children,
        });
        id
    }

    pub fn create_error(
        &mut self,
        children: Vec<SppfNodeId>,
        range: TextRange,
        message: String,
    ) -> SppfNodeId {
        let id = SppfNodeId(self.nodes.len() as u32);
        self.nodes.push(SppfNodeKind::Error {
            children,
            range,
            message,
        });
        id
    }

    pub fn create_epsilon(&mut self) -> SppfNodeId {
        let id = SppfNodeId(self.nodes.len() as u32);
        self.nodes.push(SppfNodeKind::Epsilon);
        id
    }

    pub fn get(&self, id: SppfNodeId) -> &SppfNodeKind {
        &self.nodes[id.0 as usize]
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Convert the SPPF into a lossless GreenNode tree.
    /// When ambiguity exists (multiple packed nodes), picks the first alternative.
    pub fn to_green_tree(&self, root: SppfNodeId) -> GreenNode {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::SOURCE_FILE);
        self.build_green(root, &mut builder);
        builder.finish_node();
        builder.finish()
    }

    fn build_green(&self, id: SppfNodeId, builder: &mut GreenNodeBuilder) {
        match &self.nodes[id.0 as usize] {
            SppfNodeKind::Terminal {
                text, syntax_kind, ..
            } => {
                builder.token(*syntax_kind, text.as_str());
            }
            SppfNodeKind::Symbol { name, children, .. } => {
                let kind = rule_name_to_kind(name);
                builder.start_node(kind);
                for &child in children {
                    self.build_green(child, builder);
                }
                builder.finish_node();
            }
            SppfNodeKind::Packed { children, .. } => {
                for &child in children {
                    self.build_green(child, builder);
                }
            }
            SppfNodeKind::Error {
                children,
                message: _,
                ..
            } => {
                builder.start_node(SyntaxKind::ERROR);
                for &child in children {
                    self.build_green(child, builder);
                }
                if children.is_empty() {
                    builder.token(SyntaxKind::ERROR, "");
                }
                builder.finish_node();
            }
            SppfNodeKind::Epsilon => {}
        }
    }

    /// Get the text range of an SPPF node.
    pub fn range_of(&self, id: SppfNodeId) -> TextRange {
        match &self.nodes[id.0 as usize] {
            SppfNodeKind::Terminal { range, .. } => *range,
            SppfNodeKind::Symbol { range, .. } => *range,
            SppfNodeKind::Error { range, .. } => *range,
            SppfNodeKind::Packed { children, .. } => {
                if children.is_empty() {
                    TextRange::new(TextSize::new(0), TextSize::new(0))
                } else {
                    let start = self.range_of(children[0]).start();
                    let end = self.range_of(*children.last().unwrap()).end();
                    TextRange::new(start, end)
                }
            }
            SppfNodeKind::Epsilon => TextRange::new(TextSize::new(0), TextSize::new(0)),
        }
    }
}
