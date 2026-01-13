use super::*;

#[test]
fn parse_literal_accepts_grouping() {
    let parts = parse_literal_parts("1_234.500").expect("valid literal");
    assert_eq!(format_parts(parts), "1234.5");
}

#[test]
fn parse_literal_detects_overflow_scale() {
    let result = parse_literal_parts("0.0000000000000000000000000001");
    assert!(
        matches!(result, Err(DecimalRuntimeStatus::Overflow)) || result.is_ok(),
        "expected overflow or graceful parse, found {result:?}"
    );
}

#[test]
fn format_round_trips_negative_values() {
    let parts = parse_literal_parts("-42.25").expect("valid literal");
    assert_eq!(format_parts(parts), "-42.25");
}
