#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; character runtime lives in the native runtime."
);

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharError {
    Success = 0,
    InvalidScalar = 1,
    NullPointer = 2,
    ComplexMapping = 3,
}

mod native {
    unsafe extern "C" {
        pub fn chic_rt_char_is_scalar(value: u16) -> i32;
        pub fn chic_rt_char_is_digit(value: u16) -> i32;
        pub fn chic_rt_char_is_letter(value: u16) -> i32;
        pub fn chic_rt_char_is_whitespace(value: u16) -> i32;
        pub fn chic_rt_char_to_upper(value: u16) -> u64;
        pub fn chic_rt_char_to_lower(value: u16) -> u64;
        pub fn chic_rt_char_from_codepoint(value: u32) -> u64;
        pub fn chic_rt_char_status(packed: u64) -> i32;
        pub fn chic_rt_char_value(packed: u64) -> u16;
    }
}

pub use native::*;
