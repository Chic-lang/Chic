use chic::runtime::value_ptr::{ValueConstPtr, ValueMutPtr};

use crate::collections_runtime_types::{
    ChicHashMap, ChicHashSet, HashMapDropFn, HashMapEqFn, HashSetDropFn, HashSetEqFn,
};

unsafe extern "C" {
    pub fn chic_rt_hashset_new(
        elem_size: usize,
        elem_align: usize,
        drop_fn: Option<HashSetDropFn>,
        eq_fn: Option<HashSetEqFn>,
    ) -> ChicHashSet;
    pub fn chic_rt_hashset_reserve(set: *mut ChicHashSet, additional: usize) -> i32;
    pub fn chic_rt_hashset_len(set: *const ChicHashSet) -> usize;
    pub fn chic_rt_hashset_contains(
        set: *const ChicHashSet,
        hash: u64,
        key: *const ValueConstPtr,
    ) -> i32;
    pub fn chic_rt_hashset_insert(
        set: *mut ChicHashSet,
        hash: u64,
        value: *const ValueConstPtr,
        inserted: *mut i32,
    ) -> i32;
    pub fn chic_rt_hashset_remove(
        set: *mut ChicHashSet,
        hash: u64,
        key: *const ValueConstPtr,
    ) -> i32;
    pub fn chic_rt_hashset_drop(set: *mut ChicHashSet);

    pub fn chic_rt_hashmap_new(
        key_size: usize,
        key_align: usize,
        value_size: usize,
        value_align: usize,
        key_drop_fn: Option<HashMapDropFn>,
        value_drop_fn: Option<HashMapDropFn>,
        key_eq_fn: Option<HashMapEqFn>,
    ) -> ChicHashMap;
    pub fn chic_rt_hashmap_reserve(map: *mut ChicHashMap, additional: usize) -> i32;
    pub fn chic_rt_hashmap_len(map: *const ChicHashMap) -> usize;
    pub fn chic_rt_hashmap_contains(
        map: *const ChicHashMap,
        hash: u64,
        key: *const ValueConstPtr,
    ) -> i32;
    pub fn chic_rt_hashmap_insert(
        map: *mut ChicHashMap,
        hash: u64,
        key: *const ValueConstPtr,
        value: *const ValueConstPtr,
        previous: *const ValueMutPtr,
        replaced: *mut i32,
    ) -> i32;
    pub fn chic_rt_hashmap_take(
        map: *mut ChicHashMap,
        hash: u64,
        key: *const ValueConstPtr,
        out: *const ValueMutPtr,
    ) -> i32;
    pub fn chic_rt_hashmap_remove(
        map: *mut ChicHashMap,
        hash: u64,
        key: *const ValueConstPtr,
    ) -> i32;
    pub fn chic_rt_hashmap_drop(map: *mut ChicHashMap);
}
