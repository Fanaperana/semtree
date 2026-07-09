use std::sync::Arc;

use semtree_core::SyntaxKind;
use semtree_green::{GreenNode, NodeOrToken};
use text_size::{TextRange, TextSize};

use crate::token::SyntaxToken;

/// A "red" syntax node: a view into the green tree enriched with
/// parent pointers, sibling navigation, and absolute text offsets.
///
/// Red nodes are cheap to create on the fly from a green tree root.
#[derive(Debug, Clone)]
pub struct SyntaxNode {
    green: GreenNode,
    parent: Option<Arc<SyntaxNode>>,
    /// Index of this node in its parent's children list.
    index_in_parent: usize,
    /// Absolute text offset from the start of the file.
    offset: TextSize,
}

impl SyntaxNode {
    /// Create a root syntax node from a green tree.
    pub fn new_root(green: GreenNode) -> Self {
        Self {
            green,
            parent: None,
            index_in_parent: 0,
            offset: TextSize::new(0),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.green.kind()
    }

    pub fn text_range(&self) -> TextRange {
        TextRange::at(self.offset, self.green.text_len())
    }

    pub fn text(&self) -> String {
        self.green.text()
    }

    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    pub fn parent(&self) -> Option<&SyntaxNode> {
        self.parent.as_deref()
    }

    /// Iterate over child nodes and tokens.
    pub fn children_with_tokens(&self) -> Vec<SyntaxElement> {
        let self_arc = Arc::new(self.clone());
        let mut offset = self.offset;
        self.green
            .children()
            .iter()
            .enumerate()
            .map(|(i, child)| {
                let child_offset = offset;
                offset += child.text_len();
                match child {
                    NodeOrToken::Node(green) => SyntaxElement::Node(SyntaxNode {
                        green: green.clone(),
                        parent: Some(self_arc.clone()),
                        index_in_parent: i,
                        offset: child_offset,
                    }),
                    NodeOrToken::Token(green) => SyntaxElement::Token(SyntaxToken::new(
                        green.clone(),
                        Some(self_arc.clone()),
                        i,
                        child_offset,
                    )),
                }
            })
            .collect()
    }

    /// Iterate only over child nodes (skip tokens).
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.children_with_tokens()
            .into_iter()
            .filter_map(|e| match e {
                SyntaxElement::Node(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// First child node.
    pub fn first_child(&self) -> Option<SyntaxNode> {
        self.children().into_iter().next()
    }

    /// Last child node.
    pub fn last_child(&self) -> Option<SyntaxNode> {
        self.children().into_iter().last()
    }

    /// Find a child token by kind.
    pub fn child_token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.children_with_tokens()
            .into_iter()
            .find_map(|e| match e {
                SyntaxElement::Token(t) if t.kind() == kind => Some(t),
                _ => None,
            })
    }

    /// Find a child node by kind.
    pub fn child_node(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.children().into_iter().find(|n| n.kind() == kind)
    }

    /// Next sibling node.
    pub fn next_sibling(&self) -> Option<SyntaxNode> {
        let parent = self.parent.as_ref()?;
        let siblings = parent.children();
        siblings
            .into_iter()
            .skip_while(|s| s.index_in_parent != self.index_in_parent)
            .nth(1)
    }

    /// Previous sibling node.
    pub fn prev_sibling(&self) -> Option<SyntaxNode> {
        let parent = self.parent.as_ref()?;
        let siblings = parent.children();
        let mut prev = None;
        for s in siblings {
            if s.index_in_parent == self.index_in_parent {
                return prev;
            }
            prev = Some(s);
        }
        None
    }

    /// Walk all ancestors from this node up to the root.
    pub fn ancestors(&self) -> Vec<SyntaxNode> {
        let mut result = Vec::new();
        let mut current = self.parent.clone();
        while let Some(p) = current {
            result.push((*p).clone());
            current = p.parent.clone();
        }
        result
    }

    /// Depth-first traversal of all descendants.
    pub fn descendants(&self) -> Vec<SyntaxNode> {
        let mut result = Vec::new();
        self.collect_descendants(&mut result);
        result
    }

    fn collect_descendants(&self, out: &mut Vec<SyntaxNode>) {
        for child in self.children() {
            out.push(child.clone());
            child.collect_descendants(out);
        }
    }

    /// Find the deepest node covering the given offset.
    pub fn token_at_offset(&self, offset: TextSize) -> Option<SyntaxToken> {
        if !self.text_range().contains(offset) {
            return None;
        }
        for elem in self.children_with_tokens() {
            match elem {
                SyntaxElement::Token(t) if t.text_range().contains(offset) => return Some(t),
                SyntaxElement::Node(n) => {
                    if let Some(t) = n.token_at_offset(offset) {
                        return Some(t);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

/// A syntax element: either a node or a token in the red tree.
#[derive(Debug, Clone)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(SyntaxToken),
}

impl SyntaxElement {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            SyntaxElement::Node(n) => n.kind(),
            SyntaxElement::Token(t) => t.kind(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            SyntaxElement::Node(n) => n.text_range(),
            SyntaxElement::Token(t) => t.text_range(),
        }
    }
}
