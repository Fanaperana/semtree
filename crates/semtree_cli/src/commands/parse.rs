use std::path::PathBuf;

use semtree_parser::Parser;
use semtree_red::SyntaxNode;

pub fn parse(file: PathBuf, format: String) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let result = Parser::parse(&source);

    match format.as_str() {
        "tree" => print_tree(&result.syntax(), 0),
        "sexp" => print_sexp(&result.syntax()),
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
        "{prefix}{:?}@{:?} {:?}",
        node.kind(),
        node.text_range(),
        if node.children().is_empty() {
            node.text()
        } else {
            String::new()
        }
    );

    for child in node.children_with_tokens() {
        match child {
            semtree_red::node::SyntaxElement::Node(n) => print_tree(&n, indent + 1),
            semtree_red::node::SyntaxElement::Token(t) => {
                let tp = "  ".repeat(indent + 1);
                println!("{tp}{:?}@{:?} {:?}", t.kind(), t.text_range(), t.text());
            }
        }
    }
}

fn print_sexp(node: &SyntaxNode) {
    print_sexp_inner(node);
    println!();
}

fn print_sexp_inner(node: &SyntaxNode) {
    print!("({:?}", node.kind());
    for child in node.children_with_tokens() {
        print!(" ");
        match child {
            semtree_red::node::SyntaxElement::Node(n) => print_sexp_inner(&n),
            semtree_red::node::SyntaxElement::Token(t) => {
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
            semtree_red::node::SyntaxElement::Node(n) => tree_to_json(&n),
            semtree_red::node::SyntaxElement::Token(t) => serde_json::json!({
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
