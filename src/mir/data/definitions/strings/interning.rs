use crate::frontend::diagnostics::Span;

/// Identifier for an interned string used throughout MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StrId(u32);

impl StrId {
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        StrId(raw)
    }

    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Lifetimes associated with interned strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrLifetime {
    Static,
}

/// Interned string record stored alongside MIR modules.
#[derive(Debug, Clone)]
pub struct InternedStr {
    pub id: StrId,
    pub value: String,
    pub lifetime: StrLifetime,
    pub span: Option<Span>,
}
