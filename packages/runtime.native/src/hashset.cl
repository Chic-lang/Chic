namespace Std.Runtime.Native;
// Chic-native hash table runtime with deterministic linear probing.
@extern("C", alias = "chic_rt_eq_invoke") public unsafe static extern int hashset_eq_invoke(fn @extern("C")(* const @readonly @expose_address byte,
* const @readonly @expose_address byte) -> int eqFn, * const @readonly @expose_address byte left, * const @readonly @expose_address byte right);
@repr(c) public struct ChicHashSet
{
    public * mut @expose_address byte entries;
    public * mut @expose_address byte states;
    public * mut @expose_address byte hashes;
    public usize len;
    public usize cap;
    public usize tombstones;
    public usize elem_size;
    public usize elem_align;
    public fn @extern("C")(* mut @expose_address byte) -> void drop_fn;
    public fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int eq_fn;
}
@repr(c) public struct ChicHashSetIter
{
    public * const @readonly @expose_address byte entries;
    public * const @readonly @expose_address byte states;
    public usize index;
    public usize cap;
    public usize elem_size;
    public usize elem_align;
}
public enum HashSetError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, NotFound = 4, IterationComplete = 5,
}
public static class HashSetRuntime
{
    private const byte STATE_EMPTY = 0;
    private const byte STATE_FULL = 1;
    private const byte STATE_TOMBSTONE = 2;
    private const usize MIN_CAPACITY = 8usize;
    private const usize LOAD_NUM = 7usize;
    private const usize LOAD_DEN = 10usize;
    @extern("C") private static void HashSetTestDrop(* mut @expose_address byte _ptr) {
        (void) _ptr;
    }
    @extern("C") private static int HashSetTestEq(* const @readonly @expose_address byte _left, * const @readonly @expose_address byte _right) {
        (void) _left;
        (void) _right;
        return 1;
    }
    @allow(all) private unsafe static isize AsIsize(usize value) {
        unchecked {
            return(isize) value;
        }
    }
    private static usize RoundUpPow2(usize value) {
        if (value <= MIN_CAPACITY)
        {
            return MIN_CAPACITY;
        }
        var cap = MIN_CAPACITY;
        while (cap <value)
        {
            let next = cap + cap;
            if (next <cap)
            {
                return 0usize;
            }
            cap = next;
        }
        return cap;
    }
    private static bool ShouldGrow(usize len, usize tombstones, usize cap, usize additional) {
        if (cap == 0)
        {
            return true;
        }
        let filled = len + tombstones;
        if (filled <len)
        {
            return true;
        }
        let needed = filled + additional;
        if (needed <filled)
        {
            return true;
        }
        let left = needed * LOAD_DEN;
        if (LOAD_DEN != 0 && left / LOAD_DEN != needed)
        {
            return true;
        }
        let right = cap * LOAD_NUM;
        if (LOAD_NUM != 0 && right / LOAD_NUM != cap)
        {
            return true;
        }
        return left >right;
    }
    private static bool ShouldShrink(usize len, usize cap) {
        if (cap <= MIN_CAPACITY)
        {
            return false;
        }
        let needed = len + 1usize;
        if (needed <len)
        {
            return false;
        }
        let left = needed * LOAD_DEN;
        if (LOAD_DEN != 0 && left / LOAD_DEN != needed)
        {
            return false;
        }
        let right = cap * LOAD_NUM;
        if (LOAD_NUM != 0 && right / LOAD_NUM != cap)
        {
            return false;
        }
        let doubled = left + left;
        if (doubled <left)
        {
            return false;
        }
        return doubled <right;
    }
    private unsafe static bool EqInvoke(fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int eqFn,
    * const @readonly @expose_address byte left, * const @readonly @expose_address byte right) {
        if (eqFn == null)
        {
            return false;
        }
        return hashset_eq_invoke(eqFn, left, right) != 0;
    }
    private unsafe static void DropValue(ref ChicHashSet table, * mut @expose_address byte value) {
        if (table.drop_fn == null || NativePtr.IsNull (value))
        {
            return;
        }
        SharedRuntime.chic_rt_drop_invoke(table.drop_fn, value);
    }
    private static usize BucketFor(ulong hash, usize cap) {
        return(usize)(hash & (cap - 1usize));
    }
    private unsafe static * mut @expose_address byte EntryPtrMut(* mut @expose_address byte basePtr, usize elemSize, usize index) {
        return NativePtr.OffsetMut(basePtr, AsIsize(index * elemSize));
    }
    private unsafe static * const @readonly @expose_address byte EntryPtrConst(* const @readonly @expose_address byte basePtr,
    usize elemSize, usize index) {
        return NativePtr.OffsetConst(basePtr, AsIsize(index * elemSize));
    }
    private unsafe static * mut @expose_address byte StatePtrMut(ref ChicHashSet table, usize index) {
        return NativePtr.OffsetMut(table.states, AsIsize(index));
    }
    private unsafe static byte ReadStatePtr(* mut @expose_address byte states, usize index) {
        if (NativePtr.IsNull (states))
        {
            return STATE_EMPTY;
        }
        let ptr = NativePtr.OffsetConst(NativePtr.AsConstPtr(states), AsIsize(index));
        return * ptr;
    }
    private unsafe static void WriteStatePtr(* mut @expose_address byte states, usize index, byte state) {
        if (NativePtr.IsNull (states))
        {
            return;
        }
        * NativePtr.OffsetMut(states, AsIsize(index)) = state;
    }
    private unsafe static ulong ReadHashPtr(* mut @expose_address byte hashes, usize index) {
        if (NativePtr.IsNull (hashes))
        {
            return 0ul;
        }
        var value = 0ul;
        var * mut @expose_address byte raw = & value;
        let slotPtr = NativePtr.OffsetConst(NativePtr.AsConstPtr(hashes), AsIsize(index * (usize) __sizeof <ulong >()));
        NativeAlloc.Copy(HashSetMakeMut(raw, (usize) __sizeof <ulong >(), __alignof <ulong >()), HashSetMakeConst(slotPtr,
        (usize) __sizeof <ulong >(), __alignof <ulong >()), (usize) __sizeof <ulong >());
        return value;
    }
    private unsafe static void WriteHashPtr(* mut @expose_address byte hashes, usize index, ulong value) {
        if (NativePtr.IsNull (hashes))
        {
            return;
        }
        var tmp = value;
        var * mut @expose_address byte raw = & tmp;
        let slotPtr = NativePtr.OffsetMut(hashes, AsIsize(index * (usize) __sizeof <ulong >()));
        NativeAlloc.Copy(HashSetMakeMut(slotPtr, (usize) __sizeof <ulong >(), __alignof <ulong >()), HashSetMakeConst(NativePtr.AsConstPtr(raw),
        (usize) __sizeof <ulong >(), __alignof <ulong >()), (usize) __sizeof <ulong >());
    }
    private static unsafe HashSetError AllocateBuffers(usize elemSize, usize elemAlign, usize capacity, out ValueMutPtr entries,
    out ValueMutPtr states, out ValueMutPtr hashes) {
        var entriesLocal = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = capacity * elemSize, Alignment = elemAlign == 0 ?1usize : elemAlign,
        }
        ;
        var statesLocal = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = capacity, Alignment = 1usize,
        }
        ;
        var hashesLocal = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = capacity * (usize) __sizeof <ulong >(), Alignment = __alignof <ulong >(),
        }
        ;
        if (capacity == 0)
        {
            entries = entriesLocal;
            states = statesLocal;
            hashes = hashesLocal;
            return HashSetError.Success;
        }
        let entryBytes = capacity * elemSize;
        if (elemSize != 0usize)
        {
            if (entryBytes / elemSize != capacity)
            {
                entries = entriesLocal;
                states = statesLocal;
                hashes = hashesLocal;
                return HashSetError.CapacityOverflow;
            }
        }
        let hashSize = (usize) __sizeof <ulong >();
        let hashBytes = capacity * hashSize;
        if (hashSize != 0usize)
        {
            if (hashBytes / hashSize != capacity)
            {
                entries = entriesLocal;
                states = statesLocal;
                hashes = hashesLocal;
                return HashSetError.CapacityOverflow;
            }
        }
        if (NativeAlloc.Alloc (entryBytes, elemAlign == 0 ?1usize : elemAlign, out entriesLocal) != NativeAllocationError.Success) {
            entries = entriesLocal;
            states = statesLocal;
            hashes = hashesLocal;
            return HashSetError.AllocationFailed;
        }
        if (NativeAlloc.AllocZeroed (capacity, 1usize, out statesLocal) != NativeAllocationError.Success) {
            NativeAlloc.Free(entriesLocal);
            entries = entriesLocal;
            states = statesLocal;
            hashes = hashesLocal;
            return HashSetError.AllocationFailed;
        }
        if (NativeAlloc.AllocZeroed (hashBytes, __alignof <ulong > (), out hashesLocal) != NativeAllocationError.Success) {
            NativeAlloc.Free(entriesLocal);
            NativeAlloc.Free(statesLocal);
            entries = entriesLocal;
            states = statesLocal;
            hashes = hashesLocal;
            return HashSetError.AllocationFailed;
        }
        entries = entriesLocal;
        states = statesLocal;
        hashes = hashesLocal;
        return HashSetError.Success;
    }
    private static ChicHashSet MakeEmptyTable(usize elemSize, usize elemAlign, fn @extern("C")(* mut @expose_address byte) -> void dropFn,
    fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int eqFn) {
        return new ChicHashSet {
            entries = NativePtr.NullMut(), states = NativePtr.NullMut(), hashes = NativePtr.NullMut(), len = 0usize, cap = 0usize, tombstones = 0usize, elem_size = elemSize, elem_align = elemAlign == 0 ?1usize : elemAlign, drop_fn = dropFn, eq_fn = eqFn,
        }
        ;
    }
    private static unsafe void FreeBuffers(* mut @expose_address byte entries, * mut @expose_address byte states, * mut @expose_address byte hashes) {
        if (entries != null)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = entries, Size = 0usize, Alignment = 1usize,
            }
            );
        }
        if (states != null)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = states, Size = 0usize, Alignment = 1usize,
            }
            );
        }
        if (hashes != null)
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = hashes, Size = 0usize, Alignment = 1usize,
            }
            );
        }
    }
    private static unsafe HashSetError Rehash(ChicHashSet source, usize newCapacity, out ChicHashSet rebuilt) {
        let normalized = RoundUpPow2(newCapacity);
        if (normalized == 0usize)
        {
            rebuilt = source;
            return HashSetError.CapacityOverflow;
        }
        var fresh = MakeEmptyTable(source.elem_size, source.elem_align, source.drop_fn, source.eq_fn);
        var entries = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        var states = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        var hashes = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        let allocStatus = AllocateBuffers(fresh.elem_size, fresh.elem_align, normalized, out entries, out states, out hashes);
        if (allocStatus != HashSetError.Success)
        {
            rebuilt = source;
            return allocStatus;
        }
        fresh.entries = entries.Pointer;
        fresh.states = states.Pointer;
        fresh.hashes = hashes.Pointer;
        fresh.cap = normalized;
        if (source.cap != 0usize)
        {
            var idx = 0usize;
            while (idx <source.cap)
            {
                let state = ReadStatePtr(source.states, idx);
                if (state == STATE_FULL)
                {
                    let hashValue = ReadHashPtr(source.hashes, idx);
                    let srcPtr = EntryPtrConst(NativePtr.AsConstPtr(source.entries), source.elem_size, idx);
                    let mask = fresh.cap - 1usize;
                    var insertIndex = BucketFor(hashValue, fresh.cap);
                    while (ReadStatePtr (fresh.states, insertIndex) == STATE_FULL)
                    {
                        insertIndex = (insertIndex + 1usize) & mask;
                    }
                    let destPtr = EntryPtrMut(fresh.entries, fresh.elem_size, insertIndex);
                    NativeAlloc.Copy(new ValueMutPtr {
                        Pointer = destPtr, Size = source.elem_size, Alignment = source.elem_align
                    }
                    , new ValueConstPtr {
                        Pointer = srcPtr, Size = source.elem_size, Alignment = source.elem_align
                    }
                    , source.elem_size);
                    WriteHashPtr(fresh.hashes, insertIndex, hashValue);
                    WriteStatePtr(fresh.states, insertIndex, STATE_FULL);
                    fresh.len += 1usize;
                }
                idx += 1usize;
            }
        }
        FreeBuffers(source.entries, source.states, source.hashes);
        rebuilt = fresh;
        return HashSetError.Success;
    }
    private static unsafe bool FindSlot(* mut @expose_address byte states, * mut @expose_address byte entries, * mut @expose_address byte hashes,
    usize cap, usize elemSize, usize elemAlign, fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int eqFn,
    ulong hash, ValueConstPtr key, out usize index) {
        var slotIndex = 0usize;
        if (cap == 0usize || NativePtr.IsNull (states))
        {
            index = 0usize;
            return false;
        }
        let mask = cap - 1usize;
        let start = BucketFor(hash, cap);
        var firstTombstone = 0usize;
        var hasTombstone = false;
        var current = start;
        var probes = 0usize;
        while (probes <cap)
        {
            let state = ReadStatePtr(states, current);
            if (state == STATE_EMPTY)
            {
                slotIndex = hasTombstone ?firstTombstone : current;
                index = slotIndex;
                return false;
            }
            if (state == STATE_TOMBSTONE)
            {
                if (! hasTombstone)
                {
                    firstTombstone = current;
                    hasTombstone = true;
                }
            }
            else if (ReadHashPtr (hashes, current) == hash)
            {
                let keyIsNull = IsNullValue(key);
                if (! keyIsNull && ! NativePtr.IsNull (entries))
                {
                    let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(entries), elemSize, current);
                    if (EqInvoke (eqFn, entryPtr, key.Pointer))
                    {
                        slotIndex = current;
                        index = slotIndex;
                        return true;
                    }
                }
            }
            current = (current + 1usize) & mask;
            probes += 1usize;
        }
        index = slotIndex;
        return false;
    }
    @extern("C") @export("chic_rt_hashset_new") public unsafe static ChicHashSet chic_rt_hashset_new(usize elemSize,
    usize elemAlign, fn @extern("C")(* mut @expose_address byte) -> void dropFn, fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int eqFn) {
        return MakeEmptyTable(elemSize, elemAlign, dropFn, eqFn);
    }
    @extern("C") @export("chic_rt_hashset_with_capacity") public unsafe static ChicHashSet chic_rt_hashset_with_capacity(usize elemSize,
    usize elemAlign, usize capacity, fn @extern("C")(* mut @expose_address byte) -> void dropFn, fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int eqFn) {
        var table = MakeEmptyTable(elemSize, elemAlign, dropFn, eqFn);
        let normalized = capacity == 0 ?0usize : RoundUpPow2(capacity);
        if (normalized != 0 || capacity == 0)
        {
            var entries = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
            }
            ;
            var states = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
            }
            ;
            var hashes = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
            }
            ;
            if (AllocateBuffers (table.elem_size, table.elem_align, normalized, out entries, out states, out hashes) == HashSetError.Success) {
                table.entries = entries.Pointer;
                table.states = states.Pointer;
                table.hashes = hashes.Pointer;
                table.cap = normalized;
            }
        }
        return table;
    }
    @extern("C") @export("chic_rt_hashset_len") public unsafe static usize chic_rt_hashset_len(* const ChicHashSet table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0usize;
        }
        let local = * table;
        return local.len;
    }
    @extern("C") @export("chic_rt_hashset_capacity") public unsafe static usize chic_rt_hashset_capacity(* const ChicHashSet table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0usize;
        }
        let local = * table;
        return local.cap;
    }
    @extern("C") @export("chic_rt_hashset_tombstones") public unsafe static usize chic_rt_hashset_tombstones(* const ChicHashSet table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0usize;
        }
        let local = * table;
        return local.tombstones;
    }
    @extern("C") @export("chic_rt_hashset_clear") public unsafe static HashSetError chic_rt_hashset_clear(* mut ChicHashSet table) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        var local = * table;
        if (local.cap == 0usize)
        {
            local.len = 0usize;
            local.tombstones = 0usize;
            * table = local;
            return HashSetError.Success;
        }
        var idx = 0usize;
        while (idx <local.cap)
        {
            let state = ReadStatePtr(local.states, idx);
            if (state == STATE_FULL)
            {
                let entryPtr = EntryPtrMut(local.entries, local.elem_size, idx);
                DropValue(ref local, entryPtr);
            }
            idx += 1usize;
        }
        NativeAlloc.Set(new ValueMutPtr {
            Pointer = local.states, Size = local.cap, Alignment = 1
        }
        , STATE_EMPTY, local.cap);
        NativeAlloc.Set(new ValueMutPtr {
            Pointer = local.hashes, Size = local.cap * __sizeof <ulong >(), Alignment = __alignof <ulong >()
        }
        , 0, local.cap * __sizeof <ulong >());
        local.len = 0usize;
        local.tombstones = 0usize;
        * table = local;
        return HashSetError.Success;
    }
    @extern("C") @export("chic_rt_hashset_drop") public unsafe static void chic_rt_hashset_drop(* mut ChicHashSet table) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return;
        }
        var local = * table;
        // Clear sets while preserving allocated buffers so drops happen exactly once.
        if (local.cap != 0usize)
        {
            var idx = 0usize;
            while (idx <local.cap)
            {
                let state = ReadStatePtr(local.states, idx);
                if (state == STATE_FULL)
                {
                    let entryPtr = EntryPtrMut(local.entries, local.elem_size, idx);
                    DropValue(ref local, entryPtr);
                }
                idx += 1usize;
            }
        }
        let oldEntries = local.entries;
        let oldStates = local.states;
        let oldHashes = local.hashes;
        FreeBuffers(oldEntries, oldStates, oldHashes);
        local.entries = NativePtr.NullMut();
        local.states = NativePtr.NullMut();
        local.hashes = NativePtr.NullMut();
        local.cap = 0usize;
        local.tombstones = 0usize;
        local.len = 0usize;
        local.tombstones = 0usize;
        * table = local;
    }
    @extern("C") @export("chic_rt_hashset_reserve") public unsafe static HashSetError chic_rt_hashset_reserve(* mut ChicHashSet table,
    usize additional) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        var local = * table;
        if (! ShouldGrow (local.len, local.tombstones, local.cap, additional))
        {
            return HashSetError.Success;
        }
        let needed = local.len + additional;
        if (needed <local.len)
        {
            return HashSetError.CapacityOverflow;
        }
        let doubled = needed + needed;
        if (doubled <needed)
        {
            return HashSetError.CapacityOverflow;
        }
        let desired = doubled + MIN_CAPACITY;
        if (desired <doubled)
        {
            return HashSetError.CapacityOverflow;
        }
        let target = RoundUpPow2(desired);
        if (target == 0usize)
        {
            return HashSetError.CapacityOverflow;
        }
        let status = Rehash(local, target, out var rebuilt);
        if (status == HashSetError.Success)
        {
            * table = rebuilt;
        }
        return status;
    }
    @extern("C") @export("chic_rt_hashset_shrink_to") public unsafe static HashSetError chic_rt_hashset_shrink_to(* mut ChicHashSet table,
    usize minCapacity) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        var local = * table;
        if (! ShouldShrink (local.len, local.cap))
        {
            return HashSetError.Success;
        }
        let target = local.len >minCapacity ?local.len : minCapacity;
        if (target == 0usize)
        {
            chic_rt_hashset_drop(table);
            return HashSetError.Success;
        }
        let status = Rehash(local, target, out var rebuilt);
        if (status == HashSetError.Success)
        {
            * table = rebuilt;
        }
        return status;
    }
    @extern("C") @export("chic_rt_hashset_insert") public unsafe static HashSetError chic_rt_hashset_insert(* mut ChicHashSet table,
    ulong hash, * const ValueConstPtr value, * mut int inserted) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        if (NativePtr.ToIsize (inserted) != 0)
        {
            * inserted = 0;
        }
        if (IsNullValuePtr (value))
        {
            return HashSetError.InvalidPointer;
        }
        let input = * value;
        var local = * table;
        if (local.cap == 0usize || ShouldGrow (local.len, local.tombstones, local.cap, 1usize))
        {
            let status = chic_rt_hashset_reserve(table, 1usize);
            if (status != HashSetError.Success)
            {
                return status;
            }
            local = * table;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, input, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (found)
        {
            return HashSetError.Success;
        }
        let destPtr = EntryPtrMut(local.entries, local.elem_size, index);
        NativeAlloc.Copy(new ValueMutPtr {
            Pointer = destPtr, Size = local.elem_size, Alignment = local.elem_align
        }
        , input, local.elem_size);
        WriteHashPtr(local.hashes, index, hash);
        let state = ReadStatePtr(local.states, index);
        if (state == STATE_TOMBSTONE)
        {
            local.tombstones -= 1usize;
        }
        WriteStatePtr(local.states, index, STATE_FULL);
        local.len += 1usize;
        if (NativePtr.ToIsize (inserted) != 0)
        {
            * inserted = 1;
        }
        * table = local;
        return HashSetError.Success;
    }
    @extern("C") @export("chic_rt_hashset_replace") public unsafe static HashSetError chic_rt_hashset_replace(* mut ChicHashSet table,
    ulong hash, * const ValueConstPtr value, * const ValueMutPtr destination, * mut int replaced) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        if (NativePtr.ToIsize (replaced) != 0)
        {
            * replaced = 0;
        }
        if (IsNullValuePtr (value))
        {
            return HashSetError.InvalidPointer;
        }
        let input = * value;
        let output = IsNullMutValuePtr(destination) ?HashSetMakeMut(NativePtr.NullMut(), 0usize, 0usize) : * destination;
        var local = * table;
        if (local.cap == 0usize || ShouldGrow (local.len, local.tombstones, local.cap, 1usize))
        {
            let status = chic_rt_hashset_reserve(table, 1usize);
            if (status != HashSetError.Success)
            {
                return status;
            }
            local = * table;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, input, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (found)
        {
            let entryPtr = EntryPtrMut(local.entries, local.elem_size, index);
            if (! NativePtr.IsNull (output.Pointer))
            {
                NativeAlloc.Copy(output, new ValueConstPtr {
                    Pointer = NativePtr.AsConstPtr(entryPtr), Size = local.elem_size, Alignment = local.elem_align
                }
                , local.elem_size);
            }
            DropValue(ref local, entryPtr);
            NativeAlloc.Copy(new ValueMutPtr {
                Pointer = entryPtr, Size = local.elem_size, Alignment = local.elem_align
            }
            , input, local.elem_size);
            WriteHashPtr(local.hashes, index, hash);
            if (NativePtr.ToIsize (replaced) != 0)
            {
                * replaced = 1;
            }
            * table = local;
            return HashSetError.Success;
        }
        let destPtr = EntryPtrMut(local.entries, local.elem_size, index);
        NativeAlloc.Copy(new ValueMutPtr {
            Pointer = destPtr, Size = local.elem_size, Alignment = local.elem_align
        }
        , input, local.elem_size);
        WriteHashPtr(local.hashes, index, hash);
        let state = ReadStatePtr(local.states, index);
        if (state == STATE_TOMBSTONE)
        {
            local.tombstones -= 1usize;
        }
        WriteStatePtr(local.states, index, STATE_FULL);
        local.len += 1usize;
        * table = local;
        return HashSetError.Success;
    }
    @extern("C") @export("chic_rt_hashset_contains") public unsafe static int chic_rt_hashset_contains(* const ChicHashSet table,
    ulong hash, * const ValueConstPtr key) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0;
        }
        if (IsNullValuePtr (key))
        {
            return 0;
        }
        let local = * table;
        if (local.cap == 0usize)
        {
            return 0;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, * key, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        return found ?1 : 0;
    }
    @extern("C") @export("chic_rt_hashset_get_ptr") public unsafe static ValueConstPtr chic_rt_hashset_get_ptr(* const ChicHashSet table,
    ulong hash, * const ValueConstPtr key) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return HashSetMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        let local = * table;
        if (IsNullValuePtr (key))
        {
            return HashSetMakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
        }
        if (local.cap == 0usize)
        {
            return HashSetMakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, * key, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (! found)
        {
            return HashSetMakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
        }
        let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(local.entries), local.elem_size, index);
        return HashSetMakeConst(entryPtr, local.elem_size, local.elem_align);
    }
    @extern("C") @export("chic_rt_hashset_take") public unsafe static HashSetError chic_rt_hashset_take(* mut ChicHashSet table,
    ulong hash, * const ValueConstPtr key, * const ValueMutPtr destination) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        if (IsNullValuePtr (key))
        {
            return HashSetError.InvalidPointer;
        }
        var local = * table;
        if (local.cap == 0usize)
        {
            return HashSetError.NotFound;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, * key, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (! found)
        {
            return HashSetError.NotFound;
        }
        return chic_rt_hashset_take_at(table, index, destination);
    }
    @extern("C") @export("chic_rt_hashset_remove") public unsafe static int chic_rt_hashset_remove(* mut ChicHashSet table,
    ulong hash, * const ValueConstPtr key) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return 0;
        }
        if (IsNullValuePtr (key))
        {
            return 0;
        }
        var local = * table;
        if (local.cap == 0usize)
        {
            return 0;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.elem_size, local.elem_align, local.eq_fn,
        hash, * key, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (! found)
        {
            return 0;
        }
        let entryPtr = EntryPtrMut(local.entries, local.elem_size, index);
        DropValue(ref local, entryPtr);
        WriteStatePtr(local.states, index, STATE_TOMBSTONE);
        local.len -= 1usize;
        local.tombstones += 1usize;
        * table = local;
        return 1;
    }
    @extern("C") @export("chic_rt_hashset_take_at") public unsafe static HashSetError chic_rt_hashset_take_at(* mut ChicHashSet table,
    usize index, * const ValueMutPtr destination) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        var local = * table;
        if (local.cap == 0usize || index >= local.cap)
        {
            return HashSetError.NotFound;
        }
        let state = ReadStatePtr(local.states, index);
        if (state != STATE_FULL)
        {
            return HashSetError.NotFound;
        }
        let entryPtr = EntryPtrMut(local.entries, local.elem_size, index);
        let output = IsNullMutValuePtr(destination) ?HashSetMakeMut(NativePtr.NullMut(), 0usize, 0usize) : * destination;
        if (! NativePtr.IsNull (output.Pointer))
        {
            NativeAlloc.Copy(output, new ValueConstPtr {
                Pointer = NativePtr.AsConstPtr(entryPtr), Size = local.elem_size, Alignment = local.elem_align
            }
            , local.elem_size);
        }
        DropValue(ref local, entryPtr);
        WriteStatePtr(local.states, index, STATE_TOMBSTONE);
        local.len -= 1usize;
        local.tombstones += 1usize;
        * table = local;
        return HashSetError.Success;
    }
    @extern("C") @export("chic_rt_hashset_bucket_state") public unsafe static byte chic_rt_hashset_bucket_state(* const ChicHashSet table,
    usize index) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return STATE_EMPTY;
        }
        let local = * table;
        if (local.cap == 0usize || index >= local.cap)
        {
            return STATE_EMPTY;
        }
        return ReadStatePtr(local.states, index);
    }
    @extern("C") @export("chic_rt_hashset_bucket_hash") public unsafe static ulong chic_rt_hashset_bucket_hash(* const ChicHashSet table,
    usize index) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0ul;
        }
        let local = * table;
        if (local.cap == 0usize || index >= local.cap)
        {
            return 0ul;
        }
        return ReadHashPtr(local.hashes, index);
    }
    @extern("C") @export("chic_rt_hashset_iter") public unsafe static ChicHashSetIter chic_rt_hashset_iter(* const ChicHashSet table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return new ChicHashSetIter {
                entries = NativePtr.NullConst(), states = NativePtr.NullConst(), index = 0usize, cap = 0usize, elem_size = 0usize, elem_align = 1usize,
            }
            ;
        }
        let local = * table;
        return new ChicHashSetIter {
            entries = NativePtr.AsConstPtr(local.entries), states = NativePtr.AsConstPtr(local.states), index = 0usize, cap = local.cap, elem_size = local.elem_size, elem_align = local.elem_align == 0 ?1usize : local.elem_align,
        }
        ;
    }
    @extern("C") @export("chic_rt_hashset_iter_next") public unsafe static HashSetError chic_rt_hashset_iter_next(ChicHashSetIter * iter,
    * const ValueMutPtr destination) {
        var * const @readonly @expose_address byte iterPtr = iter;
        if (NativePtr.ToIsizeConst (iterPtr) == 0)
        {
            return HashSetError.InvalidPointer;
        }
        let ptr = chic_rt_hashset_iter_next_ptr(iter);
        if (IsNullValue (ptr))
        {
            return HashSetError.IterationComplete;
        }
        let local = * iter;
        let output = IsNullMutValuePtr(destination) ?HashSetMakeMut(NativePtr.NullMut(), 0usize, 0usize) : * destination;
        if (! NativePtr.IsNull (output.Pointer))
        {
            NativeAlloc.Copy(output, ptr, local.elem_size);
        }
        return HashSetError.Success;
    }
    @extern("C") @export("chic_rt_hashset_iter_next_ptr") public unsafe static ValueConstPtr chic_rt_hashset_iter_next_ptr(ChicHashSetIter * iter) {
        var * const @readonly @expose_address byte iterPtr = iter;
        if (NativePtr.ToIsizeConst (iterPtr) == 0)
        {
            return HashSetMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        var local = * iter;
        if (local.cap == 0usize || NativePtr.IsNullConst (local.states))
        {
            return HashSetMakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
        }
        while (local.index <local.cap)
        {
            let idx = local.index;
            local.index = idx + 1usize;
            let state = * NativePtr.OffsetConst(local.states, AsIsize(idx));
            if (state == STATE_FULL)
            {
                let entryPtr = EntryPtrConst(local.entries, local.elem_size, idx);
                * iter = local;
                return HashSetMakeConst(entryPtr, local.elem_size, local.elem_align);
            }
        }
        * iter = local;
        return HashSetMakeConst(NativePtr.NullConst(), local.elem_size, local.elem_align);
    }
    private unsafe static ValueConstPtr HashSetMakeConst(* const @readonly @expose_address byte ptr, usize size, usize align) {
        return new ValueConstPtr {
            Pointer = ptr, Size = size, Alignment = align == 0 ?1usize : align,
        }
        ;
    }
    private unsafe static ValueMutPtr HashSetMakeMut(* mut @expose_address byte ptr, usize size, usize align) {
        return new ValueMutPtr {
            Pointer = ptr, Size = size, Alignment = align == 0 ?1usize : align,
        }
        ;
    }
    private unsafe static bool IsNullValue(ValueConstPtr value) {
        return NativePtr.IsNullConst(value.Pointer);
    }
    private unsafe static bool IsNullValuePtr(* const ValueConstPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    private unsafe static bool IsNullMutValuePtr(* const ValueMutPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0;
    }
    public unsafe static void TestCoverageHelpers() {
        let elemSize = (usize) __sizeof<int>();
        let elemAlign = (usize) __alignof<int>();
        let cap = RoundUpPow2(3usize);
        let _ = RoundUpPow2(0usize);
        let _ = ShouldGrow(0usize, 0usize, 0usize, 1usize);
        let _ = ShouldGrow(1usize, 0usize, cap, 0usize);
        let _ = ShouldShrink(0usize, MIN_CAPACITY);
        let _ = ShouldShrink(1usize, cap + cap);
        let _ = BucketFor(9ul, cap);
        let _ = ReadStatePtr(NativePtr.NullMut(), 0usize);
        WriteStatePtr(NativePtr.NullMut(), 0usize, STATE_EMPTY);
        let _ = ReadHashPtr(NativePtr.NullMut(), 0usize);
        WriteHashPtr(NativePtr.NullMut(), 0usize, 0ul);
        let _ = HashSetMakeConst(NativePtr.NullConst(), 0usize, 0usize);
        let _ = HashSetMakeMut(NativePtr.NullMut(), 0usize, 0usize);
        var tmp = 0;
        var * mut @expose_address byte tmpPtr = & tmp;
        let tmpConst = HashSetMakeConst(NativePtr.AsConstPtr(tmpPtr), elemSize, elemAlign);
        let tmpMut = HashSetMakeMut(tmpPtr, elemSize, elemAlign);
        let _ = IsNullValue(tmpConst);
        let _ = IsNullValuePtr(& tmpConst);
        let _ = IsNullMutValuePtr(& tmpMut);
        let _ = IsNullValuePtr((* const ValueConstPtr) NativePtr.NullConst());
        let _ = IsNullMutValuePtr((* const ValueMutPtr) NativePtr.NullConst());
        var table = MakeEmptyTable(elemSize, elemAlign, HashSetTestDrop, HashSetTestEq);
        var entries = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        var states = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        var hashes = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
        }
        ;
        let allocStatus = AllocateBuffers(elemSize, elemAlign, cap, out entries, out states, out hashes);
        if (allocStatus == HashSetError.Success)
        {
            table.entries = entries.Pointer;
            table.states = states.Pointer;
            table.hashes = hashes.Pointer;
            table.cap = cap;
            var entryValue = 7;
            var * mut @expose_address byte entryRaw = & entryValue;
            let entryPtr = EntryPtrMut(table.entries, table.elem_size, 0usize);
            NativeAlloc.Copy(HashSetMakeMut(entryPtr, elemSize, elemAlign), HashSetMakeConst(NativePtr.AsConstPtr(entryRaw), elemSize, elemAlign),
            elemSize);
            WriteStatePtr(table.states, 0usize, STATE_FULL);
            WriteHashPtr(table.hashes, 0usize, 7ul);
            let _ = ReadStatePtr(table.states, 0usize);
            let _ = ReadHashPtr(table.hashes, 0usize);
            let _ = StatePtrMut(ref table, 0usize);
            let _ = EntryPtrConst(NativePtr.AsConstPtr(table.entries), table.elem_size, 0usize);
            var slot = 0usize;
            let _ = FindSlot(table.states, table.entries, table.hashes, table.cap, table.elem_size, table.elem_align, table.eq_fn, 7ul,
            HashSetMakeConst(NativePtr.AsConstPtr(entryRaw), elemSize, elemAlign), out slot);
            var rebuilt = table;
            let _ = Rehash(table, table.cap + table.cap, out rebuilt);
            table = rebuilt;
            if (table.cap != 0usize && ! NativePtr.IsNull (table.entries))
            {
                let dropPtr = EntryPtrMut(table.entries, table.elem_size, 0usize);
                DropValue(ref table, dropPtr);
            }
            FreeBuffers(table.entries, table.states, table.hashes);
        }
    }
}
