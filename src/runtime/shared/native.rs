#![allow(unsafe_code)]

use crate::runtime::{ChicArc, ChicRc, ChicWeak, ChicWeakRc, drop_glue::DropGlueFn};

unsafe extern "C" {
    pub fn chic_rt_object_new(type_id: u64) -> *mut u8;

    pub fn chic_rt_arc_new(
        dest: *mut ChicArc,
        src: *const u8,
        size: usize,
        align: usize,
        drop_fn: Option<DropGlueFn>,
        type_id: u64,
    ) -> i32;
    pub fn chic_rt_arc_clone(dest: *mut ChicArc, src: *const ChicArc) -> i32;
    pub fn chic_rt_arc_drop(target: *mut ChicArc);
    pub fn chic_rt_arc_get(src: *const ChicArc) -> *const u8;
    pub fn chic_rt_arc_get_mut(src: *mut ChicArc) -> *mut u8;
    pub fn chic_rt_arc_get_data(handle: *const ChicArc) -> *const u8;
    pub fn chic_rt_arc_strong_count(src: *const ChicArc) -> usize;
    pub fn chic_rt_arc_weak_count(src: *const ChicArc) -> usize;
    pub fn chic_rt_arc_downgrade(dest: *mut ChicWeak, src: *const ChicArc) -> i32;
    pub fn chic_rt_weak_clone(dest: *mut ChicWeak, src: *const ChicWeak) -> i32;
    pub fn chic_rt_weak_drop(target: *mut ChicWeak);
    pub fn chic_rt_weak_upgrade(dest: *mut ChicArc, src: *const ChicWeak) -> i32;

    pub fn chic_rt_rc_new(
        dest: *mut ChicRc,
        src: *const u8,
        size: usize,
        align: usize,
        drop_fn: Option<DropGlueFn>,
        type_id: u64,
    ) -> i32;
    pub fn chic_rt_rc_clone(dest: *mut ChicRc, src: *const ChicRc) -> i32;
    pub fn chic_rt_rc_drop(target: *mut ChicRc);
    pub fn chic_rt_rc_get(src: *const ChicRc) -> *const u8;
    pub fn chic_rt_rc_get_mut(src: *mut ChicRc) -> *mut u8;
    pub fn chic_rt_rc_strong_count(src: *const ChicRc) -> usize;
    pub fn chic_rt_rc_weak_count(src: *const ChicRc) -> usize;
    pub fn chic_rt_rc_downgrade(dest: *mut ChicWeakRc, src: *const ChicRc) -> i32;
    pub fn chic_rt_weak_rc_clone(dest: *mut ChicWeakRc, src: *const ChicWeakRc) -> i32;
    pub fn chic_rt_weak_rc_drop(target: *mut ChicWeakRc);
    pub fn chic_rt_weak_rc_upgrade(dest: *mut ChicRc, src: *const ChicWeakRc) -> i32;
}
