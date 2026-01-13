#![allow(unsafe_code)]

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicString {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
    pub inline: [u8; ChicString::INLINE_CAPACITY],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicStr {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicCharSpan {
    pub ptr: *const u16,
    pub len: usize,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringError {
    Success = 0,
    Utf8 = 1,
    CapacityOverflow = 2,
    AllocationFailed = 3,
    InvalidPointer = 4,
    OutOfBounds = 5,
}

impl ChicString {
    pub const INLINE_CAPACITY: usize = 32;

    #[inline]
    pub fn is_inline(&self) -> bool {
        const INLINE_TAG: usize = usize::MAX ^ (usize::MAX >> 1);
        (self.cap & INLINE_TAG) != 0 && self.len <= Self::INLINE_CAPACITY
    }
}

impl ChicStr {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            ptr: core::ptr::null(),
            len: 0,
        }
    }
}
