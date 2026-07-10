use rustc_hash::FxHashMap;
use semtree_core::SyntaxKind;
use semtree_grammar::Grammar;
use semtree_green::{GreenElement, GreenNode, NodeOrToken};
use semtree_red::SyntaxNode;
use smol_str::SmolStr;
use text_size::{TextRange, TextSize};

use crate::runtime_lexer::{RawToken, RuntimeLexer, RuntimeTokenKind};
use crate::runtime_parser::{RuntimeParseError, RuntimeParseResult, RuntimeParser};

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

/// How the most recent [`IncrementalParser::update`] produced its tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReuseKind {
    /// Parsed from scratch (first parse, or `update` called with no prior tree
    /// or no edits). Not an incremental attempt.
    FullParse,
    /// Reparsed a single inner subtree and spliced it back into the old tree.
    DeepSplice,
    /// Reused leading/trailing sibling subtrees; only the middle was reparsed.
    SiblingSplice,
    /// An incremental splice was attempted but the result did not match the
    /// source (or no subtree could be reused), so a full reparse was used.
    SpliceMiss,
}

/// Metrics describing the most recent [`IncrementalParser::update`].
#[derive(Debug, Clone, Copy)]
pub struct ReuseInfo {
    pub kind: ReuseKind,
    /// Total size of the new source, in bytes.
    pub total_bytes: usize,
    /// Bytes that had to be reparsed.
    pub reparsed_bytes: usize,
}

impl ReuseInfo {
    /// True when a subtree was actually reused (deep or sibling splice).
    pub fn is_hit(&self) -> bool {
        matches!(self.kind, ReuseKind::DeepSplice | ReuseKind::SiblingSplice)
    }

    /// Bytes that were reused (not reparsed).
    pub fn reused_bytes(&self) -> usize {
        self.total_bytes.saturating_sub(self.reparsed_bytes)
    }

    /// Fraction of the source that was reused, in `0.0..=1.0`.
    pub fn reuse_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        self.reused_bytes() as f64 / self.total_bytes as f64
    }
}

/// Intermediate result of a recursive splice: the rebuilt subtree plus the
/// reparse metadata for the single region that had to be reparsed.
struct SpliceResult {
    green: GreenNode,
    reparsed_bytes: usize,
    errors: Vec<RuntimeParseError>,
    kind_names: FxHashMap<SyntaxKind, SmolStr>,
}

/// An incremental parser that reuses unchanged subtrees and tokens across edits.
///
/// The strategy:
/// 1. **Incremental lexing**: find which tokens overlap the edit, re-lex only
///    a small window around the edit, splice old prefix/suffix tokens with
///    shifted ranges.
/// 2. **Incremental parsing**: identify top-level children entirely before/after
///    the edit, reparse only the affected middle region, splice green nodes.
/// 3. **Lossless fallback**: if splicing produces a tree whose text doesn't match
///    the new source, fall back to a full reparse.
pub struct IncrementalParser {
    parser: RuntimeParser,
    lexer: RuntimeLexer,
    prev_tree: Option<GreenNode>,
    prev_source: String,
    prev_tokens: Vec<RawToken>,
    last_reuse: Option<ReuseInfo>,
}

impl IncrementalParser {
    pub fn new(grammar: Grammar) -> Self {
        let lexer = RuntimeLexer::new(&grammar);
        Self {
            parser: RuntimeParser::new(grammar),
            lexer,
            prev_tree: None,
            prev_source: String::new(),
            prev_tokens: Vec::new(),
            last_reuse: None,
        }
    }

    /// Metrics for the most recent [`Self::update`] (or [`Self::parse`]) call,
    /// describing whether a subtree was reused and how much was reparsed.
    /// `None` before the first parse.
    pub fn last_reuse(&self) -> Option<ReuseInfo> {
        self.last_reuse
    }

    /// Full parse from scratch.
    pub fn parse(&mut self, source: &str) -> RuntimeParseResult {
        self.prev_tokens = self.lexer.tokenize(source);
        let result = self.parser.parse(source);
        self.prev_tree = Some(result.green_tree.clone());
        self.prev_source = source.to_string();
        self.last_reuse = Some(ReuseInfo {
            kind: ReuseKind::FullParse,
            total_bytes: source.len(),
            reparsed_bytes: source.len(),
        });
        result
    }

    /// Incremental update: apply edits and reparse, reusing unchanged subtrees.
    ///
    /// Strategy, cheapest first:
    /// 1. **Recursive path-copy splice** — descend to the deepest node whose
    ///    subtree fully contains the edit, reparse only that node's text, and
    ///    `replace_child` up the spine (every untouched sibling is Arc-cloned).
    /// 2. **Top-level sibling splice** — reuse leading/trailing top-level
    ///    children (handles appends and edits spanning several top-level items).
    /// 3. **Full reparse** — always correct; used when neither splice reproduces
    ///    the new source exactly.
    ///
    /// Every splice is verified to reproduce `new_source` byte-for-byte before
    /// it is accepted, so an incorrect splice can never escape.
    pub fn update(&mut self, new_source: &str, edits: &[EditRegion]) -> RuntimeParseResult {
        if self.prev_tree.is_none() || edits.is_empty() {
            return self.parse(new_source);
        }

        let prev_tree = self.prev_tree.as_ref().unwrap().clone();
        let old_root = SyntaxNode::new_root(prev_tree.clone());

        let (affected_start, affected_old_end) = self.compute_affected_range(edits);
        let delta: i64 = edits.iter().map(|e| e.delta()).sum();
        let total = new_source.len();

        // 1. Recursive path-copy splice.
        if let Some(res) =
            self.splice_node(&prev_tree, 0, affected_start, affected_old_end, delta, new_source)
        {
            if res.reparsed_bytes < total && res.green.text() == new_source {
                self.commit(res.green.clone(), new_source);
                self.last_reuse = Some(ReuseInfo {
                    kind: ReuseKind::DeepSplice,
                    total_bytes: total,
                    reparsed_bytes: res.reparsed_bytes,
                });
                return RuntimeParseResult {
                    green_tree: res.green,
                    errors: res.errors,
                    kind_names: res.kind_names,
                };
            }
        }

        // 2. Top-level sibling splice (reuse prefix/suffix top-level children).
        let (reusable_before, reusable_after, reparse_start, reparse_end) =
            self.find_reusable_children(&old_root, affected_start, affected_old_end, delta);

        if !reusable_before.is_empty() || !reusable_after.is_empty() {
            let reparse_region = &new_source[reparse_start..reparse_end];
            let middle_result = self.parser.parse(reparse_region);

            let mut children: Vec<GreenElement> = Vec::with_capacity(
                reusable_before.len() + reusable_after.len() + middle_result.green_tree.children_count(),
            );
            children.extend(reusable_before.iter().cloned().map(NodeOrToken::Node));
            children.extend(middle_result.green_tree.children().iter().cloned());
            children.extend(reusable_after.iter().cloned().map(NodeOrToken::Node));
            let new_tree = GreenNode::new(SyntaxKind::SOURCE_FILE, children);

            if new_tree.text() == new_source {
                self.commit(new_tree.clone(), new_source);
                self.last_reuse = Some(ReuseInfo {
                    kind: ReuseKind::SiblingSplice,
                    total_bytes: total,
                    reparsed_bytes: reparse_end.saturating_sub(reparse_start),
                });
                return RuntimeParseResult {
                    green_tree: new_tree,
                    errors: middle_result.errors,
                    kind_names: middle_result.kind_names,
                };
            }
        }

        // 3. Fallback: full reparse (always correct).
        let result = self.parser.parse(new_source);
        self.commit(result.green_tree.clone(), new_source);
        self.last_reuse = Some(ReuseInfo {
            kind: ReuseKind::SpliceMiss,
            total_bytes: total,
            reparsed_bytes: total,
        });
        result
    }

    /// Record a newly produced tree as the base for the next incremental update.
    fn commit(&mut self, tree: GreenNode, new_source: &str) {
        self.prev_tree = Some(tree);
        self.prev_source = new_source.to_string();
    }

    /// Recursively descend to the deepest node whose subtree fully contains the
    /// edit, reparse only that node's text, and rebuild the spine with
    /// `replace_child` (untouched siblings are Arc-cloned — O(spine), not O(n)).
    /// Returns `None` at any level where the edit is not cleanly contained in a
    /// single node child; the caller then tries a coarser strategy.
    fn splice_node(
        &self,
        node: &GreenNode,
        node_start: u32,
        a_start: u32,
        a_old_end: u32,
        delta: i64,
        new_source: &str,
    ) -> Option<SpliceResult> {
        let mut offset = node_start;
        let mut affected: Option<(usize, GreenNode, u32)> = None;
        let mut overlap_count = 0u32;
        for (i, child) in node.children().iter().enumerate() {
            let cs = offset;
            let ce = offset + u32::from(child.text_len());
            offset = ce;
            // An insertion at point P belongs to the child whose half-open range
            // [cs, ce) contains P; a replacement/deletion overlaps any child that
            // intersects [a_start, a_old_end).
            let overlaps = if a_start == a_old_end {
                cs <= a_start && a_start < ce
            } else {
                cs < a_old_end && a_start < ce
            };
            if !overlaps {
                continue;
            }
            overlap_count += 1;
            if let NodeOrToken::Node(n) = child {
                let contains = if a_start == a_old_end {
                    cs <= a_start && a_start < ce
                } else {
                    cs <= a_start && a_old_end <= ce
                };
                if contains {
                    affected = Some((i, n.clone(), cs));
                }
            }
        }

        if overlap_count == 1 {
            if let Some((idx, child, cs)) = affected {
                // Narrow deeper if we can; otherwise reparse this child in place.
                if let Some(sub) =
                    self.splice_node(&child, cs, a_start, a_old_end, delta, new_source)
                {
                    let SpliceResult { green, reparsed_bytes, errors, kind_names } = sub;
                    return Some(SpliceResult {
                        green: node.replace_child(idx, NodeOrToken::Node(green)),
                        reparsed_bytes,
                        errors,
                        kind_names,
                    });
                }
                return self.reparse_child_in_place(node, idx, &child, cs, delta, new_source);
            }
        }
        None
    }

    /// Base case of [`Self::splice_node`]: reparse the affected child's text and
    /// splice the reparsed `source_file`'s children (trivia included) back in
    /// place of that child. This preserves every byte of source — the surviving
    /// siblings are Arc-cloned, and `update` verifies the whole tree round-trips.
    fn reparse_child_in_place(
        &self,
        parent: &GreenNode,
        idx: usize,
        child: &GreenNode,
        child_start: u32,
        delta: i64,
        new_source: &str,
    ) -> Option<SpliceResult> {
        let old_len = u32::from(child.text_len()) as i64;
        let new_end = child_start as i64 + old_len + delta;
        if new_end < child_start as i64 || new_end as usize > new_source.len() {
            return None;
        }
        let region = &new_source[child_start as usize..new_end as usize];
        let reparsed = self.parser.parse(region);
        let old_children = parent.children();
        let reparsed_children = reparsed.green_tree.children();
        let mut kids: Vec<GreenElement> =
            Vec::with_capacity(old_children.len() + reparsed_children.len());
        kids.extend_from_slice(&old_children[..idx]);
        kids.extend(reparsed_children.iter().cloned());
        kids.extend_from_slice(&old_children[idx + 1..]);
        Some(SpliceResult {
            green: GreenNode::new(parent.kind(), kids),
            reparsed_bytes: region.len(),
            errors: reparsed.errors,
            kind_names: reparsed.kind_names,
        })
    }

    /// Incremental lexing: reuse prefix/suffix tokens, re-lex only the affected window.
    pub fn incremental_lex(
        &self,
        new_source: &str,
        affected_start: u32,
        affected_old_end: u32,
        delta: i64,
    ) -> Vec<RawToken> {
        if self.prev_tokens.is_empty() {
            return self.lexer.tokenize(new_source);
        }

        // Find the first token that overlaps or comes after the edit start.
        let first_affected = self
            .prev_tokens
            .iter()
            .position(|t| u32::from(t.range.end()) > affected_start)
            .unwrap_or(self.prev_tokens.len());

        // Find the last token that overlaps or comes before the old edit end.
        let last_affected = self
            .prev_tokens
            .iter()
            .rposition(|t| u32::from(t.range.start()) < affected_old_end)
            .map(|i| i + 1)
            .unwrap_or(first_affected);

        // Widen the window by one token on each side for safety (context sensitivity).
        let relex_start_idx = first_affected.saturating_sub(1);
        let relex_end_idx = (last_affected + 1).min(self.prev_tokens.len());

        // Byte range to re-lex in the *new* source.
        let relex_byte_start = if relex_start_idx < self.prev_tokens.len() {
            u32::from(self.prev_tokens[relex_start_idx].range.start()) as usize
        } else {
            new_source.len()
        };

        // End byte in the old source for the last token in the window.
        let relex_byte_old_end = if relex_end_idx > 0 && relex_end_idx <= self.prev_tokens.len() {
            u32::from(self.prev_tokens[relex_end_idx - 1].range.end()) as usize
        } else {
            self.prev_source.len()
        };

        let relex_byte_new_end =
            ((relex_byte_old_end as i64 + delta) as usize).min(new_source.len());

        // Re-lex just this window.
        let window = &new_source[relex_byte_start..relex_byte_new_end];
        let mut relexed = self.lexer.tokenize(window);

        // Remove the trailing Eof from the relexed window (we'll get it from the suffix or add one).
        if relexed.last().map(|t| t.kind) == Some(RuntimeTokenKind::Eof) {
            relexed.pop();
        }

        // Shift relexed token ranges by the window start offset.
        let offset = TextSize::new(relex_byte_start as u32);
        for tok in &mut relexed {
            tok.range = TextRange::new(tok.range.start() + offset, tok.range.end() + offset);
        }

        // Prefix: tokens entirely before the relex window.
        let prefix = &self.prev_tokens[..relex_start_idx];

        // Suffix: tokens after the relex window, with ranges shifted by delta.
        let suffix_start_idx = relex_end_idx;
        let suffix: Vec<RawToken> = self.prev_tokens[suffix_start_idx..]
            .iter()
            .map(|t| {
                if t.kind == RuntimeTokenKind::Eof {
                    RawToken {
                        kind: RuntimeTokenKind::Eof,
                        range: TextRange::new(
                            TextSize::new(new_source.len() as u32),
                            TextSize::new(new_source.len() as u32),
                        ),
                    }
                } else {
                    let new_start = (u32::from(t.range.start()) as i64 + delta).max(0) as u32;
                    let new_end = (u32::from(t.range.end()) as i64 + delta).max(0) as u32;
                    RawToken {
                        kind: t.kind,
                        range: TextRange::new(TextSize::new(new_start), TextSize::new(new_end)),
                    }
                }
            })
            .collect();

        let mut result = Vec::with_capacity(prefix.len() + relexed.len() + suffix.len());
        result.extend_from_slice(prefix);
        result.extend(relexed);
        result.extend(suffix);

        // Ensure we have an Eof at the end.
        if result.last().map(|t| t.kind) != Some(RuntimeTokenKind::Eof) {
            result.push(RawToken {
                kind: RuntimeTokenKind::Eof,
                range: TextRange::new(
                    TextSize::new(new_source.len() as u32),
                    TextSize::new(new_source.len() as u32),
                ),
            });
        }

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

        let reparse_start = prefix_end as usize;
        let new_suffix_start = (suffix_old_start as i64 + delta) as usize;
        let new_source_len = (self.prev_source.len() as i64 + delta) as usize;
        let reparse_end = new_suffix_start.min(new_source_len);

        (reusable_before, reusable_after, reparse_start, reparse_end)
    }

    pub fn grammar(&self) -> &Grammar {
        self.parser.grammar()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semtree_grammar::parse_semtree_dsl;

    fn test_grammar() -> Grammar {
        parse_semtree_dsl(
            r#"
language simple

keyword fn
keyword let
keyword return

Function :=
    "fn" name: Identifier "(" ")" "{" Statement* "}"

Statement :=
    LetStatement | ReturnStatement

LetStatement :=
    "let" Identifier "=" Expression ";"

ReturnStatement :=
    "return" Expression ";"

Expression :=
    Identifier | Integer | StringLit

StringLit :=
    String
"#,
        )
        .unwrap()
    }

    #[test]
    fn mid_edit_reuses_subtree() {
        let grammar = test_grammar();
        let mut inc = IncrementalParser::new(grammar);
        let src = "fn a() { let x = 1; }\nfn b() { let y = 2; }\nfn c() { let z = 3; }\n";
        inc.parse(src);
        // Insert a space just before the `2` inside b's body.
        let pos = src.find('2').unwrap() as u32;
        let mut new_src = src.to_string();
        new_src.insert(pos as usize, ' ');
        let edit = EditRegion::new(pos, pos, " ");
        let res = inc.update(&new_src, &[edit]);
        assert_eq!(
            SyntaxNode::new_root(res.green_tree).text(),
            new_src,
            "incremental update must be lossless"
        );
        let info = inc.last_reuse().unwrap();
        eprintln!(
            "mid-edit reuse: kind={:?} reparsed={} total={} ratio={:.2}",
            info.kind,
            info.reparsed_bytes,
            info.total_bytes,
            info.reuse_ratio()
        );
        assert!(
            info.is_hit(),
            "mid-file insert should reuse a subtree, got {:?}",
            info.kind
        );
    }

    #[test]
    fn last_reuse_tracks_full_parse_and_splice() {        let grammar = test_grammar();
        let mut inc = IncrementalParser::new(grammar);

        let src = "fn a() { let x = 1; }\nfn b() { let y = 2; }\n";
        inc.parse(src);
        let after_parse = inc.last_reuse().expect("last_reuse set after parse");
        assert_eq!(after_parse.kind, ReuseKind::FullParse);
        assert_eq!(after_parse.total_bytes, src.len());
        assert_eq!(after_parse.reparsed_bytes, src.len());
        assert_eq!(after_parse.reused_bytes(), 0);

        // Append a whole new function at the end.
        let appended = "fn c() { let z = 3; }\n";
        let mut new_src = src.to_string();
        new_src.push_str(appended);
        let edit = EditRegion::new(src.len() as u32, src.len() as u32, appended);
        let result = inc.update(&new_src, &[edit]);

        // Losslessness: the incremental tree reproduces the new source exactly.
        assert_eq!(SyntaxNode::new_root(result.green_tree).text(), new_src);

        let info = inc.last_reuse().expect("last_reuse set after update");
        assert_eq!(info.total_bytes, new_src.len());
        assert!(info.reparsed_bytes <= info.total_bytes);
        assert!((0.0..=1.0).contains(&info.reuse_ratio()));
        assert!(
            info.is_hit(),
            "append at end should be a splice hit, got {:?}",
            info.kind
        );
        assert!(
            info.reused_bytes() > 0,
            "expected some bytes reused, got {info:?}"
        );
    }

    #[test]
    fn incremental_lex_reuses_prefix_suffix() {
        let grammar = test_grammar();
        let lexer = RuntimeLexer::new(&grammar);

        let old_source = "fn foo() { return 1; }";
        let old_tokens = lexer.tokenize(old_source);

        let inc = IncrementalParser {
            parser: RuntimeParser::new(grammar.clone()),
            lexer: RuntimeLexer::new(&grammar),
            prev_tree: None,
            prev_source: old_source.to_string(),
            prev_tokens: old_tokens.clone(),
            last_reuse: None,
        };

        // Insert a space at position 18 (between "1" and ";")
        let new_source = "fn foo() { return 1 ; }";
        let new_tokens = inc.incremental_lex(new_source, 18, 18, 1);

        // Full lex for comparison.
        let full_tokens = lexer.tokenize(new_source);

        // The incremental result should produce the same token kinds.
        assert_eq!(
            new_tokens.len(),
            full_tokens.len(),
            "incremental lex should produce same token count as full lex"
        );
        for (i, (inc_tok, full_tok)) in new_tokens.iter().zip(full_tokens.iter()).enumerate() {
            assert_eq!(
                inc_tok.kind, full_tok.kind,
                "token {i} kind mismatch: {:?} vs {:?}",
                inc_tok, full_tok
            );
        }
    }

    #[test]
    fn incremental_lex_delete() {
        let grammar = test_grammar();
        let lexer = RuntimeLexer::new(&grammar);

        let old_source = "fn foo() { return 42; }";
        let old_tokens = lexer.tokenize(old_source);

        let inc = IncrementalParser {
            parser: RuntimeParser::new(grammar.clone()),
            lexer: RuntimeLexer::new(&grammar),
            prev_tree: None,
            prev_source: old_source.to_string(),
            prev_tokens: old_tokens,
            last_reuse: None,
        };

        // Delete "42" -> ""
        let new_source = "fn foo() { return ; }";
        let new_tokens = inc.incremental_lex(new_source, 18, 20, -2);
        let full_tokens = lexer.tokenize(new_source);

        assert_eq!(new_tokens.len(), full_tokens.len());
        for (i, (a, b)) in new_tokens.iter().zip(full_tokens.iter()).enumerate() {
            assert_eq!(a.kind, b.kind, "token {i} mismatch");
        }
    }

    #[test]
    fn incremental_lex_replace() {
        let grammar = test_grammar();
        let lexer = RuntimeLexer::new(&grammar);

        let old_source = "fn foo() { return 42; }";
        let old_tokens = lexer.tokenize(old_source);

        let inc = IncrementalParser {
            parser: RuntimeParser::new(grammar.clone()),
            lexer: RuntimeLexer::new(&grammar),
            prev_tree: None,
            prev_source: old_source.to_string(),
            prev_tokens: old_tokens,
            last_reuse: None,
        };

        // Replace "42" with "999"
        let new_source = "fn foo() { return 999; }";
        let delta = 3i64 - 2; // "999".len() - "42".len()
        let new_tokens = inc.incremental_lex(new_source, 18, 20, delta);
        let full_tokens = lexer.tokenize(new_source);

        assert_eq!(new_tokens.len(), full_tokens.len());
        for (i, (a, b)) in new_tokens.iter().zip(full_tokens.iter()).enumerate() {
            assert_eq!(a.kind, b.kind, "token {i} mismatch");
        }
    }

    #[test]
    fn incremental_lex_at_start() {
        let grammar = test_grammar();
        let lexer = RuntimeLexer::new(&grammar);

        let old_source = "fn foo() {}";
        let old_tokens = lexer.tokenize(old_source);

        let inc = IncrementalParser {
            parser: RuntimeParser::new(grammar.clone()),
            lexer: RuntimeLexer::new(&grammar),
            prev_tree: None,
            prev_source: old_source.to_string(),
            prev_tokens: old_tokens,
            last_reuse: None,
        };

        let new_source = "let fn foo() {}";
        let new_tokens = inc.incremental_lex(new_source, 0, 0, 4);
        let full_tokens = lexer.tokenize(new_source);

        assert_eq!(new_tokens.len(), full_tokens.len());
    }

    #[test]
    fn incremental_lex_at_end() {
        let grammar = test_grammar();
        let lexer = RuntimeLexer::new(&grammar);

        let old_source = "fn foo() {}";
        let old_tokens = lexer.tokenize(old_source);

        let inc = IncrementalParser {
            parser: RuntimeParser::new(grammar.clone()),
            lexer: RuntimeLexer::new(&grammar),
            prev_tree: None,
            prev_source: old_source.to_string(),
            prev_tokens: old_tokens,
            last_reuse: None,
        };

        let new_source = "fn foo() {} // end";
        let delta = new_source.len() as i64 - old_source.len() as i64;
        let new_tokens = inc.incremental_lex(
            new_source,
            old_source.len() as u32,
            old_source.len() as u32,
            delta,
        );
        let full_tokens = lexer.tokenize(new_source);

        assert_eq!(new_tokens.len(), full_tokens.len());
    }
}
