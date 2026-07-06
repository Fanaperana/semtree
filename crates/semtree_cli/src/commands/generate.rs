use std::path::PathBuf;

use semtree_ast::generate_ast;
use semtree_grammar::parse_semtree_dsl;

pub fn generate(file: PathBuf, output: Option<PathBuf>) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let grammar = parse_semtree_dsl(&source)
        .map_err(|e| format!("grammar parse error: {e}"))?;

    let code = generate_ast(&grammar);

    match output {
        Some(path) => {
            std::fs::write(&path, &code)?;
            println!("Generated AST code written to: {}", path.display());
        }
        None => {
            print!("{code}");
        }
    }

    Ok(())
}
