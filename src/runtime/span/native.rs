#![allow(unsafe_code)]

use super::{ChicReadOnlySpan, ChicSpan};
use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

// External bindings to the Chic-native span runtime. Enabled when the native
// archive is linked to keep the Rust runtime as a thin FFI shim.
unsafe extern "C" {
    pub fn chic_rt_span_from_raw_mut(data: ValueMutPtr, len: usize) -> ChicSpan;
    pub fn chic_rt_span_from_raw_const(
        data: ValueConstPtr,
        len: usize,
    ) -> ChicReadOnlySpan;
    pub fn chic_rt_span_slice_mut(
        source: *const ChicSpan,
        start: usize,
        length: usize,
        dest: *mut ChicSpan,
    ) -> i32;
    pub fn chic_rt_span_ptr_at_mut(span: *const ChicSpan, index: usize) -> *mut u8;
    pub fn chic_rt_span_ptr_at_readonly(
        span: *const ChicReadOnlySpan,
        index: usize,
    ) -> *const u8;
    pub fn chic_rt_span_slice_readonly(
        source: *const ChicReadOnlySpan,
        start: usize,
        length: usize,
        dest: *mut ChicReadOnlySpan,
    ) -> i32;
    pub fn chic_rt_span_to_readonly(span: ChicSpan) -> ChicReadOnlySpan;
    pub fn chic_rt_span_copy_to(source: ChicReadOnlySpan, dest: ChicSpan) -> i32;
    pub fn chic_rt_span_fill(dest: ChicSpan, value: *const u8) -> i32;
}
