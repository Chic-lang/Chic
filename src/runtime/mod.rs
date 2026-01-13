const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; the Rust runtime backend is removed."
);

pub mod async_runtime;
pub mod async_runtime_native;
pub mod backend;
pub mod character;
pub mod clone;
#[allow(unsafe_code)]
pub mod closure;
pub mod decimal;
pub mod drop_glue;
pub mod eq_glue;
pub mod error;
pub mod ffi;
pub mod flags;
pub mod float_env;
pub mod float_ops;
pub mod hash_glue;
pub mod int128;
pub mod interface_defaults;
#[allow(unsafe_code)]
pub mod memory;
pub mod numeric;
#[allow(unsafe_code)]
pub mod region;
#[allow(unsafe_code)]
pub mod shared;
#[allow(unsafe_code)]
pub mod span;
pub mod startup;
#[allow(unsafe_code)]
pub mod string;
pub mod sync;
pub mod test_executor;
#[cfg(test)]
pub mod test_lock;
#[allow(unsafe_code)]
pub mod thread;
pub mod tracing;
pub mod type_metadata;
pub mod value_ptr;
#[allow(unsafe_code)]
pub mod vec;
#[allow(unsafe_code)]
pub mod wasm_executor;

mod exports;

pub use backend::{RuntimeBackend, runtime_backend, runtime_backend_name};
pub use exports::*;
