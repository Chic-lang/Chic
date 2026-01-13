#![allow(unsafe_code)]

use std::ffi::c_void;

pub use crate::runtime::async_runtime_native::*;

const _: () = assert!(
    cfg!(chic_native_runtime),
    "Tokio-based async runtime has been removed; `chic_native_runtime` is required."
);

#[repr(C)]
pub struct FutureHeader {
    pub state_pointer: isize,
    pub vtable_pointer: isize,
    pub executor_context: isize,
    pub flags: u32,
}

#[repr(C)]
pub struct FutureVTable {
    pub poll: unsafe extern "C" fn(*mut FutureHeader, *mut RuntimeContext) -> u32,
    pub drop: unsafe extern "C" fn(*mut FutureHeader),
}

#[repr(C)]
pub struct RuntimeContext {
    pub(crate) inner: *mut c_void,
}

#[cfg(test)]
mod tests {
    use crate::async_flags::{
        AWAIT_STATUS_PENDING, AWAIT_STATUS_READY, AwaitStatus, FUTURE_FLAG_READY,
    };

    #[test]
    fn await_status_values_match_flags() {
        assert_eq!(AwaitStatus::Pending as u32, AWAIT_STATUS_PENDING);
        assert_eq!(AwaitStatus::Ready as u32, AWAIT_STATUS_READY);
        assert_eq!(AWAIT_STATUS_READY, FUTURE_FLAG_READY);
    }
}
