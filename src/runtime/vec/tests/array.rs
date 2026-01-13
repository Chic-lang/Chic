#![allow(unsafe_code, unused_unsafe)]

use super::*;
use crate::runtime::test_lock::runtime_test_guard;

fn vec_with_capacity(
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
fn vec_into_array_zero_copy_when_packed() {
    let _guard = runtime_test_guard();
    let mut vec = vec_with_capacity(
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
        3,
        None,
    );
    for value in 0..3 {
        push_i32(&mut vec, value);
    }
    assert_eq!(vec.cap, 3);
    let original_ptr = vec.ptr;

    let mut array = ChicVec::empty();
    let status = unsafe { chic_rt_vec_into_array(&mut array, &mut vec) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_eq!(array.ptr, original_ptr);
    assert_eq!(array.len, 3);
    assert_eq!(array.cap, 3);
    assert!(vec.ptr.is_null());
    assert_eq!(vec.len, 0);

    unsafe {
        chic_rt_vec_drop(&mut array);
        chic_rt_vec_drop(&mut vec);
    }
}

#[test]
fn vec_into_array_copies_when_extra_capacity() {
    let _guard = runtime_test_guard();
    let mut vec = vec_with_capacity(
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
        8,
        None,
    );
    for value in 0..3 {
        push_i32(&mut vec, value);
    }
    assert!(vec.cap >= 3);
    let original_ptr = vec.ptr;

    let mut array = ChicVec::empty();
    let status = unsafe { chic_rt_vec_into_array(&mut array, &mut vec) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_ne!(array.ptr, original_ptr);
    assert_eq!(array.len, 3);
    assert_eq!(array.cap, 3);
    assert!(vec.ptr.is_null());
    assert_eq!(vec.len, 0);
    assert_eq!(vec.cap, 0);

    unsafe {
        chic_rt_vec_drop(&mut array);
        chic_rt_vec_drop(&mut vec);
    }
}

#[test]
fn vec_into_array_preserves_drop_glue() {
    let _guard = runtime_test_guard();
    let _guard = drop_counter_guard();
    DROP_COUNT.store(0, Ordering::SeqCst);
    let mut vec = vec_with_capacity(
        std::mem::size_of::<DropTracker>(),
        std::mem::align_of::<DropTracker>(),
        4,
        Some(drop_tracker),
    );
    let tracker_ptr = drop_tracker as usize;
    let vec_drop = unsafe { chic_rt_vec_get_drop(&vec) };
    assert_eq!(
        vec_drop.map(|f| f as usize),
        Some(tracker_ptr),
        "vec_with_capacity should preserve drop glue"
    );
    assert!(
        unsafe { chic_rt_vec_get_drop(&vec) }.is_some(),
        "vec_with_capacity should preserve drop glue"
    );
    for value in 0..4 {
        let tracker = DropTracker { value };
        let status = unsafe { chic_rt_vec_push(&mut vec, const_ptr_from(&tracker)) };
        assert_eq!(status, VecError::Success.as_i32());
    }

    let mut array = ChicVec::empty();
    let status = unsafe { chic_rt_vec_into_array(&mut array, &mut vec) };
    assert_eq!(status, VecError::Success.as_i32());
    let array_drop = unsafe { chic_rt_vec_get_drop(&array) };
    assert_eq!(
        array_drop.map(|f| f as usize),
        Some(tracker_ptr),
        "vec_into_array must propagate drop glue"
    );
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);
    assert_eq!(array.len, 4);
    assert_eq!(array.cap, 4);

    unsafe {
        chic_rt_vec_drop(&mut vec);
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);
        chic_rt_vec_drop(&mut array);
    }
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 4);
}

#[test]
fn vec_copy_to_array_shrinks_capacity() {
    let _guard = runtime_test_guard();
    let mut vec = vec_with_capacity(
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
        6,
        None,
    );
    for value in 0..4 {
        push_i32(&mut vec, value);
    }
    let mut array = ChicVec::empty();
    let status = unsafe { chic_rt_vec_copy_to_array(&mut array, &vec) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_eq!(array.len, 4);
    assert_eq!(array.cap, 4);
    assert_eq!(vec.len, 4);
    unsafe {
        chic_rt_vec_drop(&mut array);
        chic_rt_vec_drop(&mut vec);
    }
}

#[test]
fn array_into_vec_moves_storage() {
    let _guard = runtime_test_guard();
    let mut array = vec_with_capacity(
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
        3,
        None,
    );
    for value in 0..3 {
        push_i32(&mut array, value);
    }
    let original_ptr = array.ptr;

    let mut vec = ChicVec::empty();
    let status = unsafe { chic_rt_array_into_vec(&mut vec, &mut array) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_eq!(vec.ptr, original_ptr);
    assert_eq!(vec.len, 3);
    assert!(array.ptr.is_null());
    unsafe {
        chic_rt_vec_drop(&mut vec);
        chic_rt_vec_drop(&mut array);
    }
}

#[test]
fn array_copy_to_vec_clones_buffer() {
    let _guard = runtime_test_guard();
    let mut array = vec_with_capacity(
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
        3,
        None,
    );
    for value in 0..3 {
        push_i32(&mut array, value);
    }
    let original_ptr = array.ptr;

    let mut vec = ChicVec::empty();
    let status = unsafe { chic_rt_array_copy_to_vec(&mut vec, &array) };
    assert_eq!(status, VecError::Success.as_i32());
    assert_ne!(vec.ptr, original_ptr);
    assert_eq!(vec.len, 3);
    unsafe {
        chic_rt_vec_drop(&mut vec);
        chic_rt_vec_drop(&mut array);
    }
}
