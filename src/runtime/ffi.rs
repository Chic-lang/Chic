#![allow(unsafe_code)]

//! FFI surface exposed to the Chic-native runtime.
//!
//! The Rust fallback loader has been removed; all semantics live in the native
//! runtime and this module only provides the C ABI surface for the compiler and
//! backends.

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; Rust FFI resolver has been removed."
);

use std::os::raw::c_char;

mod native;
pub use native::*;

/// Descriptor shape used by the compiler when emitting FFI metadata.
#[repr(C)]
pub struct ChicFfiDescriptor {
    pub library: *const c_char,
    pub symbol: *const c_char,
    pub convention: u32,
    pub binding: u32,
    pub optional: bool,
}
