namespace Std.Runtime.Native;
// Chic-native hash map runtime with deterministic linear probing.
@extern("C", alias = "chic_rt_eq_invoke") public unsafe static extern int hashmap_eq_invoke(fn @extern("C")(* const @readonly @expose_address byte,
* const @readonly @expose_address byte) -> int eqFn, * const @readonly @expose_address byte left, * const @readonly @expose_address byte right);
@repr(c) public struct ChicHashMap
{
    public * mut @expose_address byte entries;
    public * mut @expose_address byte states;
    public * mut @expose_address byte hashes;
    public usize len;
    public usize cap;
    public usize tombstones;
    public usize key_size;
    public usize key_align;
    public usize value_size;
    public usize value_align;
    public usize entry_size;
    public usize value_offset;
    public fn @extern("C")(* mut @expose_address byte) -> void key_drop_fn;
    public fn @extern("C")(* mut @expose_address byte) -> void value_drop_fn;
    public fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int key_eq_fn;
}
@repr(c) public struct ChicHashMapIter
{
    public * const @readonly @expose_address byte entries;
    public * const @readonly @expose_address byte states;
    public usize index;
    public usize cap;
    public usize entry_size;
    public usize key_size;
    public usize key_align;
    public usize value_size;
    public usize value_align;
    public usize value_offset;
}
public enum HashMapError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, NotFound = 4, IterationComplete = 5,
}
public static class HashMapRuntime
{
    private const byte STATE_EMPTY = 0;
    private const byte STATE_FULL = 1;
    private const byte STATE_TOMBSTONE = 2;
    private const usize MIN_CAPACITY = 8usize;
    private const usize LOAD_NUM = 7usize;
    private const usize LOAD_DEN = 10usize;
    @extern("C") private static void TestDrop(* mut @expose_address byte _ptr) {
        (void) _ptr;
    }
    @extern("C") private static int TestEq(* const @readonly @expose_address byte _left, * const @readonly @expose_address byte _right) {
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
        return hashmap_eq_invoke(eqFn, left, right) != 0;
    }
    private static unsafe ValueConstPtr HashMapMakeConst(* const @readonly @expose_address byte ptr, usize size, usize align) {
        return new ValueConstPtr {
            Pointer = ptr, Size = size, Alignment = align == 0 ?1usize : align,
        }
        ;
    }
    private static unsafe ValueMutPtr HashMapMakeMut(* mut @expose_address byte ptr, usize size, usize align) {
        return new ValueMutPtr {
            Pointer = ptr, Size = size, Alignment = align == 0 ?1usize : align,
        }
        ;
    }
    private unsafe static * mut @expose_address byte EntryPtrMut(* mut @expose_address byte entries, usize entrySize, usize index) {
        return NativePtr.OffsetMut(entries, AsIsize(index * entrySize));
    }
    private unsafe static * const @readonly @expose_address byte EntryPtrConst(* const @readonly @expose_address byte entries,
    usize entrySize, usize index) {
        return NativePtr.OffsetConst(entries, AsIsize(index * entrySize));
    }
    private unsafe static * mut @expose_address byte KeyPtrMut(* mut @expose_address byte entryPtr, usize keySize, usize keyAlign) {
        let base = entryPtr;
        if (NativePtr.ToIsize (base) == 0isize)
        {
            return NativePtr.NullMut();
        }
        return base;
    }
    private unsafe static * mut @expose_address byte ValuePtrMut(* mut @expose_address byte entryPtr, usize valueOffset,
    usize valueAlign) {
        return NativePtr.OffsetMut(entryPtr, AsIsize(valueOffset));
    }
    private unsafe static * const @readonly @expose_address byte KeyPtrConst(* const @readonly @expose_address byte entryPtr,
    usize keyAlign) {
        return entryPtr;
    }
    private unsafe static * const @readonly @expose_address byte ValuePtrConst(* const @readonly @expose_address byte entryPtr,
    usize valueOffset, usize valueAlign) {
        return NativePtr.OffsetConst(entryPtr, AsIsize(valueOffset));
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
        NativeAlloc.Copy(HashMapMakeMut(raw, (usize) __sizeof <ulong >(), __alignof <ulong >()), HashMapMakeConst(slotPtr,
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
        NativeAlloc.Copy(HashMapMakeMut(slotPtr, (usize) __sizeof <ulong >(), __alignof <ulong >()), HashMapMakeConst(NativePtr.AsConstPtr(raw),
        (usize) __sizeof <ulong >(), __alignof <ulong >()), (usize) __sizeof <ulong >());
    }
    private static unsafe HashMapError AllocateTable(ref ChicHashMap table, usize capacity) {
        if (capacity == 0)
        {
            table.entries = NativePtr.NullMut();
            table.states = NativePtr.NullMut();
            table.hashes = NativePtr.NullMut();
            table.cap = 0usize;
            table.tombstones = 0usize;
            return HashMapError.Success;
        }
        let entryBytes = capacity * table.entry_size;
        if (table.entry_size != 0usize)
        {
            if (entryBytes / table.entry_size != capacity)
            {
                return HashMapError.CapacityOverflow;
            }
        }
        let hashSize = (usize) __sizeof <ulong >();
        let hashBytes = capacity * hashSize;
        if (hashSize != 0usize)
        {
            if (hashBytes / hashSize != capacity)
            {
                return HashMapError.CapacityOverflow;
            }
        }
        let entryAlign = table.key_align >table.value_align ?table.key_align : table.value_align;
        var entries = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = entryBytes, Alignment = entryAlign == 0 ?1usize : entryAlign,
        }
        ;
        var states = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = capacity, Alignment = 1usize
        }
        ;
        var hashes = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = hashBytes, Alignment = __alignof <ulong >(),
        }
        ;
        if (NativeAlloc.Alloc (entryBytes, entryAlign == 0 ?1usize : entryAlign, out entries) != NativeAllocationError.Success) {
            return HashMapError.AllocationFailed;
        }
        if (NativeAlloc.AllocZeroed (capacity, 1usize, out states) != NativeAllocationError.Success) {
            NativeAlloc.Free(entries);
            return HashMapError.AllocationFailed;
        }
        if (NativeAlloc.AllocZeroed (hashBytes, __alignof <ulong > (), out hashes) != NativeAllocationError.Success) {
            NativeAlloc.Free(entries);
            NativeAlloc.Free(states);
            return HashMapError.AllocationFailed;
        }
        table.entries = entries.Pointer;
        table.states = states.Pointer;
        table.hashes = hashes.Pointer;
        table.cap = capacity;
        table.tombstones = 0usize;
        return HashMapError.Success;
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
    private static usize AlignUp(usize value, usize align) {
        if (align == 0usize)
        {
            return value;
        }
        let mask = align - 1usize;
        let sum = value + mask;
        if (sum <value)
        {
            return 0usize;
        }
        return sum & ~ mask;
    }
    private static usize BucketFor(ulong hash, usize cap) {
        return(usize)(hash & (cap - 1usize));
    }
    private unsafe static bool IsNullValuePtr(* const ValueConstPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0isize;
    }
    private unsafe static bool IsNullMutValuePtr(* const ValueMutPtr ptr) {
        var * const @readonly @expose_address byte raw = ptr;
        return NativePtr.ToIsizeConst(raw) == 0isize;
    }
    private unsafe static bool IsNullConst(ValueConstPtr value) {
        return NativePtr.IsNullConst(value.Pointer);
    }
    private static unsafe void DropEntry(ref ChicHashMap table, * mut @expose_address byte entry) {
        if (NativePtr.IsNull (entry))
        {
            return;
        }
        let keyPtr = KeyPtrMut(entry, table.key_size, table.key_align);
        let valuePtr = ValuePtrMut(entry, table.value_offset, table.value_align);
        if (table.key_drop_fn != null && !NativePtr.IsNull (keyPtr))
        {
            SharedRuntime.chic_rt_drop_invoke(table.key_drop_fn, keyPtr);
        }
        if (table.value_drop_fn != null && !NativePtr.IsNull (valuePtr))
        {
            SharedRuntime.chic_rt_drop_invoke(table.value_drop_fn, valuePtr);
        }
    }
    private static unsafe void CopyKeyValue(ref ChicHashMap table, * mut @expose_address byte entry, ValueConstPtr key, ValueConstPtr value) {
        let keyPtr = KeyPtrMut(entry, table.key_size, table.key_align);
        let valuePtr = ValuePtrMut(entry, table.value_offset, table.value_align);
        if (!NativePtr.IsNull (keyPtr) && key.Size != 0usize)
        {
            NativeAlloc.Copy(HashMapMakeMut(keyPtr, table.key_size, table.key_align), key, table.key_size);
        }
        if (!NativePtr.IsNull (valuePtr) && value.Size != 0usize)
        {
            NativeAlloc.Copy(HashMapMakeMut(valuePtr, table.value_size, table.value_align), value, table.value_size);
        }
    }
    private static unsafe void CopyValueOut(ref ChicHashMap table, * const @readonly @expose_address byte entry, ValueMutPtr destination) {
        if (destination.Size == 0usize)
        {
            return;
        }
        let valuePtr = ValuePtrConst(entry, table.value_offset, table.value_align);
        if (NativePtr.IsNullConst (valuePtr))
        {
            return;
        }
        NativeAlloc.Copy(destination, HashMapMakeConst(valuePtr, table.value_size, table.value_align), table.value_size);
    }
    private static unsafe HashMapError Rehash(ChicHashMap table, usize newCapacity, out ChicHashMap newTable) {
        let normalized = RoundUpPow2(newCapacity);
        if (normalized == 0usize)
        {
            newTable = table;
            return HashMapError.CapacityOverflow;
        }
        var fresh = new ChicHashMap {
            entries = NativePtr.NullMut(), states = NativePtr.NullMut(), hashes = NativePtr.NullMut(), len = 0usize, cap = 0usize, tombstones = 0usize, key_size = table.key_size, key_align = table.key_align, value_size = table.value_size, value_align = table.value_align, entry_size = table.entry_size, value_offset = table.value_offset, key_drop_fn = table.key_drop_fn, value_drop_fn = table.value_drop_fn, key_eq_fn = table.key_eq_fn,
        }
        ;
        let status = AllocateTable(ref fresh, normalized);
        if (status != HashMapError.Success)
        {
            newTable = table;
            return status;
        }
        if (table.cap != 0usize)
        {
            var idx = 0usize;
            while (idx <table.cap)
            {
                let state = ReadStatePtr(table.states, idx);
                if (state == STATE_FULL)
                {
                    let hashValue = ReadHashPtr(table.hashes, idx);
                    let srcPtr = EntryPtrConst(NativePtr.AsConstPtr(table.entries), table.entry_size, idx);
                    let keyPtr = KeyPtrConst(srcPtr, table.key_align);
                    let mask = fresh.cap - 1usize;
                    var insertIndex = BucketFor(hashValue, fresh.cap);
                    while (ReadStatePtr (fresh.states, insertIndex) == STATE_FULL)
                    {
                        insertIndex = (insertIndex + 1usize) & mask;
                    }
                    let destPtr = EntryPtrMut(fresh.entries, fresh.entry_size, insertIndex);
                    NativeAlloc.Copy(new ValueMutPtr {
                        Pointer = destPtr, Size = table.entry_size, Alignment = table.key_align >table.value_align ?table.key_align : table.value_align
                    }
                    , new ValueConstPtr {
                        Pointer = srcPtr, Size = table.entry_size, Alignment = table.key_align >table.value_align ?table.key_align : table.value_align
                    }
                    , table.entry_size);
                    WriteHashPtr(fresh.hashes, insertIndex, hashValue);
                    WriteStatePtr(fresh.states, insertIndex, STATE_FULL);
                    fresh.len += 1usize;
                }
                idx += 1usize;
            }
        }
        let oldEntries = table.entries;
        let oldStates = table.states;
        let oldHashes = table.hashes;
        FreeBuffers(oldEntries, oldStates, oldHashes);
        newTable = fresh;
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_new") public unsafe static ChicHashMap chic_rt_hashmap_new(usize keySize, usize keyAlign,
    usize valueSize, usize valueAlign, fn @extern("C")(* mut @expose_address byte) -> void keyDropFn, fn @extern("C")(* mut @expose_address byte) -> void valueDropFn,
    fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int keyEqFn) {
        let maxAlign = keyAlign >valueAlign ?keyAlign : valueAlign;
        let offset = valueAlign == 0usize ?keySize : AlignUp(keySize, valueAlign);
        return new ChicHashMap {
            entries = NativePtr.NullMut(), states = NativePtr.NullMut(), hashes = NativePtr.NullMut(), len = 0usize, cap = 0usize, tombstones = 0usize, key_size = keySize, key_align = keyAlign == 0 ?1usize : keyAlign, value_size = valueSize, value_align = valueAlign == 0 ?1usize : valueAlign, entry_size = offset + valueSize, value_offset = offset, key_drop_fn = keyDropFn, value_drop_fn = valueDropFn, key_eq_fn = keyEqFn,
        }
        ;
    }
    @extern("C") @export("chic_rt_hashmap_with_capacity") public unsafe static ChicHashMap chic_rt_hashmap_with_capacity(usize keySize,
    usize keyAlign, usize valueSize, usize valueAlign, usize capacity, fn @extern("C")(* mut @expose_address byte) -> void keyDropFn,
    fn @extern("C")(* mut @expose_address byte) -> void valueDropFn, fn @extern("C")(* const @readonly @expose_address byte,
    * const @readonly @expose_address byte) -> int keyEqFn) {
        var table = chic_rt_hashmap_new(keySize, keyAlign, valueSize, valueAlign, keyDropFn, valueDropFn, keyEqFn);
        let normalized = capacity == 0 ?0usize : RoundUpPow2(capacity);
        if (normalized != 0 || capacity == 0)
        {
            let _ = AllocateTable(ref table, normalized);
        }
        return table;
    }
    @extern("C") @export("chic_rt_hashmap_len") public unsafe static usize chic_rt_hashmap_len(* const ChicHashMap table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0usize;
        }
        let local = * table;
        return local.len;
    }
    @extern("C") @export("chic_rt_hashmap_capacity") public unsafe static usize chic_rt_hashmap_capacity(* const ChicHashMap table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return 0usize;
        }
        let local = * table;
        return local.cap;
    }
    @extern("C") @export("chic_rt_hashmap_clear") public unsafe static HashMapError chic_rt_hashmap_clear(* mut ChicHashMap table) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        var local = * table;
        if (local.cap == 0usize)
        {
            local.len = 0usize;
            local.tombstones = 0usize;
            * table = local;
            return HashMapError.Success;
        }
        var idx = 0usize;
        while (idx <local.cap)
        {
            let state = ReadStatePtr(local.states, idx);
            if (state == STATE_FULL)
            {
                let entryPtr = EntryPtrMut(local.entries, local.entry_size, idx);
                DropEntry(ref local, entryPtr);
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
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_drop") public unsafe static void chic_rt_hashmap_drop(* mut ChicHashMap table) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return;
        }
        var local = * table;
        if (local.cap != 0usize)
        {
            var idx = 0usize;
            while (idx <local.cap)
            {
                let state = ReadStatePtr(local.states, idx);
                if (state == STATE_FULL)
                {
                    let entryPtr = EntryPtrMut(local.entries, local.entry_size, idx);
                    DropEntry(ref local, entryPtr);
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
        * table = local;
    }
    @extern("C") @export("chic_rt_hashmap_reserve") public unsafe static HashMapError chic_rt_hashmap_reserve(* mut ChicHashMap table,
    usize additional) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        var local = * table;
        if (!ShouldGrow (local.len, local.tombstones, local.cap, additional))
        {
            return HashMapError.Success;
        }
        let needed = local.len + local.tombstones + additional + 1usize;
        if (needed <local.len)
        {
            return HashMapError.CapacityOverflow;
        }
        var resized = local;
        let status = Rehash(local, needed, out resized);
        if (status == HashMapError.Success)
        {
            * table = resized;
        }
        return status;
    }
    private static unsafe bool FindSlot(* mut @expose_address byte states, * mut @expose_address byte entries, * mut @expose_address byte hashes,
    usize cap, usize entrySize, usize keySize, usize keyAlign, fn @extern("C")(* const @readonly @expose_address byte, * const @readonly @expose_address byte) -> int eqFn,
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
                if (!hasTombstone)
                {
                    firstTombstone = current;
                    hasTombstone = true;
                }
            }
            else if (ReadHashPtr (hashes, current) == hash)
            {
                if (!IsNullConst (key) && !NativePtr.IsNull (entries))
                {
                    let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(entries), entrySize, current);
                    let lhsKey = KeyPtrConst(entryPtr, keyAlign);
                    if (EqInvoke (eqFn, lhsKey, key.Pointer))
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
    @extern("C") @export("chic_rt_hashmap_shrink_to") public unsafe static HashMapError chic_rt_hashmap_shrink_to(* mut ChicHashMap table,
    usize minCapacity) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        var local = * table;
        if (!ShouldShrink (local.len, local.cap))
        {
            return HashMapError.Success;
        }
        let target = local.len >minCapacity ?local.len : minCapacity;
        var resized = local;
        let status = Rehash(local, target, out resized);
        if (status == HashMapError.Success)
        {
            * table = resized;
        }
        return status;
    }
    private static unsafe HashMapError InsertInternal(* mut ChicHashMap table, ulong hash, * const ValueConstPtr key, * const ValueConstPtr value,
    * const ValueMutPtr previousValue, * mut int replaced) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            if (NativePtr.ToIsize (replaced) != 0)
            {
                * replaced = 0;
            }
            return HashMapError.InvalidPointer;
        }
        if (IsNullValuePtr (key) || IsNullValuePtr (value))
        {
            if (NativePtr.ToIsize (replaced) != 0)
            {
                * replaced = 0;
            }
            return HashMapError.InvalidPointer;
        }
        let keyValue = * key;
        let valueValue = * value;
        var previousValueLocal = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize,
        }
        ;
        if (!IsNullMutValuePtr (previousValue))
        {
            previousValueLocal = * previousValue;
        }
        var local = * table;
        if (local.cap == 0usize || ShouldGrow (local.len, local.tombstones, local.cap, 1usize))
        {
            let statusReserve = chic_rt_hashmap_reserve(table, 1usize);
            if (statusReserve != HashMapError.Success)
            {
                if (NativePtr.ToIsize (replaced) != 0)
                {
                    * replaced = 0;
                }
                return statusReserve;
            }
            local = * table;
        }
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.entry_size, local.key_size, local.key_align,
        local.key_eq_fn, hash, keyValue, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        let entryPtr = EntryPtrMut(local.entries, local.entry_size, index);
        if (found)
        {
            if (previousValueLocal.Size != 0usize)
            {
                CopyValueOut(ref local, entryPtr, previousValueLocal);
            }
            else if (local.value_drop_fn != null)
            {
                let valuePtr = ValuePtrMut(entryPtr, local.value_offset, local.value_align);
                if (!NativePtr.IsNull (valuePtr))
                {
                    SharedRuntime.chic_rt_drop_invoke(local.value_drop_fn, valuePtr);
                }
            }
            CopyKeyValue(ref local, entryPtr, keyValue, valueValue);
            if (NativePtr.ToIsize (replaced) != 0)
            {
                * replaced = 1;
            }
        }
        else
        {
            CopyKeyValue(ref local, entryPtr, keyValue, valueValue);
            WriteHashPtr(local.hashes, index, hash);
            let state = ReadStatePtr(local.states, index);
            if (state == STATE_TOMBSTONE)
            {
                local.tombstones -= 1usize;
            }
            WriteStatePtr(local.states, index, STATE_FULL);
            local.len += 1usize;
            if (NativePtr.ToIsize (replaced) != 0)
            {
                * replaced = 0;
            }
        }
        * table = local;
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_insert") public unsafe static HashMapError chic_rt_hashmap_insert(* mut ChicHashMap table,
    ulong hash, * const ValueConstPtr key, * const ValueConstPtr value, * const ValueMutPtr previousValue, * mut int replaced) {
        return InsertInternal(table, hash, key, value, previousValue, replaced);
    }
    @extern("C") @export("chic_rt_hashmap_contains") public unsafe static int chic_rt_hashmap_contains(* const ChicHashMap table,
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
        let keyValue = * key;
        var local = * table;
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.entry_size, local.key_size, local.key_align,
        local.key_eq_fn, hash, keyValue, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        let _ = index;
        return found ?1 : 0;
    }
    @extern("C") @export("chic_rt_hashmap_get_ptr") public unsafe static ValueConstPtr chic_rt_hashmap_get_ptr(* const ChicHashMap table,
    ulong hash, * const ValueConstPtr key) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return HashMapMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        if (IsNullValuePtr (key))
        {
            return HashMapMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        let keyValue = * key;
        let local = * table;
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.entry_size, local.key_size, local.key_align,
        local.key_eq_fn, hash, keyValue, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (!found)
        {
            return HashMapMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(local.entries), local.entry_size, index);
        let valuePtr = ValuePtrConst(entryPtr, local.value_offset, local.value_align);
        return HashMapMakeConst(valuePtr, local.value_size, local.value_align);
    }
    @extern("C") @export("chic_rt_hashmap_take") public unsafe static HashMapError chic_rt_hashmap_take(* mut ChicHashMap table,
    ulong hash, * const ValueConstPtr key, * const ValueMutPtr destination) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        if (IsNullValuePtr (key) || IsNullMutValuePtr (destination))
        {
            return HashMapError.InvalidPointer;
        }
        let keyValue = * key;
        let destinationValue = * destination;
        var local = * table;
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.entry_size, local.key_size, local.key_align,
        local.key_eq_fn, hash, keyValue, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (!found)
        {
            return HashMapError.NotFound;
        }
        let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(local.entries), local.entry_size, index);
        if (destinationValue.Size != 0usize)
        {
            let valuePtr = ValuePtrConst(entryPtr, local.value_offset, local.value_align);
            if (valuePtr == null || destinationValue.Pointer == null)
            {
                return HashMapError.InvalidPointer;
            }
            var offset = 0usize;
            while (offset <local.value_size)
            {
                * NativePtr.OffsetMut(destinationValue.Pointer, AsIsize(offset)) = * NativePtr.OffsetConst(valuePtr, AsIsize(offset));
                offset += 1usize;
            }
        }
        let entryPtrMut = EntryPtrMut(local.entries, local.entry_size, index);
        DropEntry(ref local, entryPtrMut);
        WriteStatePtr(local.states, index, STATE_TOMBSTONE);
        local.len -= 1usize;
        local.tombstones += 1usize;
        * table = local;
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_remove") public unsafe static int chic_rt_hashmap_remove(* mut ChicHashMap table,
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
        let keyValue = * key;
        var local = * table;
        var index = 0usize;
        let entriesPtr = local.entries;
        let statesPtr = local.states;
        let hashesPtr = local.hashes;
        let found = FindSlot(local.states, local.entries, local.hashes, local.cap, local.entry_size, local.key_size, local.key_align,
        local.key_eq_fn, hash, keyValue, out index);
        local.entries = entriesPtr;
        local.states = statesPtr;
        local.hashes = hashesPtr;
        if (!found)
        {
            return 0;
        }
        let entryPtrMut = EntryPtrMut(local.entries, local.entry_size, index);
        DropEntry(ref local, entryPtrMut);
        WriteStatePtr(local.states, index, STATE_TOMBSTONE);
        local.len -= 1usize;
        local.tombstones += 1usize;
        * table = local;
        return 1;
    }
    @extern("C") @export("chic_rt_hashmap_bucket_state") public unsafe static byte chic_rt_hashmap_bucket_state(* const ChicHashMap table,
    usize index) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0 || index >= (* table).cap)
        {
            return STATE_EMPTY;
        }
        let local = * table;
        return ReadStatePtr(local.states, index);
    }
    @extern("C") @export("chic_rt_hashmap_bucket_hash") public unsafe static ulong chic_rt_hashmap_bucket_hash(* const ChicHashMap table,
    usize index) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0 || index >= (* table).cap)
        {
            return 0ul;
        }
        let local = * table;
        return ReadHashPtr(local.hashes, index);
    }
    @extern("C") @export("chic_rt_hashmap_take_at") public unsafe static HashMapError chic_rt_hashmap_take_at(* mut ChicHashMap table,
    usize index, * const ValueMutPtr keyDest, * const ValueMutPtr valueDest) {
        var * mut @expose_address byte tablePtr = table;
        if (NativePtr.ToIsize (tablePtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        if (IsNullMutValuePtr (keyDest) || IsNullMutValuePtr (valueDest))
        {
            return HashMapError.InvalidPointer;
        }
        let keyDestValue = * keyDest;
        let valueDestValue = * valueDest;
        var local = * table;
        if (index >= local.cap)
        {
            return HashMapError.NotFound;
        }
        let state = ReadStatePtr(local.states, index);
        if (state != STATE_FULL)
        {
            return HashMapError.NotFound;
        }
        let entryPtr = EntryPtrConst(NativePtr.AsConstPtr(local.entries), local.entry_size, index);
        if (keyDestValue.Size != 0usize)
        {
            let keyPtr = KeyPtrConst(entryPtr, local.key_align);
            NativeAlloc.Copy(keyDestValue, HashMapMakeConst(keyPtr, local.key_size, local.key_align), local.key_size);
        }
        if (valueDestValue.Size != 0usize)
        {
            let valuePtr = ValuePtrConst(entryPtr, local.value_offset, local.value_align);
            NativeAlloc.Copy(valueDestValue, HashMapMakeConst(valuePtr, local.value_size, local.value_align), local.value_size);
        }
        let entryPtrMut = EntryPtrMut(local.entries, local.entry_size, index);
        DropEntry(ref local, entryPtrMut);
        WriteStatePtr(local.states, index, STATE_TOMBSTONE);
        local.len -= 1usize;
        local.tombstones += 1usize;
        * table = local;
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_iter") public unsafe static ChicHashMapIter chic_rt_hashmap_iter(* const ChicHashMap table) {
        var * const @readonly @expose_address byte tablePtr = table;
        if (NativePtr.ToIsizeConst (tablePtr) == 0)
        {
            return new ChicHashMapIter {
                entries = NativePtr.NullConst(), states = NativePtr.NullConst(), index = 0usize, cap = 0usize, entry_size = 0usize, key_size = 0usize, key_align = 1usize, value_size = 0usize, value_align = 1usize, value_offset = 0usize,
            }
            ;
        }
        let local = * table;
        return new ChicHashMapIter {
            entries = NativePtr.AsConstPtr(local.entries), states = NativePtr.AsConstPtr(local.states), index = 0usize, cap = local.cap, entry_size = local.entry_size, key_size = local.key_size, key_align = local.key_align, value_size = local.value_size, value_align = local.value_align, value_offset = local.value_offset,
        }
        ;
    }
    @extern("C") @export("chic_rt_hashmap_iter_next") public unsafe static HashMapError chic_rt_hashmap_iter_next(* mut ChicHashMapIter iter,
    * const ValueMutPtr keyDest, * const ValueMutPtr valueDest) {
        var * mut @expose_address byte iterPtr = iter;
        if (NativePtr.ToIsize (iterPtr) == 0)
        {
            return HashMapError.InvalidPointer;
        }
        if (IsNullMutValuePtr (keyDest) || IsNullMutValuePtr (valueDest))
        {
            return HashMapError.InvalidPointer;
        }
        let keyDestValue = * keyDest;
        let valueDestValue = * valueDest;
        var entry = chic_rt_hashmap_iter_next_ptr(iter);
        if (NativePtr.IsNullConst (entry.Pointer))
        {
            return HashMapError.IterationComplete;
        }
        let local = * iter;
        if (!NativePtr.IsNull (keyDestValue.Pointer) && keyDestValue.Size != 0usize)
        {
            NativeAlloc.Copy(keyDestValue, HashMapMakeConst(entry.Pointer, local.key_size, local.key_align), local.key_size);
        }
        if (!NativePtr.IsNull (valueDestValue.Pointer) && valueDestValue.Size != 0usize)
        {
            let valuePtr = NativePtr.OffsetConst(entry.Pointer, AsIsize(local.value_offset));
            NativeAlloc.Copy(valueDestValue, HashMapMakeConst(valuePtr, local.value_size, local.value_align), local.value_size);
        }
        return HashMapError.Success;
    }
    @extern("C") @export("chic_rt_hashmap_iter_next_ptr") public unsafe static ValueConstPtr chic_rt_hashmap_iter_next_ptr(* mut ChicHashMapIter iter) {
        var * const @readonly @expose_address byte iterPtr = iter;
        if (NativePtr.ToIsizeConst (iterPtr) == 0)
        {
            return HashMapMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        }
        var local = * iter;
        if (local.cap == 0usize || NativePtr.IsNullConst (local.states))
        {
            return HashMapMakeConst(NativePtr.NullConst(), local.entry_size, local.key_align);
        }
        while (local.index <local.cap)
        {
            let idx = local.index;
            local.index = idx + 1usize;
            let state = * NativePtr.OffsetConst(local.states, AsIsize(idx));
            if (state == STATE_FULL)
            {
                let entryPtr = EntryPtrConst(local.entries, local.entry_size, idx);
                * iter = local;
                let align = local.key_align >local.value_align ?local.key_align : local.value_align;
                return HashMapMakeConst(entryPtr, local.entry_size, align);
            }
        }
        * iter = local;
        return HashMapMakeConst(NativePtr.NullConst(), local.entry_size, local.key_align);
    }
    public unsafe static void TestCoverageHelpers() {
        let _ = RoundUpPow2(3usize);
        let _ = ShouldGrow(0usize, 0usize, 0usize, 1usize);
        let _ = ShouldShrink(0usize, MIN_CAPACITY + 1usize);
        let _ = HashMapMakeConst(NativePtr.NullConst(), 0usize, 1usize);
        let _ = HashMapMakeMut(NativePtr.NullMut(), 0usize, 1usize);
        var map = chic_rt_hashmap_new((usize) __sizeof <u32 >(), (usize) __alignof <u32 >(), (usize) __sizeof <u32 >(), (usize) __alignof <u32 >(),
        TestDrop, TestDrop, TestEq);
        let _ = AllocateTable(ref map, 0usize);
        let _ = AllocateTable(ref map, 8usize);
        if (map.cap != 0usize && !NativePtr.IsNull (map.entries))
        {
            var keyValue = 7u32;
            var valueValue = 9u32;
            var keyPtr = new ValueConstPtr {
                Pointer = & keyValue, Size = (usize) __sizeof <u32 >(), Alignment = (usize) __alignof <u32 >(),
            }
            ;
            var valuePtr = new ValueConstPtr {
                Pointer = & valueValue, Size = (usize) __sizeof <u32 >(), Alignment = (usize) __alignof <u32 >(),
            }
            ;
            let entryPtr = EntryPtrMut(map.entries, map.entry_size, 0usize);
            CopyKeyValue(ref map, entryPtr, keyPtr, valuePtr);
            var outValue = 0u32;
            var outPtr = new ValueMutPtr {
                Pointer = & outValue, Size = (usize) __sizeof <u32 >(), Alignment = (usize) __alignof <u32 >(),
            }
            ;
            CopyValueOut(ref map, NativePtr.AsConstPtr(entryPtr), outPtr);
            DropEntry(ref map, entryPtr);
            let _ = KeyPtrMut(entryPtr, map.key_size, map.key_align);
            let _ = ValuePtrMut(entryPtr, map.value_offset, map.value_align);
            let _ = KeyPtrConst(NativePtr.AsConstPtr(entryPtr), map.key_align);
            let _ = ValuePtrConst(NativePtr.AsConstPtr(entryPtr), map.value_offset, map.value_align);
            WriteStatePtr(map.states, 0usize, STATE_FULL);
            let _ = ReadStatePtr(map.states, 0usize);
            WriteHashPtr(map.hashes, 0usize, 42ul);
            let _ = ReadHashPtr(map.hashes, 0usize);
        }
        chic_rt_hashmap_drop(& map);
    }
}
