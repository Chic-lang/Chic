namespace Std.Collections;
import Std;
import Std.Core;
import Std.Hashing;
import Std.Memory;
import Std.Numeric;
import Std.Runtime;
import Std.Runtime.Collections;
internal static class HashMapIntrinsics
{
    @extern("C") public static extern HashMapPtr chic_rt_hashmap_new(usize keySize, usize keyAlignment, usize valueSize,
    usize valueAlignment, isize keyDropFn, isize valueDropFn, isize keyEqFn);
    @extern("C") public static extern HashMapPtr chic_rt_hashmap_with_capacity(usize keySize, usize keyAlignment, usize valueSize,
    usize valueAlignment, usize capacity, isize keyDropFn, isize valueDropFn, isize keyEqFn);
    @extern("C") public static extern void chic_rt_hashmap_drop(ref HashMapPtr mapPtr);
    @extern("C") public static extern HashMapError chic_rt_hashmap_clear(ref HashMapPtr mapPtr);
    @extern("C") public static extern HashMapError chic_rt_hashmap_reserve(ref HashMapPtr mapPtr, usize additional);
    @extern("C") public static extern HashMapError chic_rt_hashmap_shrink_to(ref HashMapPtr mapPtr, usize minCapacity);
    @extern("C") public static extern usize chic_rt_hashmap_len(in HashMapPtr mapPtr);
    @extern("C") public static extern usize chic_rt_hashmap_capacity(in HashMapPtr mapPtr);
    @extern("C") public static extern HashMapError chic_rt_hashmap_insert(ref HashMapPtr mapPtr, ulong hash, ValueConstPtr key,
    ValueConstPtr value, ValueMutPtr previousValue, out int replaced);
    @extern("C") public static extern int chic_rt_hashmap_contains(in HashMapPtr mapPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern ValueConstPtr chic_rt_hashmap_get_ptr(in HashMapPtr mapPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern HashMapError chic_rt_hashmap_take(ref HashMapPtr mapPtr, ulong hash, ValueConstPtr key,
    ValueMutPtr destination);
    @extern("C") public static extern int chic_rt_hashmap_remove(ref HashMapPtr mapPtr, ulong hash, ValueConstPtr key);
    @extern("C") public static extern byte chic_rt_hashmap_bucket_state(in HashMapPtr mapPtr, usize index);
    @extern("C") public static extern ulong chic_rt_hashmap_bucket_hash(in HashMapPtr mapPtr, usize index);
    @extern("C") public static extern HashMapError chic_rt_hashmap_take_at(ref HashMapPtr mapPtr, usize index, ValueMutPtr keyDestination,
    ValueMutPtr valueDestination);
    @extern("C") public static extern HashMapIterPtr chic_rt_hashmap_iter(in HashMapPtr mapPtr);
    @extern("C") public static extern HashMapError chic_rt_hashmap_iter_next(ref HashMapIterPtr iter, ValueMutPtr keyDestination,
    ValueMutPtr valueDestination);
}
public delegate void HashMapMutator <V >(ref V value);
internal static class HashMapHelpers
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
    public static ulong HashKey <K, THasher >(in K key, THasher hasher) where THasher : IHasher, Copy {
        return Hashing.HashValue(in key, hasher);
    }
}
public struct HashMapIterator <K, V >
{
    private HashMapIterPtr _raw;
    public init(HashMapIterPtr raw) {
        _raw = raw;
    }
    public bool Next(ref this, out K key, out V value) {
        var keySlot = MaybeUninit <K >.Uninit();
        var valueSlot = MaybeUninit <V >.Uninit();
        let status = HashMapIntrinsics.chic_rt_hashmap_iter_next(ref _raw, keySlot.AsValueMutPtr(), valueSlot.AsValueMutPtr());
        if (status == HashMapError.Success)
        {
            keySlot.MarkInitialized();
            valueSlot.MarkInitialized();
            key = keySlot.AssumeInit();
            value = valueSlot.AssumeInit();
            return true;
        }
        key = Intrinsics.ZeroValue <K >();
        value = Intrinsics.ZeroValue <V >();
        return false;
    }
}
public struct HashMapWithHasher <K, V, THasher >where THasher : IHasher, Copy {
    private HashMapPtr _raw;
    private THasher _hasher;
    private init(THasher hasher, HashMapPtr raw) {
        _hasher = hasher;
        _raw = raw;
    }
    private static HashMapPtr CreateRaw() {
        let keySize = (usize) __sizeof <K >();
        let keyAlign = (usize) __alignof <K >();
        let valueSize = (usize) __sizeof <V >();
        let valueAlign = (usize) __alignof <V >();
        let keyDropFn = (isize) __drop_glue_of <K >();
        let valueDropFn = (isize) __drop_glue_of <V >();
        let keyEqFn = (isize) __eq_glue_of <K >();
        if (keyEqFn == 0)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashmap requires op_Equality for the key type"));
        }
        return HashMapIntrinsics.chic_rt_hashmap_new(keySize, keyAlign, valueSize, valueAlign, keyDropFn, valueDropFn,
        keyEqFn);
    }
    public init(THasher hasher) {
        _hasher = hasher;
        _raw = CreateRaw();
    }
    public static HashMapWithHasher <K, V, THasher >WithCapacity(THasher hasher, usize capacity) {
        let keySize = (usize) __sizeof <K >();
        let keyAlign = (usize) __alignof <K >();
        let valueSize = (usize) __sizeof <V >();
        let valueAlign = (usize) __alignof <V >();
        let keyDropFn = (isize) __drop_glue_of <K >();
        let valueDropFn = (isize) __drop_glue_of <V >();
        let keyEqFn = (isize) __eq_glue_of <K >();
        if (keyEqFn == 0)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashmap requires op_Equality for the key type"));
        }
        let raw = HashMapIntrinsics.chic_rt_hashmap_with_capacity(keySize, keyAlign, valueSize, valueAlign, capacity,
        keyDropFn, valueDropFn, keyEqFn);
        return new HashMapWithHasher <K, V, THasher >(hasher, raw);
    }
    public void dispose(ref this) {
        HashMapIntrinsics.chic_rt_hashmap_drop(ref _raw);
        _raw = CoreIntrinsics.DefaultValue <HashMapPtr >();
    }
    public usize Len(in this) => HashMapIntrinsics.chic_rt_hashmap_len(in _raw);
    public usize Capacity(in this) => HashMapIntrinsics.chic_rt_hashmap_capacity(in _raw);
    public HashMapError Reserve(ref this, usize additional) => HashMapIntrinsics.chic_rt_hashmap_reserve(ref _raw,
    additional);
    public HashMapError ShrinkTo(ref this, usize minCapacity) => HashMapIntrinsics.chic_rt_hashmap_shrink_to(ref _raw,
    minCapacity);
    public HashMapError Clear(ref this) => HashMapIntrinsics.chic_rt_hashmap_clear(ref _raw);
    public HashMapError Insert(ref this, K key, V value, out Option <V >previous) {
        let hash = HashMapHelpers.HashKey(in key, in _hasher);
        var keySlot = MaybeUninit <K >.Init(key);
        var valueSlot = MaybeUninit <V >.Init(value);
        var previousSlot = MaybeUninit <V >.Uninit();
        var flag = 0;
        let status = HashMapIntrinsics.chic_rt_hashmap_insert(ref _raw, hash, keySlot.AsValueConstPtr(), valueSlot.AsValueConstPtr(),
        previousSlot.AsValueMutPtr(), out flag);
        if (status == HashMapError.Success)
        {
            keySlot.ForgetInit();
            valueSlot.ForgetInit();
            if (flag != 0)
            {
                previousSlot.MarkInitialized();
                previous = Option <V >.Some(previousSlot.AssumeInit());
            }
            else
            {
                previous = Option <V >.None();
            }
        }
        else
        {
            previous = Option <V >.None();
        }
        return status;
    }
    public bool ContainsKey(in this, in K key) {
        let hash = HashMapHelpers.HashKey(in key, in _hasher);
        let keyHandle = HashMapHelpers.ConstPtrFrom(in key);
        return HashMapIntrinsics.chic_rt_hashmap_contains(in _raw, hash, keyHandle) != 0;
    }
    public Option <V >Get(in this, in K key) {
        let hash = HashMapHelpers.HashKey(in key, in _hasher);
        let keyHandle = HashMapHelpers.ConstPtrFrom(in key);
        let ptr = HashMapIntrinsics.chic_rt_hashmap_get_ptr(in _raw, hash, keyHandle);
        if (ValuePointer.IsNullConst (ptr))
        {
            return Option <V >.None();
        }
        let cloneGlue = __clone_glue_of <V >();
        var slot = MaybeUninit <V >.Uninit();
        if (cloneGlue != 0isize)
        {
            CloneRuntime.Invoke(cloneGlue, ptr, slot.AsValueMutPtr());
            slot.MarkInitialized();
            return Option <V >.Some(slot.AssumeInit());
        }
        let dropGlue = (isize) __drop_glue_of <V >();
        if (dropGlue != 0isize)
        {
            throw new InvalidOperationException(StringRuntime.FromStr("hashmap Get requires Copy or clone glue for the value type"));
        }
        if (ptr.Size != 0usize)
        {
            GlobalAllocator.Copy(slot.AsValueMutPtr(), ptr, ptr.Size);
        }
        slot.MarkInitialized();
        return Option <V >.Some(slot.AssumeInit());
    }
    public Option <V >Take(ref this, in K key) {
        let hash = HashMapHelpers.HashKey(in key, in _hasher);
        let keyHandle = HashMapHelpers.ConstPtrFrom(in key);
        var slot = MaybeUninit <V >.Uninit();
        let status = HashMapIntrinsics.chic_rt_hashmap_take(ref _raw, hash, keyHandle, slot.AsValueMutPtr());
        if (status != HashMapError.Success)
        {
            return Option <V >.None();
        }
        slot.MarkInitialized();
        return Option <V >.Some(slot.AssumeInit());
    }
    public bool Remove(ref this, in K key) {
        let hash = HashMapHelpers.HashKey(in key, in _hasher);
        let keyHandle = HashMapHelpers.ConstPtrFrom(in key);
        return HashMapIntrinsics.chic_rt_hashmap_remove(ref _raw, hash, keyHandle) != 0;
    }
    public HashMapIterator <K, V >Iter(in this) {
        let iter = HashMapIntrinsics.chic_rt_hashmap_iter(in _raw);
        return new HashMapIterator <K, V >(iter);
    }
}
public struct HashMap <K, V >
{
    private HashMapWithHasher <K, V, DefaultHasher >_inner;
    public init() {
        _inner = new HashMapWithHasher <K, V, DefaultHasher >(new DefaultHasher());
    }
    private init(HashMapWithHasher <K, V, DefaultHasher >inner) {
        _inner = inner;
    }
    public static HashMap <K, V >WithCapacity(usize capacity) {
        return new HashMap <K, V >(HashMapWithHasher <K, V, DefaultHasher >.WithCapacity(new DefaultHasher(), capacity));
    }
    public void dispose(ref this) {
        _inner.dispose();
    }
    public usize Len(in this) => _inner.Len();
    public usize Capacity(in this) => _inner.Capacity();
    public HashMapError Reserve(ref this, usize additional) => _inner.Reserve(additional);
    public HashMapError ShrinkTo(ref this, usize minCapacity) => _inner.ShrinkTo(minCapacity);
    public HashMapError Clear(ref this) => _inner.Clear();
    public HashMapError Insert(ref this, K key, V value, out Option <V >previous) => _inner.Insert(key, value, out previous);
    public bool ContainsKey(in this, in K key) => _inner.ContainsKey(in key);
    public Option <V >Get(in this, in K key) => _inner.Get(in key);
    public Option <V >Take(ref this, in K key) => _inner.Take(in key);
    public bool Remove(ref this, in K key) => _inner.Remove(in key);
    public HashMapIterator <K, V >Iter(in this) => _inner.Iter();
}
