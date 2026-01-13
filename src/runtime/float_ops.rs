//! Basic floating-point helpers surfaced as runtime hooks for backends that
//! lack direct instructions (e.g. WASM without f32/f64 remainder).

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; float runtime lives in the native runtime."
);

mod native {
    unsafe extern "C" {
        pub fn chic_rt_f32_rem(lhs: f32, rhs: f32) -> f32;
        pub fn chic_rt_f64_rem(lhs: f64, rhs: f64) -> f64;
    }
}

pub use native::*;
