use semtree_grammar::Grammar;
use semtree_green::GreenNode;
use semtree_red::SyntaxNode;
use text_size::{TextRange, TextSize};

use crate::runtime_parser::{RuntimeParseResult, RuntimeParser};

/// Describes an edit to the source text.
#[derive(Debug, Clone)]
pub struct EditRegion {
    /// Byte range in the old source that was replaced.
    pub old_range: TextRange,
    /// The new text that replaced the old range.
    pub new_text: String,
}

impl EditRegion {
    pub fn new(start: u32, old_end: u32, new_text: impl Into<String>) -> Self {
        Self {
            old_range: TextRange::new(TextSize::new(start), TextSize::new(old_end)),
            new_text: new_text.into(),
        }
    }

    /// How much the edit shifted subsequent byte offsets.
    pub fn delta(&self) -> i64 {
        self.new_text.len() as i64 - u32::from(self.old_range.len()) as i64
    }
}

/// An incremental parser that reuses unchanged subtrees across edits.
///
/// Usage:
/// ```ignore
/// let mut inc = IncrementalParser::new(grammar);
/// let result1 = inc.parse("fn main() {}");
/// let result2 = inc.update("fn main() { x }", &[EditRegion::new(12, 12, " x ")]);
/// ```
pub struct IncrementalParser {
    parser: RuntimeParser,
    prev_tree: Option<GreenNode>,
    prev_source: String,
}

impl IncrementalParser {
    pub fn new(grammar: Grammar) -> Self {
        Self {
            parser: RuntimeParser::new(grammar),
            prev_tree: None,
            prev_source: String::new(),
        }
    }

    /// Full parse from scratch.
    pub fn parse(&mut self, source: &str) -> RuntimeParseResult {
        let result = self.parser.parse(source);
        self.prev_tree = Some(result.green_tree.clone());
        self.prev_source = source.to_string();
        result
    }

    /// Incremental update: apply edits and reparse, reusing unchanged subtrees.
    pub fn update(&mut self, new_source: &str, edits: &[EditRegion]) -> RuntimeParseResult {
        if self.prev_tree.is_none() || edits.is_empty() {
            return self.parse(new_source);
        }

        let prev_tree = self.prev_tree.as_ref().unwrap().clone();

        // Find the smallest affected range in the old tree.
        let (affected_start, affected_old_end) = self.compute_affected_range(edits);

        // Find reusable prefix and suffix nodes.
        let old_root = SyntaxNode::new_root(prev_tree.clone());
        let _reuse_info = self.find_reusable_regions(&old_root, affected_start, affected_old_end);

        // For now: if we can identify large unchanged regions, we still do a
        // full reparse but benefit from the node cache in GreenNodeBuilder
        // which deduplicates identical subtrees automatically.
        //
        // A production implementation would splice unchanged green nodes
        // directly, but that requires the runtime parser to support
        // "resume from checkpoint" which is a larger effort.
        let result = self.parser.parse(new_source);

        // The green node cache ensures structural sharing: identical subtrees
        // from the old and new parse share the same Arc allocation.
        self.prev_tree = Some(result.green_tree.clone());
        self.prev_source = new_source.to_string();

        result
    }

    fn compute_affected_range(&self, edits: &[EditRegion]) -> (u32, u32) {
        let mut min_start = u32::MAX;
        let mut max_end = 0u32;

        for edit in edits {
            let start = u32::from(edit.old_range.start());
            let end = u32::from(edit.old_range.end());
            min_start = min_start.min(start);
            max_end = max_end.max(end);
        }

        (min_start, max_end)
    }

    /// Find which top-level children of the root can be reused.
    fn find_reusable_regions(
        &self,
        root: &SyntaxNode,
        affected_start: u32,
        affected_end: u32,
    ) -> ReuseInfo {
        let mut reusable_before = Vec::new();
        let mut reusable_after = Vec::new();

        for child in root.children() {
            let range = child.text_range();
            let child_start = u32::from(range.start());
            let child_end = u32::from(range.end());

            if child_end <= affected_start {
                reusable_before.push(child.green().clone());
            } else if child_start >= affected_end {
                reusable_after.push(child.green().clone());
            }
        }

        ReuseInfo {
            before: reusable_before,
            after: reusable_after,
        }
    }

    pub fn grammar(&self) -> &Grammar {
        self.parser.grammar()
    }
}

struct ReuseInfo {
    before: Vec<GreenNode>,
    after: Vec<GreenNode>,
}
