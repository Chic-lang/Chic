use std::fmt;
use std::path::PathBuf;

use crate::frontend::diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintCategory {
    Style,
    Correctness,
    Perf,
    Pedantic,
}

impl LintCategory {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Style => "style",
            Self::Correctness => "correctness",
            Self::Perf => "perf",
            Self::Pedantic => "pedantic",
        }
    }

    #[must_use]
    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "style" => Some(Self::Style),
            "correctness" => Some(Self::Correctness),
            "perf" | "performance" => Some(Self::Perf),
            "pedantic" => Some(Self::Pedantic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel {
    Allow,
    Warn,
    Error,
}

impl LintLevel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    #[must_use]
    pub fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "allow" | "off" => Some(Self::Allow),
            "warn" | "warning" => Some(Self::Warn),
            "deny" | "error" => Some(Self::Error),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }
}

#[derive(Debug, Clone)]
pub struct LintDescriptor {
    pub code: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: LintCategory,
    pub default_level: LintLevel,
}

#[derive(Debug, Clone)]
pub struct LintSuggestion {
    pub message: String,
    pub span: Option<Span>,
    pub replacement: Option<String>,
}

impl LintSuggestion {
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

#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub descriptor: &'static LintDescriptor,
    pub level: LintLevel,
    pub message: String,
    pub file: PathBuf,
    pub span: Option<Span>,
    pub suggestions: Vec<LintSuggestion>,
}

impl LintDiagnostic {
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.descriptor.code
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.descriptor.name
    }
}

impl fmt::Display for LintDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let span = match self.span {
            Some(span) => format!(":{}..{}", span.start, span.end),
            None => String::new(),
        };
        write!(
            f,
            "{} [{}] {}{}: {}",
            self.level.as_str(),
            self.descriptor.name,
            self.file.display(),
            span,
            self.message
        )
    }
}
