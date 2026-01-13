#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; value pointer helpers live in the native runtime."
);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueConstPtr {
    pub ptr: *const u8,
    pub size: usize,
    pub align: usize,
}

impl ValueConstPtr {
    #[inline]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueMutPtr {
    pub ptr: *mut u8,
    pub size: usize,
    pub align: usize,
}

impl ValueMutPtr {
    #[inline]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}
