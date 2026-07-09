use semtree_core::SyntaxKind;
use semtree_grammar::Grammar;
use semtree_green::{GreenElement, GreenNode, GreenNodeBuilder, NodeOrToken};
use semtree_red::SyntaxNode;
use text_size::{TextRange, TextSize};

use crate::runtime_lexer::RuntimeLexer;
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

    pub fn delta(&self) -> i64 {
        self.new_text.len() as i64 - u32::from(self.old_range.len()) as i64
    }
}

/// Apply a set of edits to a source string, producing the new source.
pub fn apply_edits(source: &str, edits: &[EditRegion]) -> String {
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(u32::from(e.old_range.start())));

    let mut result = source.to_string();
    for edit in sorted {
        let start = u32::from(edit.old_range.start()) as usize;
        let end = u32::from(edit.old_range.end()) as usize;
        result.replace_range(start..end, &edit.new_text);
    }
    result
}

/// An incremental parser that reuses unchanged subtrees across edits.
///
/// The strategy:
/// 1. Find affected byte range from the edits.
/// 2. Identify top-level children completely before/after the edit.
/// 3. Reparse only the affected region.
/// 4. Splice reusable prefix/suffix green nodes with the freshly parsed middle.
///
/// The green node cache also provides structural sharing: identical subtrees
/// from old and new parses share the same Arc allocation automatically.
pub struct IncrementalParser {
    parser: RuntimeParser,
    #[allow(dead_code)]
    lexer: RuntimeLexer,
    prev_tree: Option<GreenNode>,
    prev_source: String,
    #[allow(dead_code)]
    node_cache: Option<semtree_green::NodeCache>,
}

impl IncrementalParser {
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        Self {
            parser: RuntimeParser::new(grammar),
            lexer,
            prev_tree: None,
            prev_source: String::new(),
            node_cache: None,
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
        let old_root = SyntaxNode::new_root(prev_tree.clone());

        let (affected_start, affected_old_end) = self.compute_affected_range(edits);
        let delta: i64 = edits.iter().map(|e| e.delta()).sum();

        let (reusable_before, reusable_after, reparse_start, reparse_end) =
            self.find_reusable_children(&old_root, affected_start, affected_old_end, delta);

        // If we can reuse prefix + suffix and only reparse the middle, do so.
        if !reusable_before.is_empty() || !reusable_after.is_empty() {
            let reparse_region = &new_source[reparse_start..reparse_end];

            // Reparse just the middle.
            let middle_result = self.parser.parse(reparse_region);
            let middle_root = SyntaxNode::new_root(middle_result.green_tree.clone());

            // Build the new root by splicing: prefix + middle children + suffix.
            let mut builder = GreenNodeBuilder::new();
            builder.start_node(SyntaxKind::SOURCE_FILE);

            // Add prefix (reusable before).
            for green in &reusable_before {
                Self::emit_green_node(&mut builder, green);
            }

            // Add middle children (from the reparsed section).
            for child in middle_root.green().children() {
                Self::emit_green_element(&mut builder, child);
            }

            // Add suffix (reusable after, with offsets shifted).
            for green in &reusable_after {
                Self::emit_green_node(&mut builder, green);
            }

            builder.finish_node();
            let new_tree = builder.finish();

            // Verify lossless roundtrip — fall back to full reparse if splicing broke.
            if new_tree.text() != new_source {
                let result = self.parser.parse(new_source);
                self.prev_tree = Some(result.green_tree.clone());
                self.prev_source = new_source.to_string();
                return result;
            }

            self.prev_tree = Some(new_tree.clone());
            self.prev_source = new_source.to_string();

            return RuntimeParseResult {
                green_tree: new_tree,
                errors: middle_result.errors,
                kind_names: middle_result.kind_names,
            };
        }

        // Fall back to full reparse.
        let result = self.parser.parse(new_source);
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

    /// Find children that can be reused and the byte range to reparse in the new source.
    fn find_reusable_children(
        &self,
        root: &SyntaxNode,
        affected_start: u32,
        affected_old_end: u32,
        delta: i64,
    ) -> (Vec<GreenNode>, Vec<GreenNode>, usize, usize) {
        let mut reusable_before = Vec::new();
        let mut reusable_after = Vec::new();
        let mut prefix_end: u32 = 0;
        let mut suffix_old_start: u32 = u32::from(root.text_range().end());

        let children = root.children();

        // Collect prefix: children entirely before the edit.
        for child in &children {
            let range = child.text_range();
            let child_end = u32::from(range.end());
            if child_end <= affected_start {
                reusable_before.push(child.green().clone());
                prefix_end = child_end;
            } else {
                break;
            }
        }

        // Collect suffix: children entirely after the edit.
        for child in children.iter().rev() {
            let range = child.text_range();
            let child_start = u32::from(range.start());
            if child_start >= affected_old_end {
                reusable_after.push(child.green().clone());
                suffix_old_start = child_start;
            } else {
                break;
            }
        }
        reusable_after.reverse();

        // Compute reparse range in the *new* source.
        let reparse_start = prefix_end as usize;
        let new_suffix_start = (suffix_old_start as i64 + delta) as usize;
        let new_source_len = (self.prev_source.len() as i64 + delta) as usize;
        let reparse_end = new_suffix_start.min(new_source_len);

        (reusable_before, reusable_after, reparse_start, reparse_end)
    }

    /// Emit a GreenNode into the builder by walking its structure.
    fn emit_green_node(builder: &mut GreenNodeBuilder, node: &GreenNode) {
        builder.start_node(node.kind());
        for child in node.children() {
            Self::emit_green_element(builder, child);
        }
        builder.finish_node();
    }

    fn emit_green_element(builder: &mut GreenNodeBuilder, element: &GreenElement) {
        match element {
            NodeOrToken::Node(n) => Self::emit_green_node(builder, n),
            NodeOrToken::Token(t) => builder.token(t.kind(), t.text()),
        }
    }

    pub fn grammar(&self) -> &Grammar {
        self.parser.grammar()
    }
}
