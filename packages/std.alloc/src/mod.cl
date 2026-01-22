namespace Std.Alloc;
import Std.Runtime.Collections;
import Std.Core;
import Std.Numeric;
import Std.Core.Testing;
@repr(c) public struct AllocationTelemetry
{
    public usize AllocCalls;
    public usize AllocZeroedCalls;
    public usize ReallocCalls;
    public usize FreeCalls;
    public usize AllocBytes;
    public usize AllocZeroedBytes;
    public usize ReallocBytes;
    public usize FreedBytes;
}
/// <summary>
/// Function-pointer based allocator hooks. Use <see cref="Hooks.Install"/> to
/// register a custom allocator; pass null pointers for entries you do not
/// implement to fall back to the default runtime allocator.
/// </summary>
@repr(c) public struct AllocatorVTable
{
    public * mut @expose_address byte Context;
    public fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr Alloc;
    public fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr AllocZeroed;
    public fn @extern("C")(* mut @expose_address byte, ValueMutPtr, usize, usize, usize) -> ValueMutPtr Realloc;
    public fn @extern("C")(* mut @expose_address byte, ValueMutPtr) -> void Free;
    public static AllocatorVTable With(
        * mut @expose_address byte context,
        fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr alloc,
        fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr allocZeroed,
        fn @extern("C")(* mut @expose_address byte, ValueMutPtr, usize, usize, usize) -> ValueMutPtr realloc,
        fn @extern("C")(* mut @expose_address byte, ValueMutPtr) -> void free
    ) {
        var table = CoreIntrinsics.DefaultValue <AllocatorVTable >();
        table.Context = context;
        table.Alloc = alloc;
        table.AllocZeroed = allocZeroed;
        table.Realloc = realloc;
        table.Free = free;
        return table;
    }
}
public static class Hooks
{
    @extern("C") private static extern void chic_rt_allocator_install(AllocatorVTable vtable);
    @extern("C") private static extern void chic_rt_allocator_reset();
    @extern("C") private static extern void chic_rt_alloc_stats_fill(out AllocationTelemetry telemetry);
    @extern("C") private static extern void chic_rt_reset_alloc_stats();
    /// <summary>
    /// Installs the provided allocator vtable. All allocations performed via
    /// the runtime (GlobalAllocator, Vec, String, etc.) will route through
    /// these hooks.
    /// </summary>
    public static void Install(AllocatorVTable vtable) {
        chic_rt_allocator_install(vtable);
    }
    /// <summary>
    /// Restores the default allocator (Rust/global) and clears any custom
    /// hooks previously installed.
    /// </summary>
    public static void Reset() {
        chic_rt_allocator_reset();
    }
    /// <summary>
    /// Returns aggregated allocation telemetry from the runtime.
    /// </summary>
    public static AllocationTelemetry Telemetry() {
        var telemetry = CoreIntrinsics.DefaultValue <AllocationTelemetry >();
        chic_rt_alloc_stats_fill(out telemetry);
        return telemetry;
    }
    /// <summary>
    /// Clears allocation telemetry counters.
    /// </summary>
    public static void ResetTelemetry() {
        chic_rt_reset_alloc_stats();
    }
}

@extern("C") private static ValueMutPtr AllocStub(* mut @expose_address byte context, usize size, usize align) {
    return ValuePointer.NullMut(0usize, 0usize);
}

@extern("C") private static ValueMutPtr AllocZeroedStub(* mut @expose_address byte context, usize size, usize align) {
    return ValuePointer.NullMut(0usize, 0usize);
}

@extern("C") private static ValueMutPtr ReallocStub(
    * mut @expose_address byte context,
    ValueMutPtr current,
    usize oldSize,
    usize newSize,
    usize align
) {
    return ValuePointer.NullMut(0usize, 0usize);
}

@extern("C") private static void FreeStub(* mut @expose_address byte context, ValueMutPtr value) {
}

testcase Given_allocator_vtable_with_sets_entries_When_executed_Then_allocator_vtable_with_sets_entries()
{
    unsafe {
        let ctx = Pointer.NullMut<byte>();
        let table = AllocatorVTable.With(ctx, AllocStub, AllocZeroedStub, ReallocStub, FreeStub);
        Assert.That(Pointer.IsNull(table.Context)).IsTrue();
    }
}

testcase Given_allocator_vtable_alloc_returns_null_When_executed_Then_allocator_vtable_alloc_returns_null()
{
    unsafe {
        let ctx = Pointer.NullMut<byte>();
        let table = AllocatorVTable.With(ctx, AllocStub, AllocZeroedStub, ReallocStub, FreeStub);
        let alloc = table.Alloc;
        let result = alloc(table.Context, 1usize, 1usize);
        Assert.That(ValuePointer.IsNullMut(result)).IsTrue();
    }
}

testcase Given_alloc_hooks_reset_telemetry_alloc_calls_zero_When_executed_Then_alloc_hooks_reset_telemetry_alloc_calls_zero()
{
    Hooks.ResetTelemetry();
    let telemetry = Hooks.Telemetry();
    Assert.That(telemetry.AllocCalls == 0usize).IsTrue();
}

testcase Given_alloc_hooks_reset_telemetry_realloc_calls_zero_When_executed_Then_alloc_hooks_reset_telemetry_realloc_calls_zero()
{
    Hooks.ResetTelemetry();
    let telemetry = Hooks.Telemetry();
    Assert.That(telemetry.ReallocCalls == 0usize).IsTrue();
}

testcase Given_alloc_hooks_reset_telemetry_free_calls_zero_When_executed_Then_alloc_hooks_reset_telemetry_free_calls_zero()
{
    Hooks.ResetTelemetry();
    let telemetry = Hooks.Telemetry();
    Assert.That(telemetry.FreeCalls == 0usize).IsTrue();
}

testcase Given_alloc_hooks_install_and_reset_When_executed_Then_does_not_throw()
{
    unsafe {
        let ctx = Pointer.NullMut<byte>();
        let table = AllocatorVTable.With(ctx, AllocStub, AllocZeroedStub, ReallocStub, FreeStub);
        Hooks.Install(table);
        Hooks.Reset();
    }
}

testcase Given_allocator_stub_entries_When_executed_Then_return_null_or_noop()
{
    unsafe {
        let ctx = Pointer.NullMut<byte>();
        let table = AllocatorVTable.With(ctx, AllocStub, AllocZeroedStub, ReallocStub, FreeStub);

        let allocZeroed = table.AllocZeroed;
        let zeroed = allocZeroed(table.Context, 16usize, 8usize);
        Assert.That(ValuePointer.IsNullMut(zeroed)).IsTrue();

        let realloc = table.Realloc;
        let current = ValuePointer.NullMut(0usize, 0usize);
        let resized = realloc(table.Context, current, 0usize, 32usize, 8usize);
        Assert.That(ValuePointer.IsNullMut(resized)).IsTrue();

        let free = table.Free;
        free(table.Context, current);
    }
}
