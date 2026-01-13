namespace Foundation.Collections;
import Std.Runtime.Collections;

public enum HashMapError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, NotFound = 4, IterationComplete = 5,
}

public static class HashMapIntrinsics
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
