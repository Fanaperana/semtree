//! Load shipped grammars from `grammars/*.semtree` for benchmarks and tests.

use semtree_grammar::{Grammar, parse_semtree_dsl};
use std::path::{Path, PathBuf};

/// Resolve the repo `grammars/` directory relative to this crate.
pub fn grammars_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../grammars")
}

/// Load `grammars/{name}.semtree` (e.g. name = "python", "json").
pub fn load_shipped_grammar(name: &str) -> Grammar {
    let path = grammars_dir().join(format!("{name}.semtree"));
    load_grammar_file(&path)
}

pub fn load_grammar_file(path: &Path) -> Grammar {
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read grammar {}: {e}", path.display()));
    parse_semtree_dsl(&src)
        .unwrap_or_else(|e| panic!("failed to parse grammar {}: {e}", path.display()))
}
