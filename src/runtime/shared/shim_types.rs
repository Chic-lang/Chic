#![allow(unsafe_code)]

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ChicArc {
    pub ptr: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ChicWeak {
    pub ptr: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ChicRc {
    pub ptr: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ChicWeakRc {
    pub ptr: *mut u8,
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedError {
    Success = 0,
    InvalidPointer = -1,
    AllocationFailed = -2,
    Overflow = -3,
}
