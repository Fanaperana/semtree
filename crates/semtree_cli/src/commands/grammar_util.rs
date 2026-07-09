use std::path::{Path, PathBuf};

use semtree_grammar::{Grammar, parse_semtree_dsl};
use semtree_ts_import::import_tree_sitter_grammar;

pub const GRAMMAR_SEARCH_DIRS: &[&str] = &["grammars", "../grammars", "../../grammars"];

pub fn detect_grammar_path(file: &Path, exe_dir: &Path) -> Option<PathBuf> {
    let ext = file.extension()?.to_str()?;
    let grammar_name = match ext {
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => "javascript",
        "py" | "pyw" => "python",
        "rs" => "rust",
        "css" | "scss" | "less" => "css",
        "json" => "json",
        "toml" => "toml",
        _ => return None,
    };
    let filename = format!("{grammar_name}.semtree");

    for search_dir in GRAMMAR_SEARCH_DIRS {
        let candidate = PathBuf::from(search_dir).join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let exe_grammar = exe_dir.join("grammars").join(&filename);
    if exe_grammar.exists() {
        return Some(exe_grammar);
    }

    None
}

pub fn load_grammar(path: &Path) -> Result<Grammar, String> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    if path.extension().is_some_and(|ext| ext == "json") {
        import_tree_sitter_grammar(&src).map_err(|e| format!("failed to import grammar: {e}"))
    } else {
        parse_semtree_dsl(&src).map_err(|e| format!("failed to parse grammar: {e}"))
    }
}

pub fn resolve_grammar(
    grammar_path: Option<PathBuf>,
    file: &Path,
    exe_dir: &Path,
) -> Result<(PathBuf, Grammar), String> {
    let path = match grammar_path {
        Some(p) => p,
        None => detect_grammar_path(file, exe_dir).ok_or_else(|| {
            let ext = file
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            format!(
                "no grammar found for .{ext} files. Use -g to specify a grammar.\n\
                 Supported: .js, .py, .rs, .css, .json, .toml"
            )
        })?,
    };
    let grammar = load_grammar(&path)?;
    Ok((path, grammar))
}

#[allow(dead_code)]
pub fn grammars_dir(exe_dir: &Path) -> PathBuf {
    for search_dir in GRAMMAR_SEARCH_DIRS {
        let candidate = PathBuf::from(search_dir);
        if candidate.exists() {
            return candidate;
        }
    }
    exe_dir.join("grammars")
}
