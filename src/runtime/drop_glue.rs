#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; drop glue runtime lives in the native runtime."
);

pub type DropGlueFn = unsafe extern "C" fn(*mut u8);

#[repr(C)]
pub struct DropGlueEntry {
    pub type_id: u64,
    pub func: DropGlueFn,
}

unsafe extern "C" {
    pub fn chic_rt_drop_missing(ptr: *mut u8);
    pub fn __drop_noop(ptr: *mut u8);
    pub fn chic_rt_drop_invoke(func: Option<DropGlueFn>, value: *mut u8);
    pub fn chic_rt_drop_noop_ptr() -> Option<DropGlueFn>;
    pub fn chic_rt_drop_register(type_id: u64, func: Option<DropGlueFn>);
    pub fn chic_rt_drop_clear();
    pub fn chic_rt_install_drop_table(entries: *const DropGlueEntry, len: usize);
    pub fn chic_rt_drop_resolve(type_id: u64) -> Option<DropGlueFn>;
}
