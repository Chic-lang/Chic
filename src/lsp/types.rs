//! Minimal LSP types used by `impact-lsp`.
//!
//! These are intentionally small (only what Chic tooling uses today) and are a
//! Rust-side mirror of the Chic-native `Std.Compiler.Lsp.Types` package.

#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};

pub type Uri = String;

pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const SHUTDOWN: &str = "shutdown";
    pub const EXIT: &str = "exit";

    pub const DID_OPEN: &str = "textDocument/didOpen";
    pub const DID_CHANGE: &str = "textDocument/didChange";
    pub const DID_CLOSE: &str = "textDocument/didClose";

    pub const HOVER: &str = "textDocument/hover";
    pub const DEFINITION: &str = "textDocument/definition";

    pub const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub capabilities: ServerCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_document_sync: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover_provider: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition_provider: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

impl Position {
    #[must_use]
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    #[must_use]
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub uri: Uri,
    pub range: Range,
}

impl Location {
    #[must_use]
    pub fn new(uri: Uri, range: Range) -> Self {
        Self { uri, range }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MarkupKind {
    #[serde(rename = "plaintext")]
    PlainText,
    #[serde(rename = "markdown")]
    Markdown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hover {
    pub contents: MarkupContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifier {
    pub uri: Uri,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionedTextDocumentIdentifier {
    pub uri: Uri,
    pub version: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: Uri,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DidOpenTextDocumentParams {
    pub text_document: TextDocumentItem,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DidCloseTextDocumentParams {
    pub text_document: TextDocumentIdentifier,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentContentChangeEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams {
    pub text_document: VersionedTextDocumentIdentifier,
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

pub type HoverParams = TextDocumentPositionParams;
pub type GotoDefinitionParams = TextDocumentPositionParams;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsParams {
    pub uri: Uri,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(i64),
    String(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_information: Vec<DiagnosticRelatedInformation>,
}
