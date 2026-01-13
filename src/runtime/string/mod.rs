#![cfg_attr(chic_native_runtime, allow(dead_code, unused_imports))]
#![allow(unsafe_code)]

mod native;
mod shim_types;
#[cfg(test)]
mod tests;

pub use native::{
    chic_rt_str_as_chars, chic_rt_string_append_bool, chic_rt_string_append_char,
    chic_rt_string_append_f16, chic_rt_string_append_f32, chic_rt_string_append_f64,
    chic_rt_string_append_f128, chic_rt_string_append_signed, chic_rt_string_append_slice,
    chic_rt_string_append_unsigned, chic_rt_string_as_chars, chic_rt_string_as_slice,
    chic_rt_string_clone, chic_rt_string_clone_slice, chic_rt_string_drop,
    chic_rt_string_error_message, chic_rt_string_from_char, chic_rt_string_from_slice,
    chic_rt_string_get_cap, chic_rt_string_get_len, chic_rt_string_get_ptr,
    chic_rt_string_inline_capacity, chic_rt_string_inline_ptr, chic_rt_string_new,
    chic_rt_string_push_slice, chic_rt_string_reserve, chic_rt_string_set_cap,
    chic_rt_string_set_len, chic_rt_string_set_ptr, chic_rt_string_truncate,
    chic_rt_string_with_capacity,
};
pub use shim_types::{ChicCharSpan, ChicStr, ChicString, StringError};
