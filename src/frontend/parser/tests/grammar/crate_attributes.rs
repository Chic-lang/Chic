use crate::frontend::parser::{CrateMainSetting, CrateStdSetting, parse_module};

#[test]
fn parses_no_std_crate_attribute() {
    let source = r#"
#![no_std]
namespace Sample;
"#;

    let parsed = parse_module(source).expect("parse");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    match parsed.module.crate_attributes.std_setting {
        CrateStdSetting::NoStd { .. } => {}
        other => panic!("expected no_std crate attribute, found {other:?}"),
    }
}

#[test]
fn parses_std_crate_attribute() {
    let parsed = parse_module("#![std]\nnamespace Sample;").expect("parse");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    match parsed.module.crate_attributes.std_setting {
        CrateStdSetting::Std { .. } => {}
        other => panic!("expected std crate attribute, found {other:?}"),
    }
}

#[test]
fn rejects_conflicting_crate_attributes() {
    let err = parse_module("#![no_std]\n#![std]\nnamespace Sample { public void Main() { } }")
        .expect_err("conflict should fail parsing");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("conflicting crate attributes")),
        "expected conflicting attribute diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_duplicate_crate_attributes() {
    let err = parse_module("#![no_std]\n#![no_std]\nnamespace Sample;")
        .expect_err("duplicate crate attribute should fail");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("duplicate `#![no_std]`")),
        "expected duplicate diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_arguments_on_crate_attributes() {
    let err = parse_module("#![no_std(core)]\nnamespace Sample;")
        .expect_err("arguments should be rejected");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("do not accept arguments")),
        "expected argument rejection, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn parses_no_main_crate_attribute() {
    let parsed = parse_module("#![no_main]\nnamespace Sample;").expect("parse");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    assert!(
        matches!(
            parsed.module.crate_attributes.main_setting,
            CrateMainSetting::NoMain { .. }
        ),
        "expected no_main crate attribute"
    );
}

#[test]
fn rejects_duplicate_no_main_crate_attributes() {
    let err = parse_module("#![no_main]\n#![no_main]\nnamespace Sample;")
        .expect_err("duplicate crate attribute should fail");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("duplicate `#![no_main]`")),
        "expected duplicate diagnostic, got {:?}",
        err.diagnostics()
    );
}

#[test]
fn rejects_unknown_crate_attribute() {
    let err = parse_module("#![feature(foo)]\nnamespace Sample;")
        .expect_err("unknown crate attribute should fail");
    assert!(
        err.diagnostics()
            .iter()
            .any(|diag| diag.message.contains("unsupported crate-level attribute")),
        "expected unsupported attribute diagnostic, got {:?}",
        err.diagnostics()
    );
}
