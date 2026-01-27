//! Impact LSP server wiring and document analysis.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use lsp_server::{Connection, Message, Notification, Request as ServerRequest, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit,
    Notification as LspNotification,
};
use lsp_types::request::{GotoDefinition, HoverRequest, Request, Shutdown};
use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    GotoDefinitionResponse, Hover, HoverContents, HoverParams, InitializeResult, Location,
    MarkupContent, MarkupKind, NumberOrString, Position, PublishDiagnosticsParams, Range,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

use crate::chic_kind::ChicKind;
use crate::defines::DefineFlag;
use crate::diagnostics::{Diagnostic, FileCache, FileId, LineCol, Severity, Span};
use crate::driver::{CompilerDriver, FrontendReport};
use crate::frontend::lexer::{TokenKind, lex_with_file};
use crate::lint::{LintDiagnostic, LintLevel};
use crate::logging::LogLevel;
use crate::mir::MirFunction;
use crate::target::Target;

const METHOD_NOT_FOUND: i32 = -32601;

struct Document {
    text: String,
    version: i32,
    file_id: FileId,
}

impl Document {
    fn new(text: String, version: i32, file_id: FileId) -> Self {
        Self {
            text,
            version,
            file_id,
        }
    }

    fn apply_change(&mut self, params: &lsp_types::DidChangeTextDocumentParams) {
        self.version = params.text_document.version;
        for change in &params.content_changes {
            if change.range.is_none() {
                self.text = change.text.clone();
                continue;
            }
            if let Some(range) = change.range {
                let start = offset_at(&self.text, range.start);
                let end = offset_at(&self.text, range.end);
                if start <= end && end <= self.text.len() {
                    self.text.replace_range(start..end, &change.text);
                } else if start <= self.text.len() {
                    self.text
                        .replace_range(start..self.text.len(), &change.text);
                } else {
                    self.text.push_str(&change.text);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct SemanticSymbol {
    name: String,
    span: Option<Span>,
    signature: Option<String>,
}

#[derive(Default)]
struct DocumentAnalysis {
    files: FileCache,
    symbols: Vec<SemanticSymbol>,
}

#[derive(Default)]
struct DocumentStore {
    documents: HashMap<Uri, Document>,
    analysis: HashMap<Uri, DocumentAnalysis>,
    files: FileCache,
}

impl DocumentStore {
    fn open(&mut self, uri: Uri, text: String, version: i32) {
        let path = uri_path(&uri);
        let file_id = self.files.add_file(path, text.clone());
        let document = Document::new(text, version, file_id);
        self.documents.insert(uri.clone(), document);
        self.analysis.remove(&uri);
    }

    fn close(&mut self, uri: &Uri) {
        self.documents.remove(uri);
        self.analysis.remove(uri);
    }

    fn with_document_mut<F>(&mut self, uri: &Uri, op: F)
    where
        F: FnOnce(&mut Document),
    {
        if let Some(doc) = self.documents.get_mut(uri) {
            op(doc);
            self.files.update_source(doc.file_id, doc.text.clone());
        }
    }

    fn diagnostics(&mut self, uri: &Uri) -> Option<Vec<LspDiagnostic>> {
        let doc = self.documents.get(uri)?;
        let (diags, files, symbols) = pipeline_diagnostics_for(uri, doc);
        let converted: Vec<LspDiagnostic> = diags
            .into_iter()
            .map(|diag| convert_diagnostic(diag, &files))
            .collect();
        let analysis = DocumentAnalysis { files, symbols };
        self.analysis.insert(uri.clone(), analysis);
        Some(converted)
    }

    fn document(&self, uri: &Uri) -> Option<&Document> {
        self.documents.get(uri)
    }

    fn version(&self, uri: &Uri) -> Option<i32> {
        self.documents.get(uri).map(|doc| doc.version)
    }

    fn analysis(&self, uri: &Uri) -> Option<&DocumentAnalysis> {
        self.analysis.get(uri)
    }

    fn files(&self) -> &FileCache {
        &self.files
    }
}

fn offset_at(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    let mut lines = text.split_inclusive('\n');
    for _ in 0..position.line {
        if let Some(line) = lines.next() {
            offset = offset.saturating_add(line.len());
        } else {
            return text.len();
        }
    }
    let line = lines.next().unwrap_or("");
    let target_units = usize::try_from(position.character).unwrap_or(usize::MAX);
    let mut utf16_units = 0usize;
    for (byte_idx, ch) in line.char_indices() {
        if utf16_units >= target_units {
            return offset.saturating_add(byte_idx);
        }
        utf16_units = utf16_units.saturating_add(ch.len_utf16());
    }
    offset.saturating_add(line.len())
}

fn find_symbol<'a>(lexeme: &str, symbols: &'a [SemanticSymbol]) -> Option<&'a SemanticSymbol> {
    let trimmed = lexeme.trim_matches('`');
    symbols.iter().find(|symbol| {
        if symbol.name == trimmed {
            return true;
        }
        symbol
            .name
            .rsplit(|ch| ch == ':' || ch == '.')
            .find(|part| !part.is_empty())
            .map_or(false, |leaf| leaf == trimmed)
    })
}

fn format_signature(function: &MirFunction) -> String {
    let params = function
        .signature
        .params
        .iter()
        .enumerate()
        .map(|(idx, ty)| format!("_{}: {:?}", idx + 1, ty))
        .collect::<Vec<_>>()
        .join(", ");
    let async_prefix = if function.is_async { "async " } else { "" };
    format!(
        "{async_prefix}fn {}({}) -> {:?}",
        function.name, params, function.signature.ret
    )
}

fn collect_semantic_symbols(report: &FrontendReport) -> Vec<SemanticSymbol> {
    let mut symbols = Vec::new();
    for function in &report.mir_module.functions {
        if let Some(span) = function.span {
            symbols.push(SemanticSymbol {
                name: function.name.clone(),
                span: Some(span),
                signature: Some(format_signature(function)),
            });
        }
    }
    for testcase in &report.mir_module.test_cases {
        if let Some(span) = testcase.span {
            symbols.push(SemanticSymbol {
                name: testcase.name.clone(),
                span: Some(span),
                signature: Some(format!("test {}", testcase.qualified_name)),
            });
        } else if let Some(function) = report
            .mir_module
            .functions
            .get(testcase.function_index)
            .and_then(|func| func.span.map(|span| (func, span)))
        {
            symbols.push(SemanticSymbol {
                name: testcase.name.clone(),
                span: Some(function.1),
                signature: Some(format_signature(function.0)),
            });
        }
    }
    for static_var in &report.mir_module.statics {
        if let Some(span) = static_var.span {
            symbols.push(SemanticSymbol {
                name: static_var.qualified.clone(),
                span: Some(span),
                signature: Some(format!("static {:?}", static_var.ty)),
            });
        }
    }
    symbols
}

fn pipeline_diagnostics_for(
    uri: &Uri,
    doc: &Document,
) -> (Vec<Diagnostic>, FileCache, Vec<SemanticSymbol>) {
    let mut pre_files = FileCache::default();
    let original_path = uri_to_file_path(uri).unwrap_or_else(|| PathBuf::from(uri.as_str()));
    let _file_id = pre_files.add_file(original_path.clone(), doc.text.clone());

    let tempdir = tempfile::tempdir().expect("create temp dir for lsp analysis");
    let filename = uri_filename(uri).unwrap_or_else(|| PathBuf::from("main.ch"));
    let path = tempdir.path().join(filename);
    std::fs::write(&path, &doc.text).unwrap_or_default();

    let driver = CompilerDriver::new();
    let target = Target::host();
    let report = driver.check(
        &[path.clone()],
        &target,
        ChicKind::Executable,
        false,
        false,
        false,
        &Vec::<DefineFlag>::new(),
        LogLevel::Error,
    );

    match report {
        Ok(report) => {
            let symbols = collect_semantic_symbols(&report);
            let mut files = report.files;
            if let Some(id) = files.find_id_by_path(&path) {
                files.update_path(
                    id,
                    uri_to_file_path(uri).unwrap_or_else(|| PathBuf::from(uri.as_str())),
                );
            }
            let mut diags = Vec::new();
            for module in report.modules {
                diags.extend(module.parse.diagnostics);
            }
            diags.extend(report.type_diagnostics);
            diags.extend(
                report
                    .mir_lowering_diagnostics
                    .into_iter()
                    .map(|lower| Diagnostic::error(lower.message, lower.span)),
            );
            diags.extend(report.reachability_diagnostics);
            diags.extend(report.borrow_diagnostics);
            diags.extend(report.fallible_diagnostics);
            diags.extend(report.format_diagnostics);
            diags.extend(report.doc_diagnostics);
            diags.extend(report.lint_diagnostics.into_iter().map(lint_to_diagnostic));
            (diags, files, symbols)
        }
        Err(err) => {
            if let crate::error::Error::Parse(parse) = &err {
                return (parse.diagnostics().to_vec(), pre_files, Vec::new());
            }
            let mut diag =
                Diagnostic::error(format!("lsp analysis failed: {err}"), Some(Span::new(0, 0)));
            diag.severity = Severity::Error;
            (vec![diag], pre_files, Vec::new())
        }
    }
}

fn uri_path(uri: &Uri) -> PathBuf {
    uri_to_file_path(uri).unwrap_or_else(|| PathBuf::from(uri.as_str()))
}

fn uri_filename(uri: &Uri) -> Option<PathBuf> {
    let parsed = Url::parse(uri.as_str()).ok()?;
    parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .filter(|name| !name.is_empty())
        .map(PathBuf::from)
}

fn uri_to_file_path(uri: &Uri) -> Option<PathBuf> {
    let parsed = Url::parse(uri.as_str()).ok()?;
    parsed.to_file_path().ok()
}

fn file_path_to_uri(path: &Path) -> Option<Uri> {
    let url = Url::from_file_path(path).ok()?;
    Uri::from_str(url.as_str()).ok()
}

fn convert_diagnostic(diagnostic: Diagnostic, files: &FileCache) -> LspDiagnostic {
    let span = diagnostic
        .primary_label
        .as_ref()
        .map(|label| label.span)
        .or_else(|| diagnostic.secondary_labels.first().map(|label| label.span));
    let range = span
        .and_then(|s| span_to_range(s, files))
        .unwrap_or_else(empty_range);
    let related_information = diagnostic
        .secondary_labels
        .iter()
        .filter_map(|label| {
            let related_range = span_to_range(label.span, files)?;
            let uri = files
                .path(label.span.file_id)
                .and_then(|path| file_path_to_uri(path))?;
            Some(DiagnosticRelatedInformation {
                location: lsp_types::Location::new(uri, related_range),
                message: label.message.clone(),
            })
        })
        .collect();
    LspDiagnostic {
        range,
        severity: Some(severity_to_lsp(diagnostic.severity)),
        code: diagnostic
            .code
            .map(|code| NumberOrString::String(code.code)),
        code_description: None,
        source: Some(String::from("chic")),
        message: diagnostic.message,
        related_information: Some(related_information),
        tags: None,
        data: None,
    }
}

fn span_to_range(span: Span, files: &FileCache) -> Option<Range> {
    let start = files.line_col(span.file_id, span.start)?;
    let end = files.line_col(span.file_id, span.end)?;
    Some(Range {
        start: position_from_line_col(start),
        end: position_from_line_col(end),
    })
}

fn position_from_line_col(line_col: LineCol) -> Position {
    let line = u32::try_from(line_col.line.saturating_sub(1)).unwrap_or(u32::MAX);
    let character = u32::try_from(line_col.column.saturating_sub(1)).unwrap_or(u32::MAX);
    Position::new(line, character)
}

fn empty_range() -> Range {
    Range::new(Position::new(0, 0), Position::new(0, 0))
}

fn lint_to_diagnostic(lint: LintDiagnostic) -> Diagnostic {
    let severity = match lint.level {
        LintLevel::Error => Severity::Error,
        LintLevel::Warn => Severity::Warning,
        LintLevel::Allow => Severity::Note,
    };
    let mut diagnostic = Diagnostic::error(lint.message.clone(), lint.span);
    diagnostic.severity = severity;
    diagnostic.code = Some(crate::diagnostics::DiagnosticCode::new(
        lint.code(),
        Some("lint".into()),
    ));
    diagnostic
}

fn token_at(
    doc: &Document,
    position: Position,
    files: &FileCache,
    uri: &Uri,
) -> Option<(Range, Location, String)> {
    let lexed = lex_with_file(&doc.text, doc.file_id);
    let offset = offset_at(&doc.text, position).min(doc.text.len());
    let token = lexed
        .tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment
            )
        })
        .find(|token| token.span.start <= offset && offset < token.span.end)?;
    let range = span_to_range(token.span, files)?;
    let loc_uri = files
        .path(doc.file_id)
        .and_then(|path| file_path_to_uri(path))
        .unwrap_or_else(|| uri.clone());
    let location = Location::new(loc_uri, range);
    Some((range, location, token.lexeme.clone()))
}

fn hover_at(params: &HoverParams, store: &DocumentStore) -> Option<Hover> {
    let doc = store.document(&params.text_document_position_params.text_document.uri)?;
    let uri = &params.text_document_position_params.text_document.uri;
    let files = store.files();
    let (range, _, lexeme) = token_at(
        doc,
        params.text_document_position_params.position,
        files,
        uri,
    )?;
    if let Some(analysis) = store.analysis(uri) {
        if let Some(symbol) = find_symbol(&lexeme, &analysis.symbols) {
            if let Some(span) = symbol.span {
                if let Some(sym_range) = span_to_range(span, &analysis.files) {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::PlainText,
                            value: symbol
                                .signature
                                .clone()
                                .unwrap_or_else(|| symbol.name.clone()),
                        }),
                        range: Some(sym_range),
                    });
                }
            }
        }
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: lexeme,
        }),
        range: Some(range),
    })
}

fn definition_at(
    params: &lsp_types::GotoDefinitionParams,
    store: &DocumentStore,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let doc = store.document(uri)?;
    let files = store.files();
    let (_, _, lexeme) = token_at(
        doc,
        params.text_document_position_params.position,
        files,
        uri,
    )?;
    if let Some(analysis) = store.analysis(uri) {
        if let Some(symbol) = find_symbol(&lexeme, &analysis.symbols) {
            if let Some(span) = symbol.span {
                if let Some(range) = span_to_range(span, &analysis.files) {
                    let loc_uri = analysis
                        .files
                        .path(span.file_id)
                        .and_then(|path| file_path_to_uri(path))
                        .unwrap_or_else(|| uri.clone());
                    let location = Location::new(loc_uri, range);
                    return Some(GotoDefinitionResponse::Scalar(location));
                }
            }
        }
    }
    None
}

fn severity_to_lsp(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::HINT,
        Severity::Help => DiagnosticSeverity::INFORMATION,
    }
}

fn parse_notification<N>(notification: Notification) -> Option<N::Params>
where
    N: LspNotification,
    N::Params: DeserializeOwned,
{
    serde_json::from_value(notification.params).ok()
}

fn parse_request<R>(request: &ServerRequest) -> Option<R::Params>
where
    R: Request,
    R::Params: DeserializeOwned,
{
    serde_json::from_value(request.params.clone()).ok()
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &Uri,
    version: Option<i32>,
    diagnostics: Vec<LspDiagnostic>,
) {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version,
    };
    let notification = Notification::new(
        lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
        params,
    );
    let _ = connection.sender.send(Message::Notification(notification));
}

fn handle_request(connection: &Connection, store: &DocumentStore, request: ServerRequest) -> bool {
    match request.method.as_str() {
        Shutdown::METHOD => {
            let response = Response {
                id: request.id,
                result: Some(Value::Null),
                error: None,
            };
            let _ = connection.sender.send(Message::Response(response));
            true
        }
        HoverRequest::METHOD => {
            let params = parse_request::<HoverRequest>(&request);
            let result = params
                .as_ref()
                .and_then(|params| hover_at(params, store))
                .map(|hover| serde_json::to_value(hover).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            let response = Response {
                id: request.id,
                result: Some(result),
                error: None,
            };
            let _ = connection.sender.send(Message::Response(response));
            false
        }
        GotoDefinition::METHOD => {
            let params = parse_request::<GotoDefinition>(&request);
            let result = params
                .as_ref()
                .and_then(|params| definition_at(params, store))
                .map(|loc| serde_json::to_value(loc).unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            let response = Response {
                id: request.id,
                result: Some(result),
                error: None,
            };
            let _ = connection.sender.send(Message::Response(response));
            false
        }
        _ => {
            let response = Response::new_err(
                request.id,
                METHOD_NOT_FOUND,
                format!("unsupported request: {}", request.method),
            );
            let _ = connection.sender.send(Message::Response(response));
            false
        }
    }
}

fn handle_notification(
    connection: &Connection,
    store: &mut DocumentStore,
    notification: Notification,
) -> bool {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Some(params) = parse_notification::<DidOpenTextDocument>(notification) {
                let text = params.text_document.text;
                let version = params.text_document.version;
                store.open(params.text_document.uri.clone(), text, version);
                let uri = params.text_document.uri;
                if let Some(diags) = store.diagnostics(&uri) {
                    let version = store.version(&uri);
                    publish_diagnostics(connection, &uri, version, diags);
                }
            }
            false
        }
        DidChangeTextDocument::METHOD => {
            if let Some(params) = parse_notification::<DidChangeTextDocument>(notification) {
                let uri = params.text_document.uri.clone();
                store.with_document_mut(&uri, |doc| {
                    doc.apply_change(&params);
                });
                if let Some(diags) = store.diagnostics(&uri) {
                    let version = store.version(&uri);
                    publish_diagnostics(connection, &uri, version, diags);
                }
            }
            false
        }
        DidCloseTextDocument::METHOD => {
            if let Some(params) = parse_notification::<DidCloseTextDocument>(notification) {
                store.close(&params.text_document.uri);
                let params = PublishDiagnosticsParams {
                    uri: params.text_document.uri,
                    diagnostics: Vec::new(),
                    version: None,
                };
                let notif = Notification::new(
                    lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                    params,
                );
                let _ = connection.sender.send(Message::Notification(notif));
            }
            false
        }
        Exit::METHOD => true,
        _ => false,
    }
}

/// Run the Impact LSP server using the provided connection.
pub fn run(connection: Connection, _initialization: InitializeResult) {
    let mut store = DocumentStore::default();
    let mut shutdown_requested = false;
    for message in &connection.receiver {
        match message {
            Message::Request(req) => {
                shutdown_requested =
                    handle_request(&connection, &store, req.clone()) || shutdown_requested;
            }
            Message::Notification(notification) => {
                let should_exit =
                    handle_notification(&connection, &mut store, notification.clone());
                if should_exit {
                    break;
                }
            }
            Message::Response(_) => {}
        }
        if shutdown_requested {
            break;
        }
    }
}

/// Default server capabilities for Impact LSP.
#[must_use]
pub fn capabilities() -> InitializeResult {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        )),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        ..ServerCapabilities::default()
    };
    InitializeResult {
        capabilities,
        server_info: Some(ServerInfo {
            name: String::from("impact-lsp"),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    }
}
