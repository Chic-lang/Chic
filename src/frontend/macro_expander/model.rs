use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Token, TokenKind};
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroInvocationKind {
    Derive,
    Attribute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HygieneId(u64);

impl HygieneId {
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct MacroInvocation {
    pub kind: MacroInvocationKind,
    pub name: String,
    pub span: Option<Span>,
    pub raw: Option<String>,
    pub tokens: Vec<Token>,
    pub hygiene: HygieneId,
}

impl MacroInvocation {
    #[must_use]
    pub fn new(
        kind: MacroInvocationKind,
        name: impl Into<String>,
        span: Option<Span>,
        raw: Option<String>,
        tokens: Vec<Token>,
        hygiene: HygieneId,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            span,
            raw,
            tokens,
            hygiene,
        }
    }
}

#[derive(Clone, Eq)]
pub struct InvocationCacheKey {
    kind: MacroInvocationKind,
    name: String,
    target: String,
    raw: Option<String>,
    token_fingerprint: Option<u64>,
}

impl InvocationCacheKey {
    #[must_use]
    pub fn new(invocation: &MacroInvocation, target: &str) -> Self {
        Self {
            kind: invocation.kind,
            name: normalise_name(&invocation.name),
            target: target.to_string(),
            raw: invocation.raw.clone().map(|value| value.trim().to_string()),
            token_fingerprint: fingerprint_tokens(&invocation.tokens),
        }
    }
}

impl fmt::Debug for InvocationCacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InvocationCacheKey")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("target", &self.target)
            .field("raw", &self.raw)
            .finish()
    }
}

impl PartialEq for InvocationCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.name == other.name
            && self.target == other.target
            && self.raw == other.raw
            && self.token_fingerprint == other.token_fingerprint
    }
}

impl Hash for InvocationCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.name.hash(state);
        self.target.hash(state);
        if let Some(raw) = &self.raw {
            raw.hash(state);
        }
        if let Some(fingerprint) = self.token_fingerprint {
            fingerprint.hash(state);
        }
    }
}

#[must_use]
pub fn normalise_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

#[must_use]
pub fn trim_macro_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn fingerprint_tokens(tokens: &[Token]) -> Option<u64> {
    if tokens.is_empty() {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    for token in tokens {
        hash_token_kind(&mut hasher, &token.kind);
        hasher.write(token.lexeme.as_bytes());
    }
    Some(hasher.finish())
}

fn hash_token_kind(hasher: &mut DefaultHasher, kind: &TokenKind) {
    // Deriving Hash for TokenKind would cascade into literal structs; a Debug representation is
    // sufficient for stable cache keys because we only need equality across identical token sets.
    hasher.write(format!("{kind:?}").as_bytes());
}
