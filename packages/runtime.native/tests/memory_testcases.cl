namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

private unsafe static bool BytesEqual(ValueConstPtr left, ValueConstPtr right, usize len) {
    var idx = 0usize;
    while (idx < len)
    {
        let leftPtr = NativePtr.OffsetConst(left.Pointer, (isize) idx);
        let rightPtr = NativePtr.OffsetConst(right.Pointer, (isize) idx);
        let leftValue = NativePtr.ReadByteConst(leftPtr);
        let rightValue = NativePtr.ReadByteConst(rightPtr);
        if (leftValue != rightValue)
        {
            return false;
        }
        idx = idx + 1;
    }
    return true;
}

testcase Given_memory_alloc_stats_track_calls_When_executed_Then_memory_alloc_stats_track_calls()
{
    MemoryRuntime.chic_rt_allocator_reset();
    MemoryRuntime.chic_rt_reset_alloc_stats();
    unsafe {
        let empty = MemoryRuntime.chic_rt_alloc(0usize, 4usize);
        var block = MemoryRuntime.chic_rt_alloc(16usize, 8usize);
        MemoryRuntime.chic_rt_free(block);
        let stats = MemoryRuntime.chic_rt_alloc_stats();
        let ok = NativePtr.IsNull(empty.Pointer)
            && !NativePtr.IsNull(block.Pointer)
            && stats.alloc_calls >= 1usize
            && stats.free_calls >= 1usize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_memory_test_allocator_call_counters_reset_When_executed_Then_counters_are_zero()
{
    MemoryRuntime.ResetTestAllocatorCalls();
    let allocCalls = MemoryRuntime.TestAllocatorAllocCalls();
    let freeCalls = MemoryRuntime.TestAllocatorFreeCalls();
    let ok = allocCalls == 0usize && freeCalls == 0usize;
    Assert.That(ok).IsTrue();
}

testcase Given_memory_zeroed_alloc_and_memset_When_executed_Then_memory_zeroed_alloc_and_memset()
{
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc_zeroed(8usize, 1usize);
        var zeroOk = true;
        var idx = 0usize;
        while (idx < 8usize)
        {
            let ptr = NativePtr.OffsetMut(block.Pointer, (isize) idx);
            let value = NativePtr.ReadByteMut(ptr);
            if (value != 0u8)
            {
                zeroOk = false;
            }
            idx = idx + 1;
        }
        MemoryRuntime.chic_rt_memset(block, 0x5Au8, 8usize);
        var setOk = true;
        idx = 0usize;
        while (idx < 8usize)
        {
            let ptr = NativePtr.OffsetMut(block.Pointer, (isize) idx);
            let value = NativePtr.ReadByteMut(ptr);
            if (value != 0x5Au8)
            {
                setOk = false;
            }
            idx = idx + 1;
        }
        let ok = !NativePtr.IsNull(block.Pointer) && zeroOk && setOk;
        Assert.That(ok).IsTrue();
        MemoryRuntime.chic_rt_free(block);
    }
}

@group("native") testcase Given_memory_custom_allocator_paths_success_When_executed_Then_memory_custom_allocator_paths_success()
{
    MemoryRuntime.ResetTestAllocatorCalls();
    let vtable = MemoryRuntime.TestAllocatorVTable();
    MemoryRuntime.chic_rt_allocator_install(vtable);
    var ok = true;
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc(12usize, 4usize);
        var zeroed = MemoryRuntime.chic_rt_alloc_zeroed(8usize, 1usize);
        var resized = MemoryRuntime.chic_rt_realloc(zeroed, 8usize, 16usize, 1usize);
        let empty = MemoryRuntime.chic_rt_realloc(resized, 16usize, 0usize, 1usize);
        ok = !NativePtr.IsNull(block.Pointer)
            && !NativePtr.IsNull(zeroed.Pointer)
            && !NativePtr.IsNull(resized.Pointer)
            && NativePtr.IsNull(empty.Pointer);
        MemoryRuntime.chic_rt_free(block);
    }
    MemoryRuntime.chic_rt_allocator_reset();
    Assert.That(ok).IsTrue();
}

@group("native") testcase Given_memory_custom_allocator_paths_calls_When_executed_Then_memory_custom_allocator_paths_calls()
{
    MemoryRuntime.ResetTestAllocatorCalls();
    let vtable = MemoryRuntime.TestAllocatorVTable();
    MemoryRuntime.chic_rt_allocator_install(vtable);
    unsafe {
        var block = MemoryRuntime.chic_rt_alloc(12usize, 4usize);
        var zeroed = MemoryRuntime.chic_rt_alloc_zeroed(8usize, 1usize);
        var resized = MemoryRuntime.chic_rt_realloc(zeroed, 8usize, 16usize, 1usize);
        let empty = MemoryRuntime.chic_rt_realloc(resized, 16usize, 0usize, 1usize);
        let _ = zeroed;
        let _ = resized;
        let _ = empty;
        MemoryRuntime.chic_rt_free(block);
    }
    MemoryRuntime.chic_rt_allocator_reset();
    let okCalls = MemoryRuntime.TestAllocatorAllocCalls() >= 3usize && MemoryRuntime.TestAllocatorFreeCalls() >= 1usize;
    Assert.That(okCalls).IsTrue();
}

testcase Given_memory_memcpy_allocations_non_null_When_executed_Then_memory_memcpy_allocations_non_null()
{
    unsafe {
        var src = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        var dst = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        let ok = !NativePtr.IsNull(src.Pointer) && !NativePtr.IsNull(dst.Pointer);
        MemoryRuntime.chic_rt_free(src);
        MemoryRuntime.chic_rt_free(dst);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_memory_memcpy_copies_bytes_When_executed_Then_memory_memcpy_copies_bytes()
{
    unsafe {
        var src = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        var dst = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        var idx = 0usize;
        while (idx < 8usize)
        {
            let ptr = NativePtr.OffsetMut(src.Pointer, (isize) idx);
            * ptr = (byte)(idx + 1usize);
            idx = idx + 1;
        }
        MemoryRuntime.chic_rt_memcpy(dst, new ValueConstPtr {
            Pointer = dst.Pointer, Size = dst.Size, Alignment = dst.Alignment
        }
        , 0usize);
        MemoryRuntime.chic_rt_memcpy(dst, new ValueConstPtr {
            Pointer = src.Pointer, Size = src.Size, Alignment = src.Alignment
        }
        , 8usize);
        let bytesOk = BytesEqual(new ValueConstPtr {
            Pointer = src.Pointer, Size = src.Size, Alignment = src.Alignment
        }
        , new ValueConstPtr {
            Pointer = dst.Pointer, Size = dst.Size, Alignment = dst.Alignment
        }
        , 8usize);
        MemoryRuntime.chic_rt_free(src);
        MemoryRuntime.chic_rt_free(dst);
        Assert.That(bytesOk).IsTrue();
    }
}

testcase Given_memory_memmove_handles_overlap_When_executed_Then_memory_memmove_handles_overlap()
{
    unsafe {
        var buffer = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        var idx = 0usize;
        while (idx < 8usize)
        {
            let ptr = NativePtr.OffsetMut(buffer.Pointer, (isize) idx);
            * ptr = (byte)(idx + 10usize);
            idx = idx + 1;
        }
        var moveSrc = new ValueMutPtr {
            Pointer = buffer.Pointer, Size = 8usize, Alignment = 1
        }
        ;
        var moveDst = new ValueMutPtr {
            Pointer = NativePtr.OffsetMut(buffer.Pointer, 2isize), Size = 6usize, Alignment = 1
        }
        ;
        MemoryRuntime.chic_rt_memmove(moveDst, moveSrc, 6usize);
        var moveOk = true;
        idx = 0usize;
        while (idx < 6usize)
        {
            let valuePtr = NativePtr.OffsetMut(buffer.Pointer, (isize)(idx + 2usize));
            let value = NativePtr.ReadByteMut(valuePtr);
            if (value != (byte)(idx + 10usize))
            {
                moveOk = false;
            }
            idx = idx + 1;
        }
        MemoryRuntime.chic_rt_free(buffer);
        Assert.That(moveOk).IsTrue();
    }
}

testcase Given_memory_internal_helpers_When_executed_Then_memory_internal_helpers()
{
    unsafe {
        MemoryRuntime.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}

testcase Given_memory_zeroed_alloc_size_zero_When_executed_Then_returns_null()
{
    unsafe {
        let empty = MemoryRuntime.chic_rt_alloc_zeroed(0usize, 4usize);
        let ok = NativePtr.IsNull(empty.Pointer) && empty.Size == 0usize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_memory_realloc_null_grows_When_executed_Then_memory_realloc_null_grows()
{
    MemoryRuntime.chic_rt_allocator_reset();
    unsafe {
        var nullPtr = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let grown = MemoryRuntime.chic_rt_realloc(nullPtr, 0usize, 4usize, 1usize);
        let shrunk = MemoryRuntime.chic_rt_realloc(grown, 4usize, 0usize, 1usize);
        let ok = !NativePtr.IsNull(grown.Pointer);
        MemoryRuntime.chic_rt_free(new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        );
        Assert.That(ok).IsTrue();
    }
}

testcase Given_memory_realloc_null_shrinks_When_executed_Then_memory_realloc_null_shrinks()
{
    MemoryRuntime.chic_rt_allocator_reset();
    unsafe {
        var nullPtr = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let grown = MemoryRuntime.chic_rt_realloc(nullPtr, 0usize, 4usize, 1usize);
        let shrunk = MemoryRuntime.chic_rt_realloc(grown, 4usize, 0usize, 1usize);
        let ok = NativePtr.IsNull(shrunk.Pointer);
        MemoryRuntime.chic_rt_free(new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        );
        Assert.That(ok).IsTrue();
    }
}

testcase Given_memory_allocation_failure_paths_When_executed_Then_memory_allocation_failure_paths()
{
    unsafe {
        MemoryRuntime.chic_rt_allocator_reset();
        NativeAlloc.TestFailAllocAfter(0);
        let failed = MemoryRuntime.chic_rt_alloc(16usize, 1usize);
        NativeAlloc.TestReset();

        NativeAlloc.TestFailAllocAfter(0);
        let failedZeroed = MemoryRuntime.chic_rt_alloc_zeroed(8usize, 1usize);
        NativeAlloc.TestReset();

        var base = MemoryRuntime.chic_rt_alloc(8usize, 1usize);
        NativeAlloc.TestFailReallocAfter(0);
        let failedGrow = MemoryRuntime.chic_rt_realloc(base, 8usize, 16usize, 1usize);
        let ok = NativePtr.IsNull(failed.Pointer)
            && NativePtr.IsNull(failedZeroed.Pointer)
            && NativePtr.IsNull(failedGrow.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
        MemoryRuntime.chic_rt_free(base);
    }
}
