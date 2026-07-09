use std::path::{Path, PathBuf};

use rustc_hash::FxHashMap;
use semtree_core::SyntaxKind;
use semtree_red::SyntaxNode;
use semtree_runtime::{
    EditRegion, GlrParser, ParseSession, ParserBackend, RuntimeParser, select_backend,
};
use smol_str::SmolStr;

use super::grammar_util::resolve_grammar;

pub fn run(
    grammar_path: Option<PathBuf>,
    file: PathBuf,
    format: String,
    exe_dir: &Path,
    backend: &str,
    incremental: bool,
    edit: Option<&str>,
) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let (grammar_path, grammar) = resolve_grammar(grammar_path, &file, exe_dir)?;

    eprintln!("Using grammar: {}", grammar_path.display());

    let resolved_backend = match backend {
        "auto" => select_backend(&grammar),
        "glr" => ParserBackend::Glr,
        _ => ParserBackend::RecursiveDescent,
    };

    if resolved_backend == ParserBackend::Glr {
        eprintln!("Backend: glr (auto-selected={})", backend == "auto");
    } else {
        eprintln!("Backend: rd (auto-selected={})", backend == "auto");
    }

    let (root, names, errors, extra_info) = match resolved_backend {
        ParserBackend::Glr => {
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
        ParserBackend::RecursiveDescent | ParserBackend::Auto => {
            if incremental || edit.is_some() {
                let mut session = ParseSession::new(grammar, ParserBackend::RecursiveDescent);
                let unified = if let Some(spec) = edit {
                    let (start, end, text) = parse_edit_spec(spec)?;
                    let edits = vec![EditRegion::new(start, end, text)];
                    eprintln!(
                        "incremental reparse: edit [{start}..{end}) -> {:?}",
                        edits[0].new_text
                    );
                    session.parse(&source);
                    session.apply_edits(&edits)
                } else {
                    session.parse(&source)
                };
                let rd_errors = unified
                    .errors
                    .into_iter()
                    .map(|msg| semtree_runtime::RuntimeParseError {
                        message: msg,
                        range: text_size::TextRange::empty(text_size::TextSize::new(0)),
                    })
                    .collect();
                (
                    unified.syntax,
                    unified.kind_names,
                    rd_errors,
                    if incremental {
                        " (incremental parser)".into()
                    } else {
                        String::new()
                    },
                )
            } else {
                let parser = RuntimeParser::new(grammar);
                let result = parser.parse(&source);
                (
                    result.syntax(),
                    result.kind_names,
                    result.errors,
                    String::new(),
                )
            }
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
            return Err(format!(
                "unknown format: {format}. Use tree, sexp, sexp-pretty, inspect, or json"
            )
            .into());
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

/// Parse `START:END:TEXT` edit specification (byte offsets).
fn parse_edit_spec(spec: &str) -> Result<(u32, u32, String), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    if parts.len() < 3 {
        return Err("edit format must be START:END:TEXT (e.g. 10:10:x)".into());
    }
    let start: u32 = parts[0].parse().map_err(|_| "invalid start offset")?;
    let end: u32 = parts[1].parse().map_err(|_| "invalid end offset")?;
    let text = parts[2].to_string();
    Ok((start, end, text))
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
                println!(
                    "{tp}{}@{:?} {:?}",
                    kind_name(t.kind(), names),
                    t.text_range(),
                    t.text()
                );
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
        println!(
            "{prefix}({name}) [{}..{}]",
            u32::from(range.start()),
            u32::from(range.end())
        );
        return;
    }

    // Check if all children are tokens (leaf node with only tokens).
    let all_tokens = children
        .iter()
        .all(|c| matches!(c, semtree_red::SyntaxElement::Token(_)));

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
        println!(
            ") [{}..{}]",
            u32::from(range.start()),
            u32::from(range.end())
        );
        return;
    }

    println!(
        "{prefix}({name} [{}..{}]",
        u32::from(range.start()),
        u32::from(range.end())
    );
    for child in &children {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_sexp_pretty(n, indent + 1, names),
            semtree_red::SyntaxElement::Token(t) => {
                let tk = kind_name(t.kind(), names);
                let text = t.text();
                if tk == "whitespace" || tk == "newline" {
                    continue;
                }
                let tp = "  ".repeat(indent + 1);
                let tr = t.text_range();
                println!(
                    "{tp}({tk} {text:?}) [{}..{}]",
                    u32::from(tr.start()),
                    u32::from(tr.end())
                );
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
    println!(
        "{}|{}|{}|{}|",
        depth,
        u32::from(range.start()),
        u32::from(range.end()),
        name
    );

    for child in node.children_with_tokens() {
        match child {
            semtree_red::SyntaxElement::Node(n) => print_inspect(&n, depth + 1, names),
            semtree_red::SyntaxElement::Token(t) => {
                let tk = kind_name(t.kind(), names);
                let tr = t.text_range();
                let text = t.text().replace('\n', "\\n").replace('\r', "\\r");
                println!(
                    "{}|{}|{}|{}|{}",
                    depth + 1,
                    u32::from(tr.start()),
                    u32::from(tr.end()),
                    tk,
                    text
                );
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
