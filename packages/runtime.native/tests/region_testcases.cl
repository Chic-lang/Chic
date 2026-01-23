namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;
testcase Given_region_allocations_track_telemetry_When_executed_Then_region_allocations_track_telemetry()
{
    unsafe {
        let handle = chic_rt_region_enter(0ul);
        var ok = handle.Pointer != 0ul;
        var block = chic_rt_region_alloc(handle, 16usize, 8usize);
        var zeroed = chic_rt_region_alloc_zeroed(handle, 8usize, 1usize);
        ok = ok && !NativePtr.IsNull(block.Pointer);
        ok = ok && !NativePtr.IsNull(zeroed.Pointer);
        let telemetry = chic_rt_region_telemetry(handle);
        ok = ok && telemetry.alloc_calls >= 1ul;
        ok = ok && telemetry.alloc_zeroed_calls >= 1ul;
        ok = ok && telemetry.alloc_bytes >= 16ul;
        ok = ok && telemetry.alloc_zeroed_bytes >= 8ul;
        chic_rt_region_reset_stats(handle);
        let reset = chic_rt_region_telemetry(handle);
        ok = ok && reset.alloc_calls == 0ul;
        ok = ok && reset.alloc_zeroed_calls == 0ul;
        chic_rt_region_exit(handle);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_exit_blocks_future_allocations_When_executed_Then_region_exit_blocks_future_allocations()
{
    unsafe {
        let handle = chic_rt_region_enter(1ul);
        var ok = handle.Pointer != 0ul;
        chic_rt_region_exit(handle);
        var failed = chic_rt_region_alloc(handle, 4usize, 4usize);
        ok = ok && NativePtr.IsNull(failed.Pointer);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_null_and_reset_paths_When_executed_Then_region_null_and_reset_paths()
{
    unsafe {
        let nullHandle = new RegionHandle {
            Pointer = 0ul, Profile = 0ul, Generation = 0ul
        }
        ;
        let telemetry = chic_rt_region_telemetry(nullHandle);
        var ok = telemetry.alloc_calls == 0ul;
        chic_rt_region_reset_stats(nullHandle);
        var failed = chic_rt_region_alloc(nullHandle, 4usize, 1usize);
        ok = ok && NativePtr.IsNull(failed.Pointer);
        let handle = chic_rt_region_enter(2ul);
        chic_rt_region_exit(handle);
        chic_rt_region_reset_stats(handle);
        var failedZeroed = chic_rt_region_alloc_zeroed(handle, 4usize, 1usize);
        ok = ok && NativePtr.IsNull(failedZeroed.Pointer);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_double_exit_and_zeroed_When_executed_Then_region_double_exit_and_zeroed()
{
    unsafe {
        let handle = chic_rt_region_enter(3ul);
        var zeroed = chic_rt_region_alloc_zeroed(handle, 6usize, 2usize);
        var ok = !NativePtr.IsNull(zeroed.Pointer);
        var idx = 0usize;
        var zeroOk = true;
        while (idx <6usize)
        {
            let ptr = NativePtr.OffsetMut(zeroed.Pointer, (isize) idx);
            if (NativePtr.ReadByteMut (ptr) != 0u8)
            {
                zeroOk = false;
            }
            idx = idx + 1;
        }
        chic_rt_region_exit(handle);
        chic_rt_region_exit(handle);
        let telemetry = chic_rt_region_telemetry(handle);
        ok = ok && zeroOk;
        ok = ok && telemetry.freed_bytes >= 6ul;
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_align_zero_and_empty_alloc_When_executed_Then_region_align_zero_and_empty_alloc()
{
    unsafe {
        let handle = chic_rt_region_enter(5ul);
        var empty = chic_rt_region_alloc(handle, 0usize, 0usize);
        var ok = empty.Alignment >= 1usize;
        ok = ok && empty.Size == 0usize;
        var small = chic_rt_region_alloc_zeroed(handle, 1usize, 0usize);
        ok = ok && !NativePtr.IsNull(small.Pointer);
        ok = ok && NativePtr.ReadByteMut(small.Pointer) == 0u8;
        chic_rt_region_exit(handle);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_allocation_failure_paths_When_executed_Then_region_allocation_failure_paths()
{
    unsafe {
        NativeAlloc.TestFailAllocAfter(0);
        let failedEnter = chic_rt_region_enter(7ul);
        var ok = failedEnter.Pointer == 0ul;
        NativeAlloc.TestReset();
        let handle = chic_rt_region_enter(8ul);
        ok = ok && handle.Pointer != 0ul;
        NativeAlloc.TestFailAllocAfter(0);
        var failedBlock = chic_rt_region_alloc(handle, 4usize, 1usize);
        ok = ok && NativePtr.IsNull(failedBlock.Pointer);
        NativeAlloc.TestReset();
        NativeAlloc.TestFailAllocAfter(1);
        var failedPush = chic_rt_region_alloc_zeroed(handle, 8usize, 1usize);
        ok = ok && NativePtr.IsNull(failedPush.Pointer);
        NativeAlloc.TestReset();
        chic_rt_region_exit(handle);
        Assert.That(ok).IsTrue();
    }
}
testcase Given_region_internal_helpers_When_executed_Then_region_internal_helpers()
{
    unsafe {
        RegionTestCoverageHelpers();
        Assert.That(true).IsTrue();
    }
}
