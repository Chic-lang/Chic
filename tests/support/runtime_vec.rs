use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;

use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};
use chic::runtime::vec::{ChicVec, chic_rt_vec_drop, chic_rt_vec_new, chic_rt_vec_push};

/// RAII wrapper that manages the lifetime of a `ChicVec`.
pub struct ManagedVec<T> {
    repr: ChicVec,
    _marker: PhantomData<T>,
}

impl<T> ManagedVec<T> {
    /// Create an empty vector specialised for `T`.
    pub fn new() -> Self {
        let repr = unsafe { chic_rt_vec_new(size_of::<T>(), align_of::<T>(), None) };
        Self {
            repr,
            _marker: PhantomData,
        }
    }

    /// Push a value into the vector.
    pub fn push(&mut self, value: T) {
        let handle = value_const_ptr(&value);
        let status = unsafe { chic_rt_vec_push(&mut self.repr, handle) };
        if status != 0 {
            dbg!(handle);
            dbg!(self.repr);
        }
        assert_eq!(
            status,
            0,
            "vector push should succeed (status {status}, header {:?})",
            self.as_value_const_ptr()
        );
    }

    /// Borrow the underlying representation mutably.
    pub fn as_mut_ptr(&mut self) -> *mut ChicVec {
        &mut self.repr
    }

    /// Borrow the underlying representation immutably.
    pub fn as_ptr(&self) -> *const ChicVec {
        &self.repr
    }

    /// Length helper for tests.
    pub fn len(&self) -> usize {
        self.repr.len
    }

    /// Build a typed mutable handle to the vector's backing buffer.
    pub fn as_value_mut_ptr(&self) -> ValueMutPtr {
        let ptr = if self.repr.elem_size == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            self.repr.ptr
        };
        ValueMutPtr {
            ptr,
            size: self.repr.elem_size,
            align: self.repr.elem_align,
        }
    }

    /// Build a typed const handle to the vector's backing buffer.
    pub fn as_value_const_ptr(&self) -> ValueConstPtr {
        let ptr = if self.repr.elem_size == 0 {
            NonNull::<u8>::dangling().as_ptr()
        } else {
            self.repr.ptr
        };
        ValueConstPtr {
            ptr,
            size: self.repr.elem_size,
            align: self.repr.elem_align,
        }
    }
}

impl<T> Drop for ManagedVec<T> {
    fn drop(&mut self) {
        unsafe { chic_rt_vec_drop(&mut self.repr) };
    }
}

fn value_const_ptr<T>(value: &T) -> ValueConstPtr {
    ValueConstPtr {
        ptr: (value as *const T).cast::<u8>(),
        size: size_of::<T>(),
        align: align_of::<T>(),
    }
}
