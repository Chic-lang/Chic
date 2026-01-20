namespace Std.Collections;
import Std;
import Std.Core;
import Std.Hashing;
import Std.Memory;
import Std.Numeric;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Span;
internal static class HashSetIntrinsics
{
    @extern("C") public static extern HashSetPtr chic_rt_hashset_new(usize elementSize, usize elementAlignment, isize dropFn,
    isize eqFn);
    @extern("C") public static extern HashSetPtr chic_rt_hashset_with_capacity(usize elementSize, usize elementAlignment,
    usize capacity, isize dropFn, isize eqFn);
    @extern("C") public static extern void chic_rt_hashset_drop(ref HashSetPtr setPtr);
    @extern("C") public static extern HashSetError chic_rt_hashset_clear(ref HashSetPtr setPtr);
    @extern("C") public static extern HashSetError chic_rt_hashset_reserve(ref HashSetPtr setPtr, usize additional);
    @extern("C") public static extern HashSetError chic_rt_hashset_shrink_to(ref HashSetPtr setPtr, usize minCapacity);
    @extern("C") public static extern usize chic_rt_hashset_len(in HashSetPtr setPtr);
    @extern("C") public static extern usize chic_rt_hashset_capacity(in HashSetPtr setPtr);
    @extern("C") public static extern usize chic_rt_hashset_tombstones(in HashSetPtr setPtr);
    @extern("C") public static extern HashSetError chic_rt_hashset_insert(ref HashSetPtr setPtr, ulong hash, ValueConstPtr value,
    out int inserted);
    @extern("C") public static extern HashSetError chic_rt_hashset_replace(ref HashSetPtr setPtr, ulong hash, ValueConstPtr value,
    ValueMutPtr destination, out int replaced);
    @extern("C") public static extern int chic_rt_hashset_contains(in HashSetPtr setPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern ValueConstPtr chic_rt_hashset_get_ptr(in HashSetPtr setPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern HashSetError chic_rt_hashset_take(ref HashSetPtr setPtr, ulong hash, ValueConstPtr key,
    ValueMutPtr destination);
    @extern("C") public static extern int chic_rt_hashset_remove(ref HashSetPtr setPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern HashSetError chic_rt_hashset_take_at(ref HashSetPtr setPtr, usize index, ValueMutPtr destination);
    @extern("C") public static extern byte chic_rt_hashset_bucket_state(in HashSetPtr setPtr, usize index);
    @extern("C") public static extern ulong chic_rt_hashset_bucket_hash(in HashSetPtr setPtr, usize index);
    @extern("C") public static extern HashSetIterPtr chic_rt_hashset_iter(in HashSetPtr setPtr);
    @extern("C") public static extern HashSetError chic_rt_hashset_iter_next(ref HashSetIterPtr iter, ValueMutPtr destination);
    @extern("C") public static extern ValueConstPtr chic_rt_hashset_iter_next_ptr(ref HashSetIterPtr iter);
}
public delegate bool HashSetPredicate <T >(in T value);
public delegate void HashSetMutator <T >(ref T value);
public delegate T HashSetFactory <T >();
internal static class HashSetHelpers
{
    public static ValueConstPtr ConstPtrFrom <T >(in T value) {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        unsafe {
            var * mut @expose_address T valuePtr = & value;
            let bytes = PointerIntrinsics.AsByteConstFromMut(valuePtr);
            return ValuePointer.CreateConst(bytes, size, align);
        }
    }
    public static ulong HashValue <T, THasher >(in T value, THasher hasher) where THasher : IHasher, Copy {
        return Hashing.HashValue(in value, hasher);
    }
    public static bool ContainsRaw <T, THasher >(in HashSetPtr rawSet, THasher hasher, in T key) where THasher : IHasher, Copy {
        let hash = HashValue(in key, hasher);
        let handle = ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_contains(in rawSet, hash, handle) != 0;
    }
}
public struct HashSetIter <T >
{
    private HashSetIterPtr _raw;
    internal init(HashSetIterPtr raw) {
        _raw = raw;
    }
    public bool Next(ref this, out T value) {
        var slot = MaybeUninit <T >.Uninit();
        let status = HashSetIntrinsics.chic_rt_hashset_iter_next(ref _raw, slot.AsValueMutPtr());
        if (status == HashSetError.Success)
        {
            slot.MarkInitialized();
            value = slot.AssumeInit();
            return true;
        }
        value = Intrinsics.ZeroValue <T >();
        return false;
    }
    public ValueConstPtr NextPtr(ref this) {
        return HashSetIntrinsics.chic_rt_hashset_iter_next_ptr(ref _raw);
    }
}
public struct HashSetDrain <T >
{
    private * mut @expose_address HashSetPtr _set;
    private usize _index;
    private usize _cap;
    internal init(* mut @expose_address HashSetPtr rawSet) {
        _set = rawSet;
        _index = 0usize;
        _cap = 0usize;
        unsafe {
            if (! Pointer.IsNull (_set))
            {
                _cap = HashSetIntrinsics.chic_rt_hashset_capacity(in * _set);
            }
        }
    }
    private void Finalize(ref this) {
        unsafe {
            if (Pointer.IsNull (_set))
            {
                return;
            }
            let cap = _cap;
            var idx = _index;
            while (idx <cap)
            {
                let nullSlot = ValuePointer.NullMut(__sizeof <T >(), __alignof <T >());
                HashSetIntrinsics.chic_rt_hashset_take_at(ref * _set, idx, nullSlot);
                idx += 1usize;
            }
            HashSetIntrinsics.chic_rt_hashset_clear(ref * _set);
            _set = Pointer.NullMut <HashSetPtr >();
            _index = cap;
        }
    }
    public bool Next(ref this, out T value) {
        unsafe {
            if (Pointer.IsNull (_set))
            {
                value = Intrinsics.ZeroValue <T >();
                return false;
            }
            let cap = _cap;
            while (_index <cap)
            {
                var slot = MaybeUninit <T >.Uninit();
                let status = HashSetIntrinsics.chic_rt_hashset_take_at(ref * _set, _index, slot.AsValueMutPtr());
                _index += 1usize;
                if (status == HashSetError.Success)
                {
                    slot.MarkInitialized();
                    value = slot.AssumeInit();
                    return true;
                }
            }
            Finalize();
            value = Intrinsics.ZeroValue <T >();
            return false;
        }
    }
    public void dispose(ref this) {
        Finalize();
    }
}
public struct HashSetDrainFilter <T, THasher >where THasher : IHasher, Copy {
    private * mut @expose_address HashSetPtr _set;
    private HashSetIterPtr _iter;
    private THasher _hasher;
    private HashSetPredicate <T >_predicate;
    internal init(* mut @expose_address HashSetPtr rawSet, HashSetIterPtr iter, THasher hasher, HashSetPredicate <T >predicate) {
        _set = rawSet;
        _iter = iter;
        _hasher = hasher;
        _predicate = predicate;
    }
    public bool Next(ref this, out T value) {
        unsafe {
            if (Pointer.IsNull (_set))
            {
                value = Intrinsics.ZeroValue <T >();
                return false;
            }
            while (true)
            {
                let ptr = HashSetIntrinsics.chic_rt_hashset_iter_next_ptr(ref _iter);
                if (ValuePointer.IsNullConst (ptr))
                {
                    value = Intrinsics.ZeroValue <T >();
                    return false;
                }
                var slot = MaybeUninit <T >.Uninit();
                if (ptr.Size != 0usize)
                {
                    GlobalAllocator.Copy(slot.AsValueMutPtr(), ptr, ptr.Size);
                }
                slot.MarkInitialized();
                let valueRef = slot.AssumeInitRef();
                let keep = _predicate(in valueRef);
                if (! keep)
                {
                    slot.ForgetInit();
                    continue;
                }
                let hash = HashSetHelpers.HashValue(in valueRef, in _hasher);
                let status = HashSetIntrinsics.chic_rt_hashset_take(ref * _set, hash, slot.AsValueConstPtr(), ValuePointer.NullMut(__sizeof <T >(),
                __alignof <T >()));
                if (status != HashSetError.Success)
                {
                    slot.ForgetInit();
                    continue;
                }
                value = slot.AssumeInit();
                return true;
            }
        }
        value = Intrinsics.ZeroValue <T >();
        return false;
    }
    public void dispose(ref this) {
        var discard = Intrinsics.ZeroValue <T >();
        while (Next (out discard)) {
            // discard
        }
    }
}
/* Set combinators temporarily disabled while resolving HashSet/HashMap iterator wiring. */
public struct HashSetEntry <T, THasher >where THasher : IHasher, Copy {
    private * mut @expose_address HashSetPtr _set;
    private THasher _hasher;
    private ulong _hash;
    private MaybeUninit <T >_key;
    private bool _occupied;
    internal init(* mut @expose_address HashSetPtr rawSet, THasher hasher, ulong hash, MaybeUninit <T >key, bool occupied) {
        _set = rawSet;
        _hasher = hasher;
        _hash = hash;
        _key = key;
        _occupied = occupied;
    }
    public bool IsOccupied => _occupied;
    public bool IsVacant => ! _occupied;
    public HashSetError OrInsert(ref this, out bool inserted) {
        if (_occupied)
        {
            inserted = false;
            return HashSetError.Success;
        }
        unsafe {
            var flag = 0;
            let status = HashSetIntrinsics.chic_rt_hashset_insert(ref * _set, _hash, _key.AsValueConstPtr(), out flag);
            inserted = flag != 0;
            if (status == HashSetError.Success && inserted)
            {
                _key.ForgetInit();
            }
            return status;
        }
    }
    public HashSetError OrInsertWith(ref this, HashSetFactory <T >factory, out bool inserted) {
        if (_occupied)
        {
            inserted = false;
            return HashSetError.Success;
        }
        let value = factory();
        let hash = HashSetHelpers.HashValue(in value, in _hasher);
        var slot = MaybeUninit <T >.Init(value);
        unsafe {
            var flag = 0;
            let status = HashSetIntrinsics.chic_rt_hashset_insert(ref * _set, hash, slot.AsValueConstPtr(), out flag);
            inserted = flag != 0;
            if (status == HashSetError.Success && inserted)
            {
                slot.ForgetInit();
            }
            return status;
        }
    }
    public HashSetError AndModify(ref this, HashSetMutator <T >mutator, out bool modified) {
        modified = false;
        if (! _occupied)
        {
            return HashSetError.Success;
        }
        unsafe {
            var slot = MaybeUninit <T >.Uninit();
            let status = HashSetIntrinsics.chic_rt_hashset_take(ref * _set, _hash, _key.AsValueConstPtr(), slot.AsValueMutPtr());
            if (status != HashSetError.Success)
            {
                return status;
            }
            slot.MarkInitialized();
            var value = slot.AssumeInit();
            mutator(ref value);
            let hash = HashSetHelpers.HashValue(in value, in _hasher);
            var insertSlot = MaybeUninit <T >.Init(value);
            var replaced = 0;
            let insertStatus = HashSetIntrinsics.chic_rt_hashset_replace(ref * _set, hash, insertSlot.AsValueConstPtr(),
            ValuePointer.NullMut(__sizeof <T >(), __alignof <T >()), out replaced);
            if (insertStatus == HashSetError.Success)
            {
                insertSlot.ForgetInit();
                modified = true;
            }
            return insertStatus;
        }
    }
    public HashSetOccupiedEntry <T, THasher >IntoOccupied(this) {
        if (! _occupied)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashset entry is vacant"));
        }
        return new HashSetOccupiedEntry <T, THasher >(_set, _hasher, _hash, _key);
    }
    public HashSetVacantEntry <T, THasher >IntoVacant(this) {
        if (_occupied)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashset entry is occupied"));
        }
        return new HashSetVacantEntry <T, THasher >(_set, _hasher, _hash, _key);
    }
}
public struct HashSetOccupiedEntry <T, THasher >where THasher : IHasher, Copy {
    private * mut @expose_address HashSetPtr _set;
    private THasher _hasher;
    private ulong _hash;
    private MaybeUninit <T >_key;
    internal init(* mut @expose_address HashSetPtr rawSet, THasher hasher, ulong hash, MaybeUninit <T >key) {
        _set = rawSet;
        _hasher = hasher;
        _hash = hash;
        _key = key;
    }
    public HashSetError Remove(ref this, out T value) {
        unsafe {
            var slot = MaybeUninit <T >.Uninit();
            let status = HashSetIntrinsics.chic_rt_hashset_take(ref * _set, _hash, _key.AsValueConstPtr(), slot.AsValueMutPtr());
            if (status != HashSetError.Success)
            {
                value = Intrinsics.ZeroValue <T >();
                return status;
            }
            slot.MarkInitialized();
            value = slot.AssumeInit();
            return status;
        }
    }
}
public struct HashSetVacantEntry <T, THasher >where THasher : IHasher, Copy {
    private * mut @expose_address HashSetPtr _set;
    private THasher _hasher;
    private ulong _hash;
    private MaybeUninit <T >_key;
    internal init(* mut @expose_address HashSetPtr rawSet, THasher hasher, ulong hash, MaybeUninit <T >key) {
        _set = rawSet;
        _hasher = hasher;
        _hash = hash;
        _key = key;
    }
    public HashSetError Insert(ref this, out bool inserted) {
        unsafe {
            var flag = 0;
            let status = HashSetIntrinsics.chic_rt_hashset_insert(ref * _set, _hash, _key.AsValueConstPtr(), out flag);
            inserted = flag != 0;
            if (status == HashSetError.Success && inserted)
            {
                _key.ForgetInit();
            }
            return status;
        }
    }
}
public struct HashSetState <T, THasher >where THasher : IHasher, Copy {
    private const byte STATE_FULL = 1;
    private HashSetPtr _raw;
    private THasher _hasher;
    internal static HashSetPtr CreateRaw() {
        let size = (usize) __sizeof <T >();
        let align = (usize) __alignof <T >();
        let dropFn = (isize) __drop_glue_of <T >();
        let eqFn = (isize) __eq_glue_of <T >();
        if (eqFn == 0)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashset requires op_Equality for the key type"));
        }
        return HashSetIntrinsics.chic_rt_hashset_new(size, align, dropFn, eqFn);
    }
    public init(THasher hasher) {
        _hasher = hasher;
        _raw = CreateRaw();
    }
    public void dispose(ref this) {
        HashSetIntrinsics.chic_rt_hashset_drop(ref _raw);
        _raw = CoreIntrinsics.DefaultValue <HashSetPtr >();
    }
    public usize Len(in this) => HashSetIntrinsics.chic_rt_hashset_len(in _raw);
    public usize Capacity(in this) => HashSetIntrinsics.chic_rt_hashset_capacity(in _raw);
    public HashSetError Reserve(ref this, usize additional) => HashSetIntrinsics.chic_rt_hashset_reserve(ref _raw, additional);
    public HashSetError ShrinkTo(ref this, usize minCapacity) => HashSetIntrinsics.chic_rt_hashset_shrink_to(ref _raw, minCapacity);
    public HashSetError Clear(ref this) => HashSetIntrinsics.chic_rt_hashset_clear(ref _raw);
    public HashSetError Insert(ref this, T value, out bool inserted) {
        let hash = HashSetHelpers.HashValue(in value, in _hasher);
        var slot = MaybeUninit <T >.Init(value);
        var flag = 0;
        let status = HashSetIntrinsics.chic_rt_hashset_insert(ref _raw, hash, slot.AsValueConstPtr(), out flag);
        inserted = flag != 0;
        if (status == HashSetError.Success && inserted)
        {
            slot.ForgetInit();
        }
        return status;
    }
    public HashSetError Replace(ref this, T value, out bool replaced, out T previous) {
        let hash = HashSetHelpers.HashValue(in value, in _hasher);
        var slot = MaybeUninit <T >.Init(value);
        var previousSlot = MaybeUninit <T >.Uninit();
        var flag = 0;
        let status = HashSetIntrinsics.chic_rt_hashset_replace(ref _raw, hash, slot.AsValueConstPtr(), previousSlot.AsValueMutPtr(),
        out flag);
        replaced = flag != 0;
        if (status == HashSetError.Success)
        {
            slot.ForgetInit();
        }
        if (status == HashSetError.Success && replaced)
        {
            previousSlot.MarkInitialized();
            previous = previousSlot.AssumeInit();
        }
        else
        {
            previous = Intrinsics.ZeroValue <T >();
        }
        return status;
    }
    public bool Contains(in this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_contains(in _raw, hash, handle) != 0;
    }
    public ValueConstPtr GetPtr(in this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_get_ptr(in _raw, hash, handle);
    }
    public Option <T >Get(in this, in T key) {
        let ptr = GetPtr(in key);
        if (ValuePointer.IsNullConst (ptr))
        {
            return Option <T >.None();
        }
        let cloneGlue = __clone_glue_of <T >();
        var slot = MaybeUninit <T >.Uninit();
        if (cloneGlue != 0isize)
        {
            CloneRuntime.Invoke(cloneGlue, ptr, slot.AsValueMutPtr());
            slot.MarkInitialized();
            return Option <T >.Some(slot.AssumeInit());
        }
        let dropGlue = (isize) __drop_glue_of <T >();
        if (dropGlue != 0isize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashset Get requires Copy or clone glue"));
        }
        if (ptr.Size != 0usize)
        {
            GlobalAllocator.Copy(slot.AsValueMutPtr(), ptr, ptr.Size);
        }
        slot.MarkInitialized();
        return Option <T >.Some(slot.AssumeInit());
    }
    public Option <T >Take(ref this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        var slot = MaybeUninit <T >.Uninit();
        let status = HashSetIntrinsics.chic_rt_hashset_take(ref _raw, hash, handle, slot.AsValueMutPtr());
        if (status != HashSetError.Success)
        {
            return Option <T >.None();
        }
        slot.MarkInitialized();
        return Option <T >.Some(slot.AssumeInit());
    }
    public bool Remove(ref this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_remove(ref _raw, hash, handle) != 0;
    }
    public HashSetIter <T >Iter(in this) {
        let iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        return new HashSetIter <T >(iter);
    }
    public HashSetDrain <T >Drain(ref this) {
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetDrain <T >(setPtr);
        }
    }
    public HashSetDrainFilter <T, THasher >DrainFilter(ref this, HashSetPredicate <T >predicate) {
        let iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetDrainFilter <T, THasher >(setPtr, iter, _hasher, predicate);
        }
    }
    // Set combinators temporarily disabled while iterator wiring is corrected.
    public void Retain(ref this, HashSetPredicate <T >predicate) {
        var iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        while (true)
        {
            let ptr = HashSetIntrinsics.chic_rt_hashset_iter_next_ptr(ref iter);
            if (ValuePointer.IsNullConst (ptr))
            {
                break;
            }
            var slot = MaybeUninit <T >.Uninit();
            if (ptr.Size != 0usize)
            {
                GlobalAllocator.Copy(slot.AsValueMutPtr(), ptr, ptr.Size);
            }
            slot.MarkInitialized();
            let valueRef = slot.AssumeInitRef();
            let keep = predicate(in valueRef);
            if (! keep)
            {
                let hash = HashSetHelpers.HashValue(in valueRef, in _hasher);
                HashSetIntrinsics.chic_rt_hashset_take(ref _raw, hash, slot.AsValueConstPtr(), ValuePointer.NullMut(__sizeof <T >(),
                __alignof <T >()));
            }
            slot.ForgetInit();
        }
    }
    public HashSetError SplitOff(ref this, usize count, out HashSetState <T, THasher >other) {
        other = new HashSetState <T, THasher >(_hasher);
        let length = Len();
        if (count >= length)
        {
            return HashSetError.Success;
        }
        let cap = HashSetIntrinsics.chic_rt_hashset_capacity(in _raw);
        var kept = 0usize;
        var index = 0usize;
        while (index <cap)
        {
            let state = HashSetIntrinsics.chic_rt_hashset_bucket_state(in _raw, index);
            if (state == STATE_FULL)
            {
                if (kept <count)
                {
                    kept += 1usize;
                }
                else
                {
                    var slot = MaybeUninit <T >.Uninit();
                    let status = HashSetIntrinsics.chic_rt_hashset_take_at(ref _raw, index, slot.AsValueMutPtr());
                    if (status == HashSetError.Success)
                    {
                        slot.MarkInitialized();
                        let value = slot.AssumeInit();
                        var inserted = false;
                        let insertStatus = other.Insert(value, out inserted);
                        if (insertStatus != HashSetError.Success)
                        {
                            return insertStatus;
                        }
                    }
                }
            }
            index += 1usize;
        }
        return HashSetError.Success;
    }
    public HashSetError Extend(ref this, ReadOnlySpan <T >values) {
        var index = 0usize;
        while (index <values.Length)
        {
            var inserted = false;
            let status = Insert(values[index], out inserted);
            if (status != HashSetError.Success)
            {
                return status;
            }
            index += 1usize;
        }
        return HashSetError.Success;
    }
    public HashSetEntry <T, THasher >Entry(ref this, T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        var slot = MaybeUninit <T >.Init(key);
        let contains = HashSetIntrinsics.chic_rt_hashset_contains(in _raw, hash, slot.AsValueConstPtr()) != 0;
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetEntry <T, THasher >(setPtr, _hasher, hash, slot, contains);
        }
    }
}
public struct HashSet <T >
{
    private const byte STATE_FULL = 1;
    private HashSetPtr _raw;
    private DefaultHasher _hasher;
    public init() {
        _hasher = new DefaultHasher();
        _raw = HashSetState <T, DefaultHasher >.CreateRaw();
    }
    public static HashSetState <T, THasher >WithHasher <THasher >(THasher hasher) where THasher : IHasher, Copy {
        return new HashSetState <T, THasher >(hasher);
    }
    public void dispose(ref this) {
        HashSetIntrinsics.chic_rt_hashset_drop(ref _raw);
        _raw = CoreIntrinsics.DefaultValue <HashSetPtr >();
    }
    public usize Len(in this) => HashSetIntrinsics.chic_rt_hashset_len(in _raw);
    public usize Capacity(in this) => HashSetIntrinsics.chic_rt_hashset_capacity(in _raw);
    public HashSetError Reserve(ref this, usize additional) => HashSetIntrinsics.chic_rt_hashset_reserve(ref _raw, additional);
    public HashSetError ShrinkTo(ref this, usize minCapacity) => HashSetIntrinsics.chic_rt_hashset_shrink_to(ref _raw, minCapacity);
    public HashSetError Clear(ref this) => HashSetIntrinsics.chic_rt_hashset_clear(ref _raw);
    public HashSetError Insert(ref this, T value, out bool inserted) {
        let hash = HashSetHelpers.HashValue(in value, in _hasher);
        var slot = MaybeUninit <T >.Init(value);
        var flag = 0;
        let status = HashSetIntrinsics.chic_rt_hashset_insert(ref _raw, hash, slot.AsValueConstPtr(), out flag);
        inserted = flag != 0;
        if (status == HashSetError.Success && inserted)
        {
            slot.ForgetInit();
        }
        return status;
    }
    public HashSetError Replace(ref this, T value, out bool replaced, out T previous) {
        let hash = HashSetHelpers.HashValue(in value, in _hasher);
        var slot = MaybeUninit <T >.Init(value);
        var previousSlot = MaybeUninit <T >.Uninit();
        var flag = 0;
        let status = HashSetIntrinsics.chic_rt_hashset_replace(ref _raw, hash, slot.AsValueConstPtr(), previousSlot.AsValueMutPtr(),
        out flag);
        replaced = flag != 0;
        if (status == HashSetError.Success)
        {
            slot.ForgetInit();
        }
        if (status == HashSetError.Success && replaced)
        {
            previousSlot.MarkInitialized();
            previous = previousSlot.AssumeInit();
        }
        else
        {
            previous = Intrinsics.ZeroValue <T >();
        }
        return status;
    }
    public bool Contains(in this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_contains(in _raw, hash, handle) != 0;
    }
    public ValueConstPtr GetPtr(in this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_get_ptr(in _raw, hash, handle);
    }
    public Option <T >Get(in this, in T key) {
        let ptr = GetPtr(in key);
        if (ValuePointer.IsNullConst (ptr))
        {
            return Option <T >.None();
        }
        let cloneGlue = __clone_glue_of <T >();
        var slot = MaybeUninit <T >.Uninit();
        if (cloneGlue != 0isize)
        {
            CloneRuntime.Invoke(cloneGlue, ptr, slot.AsValueMutPtr());
            slot.MarkInitialized();
            return Option <T >.Some(slot.AssumeInit());
        }
        let dropGlue = (isize) __drop_glue_of <T >();
        if (dropGlue != 0isize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashset Get requires Copy or clone glue"));
        }
        slot.MarkInitialized();
        return Option <T >.Some(slot.AssumeInit());
    }
    public Option <T >Take(ref this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        var slot = MaybeUninit <T >.Uninit();
        let status = HashSetIntrinsics.chic_rt_hashset_take(ref _raw, hash, handle, slot.AsValueMutPtr());
        if (status != HashSetError.Success)
        {
            return Option <T >.None();
        }
        slot.MarkInitialized();
        return Option <T >.Some(slot.AssumeInit());
    }
    public bool Remove(ref this, in T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        let handle = HashSetHelpers.ConstPtrFrom(in key);
        return HashSetIntrinsics.chic_rt_hashset_remove(ref _raw, hash, handle) != 0;
    }
    public HashSetIter <T >Iter(in this) {
        let iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        return new HashSetIter <T >(iter);
    }
    public HashSetDrain <T >Drain(ref this) {
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetDrain <T >(setPtr);
        }
    }
    public HashSetDrainFilter <T, DefaultHasher >DrainFilter(ref this, HashSetPredicate <T >predicate) {
        let iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetDrainFilter <T, DefaultHasher >(setPtr, iter, _hasher, predicate);
        }
    }
    // Set combinators temporarily disabled while iterator wiring is corrected.
    public void Retain(ref this, HashSetPredicate <T >predicate) {
        var iter = HashSetIntrinsics.chic_rt_hashset_iter(in _raw);
        unsafe {
            while (true)
            {
                let ptr = HashSetIntrinsics.chic_rt_hashset_iter_next_ptr(ref iter);
                if (ValuePointer.IsNullConst (ptr))
                {
                    break;
                }
                var slot = MaybeUninit <T >.Uninit();
                if (ptr.Size != 0usize)
                {
                    GlobalAllocator.Copy(slot.AsValueMutPtr(), ptr, ptr.Size);
                }
                slot.MarkInitialized();
                let valueRef = slot.AssumeInitRef();
                let keep = predicate(in valueRef);
                if (keep)
                {
                    slot.ForgetInit();
                    continue;
                }
                let hash = HashSetHelpers.HashValue(in valueRef, in _hasher);
                HashSetIntrinsics.chic_rt_hashset_take(ref _raw, hash, slot.AsValueConstPtr(), ValuePointer.NullMut(__sizeof <T >(),
                __alignof <T >()));
            }
        }
    }
    public HashSetError SplitOff(ref this, usize count, out HashSet <T >other) {
        other = new HashSet <T >();
        let length = Len();
        if (count >= length)
        {
            return HashSetError.Success;
        }
        let cap = HashSetIntrinsics.chic_rt_hashset_capacity(in _raw);
        var kept = 0usize;
        var index = 0usize;
        while (index <cap)
        {
            let state = HashSetIntrinsics.chic_rt_hashset_bucket_state(in _raw, index);
            if (state == STATE_FULL)
            {
                if (kept <count)
                {
                    kept += 1usize;
                }
                else
                {
                    var slot = MaybeUninit <T >.Uninit();
                    let status = HashSetIntrinsics.chic_rt_hashset_take_at(ref _raw, index, slot.AsValueMutPtr());
                    if (status == HashSetError.Success)
                    {
                        slot.MarkInitialized();
                        let value = slot.AssumeInit();
                        var inserted = false;
                        let insertStatus = other.Insert(value, out inserted);
                        if (insertStatus != HashSetError.Success)
                        {
                            return insertStatus;
                        }
                    }
                }
            }
            index += 1usize;
        }
        return HashSetError.Success;
    }
    public HashSetError Extend(ref this, ReadOnlySpan <T >values) {
        var index = 0usize;
        while (index <values.Length)
        {
            var inserted = false;
            let status = Insert(values[index], out inserted);
            if (status != HashSetError.Success)
            {
                return status;
            }
            index += 1usize;
        }
        return HashSetError.Success;
    }
    public HashSetEntry <T, DefaultHasher >Entry(ref this, T key) {
        let hash = HashSetHelpers.HashValue(in key, in _hasher);
        var slot = MaybeUninit <T >.Init(key);
        let contains = HashSetIntrinsics.chic_rt_hashset_contains(in _raw, hash, slot.AsValueConstPtr()) != 0;
        unsafe {
            var * mut @expose_address HashSetPtr setPtr = & _raw;
            return new HashSetEntry <T, DefaultHasher >(setPtr, _hasher, hash, slot, contains);
        }
    }
}
