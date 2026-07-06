use std::path::PathBuf;

use semtree_format::Formatter;

pub fn format(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
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
