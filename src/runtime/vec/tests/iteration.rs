#![allow(unsafe_code, unused_unsafe)]

use super::*;
use crate::runtime::test_lock::runtime_test_guard;
use crate::runtime::value_ptr::ValueMutPtr;

#[test]
fn vec_iterator_copies_values() {
    let _guard = runtime_test_guard();
    let mut vec = unsafe {
        chic_rt_vec_new(
            std::mem::size_of::<u32>(),
            std::mem::align_of::<u32>(),
            None,
        )
    };
    for value in 1..=3u32 {
        let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr_from(&value)) };
        assert_eq!(status, VecError::Success.as_i32());
    }
    let data = unsafe { chic_rt_vec_data(&vec) };
    unsafe {
        assert_eq!(chic_rt_vec_len(&vec), 3);
        assert!(!data.ptr.is_null());
    }
    let mut iter = unsafe { chic_rt_vec_iter(&vec) };
    let mut collected = Vec::new();
    loop {
        let mut value = 0u32;
        let out = ValueMutPtr {
            ptr: &mut value as *mut u32 as *mut u8,
            size: std::mem::size_of::<u32>(),
            align: std::mem::align_of::<u32>(),
        };
        let status = unsafe { chic_rt_vec_iter_next(&mut iter, out) };
        if status == VecError::IterationComplete.as_i32() {
            break;
        }
        assert_eq!(status, VecError::Success.as_i32());
        collected.push(value);
    }
    assert_eq!(collected, vec![1, 2, 3]);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_view_reports_metadata() {
    let _guard = runtime_test_guard();
    let mut vec = unsafe {
        chic_rt_vec_new(
            std::mem::size_of::<u16>(),
            std::mem::align_of::<u16>(),
            None,
        )
    };
    for value in [10u16, 20u16] {
        unsafe {
            chic_rt_vec_push(&mut vec, const_ptr_from(&value));
        }
    }
    let view = unsafe { chic_rt_vec_view(&vec) };
    assert_eq!(view.len, 2);
    assert_eq!(view.elem_size, std::mem::size_of::<u16>());
    assert_eq!(view.elem_align, std::mem::align_of::<u16>());
    assert!(!view.data.is_null());
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_iter_next_ptr_returns_null_when_complete() {
    let _guard = runtime_test_guard();
    let mut vec =
        unsafe { chic_rt_vec_new(std::mem::size_of::<u8>(), std::mem::align_of::<u8>(), None) };
    unsafe {
        chic_rt_vec_push(&mut vec, const_ptr_from(&5u8));
    }
    let mut iter = unsafe { chic_rt_vec_iter(&vec) };
    let first = unsafe { chic_rt_vec_iter_next_ptr(&mut iter) };
    assert!(!first.ptr.is_null());
    assert_eq!(first.size, std::mem::size_of::<u8>());
    assert_eq!(first.align, std::mem::align_of::<u8>());
    let second = unsafe { chic_rt_vec_iter_next_ptr(&mut iter) };
    assert!(second.ptr.is_null());
    unsafe { chic_rt_vec_drop(&mut vec) };
}
