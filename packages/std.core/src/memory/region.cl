namespace Std.Memory;
import Std.Numeric;
import Std.Runtime;
import Std.Runtime.Collections;
import Std.Span;
import Std.Core.Testing;
@repr(c) public struct RegionHandle
{
    public ulong Pointer;
    public ulong Profile;
    public ulong Generation;
    public bool IsNull {
        get {
            return Pointer == 0ul;
        }
    }
    public void dispose(ref this) {
        if (IsNull)
        {
            return;
        }
        Region.Exit(this);
        Pointer = 0ul;
        this.Profile = 0;
        this.Generation = 0;
    }
}
@repr(c) public struct RegionTelemetry
{
    public ulong AllocCalls;
    public ulong AllocZeroedCalls;
    public ulong AllocBytes;
    public ulong AllocZeroedBytes;
    public ulong FreedBytes;
}
public static class Region
{
    @extern("C") private static extern RegionHandle chic_rt_region_enter(ulong profile);
    @extern("C") private static extern void chic_rt_region_exit(RegionHandle handle);
    @extern("C") private static extern ValueMutPtr chic_rt_region_alloc(RegionHandle regionHandle, usize size, usize align);
    @extern("C") private static extern ValueMutPtr chic_rt_region_alloc_zeroed(RegionHandle regionHandle, usize size, usize align);
    @extern("C") private static extern RegionTelemetry chic_rt_region_telemetry(RegionHandle regionHandle);
    @extern("C") private static extern void chic_rt_region_reset_stats(RegionHandle regionHandle);
    private static ulong HashProfile(string profile) {
        // FNV-1a 64-bit over the UTF-8 bytes of the profile name. Matches the runtime's
        // `RegionHandle.profile` field and lets tooling aggregate per-region telemetry.
        var span = Std.Span.ReadOnlySpan.FromString(profile);
        var hash = 14695981039346656037UL;
        // offset basis
        const ulong prime = 1099511628211UL;
        unsafe {
            var index = 0usize;
            while (index <span.Length)
            {
                let bytePtr = Std.Span.SpanIntrinsics.chic_rt_span_ptr_at_readonly(ref span.Raw, index);
                hash = (hash ^ (ulong)(* bytePtr)) * prime;
                index = index + 1usize;
            }
        }
        return hash;
    }
    public static RegionHandle Enter(string profile = "default") {
        return chic_rt_region_enter(HashProfile(profile));
    }
    public static void Exit(RegionHandle regionHandle) {
        chic_rt_region_exit(regionHandle);
    }
    private static Std.Runtime.TypeMetadataRecord Metadata <T >() {
        let typeId = __type_id_of <T >();
        return Std.Runtime.TypeMetadata.Resolve(typeId);
    }
    public static ValueMutPtr Alloc <T >(RegionHandle regionHandle, usize count = 1) {
        let metadata = Metadata <T >();
        let size = metadata.Size * count;
        return Alloc(regionHandle, size, metadata.Align);
    }
    public static ValueMutPtr Alloc(RegionHandle regionHandle, usize size, usize align) {
        return chic_rt_region_alloc(regionHandle, size, align);
    }
    public static ValueMutPtr AllocZeroed <T >(RegionHandle regionHandle, usize count = 1) {
        let metadata = Metadata <T >();
        let size = metadata.Size * count;
        return AllocZeroed(regionHandle, size, metadata.Align);
    }
    public static ValueMutPtr AllocZeroed(RegionHandle regionHandle, usize size, usize align) {
        return chic_rt_region_alloc_zeroed(regionHandle, size, align);
    }
    public static Span <T >Span <T >(RegionHandle regionHandle, usize length) {
        let metadata = Metadata <T >();
        let size = metadata.Size * length;
        let allocation = Alloc(regionHandle, size, metadata.Align);
        unsafe {
            let handle = Std.Runtime.Collections.ValuePointer.CreateMut(Std.Numeric.PointerIntrinsics.AsByteMut(allocation.Pointer),
            metadata.Size, metadata.Align);
            return Span <T >.FromValuePointer(handle, length);
        }
    }
    public static RegionTelemetry Telemetry(RegionHandle regionHandle) {
        return chic_rt_region_telemetry(regionHandle);
    }
    public static void ResetTelemetry(RegionHandle regionHandle) {
        chic_rt_region_reset_stats(regionHandle);
    }
}
testcase Given_region_enter_handle_is_not_null_When_executed_Then_region_enter_handle_is_not_null()
{
    var handle = Region.Enter();
    Assert.That(handle.IsNull).IsFalse();
    handle.dispose();
}
testcase Given_region_handle_dispose_sets_null_When_executed_Then_region_handle_dispose_sets_null()
{
    var handle = Region.Enter();
    handle.dispose();
    Assert.That(handle.IsNull).IsTrue();
}
testcase Given_region_alloc_returns_non_null_When_executed_Then_region_alloc_returns_non_null()
{
    var handle = Region.Enter();
    let allocation = Region.Alloc(handle, 16usize, 8usize);
    Assert.That(ValuePointer.IsNullMut(allocation)).IsFalse();
    handle.dispose();
}
testcase Given_region_alloc_zeroed_returns_non_null_When_executed_Then_region_alloc_zeroed_returns_non_null()
{
    var handle = Region.Enter();
    let allocation = Region.AllocZeroed(handle, 16usize, 8usize);
    Assert.That(ValuePointer.IsNullMut(allocation)).IsFalse();
    handle.dispose();
}
testcase Given_region_span_length_matches_When_executed_Then_region_span_length_matches()
{
    var handle = Region.Enter();
    let span = Region.Span <int >(handle, 3usize);
    Assert.That(span.Length == 3usize).IsTrue();
    handle.dispose();
}
