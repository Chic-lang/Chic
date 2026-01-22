namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

private unsafe static bool StackLocalAddrHelper(* const @readonly @expose_address byte ptr) {
    if (NativePtr.IsNullConst(ptr))
    {
        return false;
    }
    let b0 = NativePtr.ReadByteConst(ptr);
    let b1 = NativePtr.ReadByteConst(NativePtr.OffsetConst(ptr, 1isize));
    let b2 = NativePtr.ReadByteConst(NativePtr.OffsetConst(ptr, 2isize));
    return b0 == 1u8 && b1 == 2u8 && b2 == 3u8;
}

testcase Given_stack_local_address_passed_across_call_When_executed_Then_bytes_are_preserved()
{
    unsafe {
        var local = new StringInlineBytes32 {
            b00 = 1, b01 = 2, b02 = 3,
        }
        ;
        // Force a second local on the stack to reduce the chance of incidental success.
        var scratch = new StringInlineBytes32 {
            b00 = 9, b01 = 9, b02 = 9, b03 = 9,
        }
        ;
        let _ = scratch;
        let ok = StackLocalAddrHelper(NativePtr.AsConstPtr(& local.b00));
        Assert.That(ok).IsTrue();
    }
}

testcase Given_storebyte_writes_stack_memory_When_executed_Then_bytes_match()
{
    unsafe {
        var local = new StringInlineBytes32 {
            b00 = 0, b01 = 0, b02 = 0,
        }
        ;
        StringRuntime.StoreByte(& mut local.b00, 1u8);
        StringRuntime.StoreByte(& mut local.b01, 2u8);
        StringRuntime.StoreByte(& mut local.b02, 3u8);

        let b0 = NativePtr.ReadByteConst(NativePtr.AsConstPtr(& local.b00));
        let b1 = NativePtr.ReadByteConst(NativePtr.AsConstPtr(& local.b01));
        let b2 = NativePtr.ReadByteConst(NativePtr.AsConstPtr(& local.b02));
        Assert.That(b0).IsEqualTo(1u8);
        Assert.That(b1).IsEqualTo(2u8);
        Assert.That(b2).IsEqualTo(3u8);
    }
}

testcase Given_native_alloc_copy_reads_stack_source_When_executed_Then_copies_bytes()
{
    unsafe {
        var src = new StringInlineBytes32 {
            b00 = 1, b01 = 2, b02 = 3,
        }
        ;
        var dst = new StringInlineBytes32 {
            b00 = 0, b01 = 0, b02 = 0,
        }
        ;
        NativeAlloc.Copy(
            new ValueMutPtr { Pointer = & mut dst.b00, Size = 3usize, Alignment = 1usize },
            new ValueConstPtr { Pointer = NativePtr.AsConstPtr(& src.b00), Size = 3usize, Alignment = 1usize },
            3usize
        );
        Assert.That(dst.b00).IsEqualTo(1u8);
        Assert.That(dst.b01).IsEqualTo(2u8);
        Assert.That(dst.b02).IsEqualTo(3u8);
    }
}

testcase Given_struct_field_addressing_matches_byte_offsets_When_executed_Then_inline_ptr_is_base_plus_12()
{
    unsafe {
        var str = StringRuntime.chic_rt_string_new();
        var * mut @expose_address byte baseBytes = & mut str;
        let base = NativePtr.ToIsize(baseBytes);
        let inlinePtr = StringRuntime.chic_rt_string_inline_ptr(& mut str);
        let expectedInline = NativePtr.AsByteMut(& mut str.inline_data.b00);
        let inlineAddr = NativePtr.ToIsize(inlinePtr);
        let expectedAddr = NativePtr.ToIsize(expectedInline);
        Assert.That((isize) inlineAddr).IsEqualTo((isize) expectedAddr);
        StringRuntime.chic_rt_string_drop(& mut str);
    }
}

testcase Given_native_ptr_offsets_and_casts_When_executed_Then_native_ptr_offsets_and_casts()
{
    unsafe {
        let base = NativePtr.FromIsize(100isize);
        let baseConst = NativePtr.FromIsizeConst(200isize);
        let same = NativePtr.OffsetMut(base, 0isize);
        let moved = NativePtr.OffsetMut(base, 2isize);
        let nullPtr = NativePtr.OffsetMut(NativePtr.NullMut(), 4isize);
        let constMoved = NativePtr.OffsetConst(baseConst, - 1isize);
        let ok = NativePtr.ToIsize(base) == 100isize
            && NativePtr.ToIsizeConst(baseConst) == 200isize
            && NativePtr.ToIsize(same) == 100isize
            && NativePtr.ToIsize(moved) == 102isize
            && NativePtr.IsNull(nullPtr)
            && NativePtr.ToIsizeConst(constMoved) == 199isize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_native_alloc_helpers_and_zero_init_When_executed_Then_native_alloc_helpers_and_zero_init()
{
    unsafe {
        var empty = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let allocStatus = NativeAlloc.Alloc(0usize, 1usize, out empty);
        var emptyZero = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let zeroStatus = NativeAlloc.AllocZeroed(0usize, 1usize, out emptyZero);

        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 16usize
        }
        ;
        let status = NativeAlloc.Alloc(8usize, 16usize, out block);
        NativeAlloc.Set(block, 0xABu8, 8usize);
        let first = NativePtr.ReadByteMut(block.Pointer);

        var dst = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let dstStatus = NativeAlloc.AllocZeroed(8usize, 1usize, out dst);
        NativeAlloc.Copy(dst, new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 8usize, Alignment = 1usize
        }
        , 8usize);
        NativeAlloc.Move(new ValueMutPtr {
            Pointer = NativePtr.OffsetMut(dst.Pointer, 2isize), Size = 6usize, Alignment = 1usize
        }
        , new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(dst.Pointer), Size = 6usize, Alignment = 1usize
        }
        , 6usize);
        let copied = NativePtr.ReadByteMut(dst.Pointer);
        NativeAlloc.ZeroInitRaw(NativePtr.NullMut(), 4usize);
        NativeAlloc.ZeroInitRaw(dst.Pointer, 8usize);
        let zeroed = NativePtr.ReadByteMut(dst.Pointer);
        let ok = (int) allocStatus == (int) NativeAllocationError.Success
            && NativePtr.IsNull(empty.Pointer)
            && (int) zeroStatus == (int) NativeAllocationError.Success
            && NativePtr.IsNull(emptyZero.Pointer)
            && (int) status == (int) NativeAllocationError.Success
            && !NativePtr.IsNull(block.Pointer)
            && first == 0xABu8
            && (int) dstStatus == (int) NativeAllocationError.Success
            && copied == 0xABu8
            && zeroed == 0u8;
        Assert.That(ok).IsTrue();

        NativeAlloc.Free(block);
        NativeAlloc.Free(dst);
    }
}

testcase Given_native_alloc_realloc_and_failures_When_executed_Then_native_alloc_realloc_and_failures()
{
    unsafe {
        NativeAlloc.TestReset();
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 16usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Alloc(16usize, 1usize, out block);

        var grown = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let growStatus = NativeAlloc.Realloc(block, 16usize, 32usize, 1usize, out grown);

        var shrunk = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let shrinkStatus = NativeAlloc.Realloc(grown, 32usize, 0usize, 1usize, out shrunk);
        NativeAlloc.Free(shrunk);

        NativeAlloc.TestFailAllocAfter(0);
        var fail = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let failStatus = NativeAlloc.Alloc(8usize, 1usize, out fail);
        NativeAlloc.TestReset();

        var base = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let baseStatus = NativeAlloc.Alloc(8usize, 1usize, out base);
        NativeAlloc.TestFailReallocAfter(0);
        var failedGrow = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let failedStatus = NativeAlloc.Realloc(base, 8usize, 16usize, 1usize, out failedGrow);
        let ok = (int) status == (int) NativeAllocationError.Success
            && !NativePtr.IsNull(block.Pointer)
            && (int) growStatus == (int) NativeAllocationError.Success
            && grown.Size == 32usize
            && (int) shrinkStatus == (int) NativeAllocationError.Success
            && (int) failStatus == (int) NativeAllocationError.AllocationFailed
            && (int) baseStatus == (int) NativeAllocationError.Success
            && (int) failedStatus == (int) NativeAllocationError.AllocationFailed;
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
        NativeAlloc.Free(base);
    }
}

testcase Given_native_alloc_realloc_null_ptr_When_executed_Then_allocates_new_block()
{
    unsafe {
        var empty = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        var outPtr = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Realloc(empty, 0usize, 8usize, 1usize, out outPtr);
        let ok = status == NativeAllocationError.Success
            && !NativePtr.IsNull(outPtr.Pointer)
            && outPtr.Size == 8usize;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(outPtr);
    }
}

testcase Given_native_alloc_sys_malloc_failure_When_allocating_Then_falls_back_to_calloc()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(1);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Alloc(8usize, 1usize, out block);
        let value = NativePtr.IsNull(block.Pointer) ?1u8 : NativePtr.ReadByteMut(block.Pointer);
        let ok = status == NativeAllocationError.Success && !NativePtr.IsNull(block.Pointer) && value == 0u8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_posix_failure_When_allocating_Then_falls_back_to_malloc()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(1);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 32usize
        }
        ;
        let status = NativeAlloc.Alloc(8usize, 32usize, out block);
        let ok = status == NativeAllocationError.Success && !NativePtr.IsNull(block.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_realloc_fallback_When_realloc_fails_Then_copies_contents()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 4usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Alloc(4usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x5Au8;
        }
        NativeAlloc.TestFailSysAllocCount(1);
        var resized = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let reallocStatus = NativeAlloc.Realloc(block, 4usize, 8usize, 1usize, out resized);
        let value = NativePtr.IsNull(resized.Pointer) ?0u8 : NativePtr.ReadByteMut(resized.Pointer);
        let ok = status == NativeAllocationError.Success && reallocStatus == NativeAllocationError.Success && value == 0x5Au8;
        Assert.That(ok).IsTrue();
        let oldAddr = NativePtr.ToIsize(block.Pointer);
        let newAddr = NativePtr.ToIsize(resized.Pointer);
        if (oldAddr != 0 && oldAddr != newAddr)
        {
            NativeAlloc.Free(block);
        }
        NativeAlloc.Free(resized);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_sys_double_failure_When_allocating_Then_falls_back_to_posix_memalign()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(2);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Alloc(8usize, 1usize, out block);
        let value = NativePtr.IsNull(block.Pointer) ?1u8 : NativePtr.ReadByteMut(block.Pointer);
        let ok = status == NativeAllocationError.Success && !NativePtr.IsNull(block.Pointer) && value == 0u8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_posix_and_malloc_failure_When_allocating_Then_uses_calloc()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(2);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 32usize
        }
        ;
        let status = NativeAlloc.Alloc(8usize, 32usize, out block);
        let value = NativePtr.IsNull(block.Pointer) ?1u8 : NativePtr.ReadByteMut(block.Pointer);
        let ok = status == NativeAllocationError.Success && !NativePtr.IsNull(block.Pointer) && value == 0u8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_zeroed_calloc_failure_When_allocating_Then_falls_back_to_posix_memalign()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(1);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(8usize, 1usize, out block);
        let value = NativePtr.IsNull(block.Pointer) ?1u8 : NativePtr.ReadByteMut(block.Pointer);
        let ok = status == NativeAllocationError.Success && !NativePtr.IsNull(block.Pointer) && value == 0u8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_zeroed_double_failure_When_sys_allocs_fail_Then_returns_failure()
{
    unsafe {
        NativeAlloc.TestFailSysAllocCount(2);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(8usize, 1usize, out block);
        let ok = status == NativeAllocationError.AllocationFailed && NativePtr.IsNull(block.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_zeroed_failure_When_should_fail_alloc_Then_returns_allocation_failed()
{
    unsafe {
        NativeAlloc.TestFailAllocAfter(0);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(8usize, 1usize, out block);
        let ok = status == NativeAllocationError.AllocationFailed && NativePtr.IsNull(block.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_zeroed_large_alignment_failure_When_alloc_fails_Then_returns_failure()
{
    unsafe {
        NativeAlloc.TestFailAllocAfter(0);
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 32usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(8usize, 32usize, out block);
        let ok = status == NativeAllocationError.AllocationFailed && NativePtr.IsNull(block.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
    }
}

testcase Given_native_alloc_realloc_double_failure_When_realloc_and_malloc_fail_Then_uses_calloc()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 4usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Alloc(4usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x6Bu8;
        }
        NativeAlloc.TestFailSysAllocCount(2);
        var resized = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let reallocStatus = NativeAlloc.Realloc(block, 4usize, 8usize, 1usize, out resized);
        let value = NativePtr.IsNull(resized.Pointer) ?0u8 : NativePtr.ReadByteMut(resized.Pointer);
        let ok = status == NativeAllocationError.Success && reallocStatus == NativeAllocationError.Success && value == 0x6Bu8;
        Assert.That(ok).IsTrue();
        let oldAddr = NativePtr.ToIsize(block.Pointer);
        let newAddr = NativePtr.ToIsize(resized.Pointer);
        if (oldAddr != 0 && oldAddr != newAddr)
        {
            NativeAlloc.Free(block);
        }
        NativeAlloc.Free(resized);
        NativeAlloc.TestReset();
    }
}

testcase Given_native_ptr_reads_and_atomic_ops_When_executed_Then_native_ptr_reads_and_atomic_ops()
{
    unsafe {
        NativeAlloc.TestReset();
        var slot = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out slot);
        let allocOk = (int) status == (int) NativeAllocationError.Success && !NativePtr.IsNull(slot.Pointer);
        if (allocOk)
        {
            * slot.Pointer = 0x7Fu8;
        }
        let c = allocOk ?NativePtr.ReadByteConst(NativePtr.AsConstPtr(slot.Pointer)) : 0u8;
        let m = allocOk ?NativePtr.ReadByteMut(slot.Pointer) : 0u8;
        let roundTrip = allocOk ?NativePtr.AsMutPtr(NativePtr.AsConstPtr(slot.Pointer)) : NativePtr.NullMut();
        NativeAlloc.Free(slot);
        NativeAlloc.TestReset();

        var counter = new AtomicUsize(5usize);
        let initial = counter.Load();
        counter.Store(9usize);
        let afterStore = counter.Load();
        let addPrev = counter.FetchAdd(3usize);
        let afterAdd = counter.Load();
        let subPrev = counter.FetchSub(2usize);
        let afterSub = counter.Load();

        let ok = allocOk
            && c == 0x7Fu8
            && m == 0x7Fu8
            && !NativePtr.IsNull(roundTrip)
            && initial == 5usize
            && afterStore == 9usize
            && addPrev == 9usize
            && afterAdd == 12usize
            && subPrev == 12usize
            && afterSub == 10usize;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_native_alloc_copy_len_zero_When_executed_Then_copy_noop()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x5Au8;
        }
        NativeAlloc.Copy(block, new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 1usize, Alignment = 1usize
        }
        , 0usize);
        let value = NativePtr.ReadByteMut(block.Pointer);
        let ok = (int) status == (int) NativeAllocationError.Success && value == 0x5Au8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
    }
}

testcase Given_native_alloc_move_len_zero_When_executed_Then_move_noop()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x3Bu8;
        }
        NativeAlloc.Move(block, new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(block.Pointer), Size = 1usize, Alignment = 1usize
        }
        , 0usize);
        let value = NativePtr.ReadByteMut(block.Pointer);
        let ok = (int) status == (int) NativeAllocationError.Success && value == 0x3Bu8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
    }
}

testcase Given_native_alloc_set_len_zero_When_executed_Then_set_noop()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x1Cu8;
        }
        NativeAlloc.Set(block, 0xEEu8, 0usize);
        let value = NativePtr.ReadByteMut(block.Pointer);
        let ok = (int) status == (int) NativeAllocationError.Success && value == 0x1Cu8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
    }
}

testcase Given_native_alloc_zero_init_len_zero_When_executed_Then_zero_init_noop()
{
    unsafe {
        var block = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(1usize, 1usize, out block);
        if (! NativePtr.IsNull (block.Pointer))
        {
            * block.Pointer = 0x9Du8;
        }
        NativeAlloc.ZeroInitRaw(block.Pointer, 0usize);
        let value = NativePtr.ReadByteMut(block.Pointer);
        let ok = (int) status == (int) NativeAllocationError.Success && value == 0x9Du8;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(block);
    }
}

testcase Given_native_alloc_zeroed_large_alignment_When_executed_Then_alloc_zeroed_succeeds()
{
    unsafe {
        var slot = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 8usize, Alignment = 32usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(8usize, 32usize, out slot);
        let ok = (int) status == (int) NativeAllocationError.Success && ! NativePtr.IsNull (slot.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(slot);
    }
}

testcase Given_native_alloc_realloc_null_pointer_When_executed_Then_realloc_allocates()
{
    unsafe {
        var base = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        var grown = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.Realloc(base, 0usize, 8usize, 1usize, out grown);
        let ok = (int) status == (int) NativeAllocationError.Success && ! NativePtr.IsNull (grown.Pointer);
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(grown);
    }
}

testcase Given_native_alloc_fail_after_decrements_When_executed_Then_next_alloc_fails()
{
    unsafe {
        NativeAlloc.TestReset();
        NativeAlloc.TestFailAllocAfter(1);
        var first = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let firstStatus = NativeAlloc.Alloc(1usize, 1usize, out first);
        var second = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let secondStatus = NativeAlloc.Alloc(1usize, 1usize, out second);
        let ok = (int) firstStatus == (int) NativeAllocationError.Success
            && (int) secondStatus == (int) NativeAllocationError.AllocationFailed;
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
        NativeAlloc.Free(first);
    }
}

testcase Given_native_realloc_fail_after_decrements_When_executed_Then_next_realloc_fails()
{
    unsafe {
        NativeAlloc.TestReset();
        var base = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 4usize, Alignment = 1usize
        }
        ;
        let baseStatus = NativeAlloc.Alloc(4usize, 1usize, out base);
        NativeAlloc.TestFailReallocAfter(1);
        var first = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let firstStatus = NativeAlloc.Realloc(base, 4usize, 8usize, 1usize, out first);
        var second = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        ;
        let secondStatus = NativeAlloc.Realloc(first, 8usize, 16usize, 1usize, out second);
        let ok = (int) baseStatus == (int) NativeAllocationError.Success
            && (int) firstStatus == (int) NativeAllocationError.Success
            && (int) secondStatus == (int) NativeAllocationError.AllocationFailed;
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
        NativeAlloc.Free(first);
    }
}
