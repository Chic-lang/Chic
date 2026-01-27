namespace Foundation.Collections;
import Std.Runtime.Collections;
public enum HashSetError
{
    Success = 0, AllocationFailed = 1, InvalidPointer = 2, CapacityOverflow = 3, NotFound = 4, IterationComplete = 5,
}
public static class HashSetIntrinsics
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
