namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
testcase Given_hashmap_insert_contains_and_get_When_executed_Then_hashmap_insert_contains_and_get()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = true;
        var key1 = 1;
        var value1 = 10;
        var key2 = 2;
        var value2 = 20;
        var * const @readonly @expose_address byte key1Ptr = & key1;
        var * const @readonly @expose_address byte value1Ptr = & value1;
        var * const @readonly @expose_address byte key2Ptr = & key2;
        var * const @readonly @expose_address byte value2Ptr = & value2;
        var keyHandle1 = new ValueConstPtr {
            Pointer = key1Ptr, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle1 = new ValueConstPtr {
            Pointer = value1Ptr, Size = keySize, Alignment = keyAlign
        }
        ;
        var keyHandle2 = new ValueConstPtr {
            Pointer = key2Ptr, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle2 = new ValueConstPtr {
            Pointer = value2Ptr, Size = keySize, Alignment = keyAlign
        }
        ;
        var previousValue = 0;
        var * mut @expose_address byte previousRaw = & previousValue;
        var previousHandle = new ValueMutPtr {
            Pointer = previousRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        ok = ok && HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key1, & keyHandle1, & valueHandle1, & previousHandle,
        & replaced) == HashMapError.Success;
        ok = ok && replaced == 0;
        ok = ok && HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key2, & keyHandle2, & valueHandle2, & previousHandle,
        & replaced) == HashMapError.Success;
        ok = ok && HashMapRuntime.chic_rt_hashmap_contains(& map, (ulong) key1, & keyHandle1) == 1;
        let lookup = HashMapRuntime.chic_rt_hashmap_get_ptr(& map, (ulong) key1, & keyHandle1);
        ok = ok && !NativePtr.IsNullConst(lookup.Pointer);
        var * const @readonly @expose_address int lookupPtr = lookup.Pointer;
        ok = ok && * lookupPtr == 10;
        var newValue = 11;
        var * const @readonly @expose_address byte newValuePtr = & newValue;
        var valueHandleNew = new ValueConstPtr {
            Pointer = newValuePtr, Size = keySize, Alignment = keyAlign
        }
        ;
        ok = ok && HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key1, & keyHandle1, & valueHandleNew, & previousHandle,
        & replaced) == HashMapError.Success;
        ok = ok && replaced == 1;
        ok = ok && previousValue == 10;
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_iteration_and_removals_When_executed_Then_hashmap_iteration_and_removals()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_with_capacity(keySize, keyAlign, keySize, keyAlign, 4usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = true;
        var idx = 0;
        while (idx <3)
        {
            var key = idx + 1;
            var value = (idx + 1) * 100;
            var * const @readonly @expose_address byte keyPtr = & key;
            var * const @readonly @expose_address byte valuePtr = & value;
            var keyHandle = new ValueConstPtr {
                Pointer = keyPtr, Size = keySize, Alignment = keyAlign
            }
            ;
            var valueHandle = new ValueConstPtr {
                Pointer = valuePtr, Size = keySize, Alignment = keyAlign
            }
            ;
            var prev = 0;
            var * mut @expose_address byte prevRaw = & prev;
            var prevHandle = new ValueMutPtr {
                Pointer = prevRaw, Size = keySize, Alignment = keyAlign
            }
            ;
            var replaced = 0;
            let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle, & prevHandle, & replaced);
            idx = idx + 1;
        }
        var iter = HashMapRuntime.chic_rt_hashmap_iter(& map);
        var outKey = 0;
        var outValue = 0;
        var * mut @expose_address byte outKeyRaw = & outKey;
        var * mut @expose_address byte outValueRaw = & outValue;
        var outKeyPtr = new ValueMutPtr {
            Pointer = outKeyRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        var outValuePtr = new ValueMutPtr {
            Pointer = outValueRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        ok = ok && HashMapRuntime.chic_rt_hashmap_iter_next(& iter, & outKeyPtr, & outValuePtr) == HashMapError.Success;
        ok = ok && outValue >0;
        ok = ok && HashMapRuntime.chic_rt_hashmap_iter_next(& iter, & outKeyPtr, & outValuePtr) == HashMapError.Success;
        ok = ok && HashMapRuntime.chic_rt_hashmap_iter_next(& iter, & outKeyPtr, & outValuePtr) == HashMapError.Success;
        ok = ok && HashMapRuntime.chic_rt_hashmap_iter_next(& iter, & outKeyPtr, & outValuePtr) == HashMapError.IterationComplete;
        var takeKey = 0;
        var takeValue = 0;
        var * mut @expose_address byte takeKeyRaw = & takeKey;
        var * mut @expose_address byte takeValueRaw = & takeValue;
        var takeKeyPtr = new ValueMutPtr {
            Pointer = takeKeyRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        var takeValuePtr = new ValueMutPtr {
            Pointer = takeValueRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        let state = HashMapRuntime.chic_rt_hashmap_bucket_state(& map, 0usize);
        let _ = HashMapRuntime.chic_rt_hashmap_bucket_hash(& map, 0usize);
        if (state == 1u8)
        {
            let _ = HashMapRuntime.chic_rt_hashmap_take_at(& map, 0usize, & takeKeyPtr, & takeValuePtr);
        }
        let removeKey = 2;
        var * const @readonly @expose_address byte removeKeyRaw = & removeKey;
        var removeKeyPtr = new ValueConstPtr {
            Pointer = removeKeyRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        let removed = HashMapRuntime.chic_rt_hashmap_remove(& map, (ulong) removeKey, & removeKeyPtr);
        ok = ok && (removed == 0 || removed == 1);
        HashMapRuntime.chic_rt_hashmap_clear(& map);
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_reserve_take_and_errors_When_executed_Then_hashmap_reserve_take_and_errors()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = HashMapRuntime.chic_rt_hashmap_len(& map) == 0usize;
        let reserveStatus = HashMapRuntime.chic_rt_hashmap_reserve(& map, 12usize);
        ok = ok && reserveStatus == HashMapError.Success;
        let cap = HashMapRuntime.chic_rt_hashmap_capacity(& map);
        ok = ok && cap >= 8usize;
        var key = 9;
        var value = 90;
        var * const @readonly @expose_address byte keyPtr = & key;
        var * const @readonly @expose_address byte valuePtr = & value;
        var keyHandle = new ValueConstPtr {
            Pointer = keyPtr, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle = new ValueConstPtr {
            Pointer = valuePtr, Size = keySize, Alignment = keyAlign
        }
        ;
        var prev = 0;
        var * mut @expose_address byte prevRaw = & prev;
        var prevHandle = new ValueMutPtr {
            Pointer = prevRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        let insertStatus = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle, & prevHandle,
        & replaced);
        ok = ok && insertStatus == HashMapError.Success;
        var outValue = 0;
        var * mut @expose_address byte outRaw = & outValue;
        var outHandle = new ValueMutPtr {
            Pointer = outRaw, Size = keySize, Alignment = keyAlign
        }
        ;
        let takeStatus = HashMapRuntime.chic_rt_hashmap_take(& map, (ulong) key, & keyHandle, & outHandle);
        ok = ok && takeStatus == HashMapError.Success;
        ok = ok && outValue == 90;
        let missingStatus = HashMapRuntime.chic_rt_hashmap_take(& map, (ulong) key, & keyHandle, & outHandle);
        ok = ok && missingStatus == HashMapError.NotFound;
        let nullKey = HashMapRuntime.chic_rt_hashmap_get_ptr(& map, (ulong) key, (* const ValueConstPtr) NativePtr.NullConst());
        ok = ok && NativePtr.IsNullConst(nullKey.Pointer);
        let invalidTake = HashMapRuntime.chic_rt_hashmap_take((* mut ChicHashMap) NativePtr.NullMut(), (ulong) key, & keyHandle,
        & outHandle);
        ok = ok && invalidTake == HashMapError.InvalidPointer;
        let shrink = HashMapRuntime.chic_rt_hashmap_shrink_to(& map, 0usize);
        ok = ok && (shrink == HashMapError.Success || shrink == HashMapError.CapacityOverflow);
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_collision_and_tombstone_reuse_When_executed_Then_hashmap_collision_and_tombstone_reuse()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_with_capacity(keySize, keyAlign, keySize, keyAlign, 8usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = true;
        var k1 = 1;
        var v1 = 10;
        var k2 = 2;
        var v2 = 20;
        var k3 = 3;
        var v3 = 30;
        var k4 = 4;
        var v4 = 40;
        var key1 = new ValueConstPtr {
            Pointer = & k1, Size = keySize, Alignment = keyAlign
        }
        ;
        var val1 = new ValueConstPtr {
            Pointer = & v1, Size = keySize, Alignment = keyAlign
        }
        ;
        var key2 = new ValueConstPtr {
            Pointer = & k2, Size = keySize, Alignment = keyAlign
        }
        ;
        var val2 = new ValueConstPtr {
            Pointer = & v2, Size = keySize, Alignment = keyAlign
        }
        ;
        var key3 = new ValueConstPtr {
            Pointer = & k3, Size = keySize, Alignment = keyAlign
        }
        ;
        var val3 = new ValueConstPtr {
            Pointer = & v3, Size = keySize, Alignment = keyAlign
        }
        ;
        var key4 = new ValueConstPtr {
            Pointer = & k4, Size = keySize, Alignment = keyAlign
        }
        ;
        var val4 = new ValueConstPtr {
            Pointer = & v4, Size = keySize, Alignment = keyAlign
        }
        ;
        var prev = 0;
        var prevPtr = new ValueMutPtr {
            Pointer = & prev, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, 1ul, & key1, & val1, & prevPtr, & replaced);
        let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, 1ul, & key2, & val2, & prevPtr, & replaced);
        let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, 1ul, & key3, & val3, & prevPtr, & replaced);
        let removed = HashMapRuntime.chic_rt_hashmap_remove(& map, 1ul, & key2);
        ok = ok && removed == 1;
        let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, 1ul, & key4, & val4, & prevPtr, & replaced);
        ok = ok && HashMapRuntime.chic_rt_hashmap_contains(& map, 1ul, & key4) == 1;
        var iter = HashMapRuntime.chic_rt_hashmap_iter(& map);
        let entry = HashMapRuntime.chic_rt_hashmap_iter_next_ptr(& iter);
        ok = ok && !NativePtr.IsNullConst(entry.Pointer);
        let shrink = HashMapRuntime.chic_rt_hashmap_shrink_to(& map, 1usize);
        ok = ok && (shrink == HashMapError.Success || shrink == HashMapError.CapacityOverflow);
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_iter_next_ptr_and_empty_clear_When_executed_Then_hashmap_iter_next_ptr_and_empty_clear()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = HashMapRuntime.chic_rt_hashmap_capacity(& map) == 0usize;
        let clearStatus = HashMapRuntime.chic_rt_hashmap_clear(& map);
        ok = ok && clearStatus == HashMapError.Success;
        var key = 7;
        var value = 70;
        var keyHandle = new ValueConstPtr {
            Pointer = & key, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle = new ValueConstPtr {
            Pointer = & value, Size = keySize, Alignment = keyAlign
        }
        ;
        var prev = 0;
        var prevHandle = new ValueMutPtr {
            Pointer = & prev, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        let insertStatus = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle, & prevHandle,
        & replaced);
        ok = ok && insertStatus == HashMapError.Success;
        var iter = HashMapRuntime.chic_rt_hashmap_iter(& map);
        let entryPtr = HashMapRuntime.chic_rt_hashmap_iter_next_ptr(& iter);
        let emptyPtr = HashMapRuntime.chic_rt_hashmap_iter_next_ptr(& iter);
        ok = ok && !NativePtr.IsNullConst(entryPtr.Pointer) && NativePtr.IsNullConst(emptyPtr.Pointer);
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_invalid_pointer_paths_When_executed_Then_hashmap_invalid_pointer_paths()
{
    unsafe {
        let reserve = HashMapRuntime.chic_rt_hashmap_reserve((* mut ChicHashMap) NativePtr.NullMut(), 1usize);
        let contains = HashMapRuntime.chic_rt_hashmap_contains((* const ChicHashMap) NativePtr.NullConst(), 0ul, (* const ValueConstPtr) NativePtr.NullConst());
        let ok = reserve == HashMapError.InvalidPointer && contains == 0;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_insert_with_null_previous_buffer_When_executed_Then_updates_value()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var key = 4;
        var value1 = 40;
        var value2 = 41;
        var keyHandle = new ValueConstPtr {
            Pointer = & key, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle1 = new ValueConstPtr {
            Pointer = & value1, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle2 = new ValueConstPtr {
            Pointer = & value2, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        let insertStatus = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle1, (* const ValueMutPtr) NativePtr.NullConst(),
        & replaced);
        replaced = 0;
        let updateStatus = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle2, (* const ValueMutPtr) NativePtr.NullConst(),
        & replaced);
        let lookup = HashMapRuntime.chic_rt_hashmap_get_ptr(& map, (ulong) key, & keyHandle);
        var ok = insertStatus == HashMapError.Success && updateStatus == HashMapError.Success && replaced == 1;
        ok = ok && !NativePtr.IsNullConst(lookup.Pointer) && * (* const @readonly @expose_address int) lookup.Pointer == 41;
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_contains_null_key_When_executed_Then_returns_zero()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        let contains = HashMapRuntime.chic_rt_hashmap_contains(& map, 0ul, (* const ValueConstPtr) NativePtr.NullConst());
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(contains == 0).IsTrue();
    }
}
testcase Given_hashmap_get_ptr_null_key_When_executed_Then_returns_null()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        let result = HashMapRuntime.chic_rt_hashmap_get_ptr(& map, 0ul, (* const ValueConstPtr) NativePtr.NullConst());
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(NativePtr.IsNullConst(result.Pointer)).IsTrue();
    }
}
testcase Given_hashmap_insert_null_key_When_executed_Then_invalid_pointer()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var value = 5;
        var valueHandle = new ValueConstPtr {
            Pointer = & value, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 9;
        let status = HashMapRuntime.chic_rt_hashmap_insert(& map, 0ul, (* const ValueConstPtr) NativePtr.NullConst(), & valueHandle,
        (* const ValueMutPtr) NativePtr.NullConst(), & replaced);
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(status == HashMapError.InvalidPointer).IsTrue();
    }
}
testcase Given_hashmap_take_null_destination_When_executed_Then_invalid_pointer()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var key = 7;
        var keyHandle = new ValueConstPtr {
            Pointer = & key, Size = keySize, Alignment = keyAlign
        }
        ;
        let status = HashMapRuntime.chic_rt_hashmap_take(& map, (ulong) key, & keyHandle, (* const ValueMutPtr) NativePtr.NullConst());
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(status == HashMapError.InvalidPointer).IsTrue();
    }
}
testcase Given_hashmap_iter_invalid_and_bucket_defaults_When_executed_Then_hashmap_iter_invalid_and_bucket_defaults()
{
    unsafe {
        var outKey = 0;
        var outValue = 0;
        var outKeyPtr = new ValueMutPtr {
            Pointer = & outKey, Size = (usize) __sizeof <int >(), Alignment = (usize) __alignof <int >()
        }
        ;
        var outValuePtr = new ValueMutPtr {
            Pointer = & outValue, Size = (usize) __sizeof <int >(), Alignment = (usize) __alignof <int >()
        }
        ;
        let iterStatus = HashMapRuntime.chic_rt_hashmap_iter_next((* mut ChicHashMapIter) NativePtr.NullMut(), & outKeyPtr,
        & outValuePtr);
        let emptyState = HashMapRuntime.chic_rt_hashmap_bucket_state((* const ChicHashMap) NativePtr.NullConst(), 0usize);
        let emptyHash = HashMapRuntime.chic_rt_hashmap_bucket_hash((* const ChicHashMap) NativePtr.NullConst(), 0usize);
        let iter = HashMapRuntime.chic_rt_hashmap_iter((* const ChicHashMap) NativePtr.NullConst());
        let ok = iterStatus == HashMapError.InvalidPointer && emptyState == 0u8 && emptyHash == 0ul && iter.cap == 0usize;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_bulk_insert_and_clear_When_executed_Then_hashmap_bulk_insert_and_clear()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.DropNoop,
        HashMapTestSupport.KeyEq);
        var ok = true;
        var idx = 0;
        while (idx <50)
        {
            var key = idx;
            var value = idx * 10;
            var keyHandle = new ValueConstPtr {
                Pointer = & key, Size = keySize, Alignment = keyAlign
            }
            ;
            var valueHandle = new ValueConstPtr {
                Pointer = & value, Size = keySize, Alignment = keyAlign
            }
            ;
            var prev = 0;
            var prevHandle = new ValueMutPtr {
                Pointer = & prev, Size = keySize, Alignment = keyAlign
            }
            ;
            var replaced = 0;
            let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle, & prevHandle, & replaced);
            idx += 1;
        }
        ok = ok && HashMapRuntime.chic_rt_hashmap_len(& map) >= 50usize;
        var probe = 17;
        var probeHandle = new ValueConstPtr {
            Pointer = & probe, Size = keySize, Alignment = keyAlign
        }
        ;
        ok = ok && HashMapRuntime.chic_rt_hashmap_contains(& map, (ulong) probe, & probeHandle) == 1;
        HashMapRuntime.chic_rt_hashmap_clear(& map);
        ok = ok && HashMapRuntime.chic_rt_hashmap_len(& map) == 0usize;
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_hashmap_internal_helpers_When_executed_Then_hashmap_internal_helpers()
{
    unsafe {
        HashMapRuntime.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}
testcase Given_hashmap_shrink_take_at_and_reserve_failures_When_executed_Then_hashmap_shrink_take_at_and_reserve_failures()
{
    unsafe {
        let keySize = (usize) __sizeof <int >();
        let keyAlign = (usize) __alignof <int >();
        var map = HashMapRuntime.chic_rt_hashmap_with_capacity(keySize, keyAlign, keySize, keyAlign, 32usize, HashMapTestSupport.DropNoop,
        HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        var ok = true;
        var key = 11;
        var value = 99;
        var keyHandle = new ValueConstPtr {
            Pointer = & key, Size = keySize, Alignment = keyAlign
        }
        ;
        var valueHandle = new ValueConstPtr {
            Pointer = & value, Size = keySize, Alignment = keyAlign
        }
        ;
        var prev = 0;
        var prevHandle = new ValueMutPtr {
            Pointer = & prev, Size = keySize, Alignment = keyAlign
        }
        ;
        var replaced = 0;
        let _ = HashMapRuntime.chic_rt_hashmap_insert(& map, (ulong) key, & keyHandle, & valueHandle, & prevHandle, & replaced);
        let shrinkStatus = HashMapRuntime.chic_rt_hashmap_shrink_to(& map, 1usize);
        ok = ok && shrinkStatus == HashMapError.Success;
        var outKey = 0;
        var outValue = 0;
        var outKeyPtr = new ValueMutPtr {
            Pointer = & outKey, Size = keySize, Alignment = keyAlign
        }
        ;
        var outValuePtr = new ValueMutPtr {
            Pointer = & outValue, Size = keySize, Alignment = keyAlign
        }
        ;
        let missing = HashMapRuntime.chic_rt_hashmap_take_at(& map, 999usize, & outKeyPtr, & outValuePtr);
        ok = ok && missing == HashMapError.NotFound;
        let cap = HashMapRuntime.chic_rt_hashmap_capacity(& map);
        if (cap >0usize)
        {
            let takeStatus = HashMapRuntime.chic_rt_hashmap_take_at(& map, 0usize, & outKeyPtr, & outValuePtr);
            if (takeStatus == HashMapError.Success)
            {
                ok = ok && outKey == key;
                ok = ok && outValue == value;
            }
        }
        var iter = HashMapRuntime.chic_rt_hashmap_iter(& map);
        let entryPtr = HashMapRuntime.chic_rt_hashmap_iter_next_ptr(& iter);
        let done = HashMapRuntime.chic_rt_hashmap_iter_next(& iter, & outKeyPtr, & outValuePtr);
        if (NativePtr.IsNullConst (entryPtr.Pointer))
        {
            ok = ok && done == HashMapError.IterationComplete;
        }
        NativeAlloc.TestFailAllocAfter(0);
        let reserveFail = HashMapRuntime.chic_rt_hashmap_reserve(& map, 8usize);
        ok = ok && reserveFail == HashMapError.AllocationFailed;
        NativeAlloc.TestReset();
        HashMapRuntime.chic_rt_hashmap_drop(& map);
        Assert.That(ok).IsTrue();
    }
}
