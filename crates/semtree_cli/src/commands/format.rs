use std::path::PathBuf;

use semtree_format::Formatter;
use semtree_grammar::format_semtree_dsl;

pub fn format(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;

    // .semtree grammar files get the dedicated DSL formatter.
    // Using the Rust source formatter on them mangles the grammar.
    if file.extension().is_some_and(|ext| ext == "semtree") {
        let formatted = format_semtree_dsl(&source);
        print!("{formatted}");
        return Ok(());
    }

    let result = semtree_parser::Parser::parse(&source);

    if !result.errors.is_empty() {
        eprintln!(
            "warning: {} parse error(s), formatting may be incomplete",
            result.errors.len()
        );
    }

    let root = result.syntax();
    let formatted = Formatter::with_defaults().format(&root);
    print!("{formatted}");

    Ok(())
}
