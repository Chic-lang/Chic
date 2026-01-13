#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; Rust type metadata runtime has been removed."
);

mod native {
    use core::{ptr, slice};

    #[repr(i32)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TypeMetadataStatus {
        Success = 0,
        NotFound = 1,
        InvalidPointer = 2,
    }

    #[repr(u8)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum RuntimeGenericVariance {
        Invariant = 0,
        Covariant = 1,
        Contravariant = 2,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct VarianceSlice {
        pub ptr: *const RuntimeGenericVariance,
        pub len: usize,
    }

    impl VarianceSlice {
        pub const EMPTY: Self = Self {
            ptr: ptr::null(),
            len: 0,
        };

        pub fn as_slice(&self) -> &'static [RuntimeGenericVariance] {
            if self.ptr.is_null() || self.len == 0 {
                &[]
            } else {
                // Safety: caller guarantees pointer/length are valid.
                unsafe { slice::from_raw_parts(self.ptr, self.len) }
            }
        }
    }

    unsafe impl Send for VarianceSlice {}
    unsafe impl Sync for VarianceSlice {}

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct TypeMetadataEntry {
        pub type_id: u64,
        pub size: usize,
        pub align: usize,
        pub drop_fn: isize,
        pub variance: VarianceSlice,
        pub flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    pub struct RuntimeTypeMetadata {
        pub size: usize,
        pub align: usize,
        pub drop_fn: isize,
        pub variance: VarianceSlice,
        pub flags: u32,
    }

    unsafe extern "C" {
        pub fn chic_rt_type_size(type_id: u64) -> usize;
        pub fn chic_rt_type_align(type_id: u64) -> usize;
        pub fn chic_rt_type_drop_glue(type_id: u64) -> isize;
        pub fn chic_rt_type_clone_glue(type_id: u64) -> isize;
        pub fn chic_rt_type_hash_glue(type_id: u64) -> isize;
        pub fn chic_rt_type_eq_glue(type_id: u64) -> isize;
        pub fn chic_rt_install_type_metadata(entries: *const TypeMetadataEntry, len: usize);
        pub fn chic_rt_type_metadata(type_id: u64, out_metadata: *mut RuntimeTypeMetadata) -> i32;
        pub fn chic_rt_type_metadata_register(type_id: u64, metadata: RuntimeTypeMetadata);
        pub fn chic_rt_type_metadata_clear();
    }
}

pub use native::*;
