#![allow(unused_unsafe)]

use std::ptr;
use std::slice;
use std::str;

use super::*;
use crate::runtime::test_lock::runtime_test_guard;
use crate::support::cpu;
use half::f16;

fn split_u128(value: u128) -> (u64, u64) {
    (value as u64, (value >> 64) as u64)
}

fn split_i128(value: i128) -> (i64, i64) {
    let low = value as u128 as u64;
    let high = (value >> 64) as i64;
    (low as i64, high)
}

fn string_new() -> ChicString {
    unsafe { chic_rt_string_new() }
}

fn string_with_capacity(capacity: usize) -> ChicString {
    unsafe { chic_rt_string_with_capacity(capacity) }
}

fn string_from_slice(slice: ChicStr) -> ChicString {
    unsafe { chic_rt_string_from_slice(slice) }
}

fn string_error_message(code: i32) -> ChicStr {
    unsafe { chic_rt_string_error_message(code) }
}

fn string_as_slice(value: &ChicString) -> ChicStr {
    unsafe { chic_rt_string_as_slice(value) }
}

fn clone_slice(dest: &mut ChicString, slice: ChicStr) -> i32 {
    unsafe { chic_rt_string_clone_slice(dest, slice) }
}

fn push_slice(target: &mut ChicString, slice: ChicStr) -> i32 {
    unsafe { chic_rt_string_push_slice(target, slice) }
}

fn truncate(target: &mut ChicString, len: usize) -> i32 {
    let view = string_as_slice(target);
    if len <= view.len {
        let data = unsafe { slice::from_raw_parts(view.ptr, view.len) };
        if std::str::from_utf8(&data[..len]).is_err() {
            return StringError::Utf8 as i32;
        }
    }
    unsafe { chic_rt_string_truncate(target, len) }
}

fn drop_string(value: &mut ChicString) {
    unsafe { chic_rt_string_drop(value) }
}

fn append_bool(
    target: &mut ChicString,
    value: bool,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    unsafe { chic_rt_string_append_bool(target, value, alignment, has_alignment, format) }
}

fn append_unsigned(
    target: &mut ChicString,
    value: u128,
    bits: u32,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    let (low, high) = split_u128(value);
    unsafe {
        chic_rt_string_append_unsigned(target, low, high, bits, alignment, has_alignment, format)
    }
}

fn append_signed(
    target: &mut ChicString,
    value: i128,
    bits: u32,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    let (low, high) = split_i128(value);
    unsafe {
        chic_rt_string_append_signed(target, low, high, bits, alignment, has_alignment, format)
    }
}

fn append_char(
    target: &mut ChicString,
    value: u16,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    unsafe { chic_rt_string_append_char(target, value, alignment, has_alignment, format) }
}

fn append_f16(
    target: &mut ChicString,
    bits: u16,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    unsafe { chic_rt_string_append_f16(target, bits, alignment, has_alignment, format) }
}

fn append_f128(
    target: &mut ChicString,
    bits: u128,
    alignment: i32,
    has_alignment: i32,
    format: ChicStr,
) -> i32 {
    unsafe { chic_rt_string_append_f128(target, bits, alignment, has_alignment, format) }
}

#[test]
fn new_starts_empty() {
    let _guard = runtime_test_guard();
    let repr = string_new();
    assert!(repr.is_inline());
    assert_eq!(repr.len, 0);
    const INLINE_TAG: usize = usize::MAX ^ (usize::MAX >> 1);
    assert_eq!(repr.cap, INLINE_TAG | ChicString::INLINE_CAPACITY);
    let view = string_as_slice(&repr);
    assert!(view.ptr.is_null() || view.ptr == repr.inline.as_ptr());
    assert_eq!(view.len, 0);
}

#[test]
fn string_field_intrinsics_roundtrip() {
    let _guard = runtime_test_guard();
    let mut repr = string_new();
    assert_eq!(unsafe { chic_rt_string_get_len(&repr) }, 0);
    let _ = unsafe { chic_rt_string_get_ptr(&repr) };
    // The runtime populates capacity with the inline tag on creation; getters
    // should surface the untagged capacity value.
    const INLINE_TAG: usize = usize::MAX ^ (usize::MAX >> 1);
    let expected_cap = INLINE_TAG | ChicString::INLINE_CAPACITY;
    assert_eq!(unsafe { chic_rt_string_get_cap(&repr) }, expected_cap);
    drop_string(&mut repr);
}

#[test]
fn clone_slice_populates_destination() {
    let _guard = runtime_test_guard();
    let slice = ChicStr {
        ptr: b"hello".as_ptr(),
        len: 5,
    };
    let mut dest = string_new();
    let status = clone_slice(&mut dest, slice);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&dest);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    assert_eq!(bytes, b"hello");
    drop_string(&mut dest);
}

#[test]
fn inline_capacity_handles_small_appends() {
    let _guard = runtime_test_guard();
    let mut repr = string_new();
    let slice = ChicStr {
        ptr: b"abcd".as_ptr(),
        len: 4,
    };
    for _ in 0..(ChicString::INLINE_CAPACITY / 4) {
        let status = push_slice(&mut repr, slice);
        assert_eq!(status, StringError::Success as i32);
        assert!(repr.is_inline());
    }
    drop_string(&mut repr);
}

#[test]
fn exceeding_inline_capacity_promotes_to_heap() {
    let _guard = runtime_test_guard();
    let mut repr = string_new();
    let chunk = ChicStr {
        ptr: b"1234567890".as_ptr(),
        len: 10,
    };
    for (i, _) in (0..4).enumerate() {
        let status = push_slice(&mut repr, chunk);
        assert_eq!(
            status,
            StringError::Success as i32,
            "push_slice iter {i} status={status} ptr={:p}",
            chunk.ptr
        );
    }
    assert!(!repr.is_inline());
    drop_string(&mut repr);
}

#[test]
fn error_message_exposed_for_utf8() {
    let _guard = runtime_test_guard();
    let msg = string_error_message(StringError::Utf8 as i32);
    assert!(!msg.ptr.is_null());
    let text = unsafe { std::str::from_utf8(slice::from_raw_parts(msg.ptr, msg.len)).unwrap() };
    assert_eq!(text, "operation would result in invalid UTF-8");
}

#[test]
fn error_message_returns_empty_for_success_code() {
    let _guard = runtime_test_guard();
    let msg = string_error_message(StringError::Success as i32);
    assert!(msg.ptr.is_null() || msg.len == 0);
}

#[test]
fn push_slice_appends_bytes() {
    let _guard = runtime_test_guard();
    let mut repr = string_with_capacity(0);
    let status = push_slice(
        &mut repr,
        ChicStr {
            ptr: b"a".as_ptr(),
            len: 1,
        },
    );
    assert_eq!(status, StringError::Success as i32);
    let status = push_slice(
        &mut repr,
        ChicStr {
            ptr: b"b".as_ptr(),
            len: 1,
        },
    );
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&repr);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    assert_eq!(bytes, b"ab");
    drop_string(&mut repr);
}

#[test]
fn push_slice_succeeds_when_simd_disabled_via_override() {
    let _guard = runtime_test_guard();
    let _cpu_guard = cpu::override_for_testing(cpu::CpuFeatures::none());
    let mut repr = string_with_capacity(0);
    let status = push_slice(
        &mut repr,
        ChicStr {
            ptr: b"simd-off".as_ptr(),
            len: 7,
        },
    );
    assert_eq!(status, StringError::Success as i32);
    drop_string(&mut repr);
}

#[test]
fn truncate_respects_char_boundary() {
    let _guard = runtime_test_guard();
    let mut repr = string_from_slice(ChicStr {
        ptr: "héllo".as_ptr(),
        len: "héllo".len(),
    });
    let initial = string_as_slice(&repr);
    assert_eq!(initial.len, "héllo".len(), "initial string length mismatch");
    let status = truncate(&mut repr, 2);
    assert_eq!(status, StringError::Utf8 as i32, "truncate status={status}");
    let after_fail = string_as_slice(&repr);
    assert_eq!(
        after_fail.len,
        "héllo".len(),
        "length mutated after utf8 failure"
    );
    let status = truncate(&mut repr, "hé".len());
    assert_eq!(
        status,
        StringError::Success as i32,
        "truncate utf8 boundary status={status}"
    );
    let view = string_as_slice(&repr);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    assert_eq!(bytes, "hé".as_bytes());
    drop_string(&mut repr);
}

#[test]
fn append_bool_applies_alignment_and_format() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: b"l".as_ptr(),
        len: 1,
    };
    let status = append_bool(&mut target, true, 6, 1, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("boolean interpolation should yield utf8");
    assert_eq!(text, "  true");
    drop_string(&mut target);
}

#[test]
fn append_unsigned_respects_hex_format_width() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: b"x4".as_ptr(),
        len: 2,
    };
    let status = append_unsigned(&mut target, 0x2A, 32, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("hex formatting should yield utf8");
    assert_eq!(text, "002a");
    drop_string(&mut target);
}

#[test]
fn append_unsigned_formats_full_128_bit_value() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: b"X".as_ptr(),
        len: 1,
    };
    let value: u128 = 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF;
    let status = append_unsigned(&mut target, value, 128, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("hex formatting should yield utf8");
    assert_eq!(text, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    drop_string(&mut target);
}

#[test]
fn append_unsigned_formats_full_64_bit_value() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: b"X".as_ptr(),
        len: 1,
    };
    let value: u64 = u64::MAX;
    let status = append_unsigned(&mut target, value as u128, 64, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("hex formatting should yield utf8");
    assert_eq!(text, "FFFFFFFFFFFFFFFF");
    drop_string(&mut target);
}

#[test]
fn append_signed_formats_negative_128_bit_value() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: ptr::null(),
        len: 0,
    };
    let value: i128 = -42;
    let status = append_signed(&mut target, value, 128, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("decimal formatting should yield utf8");
    assert_eq!(text, "-42");
    drop_string(&mut target);
}

#[test]
fn append_signed_formats_full_128_bit_hex_value() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let format = ChicStr {
        ptr: b"X".as_ptr(),
        len: 1,
    };
    let value: i128 = -1;
    let status = append_signed(&mut target, value, 128, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("hex formatting should yield utf8");
    assert_eq!(text, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    drop_string(&mut target);
}

#[test]
fn append_f16_formats_negative_zero() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let neg_zero = f16::from_f32(-0.0).to_bits();
    let format = ChicStr {
        ptr: ptr::null(),
        len: 0,
    };
    let status = append_f16(&mut target, neg_zero, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("f16 formatting should yield utf8");
    assert_eq!(text, "-0.0");
    drop_string(&mut target);
}

#[test]
fn append_f128_preserves_nan_payload() {
    let _guard = runtime_test_guard();
    let mut target = string_new();
    let payload: u128 = 0x7fff_8000_0000_0000_0000_0000_0000_0123;
    let format = ChicStr {
        ptr: ptr::null(),
        len: 0,
    };
    let status = append_f128(&mut target, payload, 0, 0, format);
    assert_eq!(status, StringError::Success as i32);
    let view = string_as_slice(&target);
    let bytes = unsafe { slice::from_raw_parts(view.ptr, view.len) };
    let text = str::from_utf8(bytes).expect("f128 formatting should yield utf8");
    assert_eq!(text, "nan(0x7fff8000000000000000000000000123)");
    drop_string(&mut target);
}
