use std::path::PathBuf;

use semtree_semantic::SemanticModel;

pub fn symbols(file: PathBuf) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let parse_result = semtree_parser::Parser::parse(&source);
    let root = parse_result.syntax();
    let model = SemanticModel::analyze(&root);

    if model.symbols.is_empty() {
        println!("No symbols found.");
        return Ok(());
    }

    println!("Symbols ({} total):\n", model.symbols.len());
    for sym in model.symbols.all() {
        let start = u32::from(sym.range.start());
        let end = u32::from(sym.range.end());
        let vis = if sym.is_public { "pub " } else { "" };
        let mutability = if sym.is_mutable { "mut " } else { "" };
        println!(
            "  {vis}{mutability}{kind} {name} ({start}..{end})",
            kind = sym.kind,
            name = sym.name,
        );
    }

    println!("\nScopes: {}", model.scopes.len());
    println!("References: {}", model.references.len());

    Ok(())
}
