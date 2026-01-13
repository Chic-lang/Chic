#![cfg(test)]

use super::common::*;
use super::helpers::*;

#[test]
fn emit_numeric_try_add_emits_checked_path_and_out_store() {
    let body = emit_body_default(numeric_try_add_function());
    assert!(body.contains(&0x6A), "checked add should emit i32.add");
    assert!(
        body.windows(1).any(|w| w[0] == 0x04),
        "checked add should branch to guard overflow"
    );
}

#[test]
fn emit_numeric_leading_zero_byte_adjusts_width() {
    let body = emit_body_default(numeric_leading_zero_byte_function());
    assert!(body.contains(&0x67), "leading zero count should emit clz");
    assert!(
        body_contains_i32_const(&body, 24),
        "byte lzcnt should subtract the upper 24 bits (32 - 8) for wasm i32 results"
    );
}
