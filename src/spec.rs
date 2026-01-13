//! Access helpers for the authoritative language specification.

/// File-system location of the language specification relative to the project root.
pub const SPEC_RELATIVE_PATH: &str = "SPEC.md";

/// Embedded copy of the specification so the compiler binary can surface it without disk I/O.
pub const SPEC_TEXT: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/SPEC.md"));

/// Metadata about the current specification snapshot.
#[derive(Debug, Clone)]
pub struct Spec {
    pub relative_path: &'static str,
    pub summary: &'static str,
    pub body: &'static str,
}

impl Spec {
    /// Create a `Spec` pointing at the embedded document.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            relative_path: SPEC_RELATIVE_PATH,
            summary: "Chic language specification (draft).",
            body: SPEC_TEXT,
        }
    }

    /// Returns the number of lines in the embedded specification.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.body.lines().count()
    }
}
