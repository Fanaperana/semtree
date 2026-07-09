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

    // Search relative to CWD
    for search_dir in GRAMMAR_SEARCH_DIRS {
        let candidate = PathBuf::from(search_dir).join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Search relative to executable
    let exe_grammar = exe_dir.join("grammars").join(&filename);
    if exe_grammar.exists() {
        return Some(exe_grammar);
    }

    // Search relative to the file being parsed
    if let Some(file_dir) = file.parent() {
        for search_dir in GRAMMAR_SEARCH_DIRS {
            let candidate = file_dir.join(search_dir).join(&filename);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // Search in user data directory (~/.local/share/semtree/grammars/)
    if let Some(data_dir) = dirs_data_dir() {
        let candidate = data_dir.join("semtree").join("grammars").join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Platform data directory (XDG_DATA_HOME or platform equivalent)
fn dirs_data_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            #[cfg(target_os = "macos")]
            {
                dirs_home().map(|h| h.join("Library").join("Application Support"))
            }
            #[cfg(not(target_os = "macos"))]
            {
                dirs_home().map(|h| h.join(".local").join("share"))
            }
        })
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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
