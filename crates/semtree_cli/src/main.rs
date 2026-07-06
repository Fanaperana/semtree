use clap::{Parser as ClapParser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(ClapParser)]
#[command(
    name = "semtree",
    version,
    about = "SemTree — Universal Incremental Language Infrastructure"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new SemTree project
    Init {
        /// Language name
        #[arg(short, long)]
        name: Option<String>,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Parse a source file and print the syntax tree
    Parse {
        /// Source file to parse
        file: PathBuf,

        /// Output format: tree, json, or sexp
        #[arg(short, long, default_value = "tree")]
        format: String,
    },

    /// Check a grammar definition for errors
    Check {
        /// Grammar file (.semtree)
        file: PathBuf,
    },

    /// Format a source file (placeholder)
    Format {
        /// Source file to format
        file: PathBuf,
    },

    /// Query a syntax tree
    Query {
        /// Source file to query
        file: PathBuf,

        /// Query pattern
        pattern: String,
    },

    /// Run benchmarks on parsing
    Benchmark {
        /// Source file to benchmark
        file: PathBuf,

        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: u32,
    },

    /// Import a Tree-sitter grammar
    Import {
        /// Path to grammar.json (tree-sitter compiled grammar)
        file: PathBuf,

        /// Output path for SemTree grammar
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run diagnostics on the SemTree installation
    Doctor,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { name, output } => commands::init(name, output),
        Commands::Parse { file, format } => commands::parse(file, format),
        Commands::Check { file } => commands::check(file),
        Commands::Format { file } => commands::format(file),
        Commands::Query { file, pattern } => commands::query(file, pattern),
        Commands::Benchmark { file, iterations } => commands::benchmark(file, iterations),
        Commands::Import { file, output } => commands::import(file, output),
        Commands::Doctor => commands::doctor(),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
