pub type HashSetDropFn = unsafe extern "C" fn(*mut u8);
pub type HashSetEqFn = unsafe extern "C" fn(*const u8, *const u8) -> i32;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicHashSet {
    pub entries: *mut u8,
    pub states: *mut u8,
    pub hashes: *mut u8,
    pub len: usize,
    pub cap: usize,
    pub tombstones: usize,
    pub elem_size: usize,
    pub elem_align: usize,
    pub drop_fn: Option<HashSetDropFn>,
    pub eq_fn: Option<HashSetEqFn>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Layout mirror; iterators are exercised in runtime code, not in every Rust test.
pub struct ChicHashSetIter {
    pub entries: *const u8,
    pub states: *const u8,
    pub index: usize,
    pub cap: usize,
    pub elem_size: usize,
    pub elem_align: usize,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Error surface mirrors the runtime; most variants are only hit in FFI paths.
pub enum HashSetError {
    Success = 0,
    AllocationFailed = 1,
    InvalidPointer = 2,
    CapacityOverflow = 3,
    NotFound = 4,
    IterationComplete = 5,
}

pub type HashMapDropFn = unsafe extern "C" fn(*mut u8);
pub type HashMapEqFn = unsafe extern "C" fn(*const u8, *const u8) -> i32;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChicHashMap {
    pub entries: *mut u8,
    pub states: *mut u8,
    pub hashes: *mut u8,
    pub len: usize,
    pub cap: usize,
    pub tombstones: usize,
    pub key_size: usize,
    pub key_align: usize,
    pub value_size: usize,
    pub value_align: usize,
    pub entry_size: usize,
    pub value_offset: usize,
    pub key_drop_fn: Option<HashMapDropFn>,
    pub value_drop_fn: Option<HashMapDropFn>,
    pub key_eq_fn: Option<HashMapEqFn>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // Layout mirror; iterators are exercised in runtime code, not in every Rust test.
pub struct ChicHashMapIter {
    pub entries: *const u8,
    pub states: *const u8,
    pub index: usize,
    pub cap: usize,
    pub entry_size: usize,
    pub key_size: usize,
    pub key_align: usize,
    pub value_size: usize,
    pub value_align: usize,
    pub value_offset: usize,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Error surface mirrors the runtime; most variants are only hit in FFI paths.
pub enum HashMapError {
    Success = 0,
    AllocationFailed = 1,
    InvalidPointer = 2,
    CapacityOverflow = 3,
    NotFound = 4,
    IterationComplete = 5,
}
