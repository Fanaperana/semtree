use serde::{Deserialize, Serialize};
use text_size::TextRange;

use semtree_red::SyntaxNode;
use semtree_semantic::SemanticModel;

use crate::api;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub start: u32,
    pub end: u32,
    pub is_public: bool,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInfo {
    pub symbol_name: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeInfo {
    pub scope_id: usize,
    pub parent_scope: Option<usize>,
    pub start: u32,
    pub end: u32,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub kind: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeDiff {
    pub kind: TreeDiffKind,
    pub node_kind: String,
    pub start: u32,
    pub end: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TreeDiffKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionSuggestion {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
}

impl From<api::SymbolInfo> for SymbolInfo {
    fn from(s: api::SymbolInfo) -> Self {
        Self {
            name: s.name,
            kind: s.kind,
            start: s.start,
            end: s.end,
            is_public: s.is_public,
            is_mutable: s.is_mutable,
        }
    }
}

impl From<api::ReferenceInfo> for ReferenceInfo {
    fn from(r: api::ReferenceInfo) -> Self {
        Self {
            symbol_name: r.symbol_name,
            start: r.start,
            end: r.end,
        }
    }
}

impl From<api::ScopeInfo> for ScopeInfo {
    fn from(s: api::ScopeInfo) -> Self {
        Self {
            scope_id: s.scope_id,
            parent_scope: s.parent_scope,
            start: s.start,
            end: s.end,
            symbols: s.symbols,
        }
    }
}

impl From<api::FunctionInfo> for FunctionInfo {
    fn from(f: api::FunctionInfo) -> Self {
        Self {
            name: f.name,
            start: f.start,
            end: f.end,
        }
    }
}

impl From<api::NodeInfo> for NodeInfo {
    fn from(n: api::NodeInfo) -> Self {
        Self {
            kind: n.kind,
            start: n.start,
            end: n.end,
            text: n.text,
        }
    }
}

impl From<api::TreeDiff> for TreeDiff {
    fn from(d: api::TreeDiff) -> Self {
        Self {
            kind: match d.kind {
                api::TreeDiffKind::Added => TreeDiffKind::Added,
                api::TreeDiffKind::Removed => TreeDiffKind::Removed,
                api::TreeDiffKind::Changed => TreeDiffKind::Changed,
            },
            node_kind: d.node_kind,
            start: d.start,
            end: d.end,
            text: d.text,
        }
    }
}

impl From<api::CompletionSuggestion> for CompletionSuggestion {
    fn from(c: api::CompletionSuggestion) -> Self {
        Self {
            label: c.label,
            kind: c.kind,
            detail: c.detail,
        }
    }
}

/// Execute a named command and return the result as JSON.
///
/// Supported commands (pass args as JSON object in the command string):
///   - `find_symbol {"name": "foo"}`
///   - `find_references {"name": "foo"}`
///   - `nearest_scope {"offset": 10}`
///   - `current_function {"offset": 10}`
///   - `affected_nodes {"start": 0, "end": 10}`
///   - `diff_tree` (requires two roots — not supported via single-root command)
///   - `suggest_completion {"offset": 10}`
pub fn execute_command(
    root: &SyntaxNode,
    model: &SemanticModel,
    command: &str,
) -> serde_json::Value {
    let (cmd, args) = parse_command(command);

    match cmd {
        "find_symbol" => {
            let name = args
                .and_then(|a| a.get("name")?.as_str().map(String::from))
                .unwrap_or_default();
            let results: Vec<SymbolInfo> = api::find_symbol(root, model, &name)
                .into_iter()
                .map(Into::into)
                .collect();
            serde_json::to_value(results).unwrap_or(serde_json::Value::Null)
        }
        "find_references" => {
            let name = args
                .and_then(|a| a.get("name")?.as_str().map(String::from))
                .unwrap_or_default();
            let results: Vec<ReferenceInfo> = api::find_references(root, model, &name)
                .into_iter()
                .map(Into::into)
                .collect();
            serde_json::to_value(results).unwrap_or(serde_json::Value::Null)
        }
        "nearest_scope" => {
            let offset = args
                .and_then(|a| a.get("offset")?.as_u64())
                .unwrap_or(0) as u32;
            let result: Option<ScopeInfo> =
                api::nearest_scope(root, model, offset).map(Into::into);
            serde_json::to_value(result).unwrap_or(serde_json::Value::Null)
        }
        "current_function" => {
            let offset = args
                .and_then(|a| a.get("offset")?.as_u64())
                .unwrap_or(0) as u32;
            let result: Option<FunctionInfo> =
                api::current_function(root, offset).map(Into::into);
            serde_json::to_value(result).unwrap_or(serde_json::Value::Null)
        }
        "affected_nodes" => {
            let start = args
                .as_ref()
                .and_then(|a| a.get("start")?.as_u64())
                .unwrap_or(0) as u32;
            let end = args
                .as_ref()
                .and_then(|a| a.get("end")?.as_u64())
                .unwrap_or(0) as u32;
            let range = TextRange::new(start.into(), end.into());
            let results: Vec<NodeInfo> = api::affected_nodes(root, range)
                .into_iter()
                .map(Into::into)
                .collect();
            serde_json::to_value(results).unwrap_or(serde_json::Value::Null)
        }
        "suggest_completion" => {
            let offset = args
                .and_then(|a| a.get("offset")?.as_u64())
                .unwrap_or(0) as u32;
            let results: Vec<CompletionSuggestion> =
                api::suggest_completion(root, model, offset)
                    .into_iter()
                    .map(Into::into)
                    .collect();
            serde_json::to_value(results).unwrap_or(serde_json::Value::Null)
        }
        _ => serde_json::json!({ "error": format!("unknown command: {}", cmd) }),
    }
}

fn parse_command(input: &str) -> (&str, Option<serde_json::Value>) {
    let input = input.trim();
    if let Some(idx) = input.find('{') {
        let cmd = input[..idx].trim();
        let args = serde_json::from_str(&input[idx..]).ok();
        (cmd, args)
    } else {
        (input, None)
    }
}
