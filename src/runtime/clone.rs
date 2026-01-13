#![allow(unsafe_code)]

use crate::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; clone runtime lives in the native runtime."
);

mod native {
    use super::{ValueConstPtr, ValueMutPtr};

    unsafe extern "C" {
        pub fn chic_rt_clone_invoke(glue: isize, src: ValueConstPtr, dest: ValueMutPtr);
    }
}

pub use native::*;
