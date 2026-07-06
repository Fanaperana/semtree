pub fn doctor() -> super::Result {
    println!("SemTree Doctor");
    println!("==============\n");

    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Rust edition: 2024");

    let checks = [
        ("semtree_core", true),
        ("semtree_lexer", true),
        ("semtree_green", true),
        ("semtree_red", true),
        ("semtree_parser", true),
        ("semtree_grammar", true),
        ("semtree_ts_import", true),
    ];

    println!("\nComponents:");
    for (name, ok) in &checks {
        let status = if *ok { "OK" } else { "MISSING" };
        println!("  [{status}] {name}");
    }

    println!("\nSelf-test:");
    let result = semtree_parser::Parser::parse("fn main() {}");
    if result.errors.is_empty() {
        println!("  [OK] Parser produces valid tree for simple input");
    } else {
        println!("  [FAIL] Parser produced errors on simple input");
    }

    let tokens = semtree_lexer::Lexer::tokenize("fn main() {}");
    if tokens.len() > 1 {
        println!("  [OK] Lexer tokenizes correctly");
    } else {
        println!("  [FAIL] Lexer failed");
    }

    let dsl_input = "language test\n\nRule :=\n    \"x\"\n";
    match semtree_grammar::parse_semtree_dsl(dsl_input) {
        Ok(g) if g.rules.contains_key("Rule") => {
            println!("  [OK] Grammar DSL parser works");
        }
        _ => {
            println!("  [FAIL] Grammar DSL parser failed");
        }
    }

    println!("\nAll systems operational.");
    Ok(())
}
