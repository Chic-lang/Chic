#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; region runtime lives in the native runtime."
);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionHandle {
    pub raw: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegionTelemetry {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub blocks_active: u32,
    pub blocks_total: u32,
}

impl RegionHandle {
    #[must_use]
    pub const fn null() -> Self {
        Self {
            raw: core::ptr::null_mut(),
        }
    }

    #[must_use]
    pub const fn is_null(self) -> bool {
        self.raw.is_null()
    }
}

unsafe impl Send for RegionHandle {}
unsafe impl Sync for RegionHandle {}

mod native;
pub use native::*;
