use semtree_grammar::Grammar;
use semtree_red::SyntaxNode;
use text_size::{TextRange, TextSize};

use crate::glr::{GlrParseResult, GlrParser};
use crate::incremental::{EditRegion, IncrementalParser, apply_edits};
use crate::runtime_parser::RuntimeParseResult;

/// Parser backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserBackend {
    RecursiveDescent,
    Glr,
    Auto,
}

impl ParserBackend {
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "rd" | "recursive" | "recursive_descent" => Some(Self::RecursiveDescent),
            "glr" => Some(Self::Glr),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Choose RD when the grammar has no GLR conflicts, GLR otherwise.
pub fn select_backend(grammar: &Grammar) -> ParserBackend {
    let glr = GlrParser::new(grammar.clone());
    if glr.has_conflicts() {
        ParserBackend::Glr
    } else {
        ParserBackend::RecursiveDescent
    }
}

/// Unified parse result regardless of backend.
#[derive(Clone)]
pub struct UnifiedParseResult {
    pub green_tree: semtree_green::GreenNode,
    pub syntax: SyntaxNode,
    pub errors: Vec<String>,
    pub kind_names: rustc_hash::FxHashMap<semtree_core::SyntaxKind, smol_str::SmolStr>,
    pub backend_used: ParserBackend,
}

impl UnifiedParseResult {
    fn from_rd(result: RuntimeParseResult, backend: ParserBackend) -> Self {
        let syntax = result.syntax();
        Self {
            green_tree: result.green_tree,
            syntax,
            errors: result.errors.into_iter().map(|e| e.to_string()).collect(),
            kind_names: result.kind_names,
            backend_used: backend,
        }
    }

    fn from_glr(result: GlrParseResult, backend: ParserBackend) -> Self {
        let syntax = result.syntax();
        Self {
            green_tree: result.green_tree,
            syntax,
            errors: result.errors.into_iter().map(|e| e.to_string()).collect(),
            kind_names: result.kind_names,
            backend_used: backend,
        }
    }
}

/// Stateful parse session supporting incremental updates.
pub struct ParseSession {
    grammar: Grammar,
    backend: ParserBackend,
    incremental: IncrementalParser,
    glr: Option<GlrParser>,
    source: String,
    last_result: Option<UnifiedParseResult>,
}

impl ParseSession {
    pub fn new(grammar: Grammar, backend: ParserBackend) -> Self {
        let resolved = match backend {
            ParserBackend::Auto => select_backend(&grammar),
            other => other,
        };
        Self {
            grammar: grammar.clone(),
            backend: resolved,
            incremental: IncrementalParser::new(grammar.clone()),
            glr: if resolved == ParserBackend::Glr {
                Some(GlrParser::new(grammar))
            } else {
                None
            },
            source: String::new(),
            last_result: None,
        }
    }

    pub fn syntax(&self) -> Option<&SyntaxNode> {
        self.last_result.as_ref().map(|r| &r.syntax)
    }

    fn store_result(&mut self, result: UnifiedParseResult) -> UnifiedParseResult {
        let out = result.clone();
        self.last_result = Some(result);
        out
    }

    pub fn backend(&self) -> ParserBackend {
        self.backend
    }

    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Full parse from scratch, resetting incremental state.
    pub fn parse(&mut self, source: &str) -> UnifiedParseResult {
        self.source = source.to_string();
        let result = match self.backend {
            ParserBackend::Glr => {
                let parser = self
                    .glr
                    .get_or_insert_with(|| GlrParser::new(self.grammar.clone()));
                let result = parser.parse(source);
                UnifiedParseResult::from_glr(result, ParserBackend::Glr)
            }
            ParserBackend::RecursiveDescent | ParserBackend::Auto => {
                let result = self.incremental.parse(source);
                UnifiedParseResult::from_rd(result, ParserBackend::RecursiveDescent)
            }
        };
        self.store_result(result)
    }

    /// Apply a single edit and incrementally reparse (RD backend only).
    pub fn edit(&mut self, start: u32, old_end: u32, new_text: &str) -> UnifiedParseResult {
        let edits = vec![EditRegion::new(start, old_end, new_text.to_string())];
        self.apply_edits(&edits)
    }

    /// Apply multiple edits and incrementally reparse.
    pub fn apply_edits(&mut self, edits: &[EditRegion]) -> UnifiedParseResult {
        let new_source = apply_edits(&self.source, edits);
        self.source = new_source.clone();

        let result = match self.backend {
            ParserBackend::Glr => {
                let parser = self
                    .glr
                    .get_or_insert_with(|| GlrParser::new(self.grammar.clone()));
                let result = parser.parse(&new_source);
                UnifiedParseResult::from_glr(result, ParserBackend::Glr)
            }
            ParserBackend::RecursiveDescent | ParserBackend::Auto => {
                let result = self.incremental.update(&new_source, edits);
                UnifiedParseResult::from_rd(result, ParserBackend::RecursiveDescent)
            }
        };
        self.store_result(result)
    }

    /// Replace entire source via a single edit (for full-buffer sync).
    pub fn replace_all(&mut self, new_source: &str) -> UnifiedParseResult {
        if self.source.is_empty() {
            return self.parse(new_source);
        }
        let old_len = self.source.len() as u32;
        self.edit(0, old_len, new_source)
    }
}

/// Compute byte-range edits between old and new source (prefix/suffix diff).
pub fn diff_to_edits(old: &str, new: &str) -> Vec<EditRegion> {
    if old == new {
        return Vec::new();
    }

    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    let mut prefix = 0usize;
    let min_len = old.len().min(new.len());

    while prefix < min_len && old_bytes[prefix] == new_bytes[prefix] {
        prefix += 1;
    }

    let mut old_suffix = old.len();
    let mut new_suffix = new.len();
    while old_suffix > prefix
        && new_suffix > prefix
        && old_bytes[old_suffix - 1] == new_bytes[new_suffix - 1]
    {
        old_suffix -= 1;
        new_suffix -= 1;
    }

    vec![EditRegion {
        old_range: TextRange::new(
            TextSize::new(prefix as u32),
            TextSize::new(old_suffix as u32),
        ),
        new_text: new[prefix..new_suffix].to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use semtree_grammar::parse_semtree_dsl;

    fn mini_grammar() -> Grammar {
        parse_semtree_dsl(
            r#"
language test
keyword fn
Function := "fn" Identifier "(" ")" "{" "}"
"#,
        )
        .unwrap()
    }

    #[test]
    fn session_incremental_edit() {
        let mut session = ParseSession::new(mini_grammar(), ParserBackend::RecursiveDescent);
        let _ = session.parse("fn foo() {}");
        let result = session.edit(6, 6, "x");
        assert!(result.syntax.text().contains("foox"), "got: {}", result.syntax.text());
    }

    #[test]
    fn diff_to_edits_insert() {
        let edits = diff_to_edits("hello", "hello!");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "!");
    }

    #[test]
    fn auto_backend_selects_rd_for_simple_grammar() {
        let g = mini_grammar();
        assert_eq!(select_backend(&g), ParserBackend::RecursiveDescent);
    }
}
