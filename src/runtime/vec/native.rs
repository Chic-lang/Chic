#![allow(improper_ctypes)]
#![allow(unsafe_code)]

use crate::runtime::region::RegionHandle;
use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use crate::runtime::vec::{ChicVec, ChicVecIter, ChicVecView, VecDropFn};

// The Chic-native runtime owns all Vec/Array semantics. These bindings are a
// thin C ABI surface so MIR/codegen can link against the Chic implementation.
unsafe extern "C" {
    pub fn chic_rt_vec_new(
        elem_size: usize,
        elem_align: usize,
        drop_fn: Option<VecDropFn>,
    ) -> ChicVec;
    pub fn chic_rt_vec_new_in_region(
        elem_size: usize,
        elem_align: usize,
        drop_fn: Option<VecDropFn>,
        region: RegionHandle,
    ) -> ChicVec;
    pub fn chic_rt_vec_with_capacity(
        elem_size: usize,
        elem_align: usize,
        capacity: usize,
        drop_fn: Option<VecDropFn>,
    ) -> ChicVec;
    pub fn chic_rt_vec_with_capacity_in_region(
        elem_size: usize,
        elem_align: usize,
        capacity: usize,
        drop_fn: Option<VecDropFn>,
        region: RegionHandle,
    ) -> ChicVec;

    pub fn chic_rt_vec_drop(vec: *mut ChicVec);
    pub fn chic_rt_vec_clone(dest: *mut ChicVec, src: *const ChicVec) -> i32;
    pub fn chic_rt_vec_into_array(dest: *mut ChicVec, src: *mut ChicVec) -> i32;
    pub fn chic_rt_array_into_vec(dest: *mut ChicVec, src: *mut ChicVec) -> i32;

    pub fn chic_rt_vec_reserve(vec: *mut ChicVec, additional: usize) -> i32;
    pub fn chic_rt_vec_shrink_to_fit(vec: *mut ChicVec) -> i32;
    pub fn chic_rt_vec_push(vec: *mut ChicVec, value: ValueConstPtr) -> i32;
    pub fn chic_rt_vec_pop(vec: *mut ChicVec, out: ValueMutPtr) -> i32;
    pub fn chic_rt_vec_insert(vec: *mut ChicVec, index: usize, value: ValueConstPtr) -> i32;
    pub fn chic_rt_vec_remove(vec: *mut ChicVec, index: usize, out: ValueMutPtr) -> i32;
    pub fn chic_rt_vec_swap_remove(vec: *mut ChicVec, index: usize, out: ValueMutPtr) -> i32;
    pub fn chic_rt_vec_truncate(vec: *mut ChicVec, new_len: usize) -> i32;
    pub fn chic_rt_vec_clear(vec: *mut ChicVec) -> i32;
    pub fn chic_rt_vec_set_len(vec: *mut ChicVec, new_len: usize) -> i32;
    pub fn chic_rt_vec_copy_to_array(dest: *mut ChicVec, src: *const ChicVec) -> i32;
    pub fn chic_rt_array_copy_to_vec(dest: *mut ChicVec, src: *const ChicVec) -> i32;
    pub fn chic_rt_vec_iter_next(iter: *mut ChicVecIter, out: ValueMutPtr) -> i32;
    pub fn chic_rt_vec_iter_next_ptr(iter: *mut ChicVecIter) -> ValueConstPtr;

    pub fn chic_rt_vec_len(vec: *const ChicVec) -> usize;
    pub fn chic_rt_vec_capacity(vec: *const ChicVec) -> usize;
    pub fn chic_rt_vec_is_empty(vec: *const ChicVec) -> i32;
    #[link_name = "chic_rt_vec_view"]
    pub fn chic_rt_vec_view_raw(vec: *const ChicVec, dest: *mut ChicVecView) -> i32;
    pub fn chic_rt_vec_data(vec: *const ChicVec) -> ValueConstPtr;
    pub fn chic_rt_vec_data_mut(vec: *mut ChicVec) -> ValueMutPtr;
    pub fn chic_rt_vec_iter(vec: *const ChicVec) -> ChicVecIter;
    pub fn chic_rt_vec_inline_capacity(vec: *const ChicVec) -> usize;
    pub fn chic_rt_vec_inline_ptr(vec: *mut ChicVec) -> ValueMutPtr;
    pub fn chic_rt_vec_mark_inline(vec: *mut ChicVec, uses_inline: i32);
    pub fn chic_rt_vec_uses_inline(vec: *const ChicVec) -> i32;
    pub fn chic_rt_vec_ptr_at(vec: *const ChicVec, index: usize) -> ValueMutPtr;

    #[link_name = "chic_rt_array_view"]
    pub fn chic_rt_array_view_raw(array: *const ChicVec, dest: *mut ChicVecView) -> i32;
    pub fn chic_rt_array_data(array: *const ChicVec) -> ValueConstPtr;
    pub fn chic_rt_array_len(array: *const ChicVec) -> usize;
    pub fn chic_rt_array_is_empty(array: *const ChicVec) -> i32;
    pub fn chic_rt_array_ptr_at(array: *const ChicVec, index: usize) -> ValueMutPtr;

    pub fn chic_rt_vec_get_ptr(vec: *const ChicVec) -> ValueMutPtr;
    pub fn chic_rt_vec_set_ptr(vec: *mut ChicVec, ptr: ValueMutPtr);
    pub fn chic_rt_vec_set_cap(vec: *mut ChicVec, cap: usize);
    pub fn chic_rt_vec_elem_size(vec: *const ChicVec) -> usize;
    pub fn chic_rt_vec_elem_align(vec: *const ChicVec) -> usize;
    pub fn chic_rt_vec_set_elem_size(vec: *mut ChicVec, size: usize);
    pub fn chic_rt_vec_set_elem_align(vec: *mut ChicVec, align: usize);
    pub fn chic_rt_vec_get_drop(vec: *const ChicVec) -> Option<VecDropFn>;
    pub fn chic_rt_vec_set_drop(vec: *mut ChicVec, drop_fn: Option<VecDropFn>);
}

/// Convenience wrapper that matches the historical by-value view signature.
#[inline]
pub unsafe fn chic_rt_vec_view(vec: *const ChicVec) -> ChicVecView {
    let mut view = ChicVecView::default();
    // Ignore status; tests/assertions exercise the returned view.
    let _ = unsafe { chic_rt_vec_view_raw(vec, &mut view as *mut _) };
    view
}

/// Convenience wrapper for arrays mirroring the vec view behaviour.
#[inline]
pub unsafe fn chic_rt_array_view(array: *const ChicVec) -> ChicVecView {
    let mut view = ChicVecView::default();
    let _ = unsafe { chic_rt_array_view_raw(array, &mut view as *mut _) };
    view
}
