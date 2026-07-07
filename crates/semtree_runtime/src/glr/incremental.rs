use semtree_grammar::Grammar;
use semtree_green::GreenNode;

use crate::glr::driver::{GlrParseResult, GlrParser};
use crate::incremental::EditRegion;

/// Incremental GLR parser that caches parse state for efficient re-parsing.
///
/// Strategy:
/// 1. On first parse, build the full GLR parse result and cache the green tree.
/// 2. On subsequent edits, identify the affected region using edit ranges.
/// 3. If the edit is small relative to the file, try to reuse unchanged subtrees.
/// 4. Fall back to full reparse if subtree reuse isn't beneficial.
pub struct IncrementalGlr {
    parser: GlrParser,
    prev_tree: Option<GreenNode>,
    prev_source: String,
}

impl IncrementalGlr {
    pub fn new(grammar: Grammar) -> Self {
        Self {
            parser: GlrParser::new(grammar),
            prev_tree: None,
            prev_source: String::new(),
        }
    }

    /// Full parse from scratch.
    pub fn parse(&mut self, source: &str) -> GlrParseResult {
        let result = self.parser.parse(source);
        self.prev_tree = Some(result.green_tree.clone());
        self.prev_source = source.to_string();
        result
    }

    /// Incremental update after an edit.
    pub fn update(&mut self, new_source: &str, edits: &[EditRegion]) -> GlrParseResult {
        if self.prev_tree.is_none() || edits.is_empty() {
            return self.parse(new_source);
        }

        // For now, compute the affected byte range and check if the edit is
        // small enough that subtree reuse could help.
        let total_edit_size: usize = edits
            .iter()
            .map(|e| {
                let old_len = u32::from(e.old_range.len()) as usize;
                old_len.max(e.new_text.len())
            })
            .sum();

        let file_size = new_source.len();

        // If the edit touches less than 20% of the file, we could try subtree
        // reuse. For now, we do a full reparse but keep the green tree cache
        // so that Arc-based structural sharing gives us memory benefits.
        if total_edit_size < file_size / 5 {
            // Future optimization: implement state-checkpoint-based reuse here.
            // For now, full reparse with structural sharing via green node cache.
        }

        let result = self.parser.parse(new_source);
        self.prev_tree = Some(result.green_tree.clone());
        self.prev_source = new_source.to_string();
        result
    }

    pub fn has_conflicts(&self) -> bool {
        self.parser.has_conflicts()
    }

    pub fn state_count(&self) -> usize {
        self.parser.state_count()
    }
}
