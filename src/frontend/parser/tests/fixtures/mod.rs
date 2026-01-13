use crate::frontend::ast::{Block, FunctionDecl};
use crate::frontend::attributes::stage_builtin_attributes;
use crate::frontend::diagnostics::{Diagnostic, Severity};
use crate::frontend::lexer::{LexOutput, lex};
use crate::frontend::parser::{ParseResult, parse_module};

pub(crate) mod declarations;

pub(crate) use declarations::{
    PIXEL_SOURCE, assert_add_function, assert_geometry_namespace, assert_pixel_union,
    assert_point_struct, parse_geometry_module,
};

/// Lexes the provided source and returns the raw token stream.
#[must_use]
pub(crate) fn lex_tokens(source: &str) -> LexOutput {
    lex(source)
}

/// Parse helper that expects success and returns the [`ParseResult`].
#[must_use]
pub(crate) fn parse_ok(source: &str) -> ParseResult {
    let mut result = parse_module(source).unwrap_or_else(|err| panic!("parse failed: {err:?}"));
    let mut staging = stage_builtin_attributes(&mut result.module);
    result.diagnostics.append(&mut staging);
    result
}

/// Parse helper that expects failure and returns the collected diagnostics.
#[must_use]
pub(crate) fn parse_fail(source: &str) -> Vec<Diagnostic> {
    match parse_module(source) {
        Ok(mut result) => {
            let mut staging = stage_builtin_attributes(&mut result.module);
            result.diagnostics.append(&mut staging);
            if result
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.severity, Severity::Error))
            {
                result.diagnostics
            } else {
                panic!(
                    "expected parse to report diagnostics, but succeeded: {:?}",
                    result
                );
            }
        }
        Err(err) => err.diagnostics().to_vec(),
    }
}

#[must_use]
pub(crate) fn function_body(func: &FunctionDecl) -> &Block {
    let Some(body) = func.body.as_ref() else {
        panic!("expected function body");
    };
    body
}
