#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; hash glue runtime lives in the native runtime."
);

pub type HashGlueFn = unsafe extern "C" fn(*const u8) -> u64;

#[repr(C)]
pub struct HashGlueEntry {
    pub type_id: u64,
    pub func: HashGlueFn,
}

unsafe extern "C" {
    pub fn chic_rt_hash_register(type_id: u64, func: Option<HashGlueFn>);
    pub fn chic_rt_hash_clear();
    pub fn chic_rt_install_hash_table(entries: *const HashGlueEntry, len: usize);
    pub fn chic_rt_hash_resolve(type_id: u64) -> Option<HashGlueFn>;
    pub fn chic_rt_hash_invoke(func: Option<HashGlueFn>, value: *const u8) -> u64;
}
