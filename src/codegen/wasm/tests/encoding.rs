use crate::codegen::wasm::{ensure_u32, push_string};

#[test]
fn ensure_u32_reports_overflow_error() {
    let err = ensure_u32(usize::MAX, "overflow context").expect_err("expected ensure_u32 to fail");
    assert!(
        format!("{err}").contains("overflow context"),
        "unexpected ensure_u32 error message: {err}"
    );
}

#[test]
fn push_string_encodes_length_prefix() {
    let mut buf = Vec::new();
    push_string(&mut buf, "ok").expect("push string");
    assert_eq!(buf, vec![0x02, b'o', b'k']);
}

#[test]
fn ensure_u32_rejects_zero_length_strings_in_push_string() {
    let mut buf = Vec::new();
    push_string(&mut buf, "").expect("empty string is valid");
    assert_eq!(buf, vec![0]);
}
