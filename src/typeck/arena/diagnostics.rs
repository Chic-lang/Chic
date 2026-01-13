//! Diagnostics helpers shared across the type checker arena.

use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::mir::ParamMode;
use crate::typeck::diagnostics as typeck_diagnostics;
use crate::typeck::diagnostics::codes;

use super::lifetimes::{BorrowEscapeCategory, BorrowEscapeMessages};

pub(super) fn emit_error_with_spec_link(
    sink: &mut Vec<Diagnostic>,
    code: &'static str,
    span: Option<Span>,
    message: impl Into<String>,
) {
    sink.push(typeck_diagnostics::error(code, message.into(), span));
    attach_spec_link(sink, code, span);
}

pub(super) fn emit_warning_with_spec_link(
    sink: &mut Vec<Diagnostic>,
    code: &'static str,
    span: Option<Span>,
    message: impl Into<String>,
) {
    sink.push(typeck_diagnostics::warning(code, message.into(), span));
    attach_spec_link(sink, code, span);
}

pub(super) fn report_borrow_escape(
    sink: &mut Vec<Diagnostic>,
    function: &str,
    parameter: &str,
    mode: ParamMode,
    escape: &BorrowEscapeCategory,
    span: Option<Span>,
) {
    let messages = BorrowEscapeMessages::describe(function, parameter, mode, escape);
    sink.push(typeck_diagnostics::error(
        codes::BORROW_ESCAPE,
        messages.error,
        span,
    ));
    sink.push(typeck_diagnostics::warning(
        codes::LEGACY_BORROW_LINT,
        messages.lint,
        span,
    ));
    sink.push(typeck_diagnostics::note(messages.note, span));
}

fn attach_spec_link(sink: &mut Vec<Diagnostic>, code: &'static str, span: Option<Span>) {
    if let Some(links) = typeck_diagnostics::spec_links(code) {
        let joined = links.join(", ");
        sink.push(typeck_diagnostics::note(
            format!("See {joined} for specification details."),
            span,
        ));
    }
}
