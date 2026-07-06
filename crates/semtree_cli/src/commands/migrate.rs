use std::path::PathBuf;

use semtree_grammar::validate_grammar;
use semtree_ts_import::import_tree_sitter_grammar;

pub fn migrate(file: PathBuf, output: Option<PathBuf>) -> super::Result {
    let json_str = std::fs::read_to_string(&file)?;

    let grammar = import_tree_sitter_grammar(&json_str)
        .map_err(|e| format!("import error: {e}"))?;

    println!("Imported: {}", grammar.name);
    println!("  Rules: {}", grammar.rules.len());

    let errors = validate_grammar(&grammar);
    if errors.is_empty() {
        println!("  Validation: OK");
    } else {
        println!("  Validation warnings:");
        for e in &errors {
            println!("    - {e}");
        }
    }

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
