use std::path::{Path, PathBuf};

use semtree_grammar::parse_semtree_dsl;
use semtree_red::SyntaxNode;
use semtree_runtime::RuntimeParser;
use semtree_ts_import::import_tree_sitter_grammar;

const GRAMMAR_SEARCH_DIRS: &[&str] = &["grammars", "../grammars", "../../grammars"];

fn detect_grammar_path(file: &Path, exe_dir: &Path) -> Option<PathBuf> {
    let ext = file.extension()?.to_str()?;
    let grammar_name = match ext {
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "javascript",
        "py" | "pyw" => "python",
        "rs" => "rust",
        "css" | "scss" | "less" => "css",
        "json" => "json",
        "toml" => "toml",
        _ => return None,
    };
    let filename = format!("{grammar_name}.semtree");

    for search_dir in GRAMMAR_SEARCH_DIRS {
        let candidate = PathBuf::from(search_dir).join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    let exe_grammar = exe_dir.join("grammars").join(&filename);
    if exe_grammar.exists() {
        return Some(exe_grammar);
    }

    None
}

pub fn run(grammar_path: Option<PathBuf>, file: PathBuf, format: String, exe_dir: &Path) -> super::Result {
    let source = std::fs::read_to_string(&file)?;

    let grammar_path = match grammar_path {
        Some(p) => p,
        None => detect_grammar_path(&file, exe_dir).ok_or_else(|| {
            let ext = file.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
            format!(
                "no grammar found for .{ext} files. Use -g to specify a grammar, or add one to grammars/{ext}.semtree\n\
                 Supported: .js, .py, .rs, .css, .json, .toml"
            )
        })?,
    };

    eprintln!("Using grammar: {}", grammar_path.display());
    let grammar_src = std::fs::read_to_string(&grammar_path)?;

    let grammar = if grammar_path
        .extension()
        .is_some_and(|ext| ext == "json")
    {
        import_tree_sitter_grammar(&grammar_src)
            .map_err(|e| format!("failed to import grammar: {e}"))?
    } else {
        parse_semtree_dsl(&grammar_src).map_err(|e| format!("failed to parse grammar: {e}"))?
    };

    let parser = RuntimeParser::new(grammar);
    let result = parser.parse(&source);

    match format.as_str() {
        "tree" => print_tree(&result.syntax(), 0),
        "sexp" => {
            print_sexp(&result.syntax());
            println!();
        }
        "json" => {
            let json = tree_to_json(&result.syntax());
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            return Err(format!("unknown format: {format}. Use tree, sexp, or json").into());
        }
    }

    if !result.errors.is_empty() {
        eprintln!("\n--- {} error(s) ---", result.errors.len());
        for err in &result.errors {
            eprintln!("  {err}");
        }
    }

    Ok(())
}

fn print_tree(node: &SyntaxNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    println!(
        "{prefix}{:?}@{:?}",
        node.kind(),
        node.text_range(),
    );

    for child in node.children_with_tokens() {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_tree(&n, indent + 1),
            semtree_red::SyntaxElement::Token(t) => {
                let tp = "  ".repeat(indent + 1);
                println!("{tp}{:?}@{:?} {:?}", t.kind(), t.text_range(), t.text());
            }
        }
    }
}

fn print_sexp(node: &SyntaxNode) {
    print!("({:?}", node.kind());
    for child in node.children_with_tokens() {
        print!(" ");
        match child {
            semtree_red::SyntaxElement::Node(n) => print_sexp(&n),
            semtree_red::SyntaxElement::Token(t) => {
                print!("({:?} {:?})", t.kind(), t.text());
            }
        }
    }
    print!(")");
}

fn tree_to_json(node: &SyntaxNode) -> serde_json::Value {
    let children: Vec<serde_json::Value> = node
        .children_with_tokens()
        .into_iter()
        .map(|child| match child {
            semtree_red::SyntaxElement::Node(n) => tree_to_json(&n),
            semtree_red::SyntaxElement::Token(t) => serde_json::json!({
                "kind": format!("{:?}", t.kind()),
                "text": t.text(),
                "range": [u32::from(t.text_range().start()), u32::from(t.text_range().end())]
            }),
        })
        .collect();

    serde_json::json!({
        "kind": format!("{:?}", node.kind()),
        "range": [u32::from(node.text_range().start()), u32::from(node.text_range().end())],
        "children": children
    })
}
