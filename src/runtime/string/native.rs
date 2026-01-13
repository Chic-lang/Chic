#![allow(unsafe_code)]

use super::shim_types::{ChicCharSpan, ChicStr, ChicString};

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; string runtime lives in the native runtime."
);

unsafe extern "C" {
    pub fn chic_rt_string_error_message(code: i32) -> ChicStr;

    pub fn chic_rt_string_get_ptr(value: *const ChicString) -> *mut u8;
    pub fn chic_rt_string_set_ptr(value: *mut ChicString, ptr: *mut u8);
    pub fn chic_rt_string_get_len(value: *const ChicString) -> usize;
    pub fn chic_rt_string_set_len(value: *mut ChicString, len: usize);
    pub fn chic_rt_string_get_cap(value: *const ChicString) -> usize;
    pub fn chic_rt_string_set_cap(value: *mut ChicString, cap: usize);
    pub fn chic_rt_string_inline_ptr(value: *mut ChicString) -> *mut u8;
    pub fn chic_rt_string_inline_capacity() -> usize;

    pub fn chic_rt_string_new() -> ChicString;
    pub fn chic_rt_string_with_capacity(capacity: usize) -> ChicString;
    pub fn chic_rt_string_from_slice(slice: ChicStr) -> ChicString;
    pub fn chic_rt_string_from_char(value: u16) -> ChicString;
    pub fn chic_rt_string_drop(target: *mut ChicString);
    pub fn chic_rt_string_clone(dest: *mut ChicString, src: *const ChicString) -> i32;
    pub fn chic_rt_string_clone_slice(dest: *mut ChicString, slice: ChicStr) -> i32;
    pub fn chic_rt_string_reserve(target: *mut ChicString, additional: usize) -> i32;
    pub fn chic_rt_string_push_slice(target: *mut ChicString, slice: ChicStr) -> i32;
    pub fn chic_rt_string_truncate(target: *mut ChicString, new_len: usize) -> i32;

    pub fn chic_rt_string_as_slice(source: *const ChicString) -> ChicStr;
    pub fn chic_rt_string_as_chars(source: *const ChicString) -> ChicCharSpan;
    pub fn chic_rt_str_as_chars(slice: ChicStr) -> ChicCharSpan;

    pub fn chic_rt_string_append_slice(
        target: *mut ChicString,
        slice: ChicStr,
        alignment: i32,
        has_alignment: i32,
    ) -> i32;
    pub fn chic_rt_string_append_bool(
        target: *mut ChicString,
        value: bool,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_char(
        target: *mut ChicString,
        value: u16,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_signed(
        target: *mut ChicString,
        low: i64,
        high: i64,
        bits: u32,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_unsigned(
        target: *mut ChicString,
        low: u64,
        high: u64,
        bits: u32,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_f32(
        target: *mut ChicString,
        value: f32,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_f64(
        target: *mut ChicString,
        value: f64,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_f16(
        target: *mut ChicString,
        bits: u16,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
    pub fn chic_rt_string_append_f128(
        target: *mut ChicString,
        bits: u128,
        alignment: i32,
        has_alignment: i32,
        format: ChicStr,
    ) -> i32;
}
