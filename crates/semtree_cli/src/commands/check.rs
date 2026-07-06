use std::path::PathBuf;

use semtree_grammar::{parse_semtree_dsl, validate::validate_grammar};

pub fn check(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let grammar = parse_semtree_dsl(&source)
        .map_err(|e| format!("grammar parse error: {e}"))?;

    println!("Grammar: {}", grammar.name);
    println!("  Rules: {}", grammar.rules.len());
    println!("  Keywords: {}", grammar.keywords.len());
    println!("  Format hints: {}", grammar.format_hints.len());

    let errors = validate_grammar(&grammar);
    if errors.is_empty() {
        println!("  Status: OK");
    } else {
        println!("  Errors:");
        for e in &errors {
            println!("    - {e}");
        }
    }

    Ok(())
}
