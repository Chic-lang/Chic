#![allow(unsafe_code)]

use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; span runtime lives in the native runtime."
);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicSpan {
    pub data: ValueMutPtr,
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicReadOnlySpan {
    pub data: ValueConstPtr,
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanLayoutInfo {
    pub size: usize,
    pub offset_data: usize,
    pub offset_reserved: usize,
    pub offset_len: usize,
    pub offset_elem_size: usize,
    pub offset_elem_align: usize,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    Success = 0,
    NullPointer = 1,
    OutOfBounds = 2,
    InvalidStride = 3,
}

impl SpanError {
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

unsafe extern "C" {
    #[link_name = "chic_rt_span_from_raw_mut"]
    fn span_from_raw_mut_raw(data: *const ValueMutPtr, len: usize) -> ChicSpan;
    #[link_name = "chic_rt_span_from_raw_const"]
    fn span_from_raw_const_raw(data: *const ValueConstPtr, len: usize) -> ChicReadOnlySpan;
    #[link_name = "chic_rt_span_slice_mut"]
    fn span_slice_mut_raw(
        source: *const ChicSpan,
        start: usize,
        length: usize,
        dest: *mut ChicSpan,
    ) -> i32;
    #[link_name = "chic_rt_span_slice_readonly"]
    fn span_slice_readonly_raw(
        source: *const ChicReadOnlySpan,
        start: usize,
        length: usize,
        dest: *mut ChicReadOnlySpan,
    ) -> i32;
    #[link_name = "chic_rt_span_to_readonly"]
    fn span_to_readonly_raw(span: *const ChicSpan) -> ChicReadOnlySpan;
    #[link_name = "chic_rt_span_copy_to"]
    fn span_copy_to_raw(source: *const ChicReadOnlySpan, dest: *const ChicSpan) -> i32;
    #[link_name = "chic_rt_span_fill"]
    fn span_fill_raw(dest: *const ChicSpan, value: *const u8) -> i32;
    #[link_name = "chic_rt_span_layout_debug"]
    fn span_layout_debug_raw(out: *mut SpanLayoutInfo);
    #[link_name = "chic_rt_span_ptr_at_mut"]
    fn span_ptr_at_mut_raw(span: *const ChicSpan, index: usize) -> *mut u8;
    #[link_name = "chic_rt_span_ptr_at_readonly"]
    fn span_ptr_at_readonly_raw(span: *const ChicReadOnlySpan, index: usize) -> *const u8;
}

#[inline]
pub unsafe fn chic_rt_span_from_raw_mut(data: ValueMutPtr, len: usize) -> ChicSpan {
    unsafe { span_from_raw_mut_raw(&data, len) }
}

#[inline]
pub unsafe fn chic_rt_span_from_raw_const(data: ValueConstPtr, len: usize) -> ChicReadOnlySpan {
    unsafe { span_from_raw_const_raw(&data, len) }
}

#[inline]
pub unsafe fn chic_rt_span_slice_mut(
    source: *const ChicSpan,
    start: usize,
    length: usize,
    dest: *mut ChicSpan,
) -> i32 {
    unsafe { span_slice_mut_raw(source, start, length, dest) }
}

#[inline]
pub unsafe fn chic_rt_span_slice_readonly(
    source: *const ChicReadOnlySpan,
    start: usize,
    length: usize,
    dest: *mut ChicReadOnlySpan,
) -> i32 {
    unsafe { span_slice_readonly_raw(source, start, length, dest) }
}

#[inline]
pub unsafe fn chic_rt_span_to_readonly(span: *const ChicSpan) -> ChicReadOnlySpan {
    unsafe { span_to_readonly_raw(span) }
}

#[inline]
pub unsafe fn chic_rt_span_copy_to(source: *const ChicReadOnlySpan, dest: *const ChicSpan) -> i32 {
    unsafe { span_copy_to_raw(source, dest) }
}

#[inline]
pub unsafe fn chic_rt_span_fill(dest: *const ChicSpan, value: *const u8) -> i32 {
    unsafe { span_fill_raw(dest, value) }
}

#[inline]
pub unsafe fn chic_rt_span_layout_debug(out: *mut SpanLayoutInfo) {
    unsafe { span_layout_debug_raw(out) }
}

#[inline]
pub unsafe fn chic_rt_span_ptr_at_mut(span: *const ChicSpan, index: usize) -> *mut u8 {
    unsafe { span_ptr_at_mut_raw(span, index) }
}

#[inline]
pub unsafe fn chic_rt_span_ptr_at_readonly(
    span: *const ChicReadOnlySpan,
    index: usize,
) -> *const u8 {
    unsafe { span_ptr_at_readonly_raw(span, index) }
}
