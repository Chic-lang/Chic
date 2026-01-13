#![allow(unsafe_code)]

use crate::runtime::region::{RegionHandle, RegionTelemetry};
use crate::runtime::value_ptr::ValueMutPtr;

// External bindings to the Chic-native region allocator. When the native
// runtime archive is linked, the Rust implementation is skipped and these
// externs link against the Chic runtime.
unsafe extern "C" {
    pub fn chic_rt_region_enter(profile: u64) -> RegionHandle;
    pub fn chic_rt_region_exit(handle: RegionHandle);
    pub fn chic_rt_region_alloc(handle: RegionHandle, size: usize, align: usize) -> ValueMutPtr;
    pub fn chic_rt_region_alloc_zeroed(
        handle: RegionHandle,
        size: usize,
        align: usize,
    ) -> ValueMutPtr;
    pub fn chic_rt_region_telemetry(handle: RegionHandle) -> RegionTelemetry;
    pub fn chic_rt_region_reset_stats(handle: RegionHandle);
}
