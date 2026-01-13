#![cfg(test)]

use crate::frontend::diagnostics::Severity;
use crate::mir::ParamMode;
use crate::typeck::arena::BorrowEscapeCategory;
use crate::typeck::arena::diagnostics::{emit_error_with_spec_link, report_borrow_escape};
use crate::typeck::diagnostics::codes;

#[test]
fn emit_error_adds_spec_link_note() {
    let mut diagnostics_sink = Vec::new();

    emit_error_with_spec_link(
        &mut diagnostics_sink,
        codes::UNKNOWN_TYPE,
        None,
        "unknown type `Missing`",
    );

    assert_eq!(diagnostics_sink.len(), 2);
    assert_eq!(diagnostics_sink[0].severity, Severity::Error);
    assert!(
        diagnostics_sink[0]
            .message
            .contains("[TCK030] unknown type `Missing`"),
        "unexpected message: {}",
        diagnostics_sink[0].message
    );
    assert_eq!(diagnostics_sink[1].severity, Severity::Note);
    assert!(
        diagnostics_sink[1].message.contains("SPEC.md"),
        "missing spec link: {}",
        diagnostics_sink[1].message
    );
}

#[test]
fn borrow_escape_emits_stacked_messages() {
    let mut diagnostics_sink = Vec::new();

    report_borrow_escape(
        &mut diagnostics_sink,
        "Demo::Run",
        "reader",
        ParamMode::Ref,
        &BorrowEscapeCategory::Return,
        None,
    );

    assert_eq!(diagnostics_sink.len(), 3);
    assert_eq!(diagnostics_sink[0].severity, Severity::Error);
    assert!(
        diagnostics_sink[0].message.contains("[CL0031]"),
        "error message missing code: {}",
        diagnostics_sink[0].message
    );
    assert_eq!(diagnostics_sink[1].severity, Severity::Warning);
    assert!(
        diagnostics_sink[1].message.contains("[CLL0001]"),
        "warning missing lint code: {}",
        diagnostics_sink[1].message
    );
    assert_eq!(diagnostics_sink[2].severity, Severity::Note);
}
