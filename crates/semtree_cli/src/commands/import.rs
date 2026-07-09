use std::path::PathBuf;

use semtree_ts_import::import_tree_sitter_grammar;

pub fn import(file: PathBuf, output: Option<PathBuf>) -> super::Result {
    let json_str = std::fs::read_to_string(&file)?;

    let grammar =
        import_tree_sitter_grammar(&json_str).map_err(|e| format!("import error: {e}"))?;

    println!("Imported Tree-sitter grammar: {}", grammar.name);
    println!("  Rules: {}", grammar.rules.len());
    println!("  Keywords: {}", grammar.keywords.len());

    let semtree_json = serde_json::to_string_pretty(&grammar)?;

    match output {
        Some(path) => {
            std::fs::write(&path, &semtree_json)?;
            println!("  Written to: {}", path.display());
        }
        None => {
            println!("\n{semtree_json}");
        }
    }

    Ok(())
}
