use std::path::PathBuf;

pub fn format(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let result = semtree_parser::Parser::parse(&source);

    if !result.errors.is_empty() {
        eprintln!(
            "warning: {} parse error(s), formatting may be incomplete",
            result.errors.len()
        );
    }

    // Phase 1: pass-through formatter (echoes parsed tree text)
    let root = result.syntax();
    print!("{}", root.text());

    Ok(())
}
