const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; int128 runtime lives in the native runtime."
);

mod native;
pub use native::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Int128Parts {
    pub lo: u64,
    pub hi: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UInt128Parts {
    pub lo: u64,
    pub hi: u64,
}
