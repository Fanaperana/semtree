//! LSP server: incremental parsing, diagnostics, document symbols,
//! completion, hover, go-to-definition, find-references, semantic tokens,
//! formatting, and folding.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CompletionItem as LspCompletionItem, CompletionOptions,
    CompletionParams, CompletionResponse, Diagnostic, DiagnosticSeverity, DocumentFormattingParams,
    DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams, DocumentSymbol,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRange as LspFoldingRange, FoldingRangeKind,
    FoldingRangeParams, FoldingRangeProviderCapability, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability, Location,
    MarkupContent, MarkupKind, OneOf, Position, PrepareRenameResponse, Range, ReferenceParams,
    RenameOptions, RenameParams, SelectionRange, SelectionRangeParams,
    SelectionRangeProviderCapability, SemanticToken, SemanticTokenModifier, SemanticTokenType,
    SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensParams, SemanticTokensResult, SemanticTokensServerCapabilities,
    ServerCapabilities, SymbolKind, TextDocumentPositionParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit, Uri, WorkspaceEdit,
};
use semtree_core::SyntaxKind;
use semtree_ide::{
    classify_tokens as ide_classify_tokens, complete_at as ide_complete,
    document_symbols as ide_symbols, find_references as ide_references, folding_ranges as ide_fold,
    goto_definition as ide_goto_def, hover_info as ide_hover,
};
use text_size::TextSize;
use semtree_runtime::{ParseSession, ParserBackend};
use semtree_semantic::SemanticModel;

use super::grammar_util::resolve_grammar;

struct DocumentState {
    session: ParseSession,
    version: i32,
    errors: Vec<String>,
}

pub fn lsp(exe_dir: PathBuf, tcp: Option<String>) -> super::Result {
    let (connection, io_threads) = match tcp {
        Some(addr) => {
            eprintln!("semtree-lsp: listening on tcp {addr}");
            Connection::listen(addr.as_str())?
        }
        None => Connection::stdio(),
    };
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
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        document_formatting_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        semantic_tokens_provider: Some(
            SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: semantic_token_type_legend(),
                    token_modifiers: semantic_token_modifier_legend(),
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: Some(false),
                ..Default::default()
            }),
        ),
        ..Default::default()
    }
}

/// Ordered list of semantic token types; the index into this list is what the
/// LSP wire protocol uses. Must match `ide_token_type_index`.
fn semantic_token_type_legend() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::TYPE,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PARAMETER,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::ENUM,
        SemanticTokenType::STRING,
        SemanticTokenType::NUMBER,
        SemanticTokenType::COMMENT,
        SemanticTokenType::OPERATOR,
    ]
}

/// Ordered list of semantic token modifiers (index = bit position). Must match
/// `ide_modifier_bit`.
fn semantic_token_modifier_legend() -> Vec<SemanticTokenModifier> {
    vec![
        SemanticTokenModifier::DECLARATION,
        SemanticTokenModifier::DEFINITION,
        SemanticTokenModifier::READONLY,
    ]
}

fn ide_token_type_index(t: semtree_ide::SemanticTokenType) -> u32 {
    use semtree_ide::SemanticTokenType as T;
    match t {
        T::Keyword => 0,
        T::Type => 1,
        T::Function => 2,
        T::Variable => 3,
        T::Parameter => 4,
        T::Property => 5,
        T::Enum => 6,
        T::String => 7,
        T::Number => 8,
        T::Comment => 9,
        T::Operator => 10,
    }
}

fn ide_modifier_bit(m: semtree_ide::SemanticTokenModifier) -> u32 {
    use semtree_ide::SemanticTokenModifier as M;
    match m {
        M::Declaration => 1 << 0,
        M::Definition => 1 << 1,
        M::Readonly => 1 << 2,
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
        "textDocument/documentHighlight" => {
            let (id, params): (_, DocumentHighlightParams) =
                match req.extract("textDocument/documentHighlight") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let uri = params
                .text_document_position_params
                .text_document
                .uri
                .clone();
            let key = uri_key(&uri);
            let hls: Vec<DocumentHighlight> = documents
                .get(&key)
                .map(|doc| {
                    let offset = position_to_offset(
                        doc.session.source(),
                        params.text_document_position_params.position,
                    );
                    let Some(root) = doc.session.syntax() else {
                        return vec![];
                    };
                    let model = SemanticModel::analyze(root);
                    ide_references(root, &model, offset)
                        .into_iter()
                        .map(|range| DocumentHighlight {
                            range: Range {
                                start: byte_to_position(
                                    doc.session.source(),
                                    u32::from(range.start()),
                                ),
                                end: byte_to_position(doc.session.source(), u32::from(range.end())),
                            },
                            kind: Some(DocumentHighlightKind::TEXT),
                        })
                        .collect()
                })
                .unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, hls)))?;
        }
        "textDocument/semanticTokens/full" => {
            let (id, params): (_, SemanticTokensParams) =
                match req.extract("textDocument/semanticTokens/full") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let data = documents
                .get(&key)
                .map(semantic_tokens_for_doc)
                .unwrap_or_default();
            connection.sender.send(Message::Response(Response::new_ok(
                id,
                SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data,
                }),
            )))?;
        }
        "textDocument/selectionRange" => {
            let (id, params): (_, SelectionRangeParams) =
                match req.extract("textDocument/selectionRange") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let ranges = documents
                .get(&key)
                .map(|doc| selection_ranges_for_doc(doc, &params.positions))
                .unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, ranges)))?;
        }
        "textDocument/codeAction" => {
            let (id, params): (_, CodeActionParams) = match req.extract("textDocument/codeAction") {
                Ok(v) => v,
                Err(e) => return Err(format!("{e:?}").into()),
            };
            let uri = params.text_document.uri.clone();
            let key = uri_key(&uri);
            let actions = documents
                .get(&key)
                .map(|doc| code_actions_for_doc(doc, &uri, params.range))
                .unwrap_or_default();
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, actions)))?;
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
        "textDocument/prepareRename" => {
            let (id, params): (_, TextDocumentPositionParams) =
                match req.extract("textDocument/prepareRename") {
                    Ok(v) => v,
                    Err(e) => return Err(format!("{e:?}").into()),
                };
            let key = uri_key(&params.text_document.uri);
            let resp = documents
                .get(&key)
                .and_then(|doc| {
                    let offset = position_to_offset(doc.session.source(), params.position);
                    prepare_rename_range(doc, offset)
                })
                .map(PrepareRenameResponse::Range);
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, resp)))?;
        }
        "textDocument/rename" => {
            let (id, params): (_, RenameParams) = match req.extract("textDocument/rename") {
                Ok(v) => v,
                Err(e) => return Err(format!("{e:?}").into()),
            };
            let uri = params.text_document_position.text_document.uri.clone();
            let key = uri_key(&uri);
            let ws = documents.get(&key).map(|doc| {
                let offset = position_to_offset(
                    doc.session.source(),
                    params.text_document_position.position,
                );
                let edits = rename_edits_for_doc(doc, offset, &params.new_name);
                let mut changes = HashMap::new();
                changes.insert(uri.clone(), edits);
                WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }
            });
            connection
                .sender
                .send(Message::Response(Response::new_ok(id, ws)))?;
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
        "workspace/didChangeConfiguration" => {
            let params: lsp_types::DidChangeConfigurationParams = notif
                .extract("workspace/didChangeConfiguration")
                .map_err(|e| format!("{e:?}"))?;
            // Acknowledge the client's configuration. The `semtree` section may
            // carry `serverPath` / `trace.server`; grammar resolution stays
            // workspace-relative, so we log the change rather than silently
            // dropping the notification.
            if let Some(section) = params.settings.get("semtree") {
                eprintln!("semtree-lsp: configuration updated: {section}");
            } else {
                eprintln!("semtree-lsp: configuration updated");
            }
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
    let source = doc.session.source();

    // Heuristic: if the grammar produces many errors relative to file size,
    // the grammar doesn't fully cover the language. Only show diagnostics
    // when there are few errors (likely real syntax mistakes by the user).
    let line_count = source.lines().count().max(1);
    let error_density = doc.errors.len() as f64 / line_count as f64;

    // If more than 10% of lines have errors, suppress all diagnostics —
    // these are grammar coverage gaps, not user mistakes.
    let mut diags: Vec<Diagnostic> = if error_density > 0.1 {
        vec![]
    } else {
        doc.errors
            .iter()
            .filter_map(|msg| {
                let range = parse_error_range(msg, source)?;
                let display_msg = msg
                    .find(": ")
                    .map(|i| &msg[i + 2..])
                    .unwrap_or(msg)
                    .to_string();
                Some(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: display_msg,
                    source: Some("semtree".into()),
                    ..Default::default()
                })
            })
            .collect()
    };

    // On a well-covered parse (few parse errors), also surface lint diagnostics.
    if error_density <= 0.1 {
        if let Some(root) = doc.session.syntax() {
            let model = SemanticModel::analyze(root);
            let lints = semtree_lint::LintEngine::with_defaults().lint(root, &model);
            for d in lints.diagnostics {
                diags.push(Diagnostic {
                    range: Range {
                        start: byte_to_position(source, u32::from(d.range.start())),
                        end: byte_to_position(source, u32::from(d.range.end())),
                    },
                    severity: Some(match d.severity {
                        semtree_lint::LintSeverity::Error => DiagnosticSeverity::ERROR,
                        semtree_lint::LintSeverity::Warning => DiagnosticSeverity::WARNING,
                        semtree_lint::LintSeverity::Info => DiagnosticSeverity::INFORMATION,
                    }),
                    message: d.message,
                    source: Some(format!("semtree({})", d.rule)),
                    ..Default::default()
                });
            }
        }
    }

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

/// Parse the byte range from an error string like "error at 42..50: message"
/// and convert to an LSP Range using line/column positions.
fn parse_error_range(msg: &str, source: &str) -> Option<Range> {
    // Format: "error at START..END: ..."
    let after_at = msg.strip_prefix("error at ")?;
    let dots = after_at.find("..")?;
    let start_byte: usize = after_at[..dots].parse().ok()?;
    let rest = &after_at[dots + 2..];
    let colon = rest.find(':')?;
    let end_byte: usize = rest[..colon].parse().ok()?;

    let start_pos = byte_offset_to_position(source, start_byte);
    let end_pos = byte_offset_to_position(source, end_byte);
    Some(Range {
        start: start_pos,
        end: end_pos,
    })
}

fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() as u32;
    let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = (offset - last_newline) as u32;
    Position { line, character }
}

#[allow(deprecated)]
/// Syntax-aware selection ranges: for each position, a chain from the leaf
/// token outward through its ancestor nodes (used by editors' expand-selection).
fn selection_ranges_for_doc(doc: &DocumentState, positions: &[Position]) -> Vec<SelectionRange> {
    let src = doc.session.source();
    let Some(root) = doc.session.syntax() else {
        return positions
            .iter()
            .map(|p| SelectionRange {
                range: Range { start: *p, end: *p },
                parent: None,
            })
            .collect();
    };

    positions
        .iter()
        .map(|pos| {
            let offset = position_to_offset(src, *pos);
            let mut ranges: Vec<text_size::TextRange> = Vec::new();
            if let Some(tok) = root.token_at_offset(TextSize::new(offset)) {
                ranges.push(tok.text_range());
                if let Some(parent) = tok.parent() {
                    for anc in parent.ancestors() {
                        let r = anc.text_range();
                        if ranges.last() != Some(&r) {
                            ranges.push(r);
                        }
                    }
                }
            }
            if ranges.is_empty() {
                return SelectionRange {
                    range: Range { start: *pos, end: *pos },
                    parent: None,
                };
            }
            // Build the nested chain from outermost (root) inward.
            let mut sr: Option<Box<SelectionRange>> = None;
            for r in ranges.iter().rev() {
                sr = Some(Box::new(SelectionRange {
                    range: Range {
                        start: byte_to_position(src, u32::from(r.start())),
                        end: byte_to_position(src, u32::from(r.end())),
                    },
                    parent: sr,
                }));
            }
            *sr.unwrap()
        })
        .collect()
}

/// Offer refactorings as code actions: extract-variable (on a non-empty
/// selection) and inline-variable (on a variable identifier).
fn code_actions_for_doc(doc: &DocumentState, uri: &Uri, range: Range) -> Vec<CodeActionOrCommand> {
    let src = doc.session.source();
    let mut actions = Vec::new();

    let start = position_to_offset(src, range.start);
    let end = position_to_offset(src, range.end);

    // Extract variable: requires a non-empty selection.
    if end > start {
        let selection = text_size::TextRange::new(TextSize::new(start), TextSize::new(end));
        if let Some(extraction) = semtree_refactor::extract_variable(src, selection) {
            let edits: Vec<TextEdit> = extraction
                .edits
                .into_iter()
                .map(|e| TextEdit {
                    range: Range {
                        start: byte_to_position(src, u32::from(e.range.start())),
                        end: byte_to_position(src, u32::from(e.range.end())),
                    },
                    new_text: e.new_text,
                })
                .collect();
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: "Extract variable".into(),
                kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                edit: Some(workspace_edit(uri, edits)),
                ..Default::default()
            }));
        }
    }

    // Inline variable: cursor on a variable identifier.
    if let Some(root) = doc.session.syntax() {
        let model = SemanticModel::analyze(root);
        if let Some(refactor_edits) = semtree_refactor::inline_variable(root, &model, start) {
            let edits: Vec<TextEdit> = refactor_edits
                .into_iter()
                .map(|e| TextEdit {
                    range: Range {
                        start: byte_to_position(src, u32::from(e.range.start())),
                        end: byte_to_position(src, u32::from(e.range.end())),
                    },
                    new_text: e.new_text,
                })
                .collect();
            if !edits.is_empty() {
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Inline variable".into(),
                    kind: Some(CodeActionKind::REFACTOR_INLINE),
                    edit: Some(workspace_edit(uri, edits)),
                    ..Default::default()
                }));
            }
        }
    }

    actions
}

/// Wrap a list of edits into a single-file `WorkspaceEdit`.
fn workspace_edit(uri: &Uri, edits: Vec<TextEdit>) -> WorkspaceEdit {
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);
    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

/// The identifier range at `offset`, if a rename can start there.
fn prepare_rename_range(doc: &DocumentState, offset: u32) -> Option<Range> {
    let root = doc.session.syntax()?;
    let tok = root.token_at_offset(TextSize::new(offset))?;
    if tok.kind() != SyntaxKind::IDENT {
        return None;
    }
    let r = tok.text_range();
    let src = doc.session.source();
    Some(Range {
        start: byte_to_position(src, u32::from(r.start())),
        end: byte_to_position(src, u32::from(r.end())),
    })
}

/// Compute the LSP text edits to rename the symbol at `offset` to `new_name`.
fn rename_edits_for_doc(doc: &DocumentState, offset: u32, new_name: &str) -> Vec<TextEdit> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let model = SemanticModel::analyze(root);
    let src = doc.session.source();
    semtree_refactor::rename_symbol(root, &model, offset, new_name)
        .into_iter()
        .map(|e| TextEdit {
            range: Range {
                start: byte_to_position(src, u32::from(e.range.start())),
                end: byte_to_position(src, u32::from(e.range.end())),
            },
            new_text: e.new_text,
        })
        .collect()
}

/// Build LSP-encoded semantic tokens (delta-encoded per the spec) for a document.
fn semantic_tokens_for_doc(doc: &DocumentState) -> Vec<SemanticToken> {
    let Some(root) = doc.session.syntax() else {
        return vec![];
    };
    let model = SemanticModel::analyze(root);
    let mut classified = ide_classify_tokens(root, &model);
    classified.sort_by_key(|t| t.range.start());

    let src = doc.session.source();
    let mut data = Vec::with_capacity(classified.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;
    for t in classified {
        let pos = byte_to_position(src, u32::from(t.range.start()));
        let end = byte_to_position(src, u32::from(t.range.end()));
        // LSP semantic tokens must be single-line; skip multi-line tokens.
        if end.line != pos.line {
            continue;
        }
        let length = end.character.saturating_sub(pos.character);
        if length == 0 {
            continue;
        }
        let delta_line = pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            pos.character - prev_start
        } else {
            pos.character
        };
        let mut modifiers = 0u32;
        for m in &t.modifiers {
            modifiers |= ide_modifier_bit(*m);
        }
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ide_token_type_index(t.token_type),
            token_modifiers_bitset: modifiers,
        });
        prev_line = pos.line;
        prev_start = pos.character;
    }
    data
}

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
