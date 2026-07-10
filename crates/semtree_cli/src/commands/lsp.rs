//! LSP server: incremental parsing, diagnostics, document symbols,
//! completion, hover, go-to-definition, find-references, semantic tokens,
//! formatting, and folding.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CompletionItem as LspCompletionItem, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DocumentFormattingParams, DocumentSymbol, DocumentSymbolParams,
    DocumentSymbolResponse, FoldingRange as LspFoldingRange, FoldingRangeKind, FoldingRangeParams,
    FoldingRangeProviderCapability, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, Location, MarkupContent, MarkupKind,
    OneOf, Position, Range, ReferenceParams, SemanticToken as LspSemanticToken,
    SemanticTokenType as LspSemanticTokenType, SemanticTokens, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, SymbolKind, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit, Uri,
};
use semtree_ide::{
    classify_tokens as ide_classify, complete_at as ide_complete, document_symbols as ide_symbols,
    find_references as ide_references, folding_ranges as ide_fold, goto_definition as ide_goto_def,
    hover_info as ide_hover,
};
use semtree_runtime::{ParseSession, ParserBackend};
use semtree_semantic::SemanticModel;

use super::grammar_util::resolve_grammar;

struct DocumentState {
    session: ParseSession,
    version: i32,
    errors: Vec<String>,
}

pub fn lsp(exe_dir: PathBuf) -> super::Result {
    let (connection, io_threads) = Connection::stdio();
    let caps = serde_json::to_value(server_capabilities())?;
    let init_params = match connection.initialize(caps) {
        Ok(params) => params,
        Err(e) => {
            // Client disconnected before completing handshake — not an error
            if e.to_string().contains("disconnected") {
                return Ok(());
            }
            return Err(format!("LSP init failed: {e:?}").into());
        }
    };

    // Extract root_uri from initialization params to search for grammars
    #[allow(deprecated)]
    let root_path: Option<PathBuf> =
        serde_json::from_value::<lsp_types::InitializeParams>(init_params)
            .ok()
            .and_then(|params| {
                params
                    .root_uri
                    .and_then(|uri| uri.as_str().strip_prefix("file://").map(PathBuf::from))
                    .or_else(|| params.root_path.map(PathBuf::from))
            });

    let mut documents: HashMap<String, DocumentState> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                handle_request(&connection, &documents, req)?;
            }
            Message::Notification(notif) => {
                handle_notification(
                    &connection,
                    &mut documents,
                    &exe_dir,
                    root_path.as_deref(),
                    notif,
                )?;
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join().ok();
    Ok(())
}

fn server_capabilities() -> ServerCapabilities {
    let token_types = vec![
        LspSemanticTokenType::KEYWORD,
        LspSemanticTokenType::TYPE,
        LspSemanticTokenType::FUNCTION,
        LspSemanticTokenType::VARIABLE,
        LspSemanticTokenType::STRING,
        LspSemanticTokenType::NUMBER,
        LspSemanticTokenType::COMMENT,
        LspSemanticTokenType::OPERATOR,
        LspSemanticTokenType::PARAMETER,
        LspSemanticTokenType::PROPERTY,
        LspSemanticTokenType::ENUM_MEMBER,
    ];
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                ..Default::default()
            },
        )),
        document_symbol_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".into(), ":".into()]),
            ..Default::default()
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types,
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                ..Default::default()
            },
        )),
        ..Default::default()
    }
}

fn uri_key(uri: &Uri) -> String {
    uri.to_string()
}

fn handle_request(
    connection: &Connection,
    documents: &HashMap<String, DocumentState>,
    req: Request,
) -> super::Result {
    match req.method.as_str() {
        "textDocument/documentSymbol" => {
            let (id, params): (_, DocumentSymbolParams) =
                match req.extract("textDocument/documentSymbol") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let symbols = documents.get(&key).map(symbols_for_doc).unwrap_or_default();
            connection.sender.send(Message::Response(Response::new_ok(
                id,
                DocumentSymbolResponse::Nested(symbols),
            )))?;
        }
        "textDocument/completion" => {
            let (id, params): (_, CompletionParams) = match req.extract("textDocument/completion") {
                Ok(v) => v,
                Err(e) => return Err(format!("{e:?}").into()),
            };
            let key = uri_key(&params.text_document_position.text_document.uri);
            let items = documents
                .get(&key)
                .map(|doc| completion_for_doc(doc, params.text_document_position.position))
                .unwrap_or_default();
            connection.sender.send(Message::Response(Response::new_ok(
                id,
                CompletionResponse::Array(items),
            )))?;
        }
        "textDocument/hover" => {
            let (id, params): (_, HoverParams) = match req.extract("textDocument/hover") {
                Ok(v) => v,
                Err(e) => return Err(format!("{e:?}").into()),
            };
            let key = uri_key(&params.text_document_position_params.text_document.uri);
            let hover = documents
                .get(&key)
                .and_then(|doc| hover_for_doc(doc, params.text_document_position_params.position));
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, hover)))?;
        }
        "textDocument/definition" => {
            let (id, params): (_, GotoDefinitionParams) =
                match req.extract("textDocument/definition") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let uri = params
                .text_document_position_params
                .text_document
                .uri
                .clone();
            let key = uri_key(&uri);
            let def = documents.get(&key).and_then(|doc| {
                let offset = position_to_offset(
                    doc.session.source(),
                    params.text_document_position_params.position,
                );
                let root = doc.session.syntax()?;
                let model = SemanticModel::analyze(root);
                ide_goto_def(root, &model, offset).map(|range| {
                    let start = byte_to_position(doc.session.source(), u32::from(range.start()));
                    let end = byte_to_position(doc.session.source(), u32::from(range.end()));
                    GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: Range { start, end },
                    })
                })
            });
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, def)))?;
        }
        "textDocument/references" => {
            let (id, params): (_, ReferenceParams) = match req.extract("textDocument/references") {
                Ok(v) => v,
                Err(e) => return Err(format!("{e:?}").into()),
            };
            let uri = params.text_document_position.text_document.uri.clone();
            let key = uri_key(&uri);
            let refs: Vec<Location> = documents
                .get(&key)
                .map(|doc| {
                    let offset = position_to_offset(
                        doc.session.source(),
                        params.text_document_position.position,
                    );
                    let Some(root) = doc.session.syntax() else {
                        return vec![];
                    };
                    let model = SemanticModel::analyze(root);
                    ide_references(root, &model, offset)
                        .into_iter()
                        .map(|range| {
                            let start =
                                byte_to_position(doc.session.source(), u32::from(range.start()));
                            let end =
                                byte_to_position(doc.session.source(), u32::from(range.end()));
                            Location {
                                uri: uri.clone(),
                                range: Range { start, end },
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, refs)))?;
        }
        "textDocument/semanticTokens/full" => {
            let (id, params): (_, SemanticTokensParams) =
                match req.extract("textDocument/semanticTokens/full") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let tokens = documents
                .get(&key)
                .map(semantic_tokens_for_doc)
                .unwrap_or_default();
            connection.sender.send(Message::Response(Response::new_ok(
                id,
                SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: tokens,
                }),
            )))?;
        }
        "textDocument/formatting" => {
            let (id, params): (_, DocumentFormattingParams) =
                match req.extract("textDocument/formatting") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let edits = documents.get(&key).map(format_doc).unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, edits)))?;
        }
        "textDocument/foldingRange" => {
            let (id, params): (_, FoldingRangeParams) =
                match req.extract("textDocument/foldingRange") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let ranges = documents.get(&key).map(folding_for_doc).unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, ranges)))?;
        }
        _ => {
            connection.sender.send(Message::Response(Response::new_err(
                req.id,
                -32601,
                format!("method not found: {}", req.method),
            )))?;
        }
    }
    Ok(())
}

fn handle_notification(
    connection: &Connection,
    documents: &mut HashMap<String, DocumentState>,
    exe_dir: &Path,
    root_path: Option<&Path>,
    notif: Notification,
) -> super::Result {
    match notif.method.as_str() {
        "textDocument/didOpen" => {
            let params: lsp_types::DidOpenTextDocumentParams = notif
                .extract("textDocument/didOpen")
                .map_err(|e| format!("{e:?}"))?;
            let uri = params.text_document.uri.clone();
            let key = uri_key(&uri);
            match open_document(documents, exe_dir, root_path, params) {
                Ok(()) => {
                    publish_diagnostics(connection, documents, &key, &uri)?;
                }
                Err(e) => {
                    eprintln!("semtree-lsp: skipping document {key}: {e}");
                }
            }
        }
        "textDocument/didChange" => {
            let params: lsp_types::DidChangeTextDocumentParams = notif
                .extract("textDocument/didChange")
                .map_err(|e| format!("{e:?}"))?;
            let key = uri_key(&params.text_document.uri);
            let uri = params.text_document.uri.clone();
            change_document(documents, params)?;
            publish_diagnostics(connection, documents, &key, &uri)?;
        }
        "textDocument/didClose" => {
            let params: lsp_types::DidCloseTextDocumentParams = notif
                .extract("textDocument/didClose")
                .map_err(|e| format!("{e:?}"))?;
            documents.remove(&uri_key(&params.text_document.uri));
        }
        _ => {}
    }
    Ok(())
}

fn open_document(
    documents: &mut HashMap<String, DocumentState>,
    exe_dir: &Path,
    root_path: Option<&Path>,
    params: lsp_types::DidOpenTextDocumentParams,
) -> super::Result {
    let uri = params.text_document.uri;
    let key = uri_key(&uri);
    let path = uri_to_path(&uri)?;

    // Also search for grammars relative to workspace root
    let grammar_result = resolve_grammar(None, &path, exe_dir).or_else(|e| {
        if let Some(root) = root_path {
            let grammars_in_root = root.join("grammars");
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let grammar_name = match ext {
                "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => "javascript",
                "py" | "pyw" => "python",
                "rs" => "rust",
                "css" | "scss" | "less" => "css",
                "json" => "json",
                "toml" => "toml",
                _ => return Err(e),
            };
            let candidate = grammars_in_root.join(format!("{grammar_name}.semtree"));
            if candidate.exists() {
                let grammar = super::grammar_util::load_grammar(&candidate)?;
                Ok((candidate, grammar))
            } else {
                Err(e)
            }
        } else {
            Err(e)
        }
    });

    let (_, grammar) = grammar_result?;
    let mut session = ParseSession::new(grammar, ParserBackend::Auto);
    let result = session.parse(&params.text_document.text);
    documents.insert(
        key,
        DocumentState {
            session,
            version: params.text_document.version,
            errors: result.errors,
        },
    );
    Ok(())
}

fn change_document(
    documents: &mut HashMap<String, DocumentState>,
    params: lsp_types::DidChangeTextDocumentParams,
) -> super::Result {
    let key = uri_key(&params.text_document.uri);
    let Some(doc) = documents.get_mut(&key) else {
        return Ok(());
    };
    doc.version = params.text_document.version;

    for change in params.content_changes {
        let result = if let Some(range) = change.range {
            let start = offset_from_position(doc.session.source(), range.start);
            let end = offset_from_position(doc.session.source(), range.end);
            doc.session.edit(start, end, &change.text)
        } else {
            doc.session.parse(&change.text)
        };
        doc.errors = result.errors;
    }
    Ok(())
}

fn offset_from_position(source: &str, pos: Position) -> u32 {
    let mut offset = 0u32;
    for (i, line) in source.lines().enumerate() {
        if i == pos.line as usize {
            return offset + pos.character.min(line.len() as u32);
        }
        offset += line.len() as u32 + 1;
    }
    source.len() as u32
}

fn publish_diagnostics(
    connection: &Connection,
    documents: &HashMap<String, DocumentState>,
    key: &str,
    uri: &Uri,
) -> super::Result {
    let Some(doc) = documents.get(key) else {
        return Ok(());
    };
    let diags: Vec<Diagnostic> = doc
        .errors
        .iter()
        .enumerate()
        .map(|(i, msg)| Diagnostic {
            range: Range {
                start: Position {
                    line: i as u32,
                    character: 0,
                },
                end: Position {
                    line: i as u32,
                    character: 80,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            message: msg.clone(),
            source: Some("semtree".into()),
            ..Default::default()
        })
        .collect();

    connection.sender.send(Message::Notification(Notification {
        method: "textDocument/publishDiagnostics".into(),
        params: serde_json::to_value(lsp_types::PublishDiagnosticsParams {
            uri: uri.clone(),
            version: Some(doc.version),
            diagnostics: diags,
        })?,
    }))?;
    Ok(())
}

#[allow(deprecated)]
fn symbols_for_doc(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let model = SemanticModel::analyze(root);
    ide_symbols(root, &model)
        .into_iter()
        .map(|sym| {
            let start = byte_to_position(doc.session.source(), u32::from(sym.range.start()));
            let end = byte_to_position(doc.session.source(), u32::from(sym.range.end()));
            DocumentSymbol {
                name: sym.name.to_string(),
                detail: None,
                kind: match sym.kind {
                    semtree_semantic::SymbolKind::Function => SymbolKind::FUNCTION,
                    semtree_semantic::SymbolKind::Struct => SymbolKind::CLASS,
                    semtree_semantic::SymbolKind::Variable => SymbolKind::VARIABLE,
                    semtree_semantic::SymbolKind::Parameter => SymbolKind::VARIABLE,
                    semtree_semantic::SymbolKind::Module => SymbolKind::NAMESPACE,
                    _ => SymbolKind::VARIABLE,
                },
                tags: None,
                deprecated: None,
                range: Range { start, end },
                selection_range: Range { start, end },
                children: None,
            }
        })
        .collect()
}

fn uri_to_path(uri: &Uri) -> Result<PathBuf, String> {
    let s = uri.as_str();
    let path = s.strip_prefix("file://").unwrap_or(s);
    Ok(PathBuf::from(path))
}

fn byte_to_position(source: &str, byte: u32) -> Position {
    let mut offset = 0u32;
    for (i, line) in source.lines().enumerate() {
        let line_len = line.len() as u32 + 1;
        if offset + line_len > byte {
            return Position {
                line: i as u32,
                character: byte - offset,
            };
        }
        offset += line_len;
    }
    Position {
        line: source.lines().count().saturating_sub(1) as u32,
        character: 0,
    }
}

fn position_to_offset(source: &str, pos: Position) -> u32 {
    let mut offset = 0u32;
    for (i, line) in source.lines().enumerate() {
        if i == pos.line as usize {
            return offset + pos.character.min(line.len() as u32);
        }
        offset += line.len() as u32 + 1;
    }
    source.len() as u32
}

fn completion_for_doc(doc: &DocumentState, position: Position) -> Vec<LspCompletionItem> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let model = SemanticModel::analyze(root);
    let offset = position_to_offset(doc.session.source(), position);
    ide_complete(root, &model, offset)
        .into_iter()
        .map(|item| {
            use semtree_ide::CompletionKind;
            let kind = match item.kind {
                CompletionKind::Keyword => Some(lsp_types::CompletionItemKind::KEYWORD),
                CompletionKind::Function => Some(lsp_types::CompletionItemKind::FUNCTION),
                CompletionKind::Variable => Some(lsp_types::CompletionItemKind::VARIABLE),
                CompletionKind::Snippet => Some(lsp_types::CompletionItemKind::SNIPPET),
            };
            LspCompletionItem {
                label: item.label.to_string(),
                kind,
                detail: item.detail.map(|d| d.to_string()),
                ..Default::default()
            }
        })
        .collect()
}

fn hover_for_doc(doc: &DocumentState, position: Position) -> Option<Hover> {
    let root = doc.session.syntax()?;
    let model = SemanticModel::analyze(root);
    let offset = position_to_offset(doc.session.source(), position);
    let info = ide_hover(root, &model, offset)?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("**{}** — {}", info.name, info.kind),
        }),
        range: Some(Range {
            start: byte_to_position(doc.session.source(), u32::from(info.range.start())),
            end: byte_to_position(doc.session.source(), u32::from(info.range.end())),
        }),
    })
}

fn semantic_tokens_for_doc(doc: &DocumentState) -> Vec<LspSemanticToken> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let model = SemanticModel::analyze(root);
    let tokens = ide_classify(root, &model);

    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for tok in &tokens {
        let start = byte_to_position(doc.session.source(), u32::from(tok.range.start()));
        let end = byte_to_position(doc.session.source(), u32::from(tok.range.end()));
        let length = if start.line == end.line {
            end.character - start.character
        } else {
            u32::from(tok.range.end()) - u32::from(tok.range.start())
        };

        let delta_line = start.line - prev_line;
        let delta_start = if delta_line == 0 {
            start.character - prev_start
        } else {
            start.character
        };

        use semtree_ide::SemanticTokenType;
        let token_type = match tok.token_type {
            SemanticTokenType::Keyword => 0,
            SemanticTokenType::Type => 1,
            SemanticTokenType::Function => 2,
            SemanticTokenType::Variable => 3,
            SemanticTokenType::String => 4,
            SemanticTokenType::Number => 5,
            SemanticTokenType::Comment => 6,
            SemanticTokenType::Operator => 7,
            SemanticTokenType::Parameter => 8,
            SemanticTokenType::Property => 9,
            SemanticTokenType::Enum => 10,
        };

        result.push(LspSemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = start.line;
        prev_start = start.character;
    }

    result
}

fn format_doc(doc: &DocumentState) -> Vec<TextEdit> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let formatted = semtree_format::Formatter::with_defaults().format(root);
    let source = doc.session.source();
    if formatted == source {
        return vec![];
    }
    // Single whole-document edit
    let last_line = source.lines().count().saturating_sub(1) as u32;
    let last_col = source.lines().last().map(|l| l.len() as u32).unwrap_or(0);
    vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: last_line,
                character: last_col,
            },
        },
        new_text: formatted,
    }]
}

fn folding_for_doc(doc: &DocumentState) -> Vec<LspFoldingRange> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    ide_fold(root)
        .into_iter()
        .map(|r| {
            let start = byte_to_position(doc.session.source(), u32::from(r.range.start()));
            let end = byte_to_position(doc.session.source(), u32::from(r.range.end()));
            LspFoldingRange {
                start_line: start.line,
                start_character: Some(start.character),
                end_line: end.line,
                end_character: Some(end.character),
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            }
        })
        .collect()
}
