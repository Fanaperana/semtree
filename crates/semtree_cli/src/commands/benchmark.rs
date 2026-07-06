use std::path::PathBuf;
use std::time::Instant;

use semtree_parser::Parser;

pub fn benchmark(file: PathBuf, iterations: u32) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let bytes = source.len();
    let lines = source.lines().count();

    println!("Benchmarking: {}", file.display());
    println!("  Size: {bytes} bytes ({lines} lines)");
    println!("  Iterations: {iterations}\n");

    // Cold parse
    let cold_start = Instant::now();
    let parse_result = Parser::parse(&source);
    let cold_elapsed = cold_start.elapsed();
    println!("  Cold parse:    {:>10?}", cold_elapsed);
    println!(
        "  Parse errors:  {}",
        parse_result.errors.len()
    );

    // Warm parse (average over iterations)
    let warm_start = Instant::now();
    for _ in 0..iterations {
        let _ = Parser::parse(&source);
    }
    let warm_total = warm_start.elapsed();
    let warm_avg = warm_total / iterations;

    println!("  Warm parse:    {:>10?} (avg)", warm_avg);
    println!(
        "  Total time:    {:>10?} ({iterations} iterations)",
        warm_total
    );

    let throughput_mb = (bytes as f64 / 1_000_000.0) / warm_avg.as_secs_f64();
    println!("  Throughput:    {throughput_mb:.2} MB/s");

    // Tree stats
    let root = parse_result.syntax();
    let node_count = count_nodes(&root);
    let tree_depth = max_depth(&root);
    println!("\n  Tree stats:");
    println!("    Nodes:       {node_count}");
    println!("    Max depth:   {tree_depth}");

    // Simulated incremental parse (edit middle of file)
    if bytes > 10 {
        let mid = bytes / 2;
        let mut modified = source.clone();
        modified.insert(mid, ' ');

        let inc_start = Instant::now();
        for _ in 0..iterations {
            let _ = Parser::parse(&modified);
        }
        let inc_total = inc_start.elapsed();
        let inc_avg = inc_total / iterations;
        println!("\n  Reparse (full, simulated edit):");
        println!("    Average:     {:>10?}", inc_avg);
    }

    Ok(())
}

fn count_nodes(node: &semtree_red::SyntaxNode) -> usize {
    1 + node.children().iter().map(count_nodes).sum::<usize>()
}

fn max_depth(node: &semtree_red::SyntaxNode) -> usize {
    let child_max = node
        .children()
        .iter()
        .map(max_depth)
        .max()
        .unwrap_or(0);
    1 + child_max
}
