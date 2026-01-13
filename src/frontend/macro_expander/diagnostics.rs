use crate::frontend::diagnostics::Diagnostic;

use super::model::{MacroInvocation, MacroInvocationKind};

pub fn unknown_macro(invocation: &MacroInvocation, context: &str) -> Diagnostic {
    match invocation.kind {
        MacroInvocationKind::Derive => Diagnostic::error(
            format!("unknown derive macro `{}` on {context}", invocation.name),
            invocation.span,
        ),
        MacroInvocationKind::Attribute => Diagnostic::error(
            format!(
                "attribute macro `{}` is not registered on {context}",
                invocation.name
            ),
            invocation.span,
        ),
    }
}

pub fn unsupported_macro(invocation: &MacroInvocation, context: &str) -> Diagnostic {
    match invocation.kind {
        MacroInvocationKind::Derive => Diagnostic::error(
            format!("`@derive` is not supported on {context}"),
            invocation.span,
        ),
        MacroInvocationKind::Attribute => Diagnostic::error(
            format!(
                "attribute macro `{}` is not supported on {context}",
                invocation.name
            ),
            invocation.span,
        ),
    }
}

pub fn runaway_macros(limit: usize) -> Diagnostic {
    Diagnostic::error(
        format!(
            "macro expansion exceeded {limit} passes; expansion is likely recursive or non-terminating"
        ),
        None,
    )
}
