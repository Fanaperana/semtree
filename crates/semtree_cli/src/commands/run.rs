use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use semtree_core::SyntaxKind;
use semtree_grammar::parse_semtree_dsl;
use semtree_red::SyntaxNode;
use semtree_runtime::{RuntimeParser, GlrParser};
use semtree_ts_import::import_tree_sitter_grammar;
use smol_str::SmolStr;

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

pub fn run(grammar_path: Option<PathBuf>, file: PathBuf, format: String, exe_dir: &Path, backend: &str) -> super::Result {
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

    let (root, names, errors, extra_info) = match backend {
        "glr" => {
            let parser = GlrParser::new(grammar);
            eprintln!(
                "GLR parser: {} states, conflicts: {}",
                parser.state_count(),
                if parser.has_conflicts() { "yes" } else { "no" }
            );
            let result = parser.parse(&source);
            let extra = if result.is_ambiguous() {
                format!(" ({} ambiguities detected)", result.ambiguity_count)
            } else {
                String::new()
            };
            (result.syntax(), result.kind_names, result.errors, extra)
        }
        "rd" | _ => {
            let parser = RuntimeParser::new(grammar);
            let result = parser.parse(&source);
            (result.syntax(), result.kind_names, result.errors, String::new())
        }
    };

    match format.as_str() {
        "tree" => print_tree(&root, 0, &names),
        "sexp" => {
            print_sexp(&root, &names);
            println!();
        }
        "sexp-pretty" => print_sexp_pretty(&root, 0, &names),
        "inspect" => print_inspect(&root, 0, &names),
        "json" => {
            let json = tree_to_json(&root, &names);
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            return Err(format!("unknown format: {format}. Use tree, sexp, sexp-pretty, inspect, or json").into());
        }
    }

    if !extra_info.is_empty() {
        eprintln!("{extra_info}");
    }

    if !errors.is_empty() {
        eprintln!("\n--- {} error(s) ---", errors.len());
        for err in &errors {
            eprintln!("  {err}");
        }
    }

    Ok(())
}

fn kind_name(kind: SyntaxKind, names: &FxHashMap<SyntaxKind, SmolStr>) -> String {
    if let Some(name) = names.get(&kind) {
        name.to_string()
    } else {
        format!("SyntaxKind({})", kind.0)
    }
}

fn print_tree(node: &SyntaxNode, indent: usize, names: &FxHashMap<SyntaxKind, SmolStr>) {
    let prefix = "  ".repeat(indent);
    println!(
        "{prefix}{}@{:?}",
        kind_name(node.kind(), names),
        node.text_range(),
    );

    for child in node.children_with_tokens() {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_tree(&n, indent + 1, names),
            semtree_red::SyntaxElement::Token(t) => {
                let tp = "  ".repeat(indent + 1);
                println!("{tp}{}@{:?} {:?}", kind_name(t.kind(), names), t.text_range(), t.text());
            }
        }
    }
}

/// Prettified S-expression: indented, one node per line, with byte ranges.
fn print_sexp_pretty(node: &SyntaxNode, indent: usize, names: &FxHashMap<SyntaxKind, SmolStr>) {
    let prefix = "  ".repeat(indent);
    let range = node.text_range();
    let name = kind_name(node.kind(), names);

    let children: Vec<_> = node.children_with_tokens().into_iter().collect();
    if children.is_empty() {
        println!("{prefix}({name}) [{}..{}]", u32::from(range.start()), u32::from(range.end()));
        return;
    }

    // Check if all children are tokens (leaf node with only tokens).
    let all_tokens = children.iter().all(|c| matches!(c, semtree_red::SyntaxElement::Token(_)));

    if all_tokens && children.len() <= 3 {
        print!("{prefix}({name}");
        for child in &children {
            if let semtree_red::SyntaxElement::Token(t) = child {
                let tk = kind_name(t.kind(), names);
                let text = t.text();
                if tk == "whitespace" || tk == "newline" {
                    continue;
                }
                print!(" ({tk} {text:?})");
            }
        }
        println!(") [{}..{}]", u32::from(range.start()), u32::from(range.end()));
        return;
    }

    println!("{prefix}({name} [{}..{}]", u32::from(range.start()), u32::from(range.end()));
    for child in &children {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_sexp_pretty(&n, indent + 1, names),
            semtree_red::SyntaxElement::Token(t) => {
                let tk = kind_name(t.kind(), names);
                let text = t.text();
                if tk == "whitespace" || tk == "newline" {
                    continue;
                }
                let tp = "  ".repeat(indent + 1);
                let tr = t.text_range();
                println!("{tp}({tk} {text:?}) [{}..{}]", u32::from(tr.start()), u32::from(tr.end()));
            }
        }
    }
    println!("{prefix})");
}

/// Machine-readable inspect format: each line is "DEPTH|START|END|KIND|TEXT_OR_EMPTY".
/// Used by editor integrations for interactive tree navigation with highlighting.
fn print_inspect(node: &SyntaxNode, depth: usize, names: &FxHashMap<SyntaxKind, SmolStr>) {
    let range = node.text_range();
    let name = kind_name(node.kind(), names);
    println!("{}|{}|{}|{}|", depth, u32::from(range.start()), u32::from(range.end()), name);

    for child in node.children_with_tokens() {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_inspect(&n, depth + 1, names),
            semtree_red::SyntaxElement::Token(t) => {
                let tk = kind_name(t.kind(), names);
                let tr = t.text_range();
                let text = t.text().replace('\n', "\\n").replace('\r', "\\r");
                println!("{}|{}|{}|{}|{}", depth + 1, u32::from(tr.start()), u32::from(tr.end()), tk, text);
            }
        }
    }
}

fn print_sexp(node: &SyntaxNode, names: &FxHashMap<SyntaxKind, SmolStr>) {
    print!("({}", kind_name(node.kind(), names));
    for child in node.children_with_tokens() {
        print!(" ");
        match child {
            semtree_red::SyntaxElement::Node(n) => print_sexp(&n, names),
            semtree_red::SyntaxElement::Token(t) => {
                print!("({} {:?})", kind_name(t.kind(), names), t.text());
            }
        }
    }
    print!(")");
}

fn tree_to_json(node: &SyntaxNode, names: &FxHashMap<SyntaxKind, SmolStr>) -> serde_json::Value {
    let children: Vec<serde_json::Value> = node
        .children_with_tokens()
        .into_iter()
        .map(|child| match child {
            semtree_red::SyntaxElement::Node(n) => tree_to_json(&n, names),
            semtree_red::SyntaxElement::Token(t) => serde_json::json!({
                "kind": kind_name(t.kind(), names),
                "text": t.text(),
                "range": [u32::from(t.text_range().start()), u32::from(t.text_range().end())]
            }),
        })
        .collect();

    serde_json::json!({
        "kind": kind_name(node.kind(), names),
        "range": [u32::from(node.text_range().start()), u32::from(node.text_range().end())],
        "children": children
    })
}
