#![allow(unsafe_code)]

use super::*;
use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use std::mem::{align_of, size_of};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};

pub(super) static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);
static DROP_COUNT_LOCK: Mutex<()> = Mutex::new(());

pub(super) struct DropCounterGuard {
    _guard: MutexGuard<'static, ()>,
}

pub(super) fn drop_counter_guard() -> DropCounterGuard {
    DropCounterGuard {
        _guard: DROP_COUNT_LOCK.lock().expect("drop counter mutex poisoned"),
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct DropTracker {
    pub value: i32,
}

pub(super) unsafe extern "C" fn drop_tracker(_ptr: *mut u8) {
    DROP_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub(super) fn push_i32(vec: &mut ChicVec, value: i32) {
    let status = unsafe { chic_rt_vec_push(vec, const_ptr_from(&value)) };
    assert_eq!(status, VecError::Success.as_i32());
}

pub(super) fn const_ptr_from<T>(value: &T) -> ValueConstPtr {
    ValueConstPtr {
        ptr: (value as *const T).cast::<u8>(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}

pub(super) fn mut_ptr_from<T>(value: &mut T) -> ValueMutPtr {
    ValueMutPtr {
        ptr: (value as *mut T).cast::<u8>(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}

pub(super) fn null_const_ptr_for(size: usize, align: usize) -> ValueConstPtr {
    ValueConstPtr {
        ptr: std::ptr::null(),
        size,
        align,
    }
}

pub(super) fn null_mut_ptr_for(size: usize, align: usize) -> ValueMutPtr {
    ValueMutPtr {
        ptr: std::ptr::null_mut(),
        size,
        align,
    }
}

mod array;
mod iteration;
mod mutation;

#[test]
fn vec_field_intrinsics_roundtrip() {
    let mut vec = ChicVec::empty();
    unsafe {
        chic_rt_vec_set_elem_size(&mut vec, 4);
        chic_rt_vec_set_elem_align(&mut vec, 4);
        chic_rt_vec_set_len(&mut vec, 3);
        chic_rt_vec_set_cap(&mut vec, 8);
        chic_rt_vec_mark_inline(&mut vec, 1);
    }
    assert_eq!(unsafe { chic_rt_vec_elem_size(&vec) }, 4);
    assert_eq!(unsafe { chic_rt_vec_elem_align(&vec) }, 4);
    assert_eq!(unsafe { chic_rt_vec_len(&vec) }, 3);
    assert_eq!(unsafe { chic_rt_vec_capacity(&vec) }, 8);
    assert_eq!(unsafe { chic_rt_vec_uses_inline(&vec) }, 1);
}
