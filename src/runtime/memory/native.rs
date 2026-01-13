#![allow(unsafe_code)]

use crate::runtime::memory::{AllocationTelemetry, AllocatorVTable};
use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

// External bindings to the Chic-native allocator/memory shims. When the native
// runtime archive is linked, the Rust implementation is skipped and these
// symbols are provided by the Chic runtime.
unsafe extern "C" {
    pub fn chic_rt_allocator_install(vtable: AllocatorVTable);
    pub fn chic_rt_allocator_reset();
    pub fn chic_rt_alloc(size: usize, align: usize) -> ValueMutPtr;
    pub fn chic_rt_alloc_zeroed(size: usize, align: usize) -> ValueMutPtr;
    pub fn chic_rt_realloc(
        ptr: ValueMutPtr,
        old_size: usize,
        new_size: usize,
        align: usize,
    ) -> ValueMutPtr;
    pub fn chic_rt_free(ptr: ValueMutPtr);
    pub fn chic_rt_alloc_stats() -> AllocationTelemetry;
    pub fn chic_rt_reset_alloc_stats();
    pub fn chic_rt_memcpy(dst: ValueMutPtr, src: ValueConstPtr, len: usize);
    pub fn chic_rt_memmove(dst: ValueMutPtr, src: ValueMutPtr, len: usize);
    pub fn chic_rt_memset(dst: ValueMutPtr, value: u8, len: usize);
}
