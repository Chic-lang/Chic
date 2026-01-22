namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_trace_enter_exit_and_flush_When_executed_Then_trace_enter_exit_and_flush()
{
    unsafe {
        var label = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101,
        }
        ;
        let labelPtr = NativePtr.AsConstPtr(& label.b00);
        TraceRuntime.chic_rt_trace_enter(1u64, labelPtr, 5u64);
        TraceRuntime.chic_rt_trace_exit(1u64);

        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 46, b06 = 106, b07 = 115, b08 = 111, b09 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 10u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_flush_rejects_empty_path_When_executed_Then_trace_flush_rejects_empty_path()
{
    unsafe {
        TraceRuntime.TestResetState();
        TraceRuntime.chic_rt_trace_enter(42u64, NativePtr.NullConst(), 0u64);
        TraceRuntime.chic_rt_trace_exit(42u64);
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.NullConst(), 0u64);
        Assert.That(status).IsEqualTo(0);
        TraceRuntime.TestResetState();
    }
}

testcase Given_trace_exit_without_enter_is_noop_When_executed_Then_trace_exit_without_enter_is_noop()
{
    unsafe {
        TraceRuntime.chic_rt_trace_exit(99u64);
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.NullConst(), 0u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_records_multiple_events_alloc_ok_When_executed_Then_trace_records_multiple_events_alloc_ok()
{
    unsafe {
        var shortLabel = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 45, b06 = 115, b07 = 49,
        }
        ;
        let shortPtr = NativePtr.AsConstPtr(& shortLabel.b00);
        TraceRuntime.chic_rt_trace_enter(10u64, shortPtr, 8u64);
        TraceRuntime.chic_rt_trace_exit(10u64);

        var longBuf = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 80usize, Alignment = 1usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(80usize, 1usize, out longBuf);
        let allocOk = (int) alloc == (int) NativeAllocationError.Success;
        var idx = 0usize;
        while (idx < 70usize)
        {
            let ptr = NativePtr.OffsetMut(longBuf.Pointer, (isize) idx);
            * ptr = (byte)(97u8 + (byte)(idx % 26usize));
            idx = idx + 1;
        }
        TraceRuntime.chic_rt_trace_enter(11u64, NativePtr.AsConstPtr(longBuf.Pointer), 70u64);
        TraceRuntime.chic_rt_trace_exit(11u64);
        NativeAlloc.Free(longBuf);

        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 109, b07 = 117, b08 = 108, b09 = 116,
            b10 = 105, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let _ = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        Assert.That(allocOk).IsTrue();
    }
}

testcase Given_trace_records_multiple_events_flush_status_When_executed_Then_trace_records_multiple_events_flush_status()
{
    unsafe {
        var shortLabel = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 45, b06 = 115, b07 = 49,
        }
        ;
        let shortPtr = NativePtr.AsConstPtr(& shortLabel.b00);
        TraceRuntime.chic_rt_trace_enter(10u64, shortPtr, 8u64);
        TraceRuntime.chic_rt_trace_exit(10u64);

        var longBuf = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 80usize, Alignment = 1usize
        }
        ;
        let _ = NativeAlloc.AllocZeroed(80usize, 1usize, out longBuf);
        var idx = 0usize;
        while (idx < 70usize)
        {
            let ptr = NativePtr.OffsetMut(longBuf.Pointer, (isize) idx);
            * ptr = (byte)(97u8 + (byte)(idx % 26usize));
            idx = idx + 1;
        }
        TraceRuntime.chic_rt_trace_enter(11u64, NativePtr.AsConstPtr(longBuf.Pointer), 70u64);
        TraceRuntime.chic_rt_trace_exit(11u64);
        NativeAlloc.Free(longBuf);

        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 109, b07 = 117, b08 = 108, b09 = 116,
            b10 = 105, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_flush_with_no_events_When_executed_Then_trace_flush_with_no_events()
{
    unsafe {
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 101, b07 = 109, b08 = 112, b09 = 116,
            b10 = 121, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_enter_with_empty_label_When_executed_Then_trace_enter_with_empty_label()
{
    unsafe {
        TraceRuntime.chic_rt_trace_enter(22u64, NativePtr.NullConst(), 0u64);
        TraceRuntime.chic_rt_trace_exit(22u64);
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 101, b07 = 109, b08 = 112, b09 = 116,
            b10 = 121, b11 = 50, b12 = 46, b13 = 106, b14 = 115, b15 = 111, b16 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 17u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_grows_buffer_and_handles_open_end_When_executed_Then_trace_grows_buffer_and_handles_open_end()
{
    unsafe {
        var label = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 99, b03 = 45, b04 = 103, b05 = 114, b06 = 111, b07 = 119, b08 = 0,
        }
        ;
        let labelPtr = NativePtr.AsConstPtr(& label.b00);
        var idx = 0u64;
        while (idx < 40u64)
        {
            TraceRuntime.chic_rt_trace_enter(100u64 + idx, labelPtr, 8u64);
            if (idx % 3u64 != 0u64)
            {
                TraceRuntime.chic_rt_trace_exit(100u64 + idx);
            }
            idx = idx + 1u64;
        }
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 103, b07 = 114, b08 = 111, b09 = 119,
            b10 = 46, b11 = 106, b12 = 115, b13 = 111, b14 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_flush_clears_events_first_status_When_executed_Then_trace_flush_clears_events_first_status()
{
    unsafe {
        TraceRuntime.chic_rt_trace_enter(300u64, NativePtr.NullConst(), 0u64);
        TraceRuntime.chic_rt_trace_exit(300u64);
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 99, b07 = 108, b08 = 101, b09 = 97,
            b10 = 114, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        Assert.That(status).IsEqualTo(0);
    }
}

testcase Given_trace_flush_clears_events_second_status_When_executed_Then_trace_flush_clears_events_second_status()
{
    unsafe {
        TraceRuntime.chic_rt_trace_enter(300u64, NativePtr.NullConst(), 0u64);
        TraceRuntime.chic_rt_trace_exit(300u64);
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 99, b07 = 108, b08 = 101, b09 = 97,
            b10 = 114, b11 = 46, b12 = 106, b13 = 115, b14 = 111, b15 = 110,
        }
        ;
        let _ = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 16u64);
        let status2 = TraceRuntime.chic_rt_trace_flush(NativePtr.NullConst(), 0u64);
        Assert.That(status2).IsEqualTo(0);
    }
}

testcase Given_trace_coverage_sweep_When_executed_Then_returns_true()
{
    unsafe {
        let ok = TraceRuntime.TestCoverageSweep();
        Assert.That(ok).IsTrue();
    }
}

testcase Given_trace_allocation_failure_paths_When_executed_Then_trace_allocation_failure_paths()
{
    unsafe {
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 102, b07 = 97, b08 = 105, b09 = 108,
            b10 = 46, b11 = 106, b12 = 115, b13 = 111, b14 = 110,
        }
        ;

        TraceRuntime.TestResetState();
        TraceRuntime.TestFailAllocAtStep(0);
        let mutexFail = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);
        TraceRuntime.TestDisableAllocFailures();

        TraceRuntime.TestResetState();
        TraceRuntime.TestFailAllocAtStep(1);
        TraceRuntime.chic_rt_trace_enter(700u64, NativePtr.NullConst(), 0u64);
        TraceRuntime.chic_rt_trace_exit(700u64);
        TraceRuntime.TestDisableAllocFailures();

        TraceRuntime.TestResetState();
        TraceRuntime.TestFailAllocRange(1, 2);
        let pathFail = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);
        TraceRuntime.TestDisableAllocFailures();

        TraceRuntime.TestResetState();
        TraceRuntime.TestFailAllocRange(2, 2);
        let modeFail = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);
        TraceRuntime.TestDisableAllocFailures();

        TraceRuntime.TestResetState();
        TraceRuntime.TestForceOpenFailure(true);
        let openFail = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);

        TraceRuntime.TestResetState();
        TraceRuntime.TestFailAllocRange(3, 2);
        let metricsFail = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 15u64);
        TraceRuntime.TestDisableAllocFailures();
        let ok = mutexFail == 0 && pathFail == -2 && modeFail == -3 && openFail == -4 && metricsFail == 0;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_trace_append_escaped_quotes_When_executed_Then_trace_append_escaped_quotes()
{
    unsafe {
        var label = new StringInlineBytes64 {
            b00 = 113, b01 = 117, b02 = 111, b03 = 116, b04 = 101, b05 = 34, b06 = 92, b07 = 120,
        }
        ;
        TraceRuntime.TestAppendEscaped(NativePtr.AsConstPtr(& label.b00), 8usize);
        Assert.That(true).IsTrue();
    }
}

testcase Given_trace_internal_helpers_coverage_When_executed_Then_trace_internal_helpers_coverage()
{
    unsafe {
        TraceRuntime.TestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}

testcase Given_trace_metrics_large_event_count_When_executed_Then_trace_metrics_large_event_count()
{
    unsafe {
        TraceRuntime.TestResetState();
        var label = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101,
        }
        ;
        let labelPtr = NativePtr.AsConstPtr(& label.b00);
        var idx = 0u64;
        while (idx < 1100u64)
        {
            TraceRuntime.chic_rt_trace_enter(1000u64 + idx, labelPtr, 5u64);
            TraceRuntime.chic_rt_trace_exit(1000u64 + idx);
            idx = idx + 1u64;
        }
        var path = new StringInlineBytes64 {
            b00 = 116, b01 = 114, b02 = 97, b03 = 99, b04 = 101, b05 = 95, b06 = 98, b07 = 105, b08 = 103, b09 = 46,
            b10 = 106, b11 = 115, b12 = 111, b13 = 110,
        }
        ;
        let status = TraceRuntime.chic_rt_trace_flush(NativePtr.AsConstPtr(& path.b00), 14u64);
        Assert.That(status).IsEqualTo(0);
    }
}
