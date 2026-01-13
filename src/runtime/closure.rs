#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; closure runtime lives in the native runtime."
);

mod native {
    unsafe extern "C" {
        pub fn chic_rt_closure_env_alloc(size: usize, align: usize) -> *mut u8;
        pub fn chic_rt_closure_env_free(ptr: *mut u8, size: usize, align: usize);
        pub fn chic_rt_closure_env_clone(src: *const u8, size: usize, align: usize) -> *mut u8;
    }
}

pub use native::*;
