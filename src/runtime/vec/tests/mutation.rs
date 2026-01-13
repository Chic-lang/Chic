#![allow(unsafe_code, unused_unsafe)]

use super::*;
use crate::runtime::test_lock::runtime_test_guard;
use std::mem::{align_of, size_of};

fn new_vec(elem_size: usize, elem_align: usize, drop_fn: Option<VecDropFn>) -> ChicVec {
    unsafe { chic_rt_vec_new(elem_size, elem_align, drop_fn) }
}

fn new_vec_with_capacity(
    elem_size: usize,
    elem_align: usize,
    cap: usize,
    drop_fn: Option<VecDropFn>,
) -> ChicVec {
    let mut vec = unsafe { chic_rt_vec_with_capacity(elem_size, elem_align, cap, drop_fn) };
    if drop_fn.is_some() {
        unsafe { chic_rt_vec_set_drop(&mut vec, drop_fn) };
    }
    vec
}

#[test]
fn vec_new_has_zero_capacity_for_non_zero_sized() {
    let _guard = runtime_test_guard();
    let vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    assert!(vec.ptr.is_null());
    assert_eq!(vec.len, 0);
    assert_eq!(vec.cap, 0);
}

#[test]
fn vec_inline_buffer_growth_and_promotion() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(size_of::<u32>(), align_of::<u32>(), None);
    let inline_cap = vec.inline_capacity();
    assert!(
        inline_cap >= 8,
        "expected inline capacity to hold multiple elements"
    );

    let mut caps = Vec::new();
    for value in 0..=inline_cap {
        let val = value as u32;
        let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr_from(&val)) };
        assert_eq!(
            status,
            VecError::Success.as_i32(),
            "push {value} failed (len {}, cap {}, elem_size {}, elem_align {})",
            vec.len,
            vec.cap,
            vec.elem_size,
            vec.elem_align
        );
        assert!(vec.len >= 1 && vec.cap >= vec.len);
        caps.push(vec.cap);
    }
    assert_eq!(vec.len, inline_cap + 1);
    assert!(
        caps.windows(2).all(|pair| pair[1] >= pair[0]),
        "capacity should not shrink while pushing elements: {caps:?}"
    );

    unsafe { chic_rt_vec_drop(&mut vec) };
}
#[test]
fn vec_with_capacity_allocates_buffer() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec_with_capacity(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        8,
        None,
    );
    assert!(!vec.ptr.is_null());
    assert_eq!(vec.cap, 8);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_push_and_pop_roundtrip() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    for value in 0..4u32 {
        let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr_from(&value)) };
        assert_eq!(status, VecError::Success.as_i32());
    }
    assert_eq!(vec.len, 4);

    for expected in (0..4u32).rev() {
        let mut out = 0u32;
        let status = unsafe { chic_rt_vec_pop(&mut vec, mut_ptr_from(&mut out)) };
        assert_eq!(status, VecError::Success.as_i32());
        assert_eq!(out, expected);
    }
    assert_eq!(vec.len, 0);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_drop_invokes_drop_fn() {
    let _runtime_guard = runtime_test_guard();
    let _drop_guard = drop_counter_guard();
    DROP_COUNT.store(0, Ordering::SeqCst);
    let mut vec = new_vec(
        std::mem::size_of::<DropTracker>(),
        std::mem::align_of::<DropTracker>(),
        Some(drop_tracker),
    );
    let drop_ptr = unsafe { chic_rt_vec_get_drop(&vec) };
    assert!(
        drop_ptr.is_some(),
        "vec_new should preserve drop glue pointer"
    );
    let tracker = DropTracker { value: 42 };
    unsafe {
        let status1 = chic_rt_vec_push(&mut vec, const_ptr_from(&tracker));
        let status2 = chic_rt_vec_push(&mut vec, const_ptr_from(&tracker));
        assert_eq!(status1, VecError::Success.as_i32());
        assert_eq!(status2, VecError::Success.as_i32());
    }
    assert_eq!(vec.len, 2, "vec len should reflect pushes");
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);
    unsafe { chic_rt_vec_drop(&mut vec) };
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 2);
}

#[test]
fn vec_insert_and_remove_behaviour() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    for value in 0..3u32 {
        unsafe {
            chic_rt_vec_push(&mut vec, const_ptr_from(&value));
        }
    }
    let insert_val = 99u32;
    let insert_ptr = const_ptr_from(&insert_val);
    unsafe {
        let status = chic_rt_vec_insert(&mut vec, 1, insert_ptr);
        assert_eq!(status, VecError::Success.as_i32());
    }
    assert_eq!(vec.len, 4);
    let mut removed = 0u32;
    let status = unsafe { chic_rt_vec_remove(&mut vec, 1, mut_ptr_from(&mut removed)) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_eq!(removed, insert_val);
    assert_eq!(vec.len, 3);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_truncate_and_clear_drop_elements() {
    let _runtime_guard = runtime_test_guard();
    let _drop_guard = drop_counter_guard();
    DROP_COUNT.store(0, Ordering::SeqCst);
    let mut vec = new_vec(
        std::mem::size_of::<DropTracker>(),
        std::mem::align_of::<DropTracker>(),
        Some(drop_tracker),
    );
    assert!(
        unsafe { chic_rt_vec_get_drop(&vec) }.is_some(),
        "drop glue should be retained for truncate/clear"
    );
    let tracker = DropTracker { value: 5 };
    for _ in 0..4 {
        unsafe {
            chic_rt_vec_push(&mut vec, const_ptr_from(&tracker));
        }
    }
    unsafe {
        chic_rt_vec_truncate(&mut vec, 2);
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 2);
    unsafe {
        chic_rt_vec_clear(&mut vec);
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 4);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_swap_remove_returns_last() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    for value in 1..=3u32 {
        unsafe {
            chic_rt_vec_push(&mut vec, const_ptr_from(&value));
        }
    }
    let mut out = 0u32;
    let status = unsafe { chic_rt_vec_swap_remove(&mut vec, 1, mut_ptr_from(&mut out)) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_eq!(out, 2);
    assert_eq!(vec.len, 2);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_remove_out_of_bounds_reports_error() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    let status = unsafe {
        chic_rt_vec_remove(
            &mut vec,
            0,
            null_mut_ptr_for(size_of::<u32>(), align_of::<u32>()),
        )
    };
    assert_eq!(status, VecError::OutOfBounds.as_i32());
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_zero_sized_elements_track_len_and_drop() {
    let _runtime_guard = runtime_test_guard();
    let _drop_guard = drop_counter_guard();
    DROP_COUNT.store(0, Ordering::SeqCst);
    let mut vec = new_vec(0, 1, Some(drop_tracker));
    assert!(
        unsafe { chic_rt_vec_get_drop(&vec) }.is_some(),
        "zero-sized elements should still carry drop glue"
    );
    for _ in 0..3 {
        unsafe { chic_rt_vec_push(&mut vec, null_const_ptr_for(0, 1)) };
    }
    assert_eq!(vec.len, 3);
    unsafe { chic_rt_vec_pop(&mut vec, null_mut_ptr_for(0, 1)) };
    assert_eq!(vec.len, 2);
    unsafe { chic_rt_vec_clear(&mut vec) };
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_ptr_at_returns_pointer_to_element() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    for value in [11u32, 27u32, 39u32] {
        unsafe {
            chic_rt_vec_push(&mut vec, const_ptr_from(&value));
        }
    }
    let ptr = unsafe { chic_rt_vec_ptr_at(&vec, 1) };
    assert!(!ptr.ptr.is_null());
    assert_eq!(ptr.size, std::mem::size_of::<u32>());
    assert_eq!(ptr.align, std::mem::align_of::<u32>());
    let data_ptr = ptr.ptr;
    std::hint::black_box(data_ptr);
    let value = unsafe { *(data_ptr as *const u32) };
    assert_eq!(value, 27u32);
    unsafe { chic_rt_vec_drop(&mut vec) };
}

#[test]
fn vec_reserve_overflow_reports_error() {
    let _guard = runtime_test_guard();
    let mut vec = new_vec(
        std::mem::size_of::<u32>(),
        std::mem::align_of::<u32>(),
        None,
    );
    let status = unsafe { chic_rt_vec_reserve(&mut vec, usize::MAX) };
    assert_eq!(status, VecError::CapacityOverflow.as_i32());
    unsafe { chic_rt_vec_drop(&mut vec) };
}
