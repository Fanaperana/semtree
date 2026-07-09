//! Minimal LSP server for the `todo` language, powered by the `semtree` CLI.
//!
//! Speaks LSP over stdin/stdout. Designed as a template: point TODOLSP_GRAMMAR
//! at any .semtree file to reuse the same server for another language.

use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

struct State {
    documents: std::collections::HashMap<String, String>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn main() {
    *STATE.lock().unwrap() = Some(State {
        documents: std::collections::HashMap::new(),
    });

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        let msg = match read_message(&mut reader) {
            Ok(m) => m,
            Err(_) => break,
        };

        if let Some(response) = handle(&msg) {
            if let Err(_) = write_message(&mut stdout, &response) {
                break;
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Value> {
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
    }

    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    Ok(serde_json::from_str(&text).unwrap_or(json!({})))
}

fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_string(value)?;
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )?;
    writer.flush()
}

fn handle(msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = msg.get("id").cloned();

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": true,
                        "change": 1,
                        "save": { "includeText": true }
                    },
                    "hoverProvider": true,
                    "documentSymbolProvider": true
                },
                "serverInfo": { "name": "todo-lsp", "version": "0.1.0" }
            }
        })),
        "initialized" | "shutdown" => {
            if method == "shutdown" {
                Some(json!({ "jsonrpc": "2.0", "id": id, "result": null }))
            } else {
                None
            }
        }
        "exit" => std::process::exit(0),
        "textDocument/didOpen" => {
            on_did_open(msg);
            None
        }
        "textDocument/didChange" => {
            on_did_change(msg);
            None
        }
        "textDocument/didSave" => {
            on_did_save(msg);
            None
        }
        "textDocument/hover" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": hover(msg)
        })),
        "textDocument/documentSymbol" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": document_symbols(msg)
        })),
        _ => {
            if id.is_some() {
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("Method not found: {method}") }
                }))
            } else {
                None
            }
        }
    }
}

fn grammar_path() -> PathBuf {
    std::env::var_os("TODOLSP_GRAMMAR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("grammars/todo.semtree"))
}

fn semtree_bin() -> PathBuf {
    std::env::var_os("TODOLSP_SEMTREE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("semtree"))
}

fn on_did_open(msg: &Value) {
    let params = &msg["params"];
    let uri = params["textDocument"]["uri"].as_str().unwrap_or("").to_string();
    let text = params["textDocument"]["text"].as_str().unwrap_or("").to_string();
    if let Some(state) = STATE.lock().unwrap().as_mut() {
        state.documents.insert(uri.clone(), text);
    }
    publish_diagnostics(&uri);
}

fn on_did_change(msg: &Value) {
    let params = &msg["params"];
    let uri = params["textDocument"]["uri"].as_str().unwrap_or("").to_string();
    if let Some(text) = params["contentChanges"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
    {
        if let Some(state) = STATE.lock().unwrap().as_mut() {
            state.documents.insert(uri.clone(), text.to_string());
        }
        publish_diagnostics(&uri);
    }
}

fn on_did_save(msg: &Value) {
    let uri = msg["params"]["textDocument"]["uri"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if let Some(text) = msg["params"].get("text").and_then(|t| t.as_str()) {
        if let Some(state) = STATE.lock().unwrap().as_mut() {
            state.documents.insert(uri.clone(), text.to_string());
        }
    }
    publish_diagnostics(&uri);
}

#[derive(Clone, Debug)]
struct Node {
    depth: usize,
    start: u32,
    end: u32,
    kind: String,
    text: String,
}

fn parse_document(uri: &str) -> (Vec<Node>, Vec<String>) {
    let text = {
        let guard = STATE.lock().unwrap();
        guard
            .as_ref()
            .and_then(|s| s.documents.get(uri).cloned())
            .unwrap_or_default()
    };

    // Write to a temp file so `semtree run` can read it.
    let tmp = std::env::temp_dir().join(format!(
        "todo-lsp-{}.todo",
        std::process::id()
    ));
    let _ = std::fs::write(&tmp, &text);

    let output = Command::new(semtree_bin())
        .arg("run")
        .arg("-g")
        .arg(grammar_path())
        .arg("-f")
        .arg("inspect")
        .arg(&tmp)
        .output();

    let _ = std::fs::remove_file(&tmp);

    let mut nodes = Vec::new();
    let mut errors = Vec::new();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(n) = parse_inspect_line(line) {
                    nodes.push(n);
                }
            }
            let stderr = String::from_utf8_lossy(&out.stderr);
            for line in stderr.lines() {
                if line.contains("error") || line.contains("unexpected") {
                    errors.push(line.to_string());
                }
            }
        }
        Err(e) => errors.push(format!("failed to run semtree: {e}")),
    }

    (nodes, errors)
}

fn parse_inspect_line(line: &str) -> Option<Node> {
    let parts: Vec<&str> = line.splitn(5, '|').collect();
    if parts.len() < 4 {
        return None;
    }
    Some(Node {
        depth: parts[0].parse().ok()?,
        start: parts[1].parse().ok()?,
        end: parts[2].parse().ok()?,
        kind: parts[3].to_string(),
        text: parts.get(4).unwrap_or(&"").to_string(),
    })
}

fn publish_diagnostics(uri: &str) {
    let (nodes, errors) = parse_document(uri);
    let text = {
        let guard = STATE.lock().unwrap();
        guard
            .as_ref()
            .and_then(|s| s.documents.get(uri).cloned())
            .unwrap_or_default()
    };

    let mut diagnostics = Vec::new();

    // Map stderr errors into line-level diagnostics when possible.
    for (i, err) in errors.iter().enumerate() {
        diagnostics.push(json!({
            "range": {
                "start": { "line": i.min(text.lines().count().saturating_sub(1)), "character": 0 },
                "end": { "line": i.min(text.lines().count().saturating_sub(1)), "character": 80 }
            },
            "severity": 1,
            "source": "todo-lsp",
            "message": err
        }));
    }

    // If the tree is only trivia / empty while source is non-empty, warn.
    if diagnostics.is_empty() && !text.trim().is_empty() && nodes.len() < 2 {
        diagnostics.push(json!({
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 1 }
            },
            "severity": 2,
            "source": "todo-lsp",
            "message": "parse produced a very small tree — check grammar / input"
        }));
    }

    let note = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": uri,
            "diagnostics": diagnostics
        }
    });

    let mut stdout = io::stdout();
    let _ = write_message(&mut stdout, &note);
}

fn hover(msg: &Value) -> Value {
    let uri = msg["params"]["textDocument"]["uri"].as_str().unwrap_or("");
    let line = msg["params"]["position"]["line"].as_u64().unwrap_or(0) as usize;
    let character = msg["params"]["position"]["character"].as_u64().unwrap_or(0) as usize;

    let text = {
        let guard = STATE.lock().unwrap();
        guard
            .as_ref()
            .and_then(|s| s.documents.get(uri).cloned())
            .unwrap_or_default()
    };

    let offset = position_to_offset(&text, line, character);
    let (nodes, _) = parse_document(uri);

    let mut best: Option<&Node> = None;
    for n in &nodes {
        if n.start as usize <= offset && offset < n.end as usize {
            match best {
                None => best = Some(n),
                Some(b) if (n.end - n.start) < (b.end - b.start) => best = Some(n),
                _ => {}
            }
        }
    }

    match best {
        Some(n) => {
            let snippet = if n.text.is_empty() {
                format!("{} [{}..{}]", n.kind, n.start, n.end)
            } else {
                format!("{} `{}` [{}..{}]", n.kind, n.text, n.start, n.end)
            };
            json!({
                "contents": {
                    "kind": "markdown",
                    "value": format!("**{}**\n\n{}", n.kind, snippet)
                }
            })
        }
        None => Value::Null,
    }
}

fn document_symbols(msg: &Value) -> Value {
    let uri = msg["params"]["textDocument"]["uri"].as_str().unwrap_or("");
    let text = {
        let guard = STATE.lock().unwrap();
        guard
            .as_ref()
            .and_then(|s| s.documents.get(uri).cloned())
            .unwrap_or_default()
    };
    let (nodes, _) = parse_document(uri);

    let interesting = ["TodoItem", "DoneItem", "Item", "Document"];
    let mut symbols = Vec::new();

    for n in &nodes {
        if !interesting.contains(&n.kind.as_str()) {
            continue;
        }
        let (sl, sc) = offset_to_position(&text, n.start as usize);
        let (el, ec) = offset_to_position(&text, n.end as usize);
        let name = if n.text.is_empty() {
            n.kind.clone()
        } else {
            format!("{} {}", n.kind, n.text)
        };
        symbols.push(json!({
            "name": name,
            "kind": 13,
            "range": {
                "start": { "line": sl, "character": sc },
                "end": { "line": el, "character": ec }
            },
            "selectionRange": {
                "start": { "line": sl, "character": sc },
                "end": { "line": el, "character": ec }
            }
        }));
    }

    Value::Array(symbols)
}

fn position_to_offset(text: &str, line: usize, character: usize) -> usize {
    let mut offset = 0usize;
    for (i, l) in text.split_inclusive('\n').enumerate() {
        if i == line {
            return offset + character.min(l.len());
        }
        offset += l.len();
    }
    text.len()
}

fn offset_to_position(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut col = 0usize;
    for (i, ch) in text.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}
