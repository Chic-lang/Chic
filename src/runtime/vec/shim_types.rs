#![allow(unsafe_code, dead_code)]

use std::mem::MaybeUninit;
use std::ptr;

use crate::runtime::region::RegionHandle;

pub type VecDropFn = unsafe extern "C" fn(*mut u8);

const INLINE_BYTES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InlineBuffer([u8; INLINE_BYTES]);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicVec {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
    pub elem_size: usize,
    pub elem_align: usize,
    pub drop_fn: Option<VecDropFn>,
    pub region: RegionHandle,
    pub uses_inline: u8,
    pub inline_pad: [u8; 7],
    pub inline_storage: MaybeUninit<InlineBuffer>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicVecView {
    pub data: *const u8,
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}

impl Default for ChicVecView {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
            elem_size: 0,
            elem_align: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicVecIter {
    pub data: *const u8,
    pub index: usize,
    pub len: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VecError {
    Success = 0,
    AllocationFailed = 1,
    InvalidPointer = 2,
    CapacityOverflow = 3,
    OutOfBounds = 4,
    LengthOverflow = 5,
    IterationComplete = 6,
}

impl VecError {
    pub(crate) const fn as_i32(self) -> i32 {
        self as i32
    }
}

impl ChicVec {
    pub const fn empty() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
            elem_size: 0,
            elem_align: 1,
            drop_fn: None,
            region: RegionHandle::null(),
            uses_inline: 0,
            inline_pad: [0; 7],
            inline_storage: MaybeUninit::uninit(),
        }
    }

    fn inline_available(&self) -> bool {
        self.elem_size > 0 && self.elem_size <= INLINE_BYTES
    }

    pub fn inline_capacity(&self) -> usize {
        if !self.inline_available() {
            0
        } else {
            (INLINE_BYTES / self.elem_size).max(1)
        }
    }

    pub fn uses_inline_storage(&self) -> bool {
        self.inline_available() && self.uses_inline != 0
    }
}
