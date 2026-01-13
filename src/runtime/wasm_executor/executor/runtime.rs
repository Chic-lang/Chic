#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BorrowRuntimeKind {
    Shared,
    Unique,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BorrowRuntimeRecord {
    pub(crate) kind: BorrowRuntimeKind,
    pub(crate) address: u32,
    pub(crate) ref_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct BorrowRuntimeKey {
    pub(crate) borrow_id: i32,
    pub(crate) function: u32,
    pub(crate) frame_depth: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmStringRepr {
    pub(crate) ptr: u32,
    pub(crate) len: u32,
    pub(crate) cap: u32,
}

pub(crate) const STRING_EMPTY_PTR: u32 = 0;
pub(crate) const STRING_INLINE_CAPACITY: u32 = 32;
pub(crate) const STRING_INLINE_TAG: u32 = u32::MAX ^ (u32::MAX >> 1);

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmVecRepr {
    pub(crate) ptr: u32,
    pub(crate) len: u32,
    pub(crate) cap: u32,
    pub(crate) elem_size: u32,
    pub(crate) elem_align: u32,
    pub(crate) drop_fn: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmHashSetRepr {
    pub(crate) entries: u32,
    pub(crate) states: u32,
    pub(crate) hashes: u32,
    pub(crate) len: u32,
    pub(crate) cap: u32,
    pub(crate) tombstones: u32,
    pub(crate) elem_size: u32,
    pub(crate) elem_align: u32,
    pub(crate) drop_fn: u32,
    pub(crate) eq_fn: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmHashSetIterRepr {
    pub(crate) entries: u32,
    pub(crate) states: u32,
    pub(crate) index: u32,
    pub(crate) cap: u32,
    pub(crate) elem_size: u32,
    pub(crate) elem_align: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmHashMapRepr {
    pub(crate) entries: u32,
    pub(crate) states: u32,
    pub(crate) hashes: u32,
    pub(crate) len: u32,
    pub(crate) cap: u32,
    pub(crate) tombstones: u32,
    pub(crate) key_size: u32,
    pub(crate) key_align: u32,
    pub(crate) value_size: u32,
    pub(crate) value_align: u32,
    pub(crate) entry_size: u32,
    pub(crate) value_offset: u32,
    pub(crate) key_drop_fn: u32,
    pub(crate) value_drop_fn: u32,
    pub(crate) key_eq_fn: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WasmHashMapIterRepr {
    pub(crate) entries: u32,
    pub(crate) states: u32,
    pub(crate) index: u32,
    pub(crate) cap: u32,
    pub(crate) entry_size: u32,
    pub(crate) key_size: u32,
    pub(crate) key_align: u32,
    pub(crate) value_size: u32,
    pub(crate) value_align: u32,
    pub(crate) value_offset: u32,
}
