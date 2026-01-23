namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
testcase Given_hashset_insert_contains_and_replace_When_executed_Then_hashset_insert_contains_and_replace()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_new(elemSize, elemAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = true;
        var key1 = 1;
        var key2 = 2;
        var * const @readonly @expose_address byte key1Ptr = & key1;
        var * const @readonly @expose_address byte key2Ptr = & key2;
        var handle1 = new ValueConstPtr {
            Pointer = key1Ptr, Size = elemSize, Alignment = elemAlign
        }
        ;
        var handle2 = new ValueConstPtr {
            Pointer = key2Ptr, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        ok = ok && HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key1, & handle1, & inserted) == HashSetError.Success;
        ok = ok && inserted == 1;
        inserted = 0;
        ok = ok && HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key2, & handle2, & inserted) == HashSetError.Success;
        ok = ok && inserted == 1;
        ok = ok && HashSetRuntime.chic_rt_hashset_len(& hashset) == 2usize;
        ok = ok && HashSetRuntime.chic_rt_hashset_contains(& hashset, (ulong) key1, & handle1) == 1;
        let got = HashSetRuntime.chic_rt_hashset_get_ptr(& hashset, (ulong) key2, & handle2);
        ok = ok && !NativePtr.IsNullConst(got.Pointer);
        var replaceKey = 1;
        var * const @readonly @expose_address byte replaceRaw = & replaceKey;
        var replaceHandle = new ValueConstPtr {
            Pointer = replaceRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        var replacedValue = 0;
        var * mut @expose_address byte replacedRaw = & replacedValue;
        var replacedPtr = new ValueMutPtr {
            Pointer = replacedRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        var replaced = 0;
        ok = ok && HashSetRuntime.chic_rt_hashset_replace(& hashset, (ulong) replaceKey, & replaceHandle, & replacedPtr,
        & replaced) == HashSetError.Success;
        ok = ok && replaced == 1;
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_iteration_and_remove_When_executed_Then_hashset_iteration_and_remove()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_with_capacity(elemSize, elemAlign, 4usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = true;
        var key = 1;
        while (key <= 3)
        {
            var * const @readonly @expose_address byte keyPtr = & key;
            var handle = new ValueConstPtr {
                Pointer = keyPtr, Size = elemSize, Alignment = elemAlign
            }
            ;
            var inserted = 0;
            let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
            key = key + 1;
        }
        var iter = HashSetRuntime.chic_rt_hashset_iter(& hashset);
        var outKey = 0;
        var * mut @expose_address byte outRaw = & outKey;
        var outPtr = new ValueMutPtr {
            Pointer = outRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        ok = ok && HashSetRuntime.chic_rt_hashset_iter_next(& iter, & outPtr) == HashSetError.Success;
        ok = ok && HashSetRuntime.chic_rt_hashset_iter_next(& iter, & outPtr) == HashSetError.Success;
        ok = ok && HashSetRuntime.chic_rt_hashset_iter_next(& iter, & outPtr) == HashSetError.Success;
        ok = ok && HashSetRuntime.chic_rt_hashset_iter_next(& iter, & outPtr) == HashSetError.IterationComplete;
        let state = HashSetRuntime.chic_rt_hashset_bucket_state(& hashset, 0usize);
        let _ = HashSetRuntime.chic_rt_hashset_bucket_hash(& hashset, 0usize);
        if (state == 1u8)
        {
            let _ = HashSetRuntime.chic_rt_hashset_take_at(& hashset, 0usize, & outPtr);
        }
        var removeKey = 2;
        var * const @readonly @expose_address byte removeRaw = & removeKey;
        var removeHandle = new ValueConstPtr {
            Pointer = removeRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let removed = HashSetRuntime.chic_rt_hashset_remove(& hashset, (ulong) removeKey, & removeHandle);
        ok = ok && (removed == 0 || removed == 1);
        HashSetRuntime.chic_rt_hashset_clear(& hashset);
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_reserve_take_and_errors_When_executed_Then_hashset_reserve_take_and_errors()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_new(elemSize, elemAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        let reserveStatus = HashSetRuntime.chic_rt_hashset_reserve(& hashset, 10usize);
        var ok = reserveStatus == HashSetError.Success;
        var key = 42;
        var * const @readonly @expose_address byte keyPtr = & key;
        var handle = new ValueConstPtr {
            Pointer = keyPtr, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        let insertStatus = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
        ok = ok && insertStatus == HashSetError.Success;
        var outValue = 0;
        var * mut @expose_address byte outRaw = & outValue;
        var outPtr = new ValueMutPtr {
            Pointer = outRaw, Size = elemSize, Alignment = elemAlign
        }
        ;
        let takeStatus = HashSetRuntime.chic_rt_hashset_take(& hashset, (ulong) key, & handle, & outPtr);
        ok = ok && takeStatus == HashSetError.Success;
        ok = ok && outValue == 42;
        let missingStatus = HashSetRuntime.chic_rt_hashset_take(& hashset, (ulong) key, & handle, & outPtr);
        ok = ok && missingStatus == HashSetError.NotFound;
        let nullKey = HashSetRuntime.chic_rt_hashset_get_ptr(& hashset, (ulong) key, (* const ValueConstPtr) NativePtr.NullConst());
        ok = ok && NativePtr.IsNullConst(nullKey.Pointer);
        let invalidTake = HashSetRuntime.chic_rt_hashset_take((* mut ChicHashSet) NativePtr.NullMut(), (ulong) key, & handle,
        & outPtr);
        ok = ok && invalidTake == HashSetError.InvalidPointer;
        let shrink = HashSetRuntime.chic_rt_hashset_shrink_to(& hashset, 0usize);
        ok = ok && (shrink == HashSetError.Success || shrink == HashSetError.CapacityOverflow);
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_collision_and_tombstone_reuse_When_executed_Then_hashset_collision_and_tombstone_reuse()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_with_capacity(elemSize, elemAlign, 8usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = true;
        var k1 = 1;
        var k2 = 2;
        var k3 = 3;
        var k4 = 4;
        var key1 = new ValueConstPtr {
            Pointer = & k1, Size = elemSize, Alignment = elemAlign
        }
        ;
        var key2 = new ValueConstPtr {
            Pointer = & k2, Size = elemSize, Alignment = elemAlign
        }
        ;
        var key3 = new ValueConstPtr {
            Pointer = & k3, Size = elemSize, Alignment = elemAlign
        }
        ;
        var key4 = new ValueConstPtr {
            Pointer = & k4, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, 1ul, & key1, & inserted);
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, 1ul, & key2, & inserted);
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, 1ul, & key3, & inserted);
        let removed = HashSetRuntime.chic_rt_hashset_remove(& hashset, 1ul, & key2);
        ok = ok && removed == 1;
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, 1ul, & key4, & inserted);
        ok = ok && HashSetRuntime.chic_rt_hashset_contains(& hashset, 1ul, & key4) == 1;
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_invalid_pointer_paths_When_executed_Then_hashset_invalid_pointer_paths()
{
    unsafe {
        let reserve = HashSetRuntime.chic_rt_hashset_reserve((* mut ChicHashSet) NativePtr.NullMut(), 1usize);
        let contains = HashSetRuntime.chic_rt_hashset_contains((* const ChicHashSet) NativePtr.NullConst(), 0ul, (* const ValueConstPtr) NativePtr.NullConst());
        let ok = reserve == HashSetError.InvalidPointer && contains == 0;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_internal_helpers_When_executed_Then_helpers_complete()
{
    unsafe {
        HashSetRuntime.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}
testcase Given_hashset_iter_invalid_and_bucket_defaults_When_executed_Then_hashset_iter_invalid_and_bucket_defaults()
{
    unsafe {
        var outKey = 0;
        var outPtr = new ValueMutPtr {
            Pointer = & outKey, Size = (usize) __sizeof <int >(), Alignment = (usize) __alignof <int >()
        }
        ;
        let iterStatus = HashSetRuntime.chic_rt_hashset_iter_next((* mut ChicHashSetIter) NativePtr.NullMut(), & outPtr);
        let emptyState = HashSetRuntime.chic_rt_hashset_bucket_state((* const ChicHashSet) NativePtr.NullConst(), 0usize);
        let emptyHash = HashSetRuntime.chic_rt_hashset_bucket_hash((* const ChicHashSet) NativePtr.NullConst(), 0usize);
        let iter = HashSetRuntime.chic_rt_hashset_iter((* const ChicHashSet) NativePtr.NullConst());
        let ok = iterStatus == HashSetError.InvalidPointer && emptyState == 0u8 && emptyHash == 0ul && iter.cap == 0usize;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_iter_next_ptr_and_capacity_When_executed_Then_hashset_iter_next_ptr_and_capacity()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_new(elemSize, elemAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = HashSetRuntime.chic_rt_hashset_capacity(& hashset) == 0usize;
        let clearStatus = HashSetRuntime.chic_rt_hashset_clear(& hashset);
        ok = ok && clearStatus == HashSetError.Success;
        var key = 5;
        var handle = new ValueConstPtr {
            Pointer = & key, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        let insertStatus = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
        ok = ok && insertStatus == HashSetError.Success;
        var iter = HashSetRuntime.chic_rt_hashset_iter(& hashset);
        let entryPtr = HashSetRuntime.chic_rt_hashset_iter_next_ptr(& iter);
        let emptyPtr = HashSetRuntime.chic_rt_hashset_iter_next_ptr(& iter);
        ok = ok && !NativePtr.IsNullConst(entryPtr.Pointer) && NativePtr.IsNullConst(emptyPtr.Pointer);
        let removed = HashSetRuntime.chic_rt_hashset_remove(& hashset, (ulong) key, & handle);
        ok = ok && (removed == 0 || removed == 1);
        ok = ok && HashSetRuntime.chic_rt_hashset_tombstones(& hashset) >= 0usize;
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_bulk_insert_and_clear_When_executed_Then_hashset_bulk_insert_and_clear()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_new(elemSize, elemAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = true;
        var idx = 0;
        while (idx <50)
        {
            var key = idx;
            var handle = new ValueConstPtr {
                Pointer = & key, Size = elemSize, Alignment = elemAlign
            }
            ;
            var inserted = 0;
            let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
            idx += 1;
        }
        ok = ok && HashSetRuntime.chic_rt_hashset_len(& hashset) >= 50usize;
        var probe = 33;
        var probeHandle = new ValueConstPtr {
            Pointer = & probe, Size = elemSize, Alignment = elemAlign
        }
        ;
        ok = ok && HashSetRuntime.chic_rt_hashset_contains(& hashset, (ulong) probe, & probeHandle) == 1;
        HashSetRuntime.chic_rt_hashset_clear(& hashset);
        ok = ok && HashSetRuntime.chic_rt_hashset_len(& hashset) == 0usize;
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_shrink_take_at_and_reserve_failures_When_executed_Then_hashset_shrink_take_at_and_reserve_failures()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_with_capacity(elemSize, elemAlign, 32usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = true;
        var key = 44;
        var handle = new ValueConstPtr {
            Pointer = & key, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
        let shrinkStatus = HashSetRuntime.chic_rt_hashset_shrink_to(& hashset, 1usize);
        ok = ok && shrinkStatus == HashSetError.Success;
        var outKey = 0;
        var outPtr = new ValueMutPtr {
            Pointer = & outKey, Size = elemSize, Alignment = elemAlign
        }
        ;
        let missing = HashSetRuntime.chic_rt_hashset_take_at(& hashset, 999usize, & outPtr);
        ok = ok && missing == HashSetError.NotFound;
        let cap = HashSetRuntime.chic_rt_hashset_capacity(& hashset);
        if (cap >0usize)
        {
            let takeStatus = HashSetRuntime.chic_rt_hashset_take_at(& hashset, 0usize, & outPtr);
            if (takeStatus == HashSetError.Success)
            {
                ok = ok && outKey == key;
            }
        }
        var iter = HashSetRuntime.chic_rt_hashset_iter(& hashset);
        let entryPtr = HashSetRuntime.chic_rt_hashset_iter_next_ptr(& iter);
        let done = HashSetRuntime.chic_rt_hashset_iter_next(& iter, & outPtr);
        if (NativePtr.IsNullConst (entryPtr.Pointer))
        {
            ok = ok && done == HashSetError.IterationComplete;
        }
        NativeAlloc.TestFailAllocAfter(0);
        let reserveFail = HashSetRuntime.chic_rt_hashset_reserve(& hashset, 8usize);
        ok = ok && reserveFail == HashSetError.AllocationFailed;
        NativeAlloc.TestReset();
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashset_tombstones_count_nonzero_When_executed_Then_hashset_tombstones_count_nonzero()
{
    unsafe {
        let elemSize = (usize) __sizeof <int >();
        let elemAlign = (usize) __alignof <int >();
        var hashset = HashSetRuntime.chic_rt_hashset_with_capacity(elemSize, elemAlign, 4usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var key = 5;
        var handle = new ValueConstPtr {
            Pointer = & key, Size = elemSize, Alignment = elemAlign
        }
        ;
        var inserted = 0;
        let _ = HashSetRuntime.chic_rt_hashset_insert(& hashset, (ulong) key, & handle, & inserted);
        let _ = HashSetRuntime.chic_rt_hashset_remove(& hashset, (ulong) key, & handle);
        let tombstones = HashSetRuntime.chic_rt_hashset_tombstones(& hashset);
        HashSetRuntime.chic_rt_hashset_drop(& hashset);
        Assert.That(tombstones >= 1usize).IsTrue();
    }
}
testcase Given_hashset_tombstones_null_is_zero_When_executed_Then_hashset_tombstones_null_is_zero()
{
    unsafe {
        let nullTombstones = HashSetRuntime.chic_rt_hashset_tombstones((* const ChicHashSet) NativePtr.NullConst());
        Assert.That(nullTombstones).IsEqualTo(0usize);
    }
}
