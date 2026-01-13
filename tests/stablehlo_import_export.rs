use chic::export::stablehlo::builder::build_stablehlo;
use chic::import::stablehlo::loader::{STABLEHLO_VERSION, load_stablehlo};
use chic::import::stablehlo::lower::lower_stablehlo;

#[test]
fn stablehlo_version_is_pinned() {
    assert_eq!(STABLEHLO_VERSION, "0.0.0-stub");
}

#[test]
fn stablehlo_loader_reports_stub() {
    let err = load_stablehlo(&[]).unwrap_err();
    assert!(err.contains("not implemented"));
}

#[test]
fn stablehlo_lower_reports_stub() {
    let err = lower_stablehlo().unwrap_err();
    assert!(err.contains("stub"));
}

#[test]
fn stablehlo_export_stub() {
    let err = build_stablehlo().unwrap_err();
    assert!(err.contains("stub"));
}
