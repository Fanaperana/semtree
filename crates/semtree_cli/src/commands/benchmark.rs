use std::path::PathBuf;
use std::time::Instant;

use semtree_parser::Parser;

pub fn benchmark(file: PathBuf, iterations: u32) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let bytes = source.len();

    println!("Benchmarking: {}", file.display());
    println!("  Size: {} bytes ({} lines)", bytes, source.lines().count());
    println!("  Iterations: {iterations}\n");

    // Cold parse
    let cold_start = Instant::now();
    let _ = Parser::parse(&source);
    let cold_elapsed = cold_start.elapsed();
    println!("  Cold parse: {:?}", cold_elapsed);

    // Warm parse (average over iterations)
    let warm_start = Instant::now();
    for _ in 0..iterations {
        let _ = Parser::parse(&source);
    }
    let warm_total = warm_start.elapsed();
    let warm_avg = warm_total / iterations;

    println!("  Warm parse (avg): {:?}", warm_avg);
    println!("  Total time ({iterations} iterations): {:?}", warm_total);

    let throughput_mb = (bytes as f64 / 1_000_000.0) / warm_avg.as_secs_f64();
    println!("  Throughput: {throughput_mb:.2} MB/s");

    Ok(())
}
