#![allow(unsafe_code)]

const _: () = assert!(
    cfg!(chic_native_runtime),
    "chic_native_runtime must be enabled; eq glue runtime lives in the native runtime."
);

pub type EqGlueFn = unsafe extern "C" fn(*const u8, *const u8) -> i32;

#[repr(C)]
pub struct EqGlueEntry {
    pub type_id: u64,
    pub func: EqGlueFn,
}

unsafe extern "C" {
    pub fn chic_rt_eq_register(type_id: u64, func: Option<EqGlueFn>);
    pub fn chic_rt_eq_clear();
    pub fn chic_rt_install_eq_table(entries: *const EqGlueEntry, len: usize);
    pub fn chic_rt_eq_resolve(type_id: u64) -> Option<EqGlueFn>;
    pub fn chic_rt_eq_invoke(func: Option<EqGlueFn>, left: *const u8, right: *const u8) -> i32;
}
