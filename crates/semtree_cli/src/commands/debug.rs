//! Grammar debugger: token stream, parse errors, and tree summary.

use std::path::{Path, PathBuf};

use semtree_runtime::{ParserBackend, RuntimeLexer, RuntimeParser, select_backend};

use super::grammar_util::resolve_grammar;

pub fn debug(grammar_path: Option<PathBuf>, file: PathBuf, exe_dir: &Path) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let (grammar_path, grammar) = resolve_grammar(grammar_path, &file, exe_dir)?;

    eprintln!("Grammar: {}", grammar_path.display());
    let backend = select_backend(&grammar);
    eprintln!(
        "Backend: {} (auto-selected)",
        match backend {
            ParserBackend::Glr => "glr",
            ParserBackend::RecursiveDescent => "rd",
            ParserBackend::Auto => "auto",
        }
    );

    let lexer = RuntimeLexer::new(&grammar);
    let tokens = lexer.tokenize(&source);

    println!("=== Token stream ({} tokens) ===", tokens.len());
    for (i, tok) in tokens.iter().enumerate().take(200) {
        println!(
            "  [{i:4}] {:?} {:?} [{}..{}]",
            tok.kind,
            tok.text(&source),
            u32::from(tok.range.start()),
            u32::from(tok.range.end())
        );
    }
    if tokens.len() > 200 {
        println!("  ... ({} more tokens)", tokens.len() - 200);
    }

    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(&source);
    let root = result.syntax();

    println!("\n=== Parse result ===");
    println!("  Root kind: {:?}", root.kind());
    println!("  Tree text len: {}", root.text().len());
    println!("  Source len:    {}", source.len());
    println!(
        "  Lossless:      {}",
        if root.text() == source { "yes" } else { "no" }
    );
    println!("  Top-level children: {}", root.children().len());
    println!("  Errors: {}", result.errors.len());

    if !result.errors.is_empty() {
        println!("\n=== Errors ===");
        for (i, err) in result.errors.iter().enumerate().take(20) {
            println!("  [{i}] {err}");
        }
    }

    println!("\n=== Top-level structure ===");
    for (i, child) in root.children().iter().enumerate().take(30) {
        let r = child.text_range();
        println!(
            "  [{i}] {:?} [{}..{}] {:?}",
            child.kind(),
            u32::from(r.start()),
            u32::from(r.end()),
            child.text().chars().take(40).collect::<String>()
        );
    }

    Ok(())
}
