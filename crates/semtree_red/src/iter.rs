use crate::node::{SyntaxElement, SyntaxNode};

/// Events emitted during a preorder traversal.
#[derive(Debug, Clone)]
pub enum PreorderEvent {
    Enter(SyntaxElement),
    Leave(SyntaxElement),
}

/// Preorder depth-first iterator over a syntax tree.
pub struct Preorder {
    events: Vec<PreorderEvent>,
    index: usize,
}

impl Preorder {
    pub fn new(root: &SyntaxNode) -> Self {
        let mut events = Vec::new();
        Self::collect(root, &mut events);
        Self { events, index: 0 }
    }

    fn collect(node: &SyntaxNode, events: &mut Vec<PreorderEvent>) {
        events.push(PreorderEvent::Enter(SyntaxElement::Node(node.clone())));
        for child in node.children_with_tokens() {
            match &child {
                SyntaxElement::Node(n) => Self::collect(n, events),
                SyntaxElement::Token(_) => {
                    events.push(PreorderEvent::Enter(child.clone()));
                    events.push(PreorderEvent::Leave(child));
                }
            }
        }
        events.push(PreorderEvent::Leave(SyntaxElement::Node(node.clone())));
    }
}

impl Iterator for Preorder {
    type Item = PreorderEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.events.len() {
            let event = self.events[self.index].clone();
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}
