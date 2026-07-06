use semtree_core::Token;
use semtree_green::{GreenNode, GreenNodeBuilder};

use crate::event::Event;

/// Converts a flat event stream into a green tree, handling forward parents.
pub(crate) struct Sink<'a> {
    tokens: &'a [Token],
    events: Vec<Event>,
    token_idx: usize,
    builder: GreenNodeBuilder,
}

impl<'a> Sink<'a> {
    pub fn new(tokens: &'a [Token], events: Vec<Event>) -> Self {
        Self {
            tokens,
            events,
            token_idx: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    pub fn finish(mut self) -> GreenNode {
        // Track which events have been consumed as forward_parent targets
        let mut eaten = vec![false; self.events.len()];

        // First pass: mark all events reachable through forward_parent chains
        for i in 0..self.events.len() {
            if let Event::StartNode { forward_parent: Some(offset), .. } = &self.events[i] {
                let target = i + offset;
                eaten[target] = true;
                // Follow the chain
                let mut idx = target;
                loop {
                    if let Event::StartNode { forward_parent: Some(off), .. } = &self.events[idx] {
                        let next = idx + off;
                        eaten[next] = true;
                        idx = next;
                    } else {
                        break;
                    }
                }
            }
        }

        let mut forward_parents = Vec::new();

        for i in 0..self.events.len() {
            match self.events[i].clone() {
                Event::StartNode { kind, forward_parent } => {
                    // Skip events that are only reachable as forward_parent targets
                    if eaten[i] {
                        continue;
                    }

                    forward_parents.push(kind);
                    let mut idx = i;
                    let mut fp = forward_parent;
                    while let Some(offset) = fp {
                        idx += offset;
                        match &self.events[idx] {
                            Event::StartNode { kind, forward_parent } => {
                                forward_parents.push(*kind);
                                fp = *forward_parent;
                            }
                            _ => break,
                        }
                    }

                    for kind in forward_parents.drain(..).rev() {
                        self.builder.start_node(kind);
                    }
                }
                Event::AddToken => {
                    self.eat_trivia();
                    if self.token_idx < self.tokens.len() {
                        let token = &self.tokens[self.token_idx];
                        self.builder.token(token.kind, token.text.as_str());
                        self.token_idx += 1;
                    }
                }
                Event::FinishNode => {
                    self.builder.finish_node();
                }
                Event::Placeholder => {}
            }
        }

        self.builder.finish()
    }

    fn eat_trivia(&mut self) {
        while self.token_idx < self.tokens.len() && self.tokens[self.token_idx].kind.is_trivia() {
            let token = &self.tokens[self.token_idx];
            self.builder.token(token.kind, token.text.as_str());
            self.token_idx += 1;
        }
    }
}
