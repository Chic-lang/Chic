use std::slice;

use chic::runtime::string::{
    ChicStr, ChicString, chic_rt_string_as_slice, chic_rt_string_drop, chic_rt_string_from_slice,
    chic_rt_string_new, chic_rt_string_with_capacity,
};

/// RAII wrapper that manages the lifetime of a `ChicString`.
pub struct ManagedString {
    repr: ChicString,
}

impl ManagedString {
    /// Construct an empty string using `chic_rt_string_new`.
    pub fn new() -> Self {
        Self {
            repr: unsafe { chic_rt_string_new() },
        }
    }

    /// Construct a string with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            repr: unsafe { chic_rt_string_with_capacity(capacity) },
        }
    }

    /// Construct a string by copying from the provided `&str`.
    pub fn from_str(text: &str) -> Self {
        let slice = str_to_chic(text);
        let repr = unsafe { chic_rt_string_from_slice(slice) };
        Self { repr }
    }

    /// Borrow the underlying representation.
    pub fn as_raw(&self) -> &ChicString {
        &self.repr
    }

    /// View the contents as a Rust `&str`.
    pub unsafe fn as_rust_str(&self) -> &str {
        if self.repr.len == 0 {
            return "";
        }
        unsafe {
            let view = chic_rt_string_as_slice(&self.repr);
            let slice = slice::from_raw_parts(view.ptr, view.len);
            std::str::from_utf8(slice).expect("runtime string should produce utf-8")
        }
    }

    /// Obtain a mutable pointer that can be passed to FFI helpers.
    pub fn as_mut_ptr(&mut self) -> *mut ChicString {
        &mut self.repr
    }
}

impl Drop for ManagedString {
    fn drop(&mut self) {
        unsafe { chic_rt_string_drop(&mut self.repr) };
        self.repr.ptr = std::ptr::null_mut();
        self.repr.len = 0;
        self.repr.cap = 0;
    }
}

/// Helper to build a `ChicStr` from a Rust `&str`.
#[must_use]
pub fn str_to_chic(text: &str) -> ChicStr {
    ChicStr {
        ptr: text.as_ptr(),
        len: text.len(),
    }
}

/// Helper to build a `ChicStr` from raw bytes.
#[must_use]
pub fn bytes_to_chic(bytes: &[u8]) -> ChicStr {
    ChicStr {
        ptr: bytes.as_ptr(),
        len: bytes.len(),
    }
}
