//! Parity benchmark: full parse vs incremental reparse.

use std::path::{Path, PathBuf};
use std::time::Instant;

use semtree_grammar::parse_semtree_dsl;
use semtree_runtime::{EditRegion, ParseSession, ParserBackend};

use super::grammar_util::{load_grammar, resolve_grammar};

pub fn parity(
    grammar_path: Option<PathBuf>,
    file: Option<PathBuf>,
    lines: u32,
    iterations: u32,
    exe_dir: &Path,
) -> super::Result {
    let (source, label) = if let Some(ref f) = file {
        let s = std::fs::read_to_string(f)?;
        (s, f.display().to_string())
    } else {
        let s = generate_source(lines as usize);
        (s, format!("synthetic ({lines} lines)"))
    };

    let grammar = if let Some(path) = grammar_path {
        load_grammar(&path)?
    } else if let Some(ref f) = file {
        resolve_grammar(None, f, exe_dir)?.1
    } else {
        parse_semtree_dsl(
            r#"
language bench
keyword fn
Function := "fn" Identifier "(" ")" "{" Statement* "}"
Statement := "let" Identifier "=" Integer ";"
"#,
        )
        .map_err(|e| e.to_string())?
    };

    let bytes = source.len();
    let line_count = source.lines().count();

    println!("SemTree parity benchmark");
    println!("  Source: {label}");
    println!("  Size:   {bytes} bytes ({line_count} lines)");
    println!("  Iterations: {iterations}\n");

    // Cold full parse
    let cold_start = Instant::now();
    let mut session = ParseSession::new(grammar.clone(), ParserBackend::Auto);
    let _ = session.parse(&source);
    let cold = cold_start.elapsed();

    // Warm full parse
    let warm_start = Instant::now();
    for _ in 0..iterations {
        let mut s = ParseSession::new(grammar.clone(), ParserBackend::Auto);
        let _ = s.parse(&source);
    }
    let warm_avg = warm_start.elapsed() / iterations;

    // Incremental: single-char insert at middle
    let insert_pos = source.len() / 2;
    let mut new_source = source.clone();
    new_source.insert(insert_pos, ' ');
    let edits = vec![EditRegion::new(insert_pos as u32, insert_pos as u32, " ")];

    let mut inc_session = ParseSession::new(grammar, ParserBackend::RecursiveDescent);
    let _ = inc_session.parse(&source);

    let inc_start = Instant::now();
    for _ in 0..iterations {
        let _ = inc_session.apply_edits(&edits);
        let reverse = EditRegion::new(insert_pos as u32, (insert_pos + 1) as u32, "");
        let _ = inc_session.apply_edits(&[reverse]);
    }
    let inc_avg = inc_start.elapsed() / iterations;

    let speedup = warm_avg.as_secs_f64() / inc_avg.as_secs_f64();

    println!("  Cold parse:        {cold:?}");
    println!("  Full parse (avg):  {warm_avg:?}");
    println!("  Incremental (avg): {inc_avg:?}");
    println!("  Speedup:           {speedup:.1}x");

    let inc_ms = inc_avg.as_secs_f64() * 1000.0;
    if line_count >= 10_000 {
        if inc_ms <= 1.0 {
            println!("  Target (<1ms @ 10K lines): PASS ({inc_ms:.3}ms)");
        } else {
            println!("  Target (<1ms @ 10K lines): FAIL ({inc_ms:.3}ms)");
        }
    }

    println!("\n--- Summary ---");
    println!("Incremental reparse is {speedup:.1}x faster than full parse on this input.");

    Ok(())
}

fn generate_source(line_count: usize) -> String {
    let mut out = String::new();
    for i in 0..line_count {
        out.push_str(&format!("fn func_{i}() {{ let x = {i}; return x; }}\n"));
    }
    out
}
