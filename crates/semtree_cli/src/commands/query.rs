use std::path::PathBuf;

use semtree_core::SyntaxKind;
use semtree_parser::Parser;
use semtree_query::engine::QueryEngine;
use semtree_query::pattern::parse_query;

pub fn query(file: PathBuf, pattern: String) -> super::Result {
    let source = std::fs::read_to_string(&file)?;
    let result = Parser::parse(&source);
    let root = result.syntax();

    // Try S-expression query syntax first.
    if pattern.starts_with('(') {
        let query = parse_query(&pattern).map_err(|e| format!("query parse error: {e}"))?;
        let matches = QueryEngine::query(&root, &query);

        println!("Found {} match(es):\n", matches.len());
        for m in &matches {
            let range = m.node.text_range();
            println!(
                "  [{start}..{end}] {kind:?}",
                start = u32::from(range.start()),
                end = u32::from(range.end()),
                kind = m.node.kind(),
            );
            for cap in &m.captures {
                println!(
                    "    @{name} = {text}",
                    name = cap.name,
                    text = truncate(&cap.text(), 60),
                );
            }
        }
        return Ok(());
    }

    // Simple kind-name query.
    let target_kind = match pattern.as_str() {
        "Function" | "function" | "fn" => Some(SyntaxKind::FUNCTION),
        "Struct" | "struct" => Some(SyntaxKind::STRUCT_DEF),
        "Enum" | "enum" => Some(SyntaxKind::ENUM_DEF),
        "Let" | "let" => Some(SyntaxKind::LET_STMT),
        "If" | "if" => Some(SyntaxKind::IF_EXPR),
        "While" | "while" => Some(SyntaxKind::WHILE_EXPR),
        "For" | "for" => Some(SyntaxKind::FOR_EXPR),
        "Return" | "return" => Some(SyntaxKind::RETURN_STMT),
        "Block" | "block" => Some(SyntaxKind::BLOCK),
        "Impl" | "impl" => Some(SyntaxKind::IMPL_DEF),
        "Trait" | "trait" => Some(SyntaxKind::TRAIT_DEF),
        "Use" | "use" => Some(SyntaxKind::USE_DECL),
        _ => None,
    };

    match target_kind {
        Some(kind) => {
            let matches = QueryEngine::find_by_kind(&root, kind);
            println!("Found {} match(es) for '{pattern}':\n", matches.len());
            for node in &matches {
                let range = node.text_range();
                println!(
                    "  [{start}..{end}] {text}",
                    start = u32::from(range.start()),
                    end = u32::from(range.end()),
                    text = truncate(&node.text(), 80),
                );
            }
        }
        None => {
            let matches = QueryEngine::find_by_text(&root, &pattern);
            println!("Found {} node(s) containing '{pattern}':\n", matches.len());
            for node in &matches {
                let range = node.text_range();
                println!(
                    "  [{start}..{end}] {kind:?}: {text}",
                    start = u32::from(range.start()),
                    end = u32::from(range.end()),
                    kind = node.kind(),
                    text = truncate(&node.text(), 80),
                );
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let oneline = s.replace('\n', " ").replace('\r', "");
    if oneline.len() > max {
        format!("{}...", &oneline[..max])
    } else {
        oneline
    }
}
