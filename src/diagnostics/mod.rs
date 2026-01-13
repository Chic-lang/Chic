//! Shared diagnostics model and formatting utilities for CLI/LSP/test consumers.

mod files;
mod formatter;

use blake3::Hasher;
pub use files::{FileCache, FileId, LineCol, SourceFile};
pub use formatter::{
    ColorMode, ErrorFormat, FormatOptions, JSON_SCHEMA_VERSION, format_diagnostics,
};
use serde::Serialize;
use std::fmt;

/// Span into a source file (byte offsets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            file_id: FileId::UNKNOWN,
            start,
            end,
        }
    }

    #[must_use]
    pub fn in_file(file_id: FileId, start: usize, end: usize) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    #[must_use]
    pub fn with_file(self, file_id: FileId) -> Self {
        Self { file_id, ..self }
    }
}

/// Severity level of a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }

    #[must_use]
    pub fn is_error(self) -> bool {
        matches!(self, Severity::Error)
    }
}

/// Structured identifier for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiagnosticCode {
    pub code: String,
    pub category: Option<String>,
}

impl DiagnosticCode {
    #[must_use]
    pub fn new(code: impl Into<String>, category: Option<String>) -> Self {
        Self {
            code: code.into(),
            category,
        }
    }
}

/// Highlight for a particular span within the diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub message: String,
    pub is_primary: bool,
}

impl Label {
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: true,
        }
    }

    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            is_primary: false,
        }
    }
}

/// Fix-it suggestion for the developer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    pub span: Option<Span>,
    pub replacement: Option<String>,
}

impl Suggestion {
    #[must_use]
    pub fn new(
        message: impl Into<String>,
        span: Option<Span>,
        replacement: Option<String>,
    ) -> Self {
        Self {
            message: message.into(),
            span,
            replacement,
        }
    }
}

/// Rich diagnostic entry with optional labels, notes, and suggestions.
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<DiagnosticCode>,
    pub message: String,
    pub primary_label: Option<Label>,
    pub secondary_labels: Vec<Label>,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(Severity::Error, message, span)
    }

    #[must_use]
    pub fn warning(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(Severity::Warning, message, span)
    }

    #[must_use]
    pub fn note(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(Severity::Note, message, span)
    }

    #[must_use]
    pub fn help(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(Severity::Help, message, span)
    }

    #[must_use]
    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    #[must_use]
    pub fn with_primary_label(mut self, message: impl Into<String>) -> Self {
        if let Some(label) = self.primary_label.take() {
            self.primary_label = Some(Label::primary(label.span, message));
        }
        self
    }

    #[must_use]
    pub fn with_secondary(mut self, label: Label) -> Self {
        self.secondary_labels.push(label);
        self
    }

    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn add_suggestion(&mut self, suggestion: Suggestion) {
        self.suggestions.push(suggestion);
    }

    #[must_use]
    fn new(severity: Severity, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            severity,
            code: None,
            message: message.into(),
            primary_label: span.map(|span| Label::primary(span, String::new())),
            secondary_labels: Vec::new(),
            notes: Vec::new(),
            suggestions: Vec::new(),
        }
    }
}

/// Collection helper used to accumulate diagnostics during compilation.
#[derive(Debug)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
    namespace: String,
}

impl DiagnosticSink {
    #[must_use]
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            diagnostics: Vec::new(),
            namespace: namespace.into(),
        }
    }

    pub fn push(&mut self, mut diagnostic: Diagnostic) {
        if diagnostic.code.is_none() {
            diagnostic.code = Some(self.auto_code(&diagnostic));
        }
        self.diagnostics.push(diagnostic);
    }

    pub fn push_error(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.push(Diagnostic::error(message, span));
    }

    pub fn push_warning(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.push(Diagnostic::warning(message, span));
    }

    pub fn push_note(&mut self, message: impl Into<String>, span: Option<Span>) {
        self.push(Diagnostic::note(message, span));
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn auto_code(&self, diagnostic: &Diagnostic) -> DiagnosticCode {
        let mut hasher = Hasher::new();
        hasher.update(self.namespace.as_bytes());
        hasher.update(diagnostic.message.as_bytes());
        if let Some(label) = diagnostic.primary_label.as_ref() {
            hasher.update(&label.span.start.to_le_bytes());
            hasher.update(&label.span.end.to_le_bytes());
        }
        let hash = hasher.finalize();
        let raw = u32::from_le_bytes(hash.as_bytes()[..4].try_into().unwrap());
        let suffix = raw % 100_000;
        let code = format!("{}{:05}", self.namespace.to_ascii_uppercase(), suffix);
        DiagnosticCode::new(code, Some(self.namespace.clone()))
    }
}

impl Default for DiagnosticSink {
    fn default() -> Self {
        Self::new("GEN")
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self
            .code
            .as_ref()
            .map(|c| c.code.as_str())
            .unwrap_or("UNKNOWN");
        write!(f, "{}[{code}]: {}", self.severity.as_str(), self.message)
    }
}
