namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_crypto_random_repeated_calls_When_executed_Then_crypto_random_repeated_calls()
{
    unsafe {
        CryptoRandom.TestUseFakeIo(true);
        CryptoRandom.TestSetFakeByte(17u8);
        CryptoRandom.TestSetReadLimit(0usize);
        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 64usize, Alignment = 1usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(64usize, 1usize, out buffer);
        var ok = alloc == NativeAllocationError.Success;
        let filled = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 64usize);
        ok = ok && filled;
        let filledSmall = CryptoRandom.chic_rt_random_fill(buffer.Pointer, 1usize);
        ok = ok && filledSmall;
        NativeAlloc.Free(buffer);
        CryptoRandom.TestUseFakeIo(false);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_trace_capacity_and_escape_paths_When_executed_Then_trace_capacity_and_escape_paths()
{
    unsafe {
        var label = new StringInlineBytes64 {
            b00 = 34, b01 = 92, b02 = 65,
        }
        ;
        let labelPtr = NativePtr.AsConstPtr(& label.b00);
        var idx = 0u64;
        while (idx <40u64)
        {
            TraceRuntime.chic_rt_trace_enter(100u64 + idx, labelPtr, 3u64);
            TraceRuntime.chic_rt_trace_exit(100u64 + idx);
            idx += 1u64;
        }
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 115, b07 = 119, b08 = 101, b09 = 101,
            b10 = 112, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_region_full_cycle_and_errors_When_executed_Then_region_full_cycle_and_errors()
{
    unsafe {
        let handle = chic_rt_region_enter(7ul);
        let block = chic_rt_region_alloc(handle, 8usize, 2usize);
        let zeroed = chic_rt_region_alloc_zeroed(handle, 16usize, 4usize);
        var ok = !NativePtr.IsNull(block.Pointer) && !NativePtr.IsNull(zeroed.Pointer);
        let telemetry = chic_rt_region_telemetry(handle);
        ok = ok && (telemetry.alloc_calls >0ul || telemetry.alloc_zeroed_calls >0ul);
        chic_rt_region_reset_stats(handle);
        let afterReset = chic_rt_region_telemetry(handle);
        ok = ok && afterReset.alloc_calls == 0ul;
        ok = ok && afterReset.alloc_zeroed_calls == 0ul;
        chic_rt_region_exit(handle);
        chic_rt_region_exit(handle);
        let failed = chic_rt_region_alloc(handle, 4usize, 1usize);
        ok = ok && NativePtr.IsNull(failed.Pointer);
        let missing = chic_rt_region_telemetry(new RegionHandle {
            Pointer = 0ul,
            Profile = 0ul,
            Generation = 0ul
        }
        );
        ok = ok && missing.alloc_calls == 0ul;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_span_layout_and_pointer_access_When_executed_Then_span_layout_and_pointer_access()
{
    unsafe {
        var layout = new SpanLayoutInfo {
            size = 0, offset_data = 0, offset_reserved = 0, offset_len = 0, offset_elem_size = 0, offset_elem_align = 0,
        }
        ;
        SpanRuntime.chic_rt_span_layout_debug(& layout);
        var ok = layout.size >0usize;

        var buffer = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 12usize, Alignment = 4usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(buffer.Size, buffer.Alignment, out buffer);
        ok = ok && alloc == NativeAllocationError.Success;
        let span = SpanRuntime.chic_rt_span_from_raw_mut(& buffer, 3usize);
        let ro = SpanRuntime.chic_rt_span_to_readonly(& span);
        let ptr0 = SpanRuntime.chic_rt_span_ptr_at_mut(& span, 0usize);
        ok = ok && !NativePtr.IsNull(ptr0);
        let ptrOob = SpanRuntime.chic_rt_span_ptr_at_mut(& span, 9usize);
        ok = ok && NativePtr.IsNull(ptrOob);
        let roPtr = SpanRuntime.chic_rt_span_ptr_at_readonly(& ro, 1usize);
        ok = ok && !NativePtr.IsNullConst(roPtr);

        var bad = new ValueMutPtr {
            Pointer = buffer.Pointer, Size = 1usize, Alignment = 3usize
        }
        ;
        let badSpan = SpanRuntime.chic_rt_span_from_raw_mut(& bad, 1usize);
        var sliced = new ChicSpan {
            data = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            , len = 0, elem_size = 0, elem_align = 1,
        }
        ;
        let badStatus = SpanRuntime.chic_rt_span_slice_mut(& badSpan, 0usize, 1usize, & sliced);
        ok = ok && badStatus == (int) SpanError.InvalidStride;
        NativeAlloc.Free(buffer);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_string_error_messages_and_clone_slice_When_executed_Then_string_error_messages_and_clone_slice()
{
    unsafe {
        let msg = StringRuntime.chic_rt_string_error_message(1);
        var ok = msg.len >0usize;
        let empty = StringRuntime.chic_rt_string_error_message(9999);
        ok = ok && empty.len == 0usize;
        var str = StringRuntime.chic_rt_string_new();
        let slice = new ChicStr {
            ptr = msg.ptr, len = msg.len
        }
        ;
        let cloneStatus = StringRuntime.chic_rt_string_clone_slice(& str, slice);
        ok = ok && cloneStatus == 0;
        StringRuntime.chic_rt_string_drop(& str);
        if (msg.len >0usize)
        {
            let msgAlloc = new ValueMutPtr {
                Pointer = NativePtr.AsMutPtr(msg.ptr), Size = msg.len, Alignment = 1usize
            }
            ;
            NativeAlloc.Free(msgAlloc);
        }
        Assert.That(ok).IsTrue();
    }
}

testcase Given_native_alloc_failure_paths_When_executed_Then_native_alloc_failure_paths()
{
    unsafe {
        NativeAlloc.TestFailAllocAfter(0);
        let failedRegion = chic_rt_region_enter(9ul);
        var ok = failedRegion.Pointer == 0ul;
        NativeAlloc.TestReset();

        let keySize = (usize) __sizeof<int>();
        let keyAlign = (usize) __alignof<int>();
        var map = HashMapRuntime.chic_rt_hashmap_new(keySize, keyAlign, keySize, keyAlign, HashMapTestSupport.DropNoop,
        HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        NativeAlloc.TestFailAllocAfter(0);
        let mapReserve = HashMapRuntime.chic_rt_hashmap_reserve(& map, 16usize);
        ok = ok && mapReserve == HashMapError.AllocationFailed;
        NativeAlloc.TestReset();
        HashMapRuntime.chic_rt_hashmap_drop(& map);

        var hashSet = HashSetRuntime.chic_rt_hashset_new(keySize, keyAlign, HashMapTestSupport.DropNoop, HashMapTestSupport.KeyEq);
        NativeAlloc.TestFailAllocAfter(0);
        let setReserve = HashSetRuntime.chic_rt_hashset_reserve(& hashSet, 16usize);
        ok = ok && setReserve == HashSetError.AllocationFailed;
        NativeAlloc.TestReset();
        HashSetRuntime.chic_rt_hashset_drop(& hashSet);

        var vec = VecRuntime.chic_rt_vec_new(1usize, 1usize, HashMapTestSupport.DropNoop);
        let inlineCap = VecRuntime.chic_rt_vec_inline_capacity(& vec);
        NativeAlloc.TestFailAllocAfter(0);
        let vecReserve = VecRuntime.chic_rt_vec_reserve(& vec, inlineCap + 1usize);
        ok = ok && vecReserve == (int) VecError.AllocationFailed;
        NativeAlloc.TestReset();

        var value = 1u8;
        var input = new ValueConstPtr {
            Pointer = & value, Size = 1usize, Alignment = 1usize
        }
        ;
        var idx = 0usize;
        while (idx < inlineCap + 2usize)
        {
            let _ = VecRuntime.chic_rt_vec_push(& vec, & input);
            idx += 1usize;
        }
        NativeAlloc.TestFailReallocAfter(0);
        let shrink = VecRuntime.chic_rt_vec_shrink_to_fit(& vec);
        ok = ok && shrink == 1;
        NativeAlloc.TestReset();
        VecRuntime.chic_rt_vec_drop(& vec);
        Assert.That(ok).IsTrue();
    }
}

testcase Given_trace_runtime_coverage_sweep_When_executed_Then_trace_runtime_coverage_sweep()
{
    unsafe {
        Assert.That(TraceRuntime.TestCoverageSweep()).IsTrue();
    }
}

testcase Given_region_runtime_coverage_sweep_When_executed_Then_region_runtime_coverage_sweep()
{
    unsafe {
        Assert.That(RegionTestCoverageSweep()).IsTrue();
    }
}

testcase Given_thread_runtime_coverage_sweep_When_executed_Then_thread_runtime_coverage_sweep()
{
    unsafe {
        Assert.That(ThreadTestCoverageSweep()).IsTrue();
    }
}
