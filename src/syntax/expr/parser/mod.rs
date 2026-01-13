//! Lightweight expression parser for MIR lowering.

use crate::frontend::lexer::{LexOutput, TokenKind, lex};
use crate::syntax::expr::builders::ExprNode;

mod calls;
mod core;
mod inline_asm;
mod interpolation;
mod lambda;
mod operators;
mod primary;

pub use core::ExprError;
pub(crate) use core::ExprParser;

/// Result of parsing an expression snippet.
///
/// # Errors
/// Returns an [`ExprError`] when lexing or parsing the supplied source fails.
pub fn parse_expression(source: &str) -> Result<ExprNode, ExprError> {
    let LexOutput {
        tokens,
        diagnostics,
        ..
    } = lex(source);
    if let Some(diag) = diagnostics.into_iter().find(|d| d.severity.is_error()) {
        return Err(ExprError::new(
            format!("lex error while parsing expression: {}", diag.message),
            diag.primary_label.as_ref().map(|label| label.span),
        ));
    }
    let tokens = tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment))
        .collect::<Vec<_>>();
    let mut parser = ExprParser::new(tokens, source);
    match parser.parse_expression() {
        Ok(expr) => {
            parser.expect_end()?;
            Ok(expr)
        }
        Err(err) => {
            if std::env::var_os("CHIC_DEBUG_MIR_DIAGNOSTICS").is_some() {
                eprintln!(
                    "[chic-debug] parse_expression failed for `{}`: {}",
                    source, err.message
                );
            }
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests;
