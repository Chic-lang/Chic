#![allow(unsafe_code)]

//! Runtime-facing helpers for typed exception propagation. The Chic-native runtime
//! owns all semantics; this module is a thin ABI declaration surface.

use blake3::Hash;

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; exception runtime lives in the native runtime."
);

/// Captured metadata for a runtime `throw` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeThrownException {
    /// Pointer-sized payload address forwarded by the backend.
    pub payload: u64,
    /// Stable type identity computed from the exception's canonical name.
    pub type_id: u64,
}

/// Compute a stable 64-bit identity for a Chic exception type.
#[must_use]
pub fn exception_type_identity(name: &str) -> u64 {
    let normalized = match name {
        "Exception" | "System::Exception" | "Std::Exception" | "Error" | "Std::Error" => {
            "System::Error"
        }
        other => other,
    };
    let digest: Hash = blake3::hash(normalized.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

pub use native::*;

mod native {
    unsafe extern "C" {
        pub fn chic_rt_throw(payload: i64, type_id: i64);
        pub fn chic_rt_has_pending_exception() -> i32;
        pub fn chic_rt_peek_pending_exception(payload: *mut i64, type_id: *mut i64) -> i32;
        pub fn chic_rt_clear_pending_exception();
        pub fn chic_rt_take_pending_exception(payload: *mut i64, type_id: *mut i64) -> i32;
        pub fn chic_rt_abort_unhandled_exception();
    }
}

#[must_use]
pub fn take_pending_exception() -> Option<RuntimeThrownException> {
    unsafe {
        let mut payload = 0i64;
        let mut type_id = 0i64;
        if chic_rt_take_pending_exception(&mut payload, &mut type_id) == 0 {
            None
        } else {
            Some(RuntimeThrownException {
                payload: payload as u64,
                type_id: type_id as u64,
            })
        }
    }
}

#[must_use]
pub fn peek_pending_exception() -> Option<RuntimeThrownException> {
    unsafe {
        let mut payload = 0i64;
        let mut type_id = 0i64;
        if chic_rt_peek_pending_exception(&mut payload, &mut type_id) == 0 {
            None
        } else {
            Some(RuntimeThrownException {
                payload: payload as u64,
                type_id: type_id as u64,
            })
        }
    }
}
