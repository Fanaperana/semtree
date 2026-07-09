use semtree_core::SyntaxKind;
use semtree_red::{SyntaxElement, SyntaxNode};

use crate::captures::{QueryCapture, QueryMatch};
use crate::pattern::{PatternNode, QueryPattern};

/// The query engine: executes query patterns against syntax trees.
pub struct QueryEngine;

impl QueryEngine {
    /// Execute a query pattern against a syntax tree, returning all matches.
    pub fn query(root: &SyntaxNode, pattern: &QueryPattern) -> Vec<QueryMatch> {
        let mut matches = Vec::new();

        for pattern_node in &pattern.nodes {
            Self::find_matches(root, pattern_node, &mut matches);
        }

        matches
    }

    /// Find all nodes matching a specific SyntaxKind.
    pub fn find_by_kind(root: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
        let mut results = Vec::new();
        Self::collect_by_kind(root, kind, &mut results);
        results
    }

    /// Find all nodes whose text contains the given substring.
    pub fn find_by_text(root: &SyntaxNode, text: &str) -> Vec<SyntaxNode> {
        let mut results = Vec::new();
        Self::collect_by_text(root, text, &mut results);
        results
    }

    /// Find all identifier tokens in the tree.
    pub fn find_identifiers(root: &SyntaxNode) -> Vec<(String, SyntaxNode)> {
        let mut results = Vec::new();
        Self::collect_identifiers(root, &mut results);
        results
    }

    /// Find the deepest node at a given byte offset.
    pub fn node_at_offset(root: &SyntaxNode, offset: u32) -> Option<SyntaxNode> {
        let offset = text_size::TextSize::new(offset);
        if !root.text_range().contains(offset) {
            return None;
        }

        let mut current = root.clone();
        'outer: loop {
            for child in current.children() {
                if child.text_range().contains(offset) {
                    current = child;
                    continue 'outer;
                }
            }
            break;
        }
        Some(current)
    }

    // ── Internal matching ────────────────────────────────────

    fn find_matches(node: &SyntaxNode, pattern: &PatternNode, matches: &mut Vec<QueryMatch>) {
        // Try matching this node against the pattern.
        let mut captures = Vec::new();
        if Self::matches_node(node, pattern, &mut captures) {
            matches.push(QueryMatch {
                node: node.clone(),
                captures,
            });
        }

        // Recurse into children.
        for child in node.children() {
            Self::find_matches(&child, pattern, matches);
        }
    }

    fn matches_node(
        node: &SyntaxNode,
        pattern: &PatternNode,
        captures: &mut Vec<QueryCapture>,
    ) -> bool {
        // Check kind match.
        if let Some(kind) = pattern.kind
            && node.kind() != kind
        {
            return false;
        }

        // Check kind name match (runtime grammar rules use hashed kinds).
        if let Some(ref kind_name) = pattern.kind_name
            && !Self::kind_name_matches(node, kind_name)
        {
            return false;
        }

        // Check text match.
        if let Some(ref expected_text) = pattern.text_match {
            let node_text = node.text();
            if node_text.trim() != expected_text.as_str() {
                return false;
            }
        }

        // Check child patterns (subsequence matching).
        if !pattern.children.is_empty() {
            let children = node.children_with_tokens();
            let child_nodes: Vec<_> = children
                .into_iter()
                .filter_map(|e| match e {
                    SyntaxElement::Node(n) => Some(n),
                    _ => None,
                })
                .collect();

            let mut child_idx = 0;
            for child_pattern in &pattern.children {
                let mut found = false;
                while child_idx < child_nodes.len() {
                    let mut child_caps = Vec::new();
                    if Self::matches_node(&child_nodes[child_idx], child_pattern, &mut child_caps) {
                        captures.extend(child_caps);
                        child_idx += 1;
                        found = true;
                        break;
                    }
                    child_idx += 1;
                }
                if !found && child_pattern.text_match.is_none() {
                    // Text match children are checked against tokens, not nodes.
                    // For now, skip non-matching text patterns.
                    if child_pattern.kind.is_some() || child_pattern.kind_name.is_some() {
                        return false;
                    }
                }
            }
        }

        // Add capture for this node.
        if let Some(ref name) = pattern.capture {
            captures.push(QueryCapture {
                name: name.clone(),
                node: node.clone(),
            });
        }

        true
    }

    /// Check if a node's kind matches a named kind from a grammar.
    /// We use the same hashing scheme as RuntimeParser::rule_name_to_kind.
    fn kind_name_matches(node: &SyntaxNode, name: &str) -> bool {
        // Check well-known kinds by name.
        let expected = match name {
            "SOURCE_FILE" | "SourceFile" | "source_file" => Some(SyntaxKind::SOURCE_FILE),
            "FUNCTION" | "Function" | "function" => Some(SyntaxKind::FUNCTION),
            "PARAM_LIST" | "ParamList" | "param_list" => Some(SyntaxKind::PARAM_LIST),
            "PARAM" | "Param" | "param" => Some(SyntaxKind::PARAM),
            "BLOCK" | "Block" | "block" => Some(SyntaxKind::BLOCK),
            "LET_STMT" | "LetStatement" | "let_stmt" => Some(SyntaxKind::LET_STMT),
            "EXPR_STMT" | "ExpressionStatement" | "expr_stmt" => Some(SyntaxKind::EXPR_STMT),
            "RETURN_STMT" | "ReturnStatement" | "return_stmt" => Some(SyntaxKind::RETURN_STMT),
            "IF_EXPR" | "IfExpr" | "if_expr" => Some(SyntaxKind::IF_EXPR),
            "BINARY_EXPR" | "BinaryExpr" | "binary_expr" => Some(SyntaxKind::BINARY_EXPR),
            "CALL_EXPR" | "CallExpr" | "call_expr" => Some(SyntaxKind::CALL_EXPR),
            "STRUCT_DEF" | "StructDef" | "struct_def" => Some(SyntaxKind::STRUCT_DEF),
            "ENUM_DEF" | "EnumDef" | "enum_def" => Some(SyntaxKind::ENUM_DEF),
            "LITERAL" | "Literal" | "literal" => Some(SyntaxKind::LITERAL),
            "PATH_EXPR" | "PathExpr" | "path_expr" => Some(SyntaxKind::PATH_EXPR),
            "ERROR" | "error" => Some(SyntaxKind::ERROR),
            _ => None,
        };

        if let Some(expected_kind) = expected {
            return node.kind() == expected_kind;
        }

        // For runtime grammar rules, use the same hash function.
        let hashed_kind = runtime_kind_hash(name);
        node.kind() == hashed_kind
    }

    fn collect_by_kind(node: &SyntaxNode, kind: SyntaxKind, results: &mut Vec<SyntaxNode>) {
        if node.kind() == kind {
            results.push(node.clone());
        }
        for child in node.children() {
            Self::collect_by_kind(&child, kind, results);
        }
    }

    fn collect_by_text(node: &SyntaxNode, text: &str, results: &mut Vec<SyntaxNode>) {
        for child in node.children_with_tokens() {
            match child {
                SyntaxElement::Token(t) if t.text().contains(text) => {
                    if let Some(parent) = t.parent() {
                        results.push(parent.clone());
                    }
                }
                SyntaxElement::Node(n) => Self::collect_by_text(&n, text, results),
                _ => {}
            }
        }
    }

    fn collect_identifiers(node: &SyntaxNode, results: &mut Vec<(String, SyntaxNode)>) {
        for child in node.children_with_tokens() {
            match child {
                SyntaxElement::Token(t) if t.kind() == SyntaxKind::IDENT => {
                    results.push((t.text().to_string(), node.clone()));
                }
                SyntaxElement::Node(n) => Self::collect_identifiers(&n, results),
                _ => {}
            }
        }
    }
}

/// The same deterministic hash used by RuntimeParser::rule_name_to_kind.
fn runtime_kind_hash(name: &str) -> SyntaxKind {
    let mut hash: u16 = 4096;
    for (i, b) in name.bytes().enumerate() {
        hash = hash
            .wrapping_add(b as u16)
            .wrapping_mul(31)
            .wrapping_add(i as u16);
    }
    if hash < 4096 {
        hash += 4096;
    }
    SyntaxKind(hash)
}
