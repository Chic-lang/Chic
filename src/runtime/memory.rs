#![allow(unsafe_code)]

use crate::runtime::value_ptr::ValueMutPtr;

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; memory runtime lives in the native runtime."
);

mod native;
pub use native::*;

/// Optional custom allocator hooks installed by the native runtime.
type AllocFn = unsafe extern "C" fn(*mut u8, usize, usize) -> ValueMutPtr;
type ReallocFn = unsafe extern "C" fn(*mut u8, ValueMutPtr, usize, usize, usize) -> ValueMutPtr;
type FreeFn = unsafe extern "C" fn(*mut u8, ValueMutPtr);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AllocatorVTable {
    pub context: *mut u8,
    pub alloc: Option<AllocFn>,
    pub alloc_zeroed: Option<AllocFn>,
    pub realloc: Option<ReallocFn>,
    pub free: Option<FreeFn>,
}

unsafe impl Send for AllocatorVTable {}
unsafe impl Sync for AllocatorVTable {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AllocationTelemetry {
    pub alloc_calls: usize,
    pub alloc_zeroed_calls: usize,
    pub realloc_calls: usize,
    pub free_calls: usize,
    pub alloc_bytes: usize,
    pub alloc_zeroed_bytes: usize,
    pub realloc_bytes: usize,
    pub freed_bytes: usize,
}
