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

    /// Parse a source file using a grammar definition (grammar-driven)
    Run {
        /// Grammar file (.semtree or .json). If omitted, auto-detects from file extension.
        #[arg(short, long)]
        grammar: Option<PathBuf>,

        /// Source file to parse
        file: PathBuf,

        /// Output format: tree, json, or sexp
        #[arg(short, long, default_value = "tree")]
        format: String,

        /// Parser backend: rd (recursive descent) or glr
        #[arg(long, default_value = "rd")]
        backend: String,
    },

    /// Check a grammar definition for errors
    Check {
        /// Grammar file (.semtree)
        file: PathBuf,
    },

    /// Format a source file
    Format {
        /// Source file to format
        file: PathBuf,
    },

    /// Query a syntax tree using S-expression patterns
    Query {
        /// Source file to query
        file: PathBuf,

        /// Query pattern (S-expression or kind name)
        pattern: String,
    },

    /// Lint a source file for common issues
    Lint {
        /// Source file to lint
        file: PathBuf,
    },

    /// Show symbols (functions, variables, types) in a source file
    Symbols {
        /// Source file to analyze
        file: PathBuf,
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

    /// Generate typed AST code from a grammar file
    Generate {
        /// Grammar file (.semtree)
        file: PathBuf,

        /// Output file for generated code
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run grammar test suites (parse all test files and verify lossless roundtrip)
    Test {
        /// Directory containing test source files
        dir: PathBuf,
    },

    /// Migrate a Tree-sitter grammar (import + validate)
    Migrate {
        /// Path to grammar.json (tree-sitter compiled grammar)
        file: PathBuf,

        /// Output path for SemTree grammar
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { name, output } => commands::init(name, output),
        Commands::Parse { file, format } => commands::parse(file, format),
        Commands::Run {
            grammar,
            file,
            format,
            backend,
        } => commands::run(grammar, file, format, &exe_dir(), &backend),
        Commands::Check { file } => commands::check(file),
        Commands::Format { file } => commands::format(file),
        Commands::Query { file, pattern } => commands::query(file, pattern),
        Commands::Lint { file } => commands::lint(file),
        Commands::Symbols { file } => commands::symbols(file),
        Commands::Benchmark { file, iterations } => commands::benchmark(file, iterations),
        Commands::Import { file, output } => commands::import(file, output),
        Commands::Doctor => commands::doctor(),
        Commands::Generate { file, output } => commands::generate(file, output),
        Commands::Test { dir } => commands::test(dir),
        Commands::Migrate { file, output } => commands::migrate(file, output),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}
