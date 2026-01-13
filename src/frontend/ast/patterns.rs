//! Pattern AST nodes shared between statements and expressions.

use super::expressions::Expression;
use crate::frontend::diagnostics::Span;
use crate::syntax::pattern::PatternAst;

/// Pattern captured from the source text plus its parsed representation.
#[derive(Debug, Clone)]
pub struct CasePattern {
    /// Raw source text as captured by the parser (sans guards).
    pub raw: Expression,
    /// Parsed Chic pattern tree. `None` when parsing failed.
    pub ast: Option<PatternAst>,
}

impl CasePattern {
    #[must_use]
    pub fn new(raw: Expression, ast: Option<PatternAst>) -> Self {
        Self { raw, ast }
    }
}

/// Guard expression attached to a pattern (`when` clause).
#[derive(Debug, Clone)]
pub struct PatternGuard {
    pub expression: Expression,
    pub depth: usize,
    pub keyword_span: Option<Span>,
}
