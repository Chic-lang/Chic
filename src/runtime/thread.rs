#![allow(unsafe_code)]

use crate::runtime::value_ptr::ValueMutPtr;

#[repr(C)]
pub struct ThreadStart {
    pub context: ValueMutPtr,
    pub name: ValueMutPtr,
    pub has_name: bool,
    pub use_thread_id_name: bool,
}

#[repr(C)]
#[derive(Default)]
pub struct ThreadHandle {
    pub raw: *mut u8,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadStatus {
    Success = 0,
    NotSupported = 1,
    Invalid = 2,
    SpawnFailed = 3,
}

// Chic-owned thread exports are provided by the native runtime archive built from
// Std.Platform.Thread. No Rust runtime semantics remain here.
unsafe extern "C" {
    pub fn chic_rt_thread_spawn(
        start: *const ThreadStart,
        handle: *mut ThreadHandle,
    ) -> ThreadStatus;
    pub fn chic_rt_thread_join(handle: *mut ThreadHandle) -> ThreadStatus;
    pub fn chic_rt_thread_detach(handle: *mut ThreadHandle) -> ThreadStatus;
    pub fn chic_rt_thread_sleep_ms(millis: u64);
    pub fn chic_rt_thread_yield();
    pub fn chic_rt_thread_spin_wait(iterations: u32);
}
