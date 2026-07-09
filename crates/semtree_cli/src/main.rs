use std::path::PathBuf;

use clap::{Parser as ClapParser, Subcommand};

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
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Parse a source file and print the syntax tree
    Parse {
        file: PathBuf,
        #[arg(short, long, default_value = "tree")]
        format: String,
    },

    /// Parse a source file using a grammar definition (grammar-driven)
    Run {
        #[arg(short, long)]
        grammar: Option<PathBuf>,
        file: PathBuf,
        #[arg(short, long, default_value = "tree")]
        format: String,
        #[arg(long, default_value = "auto")]
        backend: String,
        #[arg(long)]
        incremental: bool,
        #[arg(long)]
        edit: Option<String>,
    },

    Check {
        file: PathBuf,
    },

    Format {
        file: PathBuf,
    },

    Query {
        file: PathBuf,
        pattern: String,
    },

    Lint {
        file: PathBuf,
    },

    Symbols {
        file: PathBuf,
    },

    Benchmark {
        file: PathBuf,
        #[arg(short, long, default_value = "100")]
        iterations: u32,
    },

    /// Compare full parse vs incremental reparse performance
    Parity {
        #[arg(short, long)]
        grammar: Option<PathBuf>,
        file: Option<PathBuf>,
        #[arg(long, default_value = "10000")]
        lines: u32,
        #[arg(short, long, default_value = "50")]
        iterations: u32,
    },

    /// Debug grammar parsing: token stream, errors, tree summary
    Debug {
        #[arg(short, long)]
        grammar: Option<PathBuf>,
        file: PathBuf,
    },

    /// Start LSP server (stdio) with incremental parsing
    Lsp,

    Import {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    Doctor,

    Generate {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    Test {
        dir: PathBuf,
    },

    Migrate {
        file: PathBuf,
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
            incremental,
            edit,
        } => commands::run(
            grammar,
            file,
            format,
            &exe_dir(),
            &backend,
            incremental,
            edit.as_deref(),
        ),
        Commands::Check { file } => commands::check(file),
        Commands::Format { file } => commands::format(file),
        Commands::Query { file, pattern } => commands::query(file, pattern),
        Commands::Lint { file } => commands::lint(file),
        Commands::Symbols { file } => commands::symbols(file),
        Commands::Benchmark { file, iterations } => commands::benchmark(file, iterations),
        Commands::Parity {
            grammar,
            file,
            lines,
            iterations,
        } => commands::parity(grammar, file, lines, iterations, &exe_dir()),
        Commands::Debug { grammar, file } => commands::debug(grammar, file, &exe_dir()),
        Commands::Lsp => commands::lsp(exe_dir()),
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
