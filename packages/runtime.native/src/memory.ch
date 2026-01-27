namespace Std.Runtime.Native;
// Native allocation primitives with simple telemetry, mirroring the Rust runtime surface.
@repr(c) public struct AllocationTelemetry
{
    public usize alloc_calls;
    public usize alloc_zeroed_calls;
    public usize realloc_calls;
    public usize free_calls;
    public usize alloc_bytes;
    public usize alloc_zeroed_bytes;
    public usize realloc_bytes;
    public usize freed_bytes;
}
@repr(c) public struct ChicAllocatorVTable
{
    public * mut @expose_address byte context;
    public fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr alloc;
    public fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr alloc_zeroed;
    public fn @extern("C")(* mut @expose_address byte, ValueMutPtr, usize, usize, usize) -> ValueMutPtr realloc;
    public fn @extern("C")(* mut @expose_address byte, ValueMutPtr) -> void free;
}
internal static class AllocTelemetry
{
    private static usize _allocCalls;
    private static usize _allocZeroedCalls;
    private static usize _reallocCalls;
    private static usize _freeCalls;
    private static usize _allocBytes;
    private static usize _allocZeroedBytes;
    private static usize _reallocBytes;
    private static usize _freedBytes;
    public static void RecordAlloc(usize bytes, bool zeroed) {
        if (zeroed)
        {
            _allocZeroedCalls = _allocZeroedCalls + 1;
            _allocZeroedBytes = _allocZeroedBytes + bytes;
        }
        else
        {
            _allocCalls = _allocCalls + 1;
            _allocBytes = _allocBytes + bytes;
        }
    }
    public static void RecordRealloc(usize newSize, usize oldSize) {
        _reallocCalls = _reallocCalls + 1;
        _reallocBytes = _reallocBytes + newSize;
        if (oldSize != 0)
        {
            _freedBytes = _freedBytes + oldSize;
        }
    }
    public static void RecordFree(usize bytes) {
        _freeCalls = _freeCalls + 1;
        _freedBytes = _freedBytes + bytes;
    }
    public static AllocationTelemetry Snapshot() {
        return new AllocationTelemetry {
            alloc_calls = _allocCalls, alloc_zeroed_calls = _allocZeroedCalls, realloc_calls = _reallocCalls, free_calls = _freeCalls, alloc_bytes = _allocBytes, alloc_zeroed_bytes = _allocZeroedBytes, realloc_bytes = _reallocBytes, freed_bytes = _freedBytes,
        }
        ;
    }
    public static void Reset() {
        _allocCalls = 0;
        _allocZeroedCalls = 0;
        _reallocCalls = 0;
        _freeCalls = 0;
        _allocBytes = 0;
        _allocZeroedBytes = 0;
        _reallocBytes = 0;
        _freedBytes = 0;
    }
}
public static class MemoryRuntime
{
    private static * mut @expose_address byte _allocContext;
    private static fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr _allocFn;
    private static fn @extern("C")(* mut @expose_address byte, usize, usize) -> ValueMutPtr _allocZeroedFn;
    private static fn @extern("C")(* mut @expose_address byte, ValueMutPtr, usize, usize, usize) -> ValueMutPtr _reallocFn;
    private static fn @extern("C")(* mut @expose_address byte, ValueMutPtr) -> void _freeFn;
    private static usize _testAllocCalls;
    private static usize _testFreeCalls;
    private static bool _initialized = false;
    private static bool _use_custom_allocator = false;
    private unsafe static ValueMutPtr DefaultAlloc(usize size, usize alignment) {
        let effectiveAlign = alignment == 0 ?1 : alignment;
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = effectiveAlign
        }
        ;
        if (size == 0)
        {
            return result;
        }
        if (NativeAlloc.Alloc (size, effectiveAlign, out result) == NativeAllocationError.Success) {
            if (!NativePtr.IsNull (result.Pointer))
            {
                AllocTelemetry.RecordAlloc(size, false);
            }
        }
        return result;
    }
    private unsafe static ValueMutPtr DefaultAllocZeroed(usize size, usize alignment) {
        let effectiveAlign = alignment == 0 ?1 : alignment;
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = effectiveAlign
        }
        ;
        if (size == 0)
        {
            return result;
        }
        if (NativeAlloc.AllocZeroed (size, effectiveAlign, out result) == NativeAllocationError.Success) {
            if (!NativePtr.IsNull (result.Pointer))
            {
                AllocTelemetry.RecordAlloc(size, true);
            }
        }
        return result;
    }
    private unsafe static ValueMutPtr DefaultRealloc(ValueMutPtr ptr, usize oldSize, usize newSize, usize alignment) {
        let effectiveAlign = alignment == 0 ?1 : alignment;
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = effectiveAlign
        }
        ;
        if (NativeAlloc.Realloc (ptr, oldSize, newSize, effectiveAlign, out result) == NativeAllocationError.Success) {
            if (!NativePtr.IsNull (result.Pointer))
            {
                AllocTelemetry.RecordRealloc(newSize, oldSize);
            }
        }
        return result;
    }
    private unsafe static void DefaultFree(ValueMutPtr ptr) {
        if (NativePtr.IsNull (ptr.Pointer))
        {
            return;
        }
        AllocTelemetry.RecordFree(ptr.Size);
        NativeAlloc.Free(ptr);
    }
    @extern("C") private unsafe static ValueMutPtr DefaultAllocHook(* mut @expose_address byte _ctx, usize size, usize alignment) {
        return DefaultAlloc(size, alignment);
    }
    @extern("C") private unsafe static ValueMutPtr DefaultAllocZeroedHook(* mut @expose_address byte _ctx, usize size, usize alignment) {
        return DefaultAllocZeroed(size, alignment);
    }
    @extern("C") private unsafe static ValueMutPtr DefaultReallocHook(* mut @expose_address byte _ctx, ValueMutPtr ptr, usize oldSize,
    usize newSize, usize alignment) {
        return DefaultRealloc(ptr, oldSize, newSize, alignment);
    }
    @extern("C") private unsafe static void DefaultFreeHook(* mut @expose_address byte _ctx, ValueMutPtr ptr) {
        DefaultFree(ptr);
    }
    @extern("C") private unsafe static ValueMutPtr TestAllocHook(* mut @expose_address byte _ctx, usize size, usize alignment) {
        _testAllocCalls = _testAllocCalls + 1;
        return DefaultAlloc(size, alignment);
    }
    @extern("C") private unsafe static ValueMutPtr TestAllocZeroedHook(* mut @expose_address byte _ctx, usize size, usize alignment) {
        _testAllocCalls = _testAllocCalls + 1;
        return DefaultAllocZeroed(size, alignment);
    }
    @extern("C") private unsafe static ValueMutPtr TestReallocHook(* mut @expose_address byte _ctx, ValueMutPtr ptr, usize oldSize,
    usize newSize, usize alignment) {
        _testAllocCalls = _testAllocCalls + 1;
        return DefaultRealloc(ptr, oldSize, newSize, alignment);
    }
    @extern("C") private unsafe static void TestFreeHook(* mut @expose_address byte _ctx, ValueMutPtr ptr) {
        _testFreeCalls = _testFreeCalls + 1;
        DefaultFree(ptr);
    }
    private static ChicAllocatorVTable DefaultVTable() {
        _allocContext = NativePtr.NullMut();
        _allocFn = DefaultAllocHook;
        _allocZeroedFn = DefaultAllocZeroedHook;
        _reallocFn = DefaultReallocHook;
        _freeFn = DefaultFreeHook;
        _use_custom_allocator = false;
        return new ChicAllocatorVTable {
            context = _allocContext, alloc = _allocFn, alloc_zeroed = _allocZeroedFn, realloc = _reallocFn, free = _freeFn,
        }
        ;
    }
    private static ChicAllocatorVTable EnsureAllocatorVTable() {
        if (!_initialized)
        {
            DefaultVTable();
            _initialized = true;
        }
        return new ChicAllocatorVTable {
            context = _allocContext, alloc = _allocFn, alloc_zeroed = _allocZeroedFn, realloc = _reallocFn, free = _freeFn,
        }
        ;
    }
    @extern("C") @export("chic_rt_allocator_install") public static void chic_rt_allocator_install(ChicAllocatorVTable vtable) {
        _allocContext = vtable.context;
        _allocFn = vtable.alloc;
        _allocZeroedFn = vtable.alloc_zeroed;
        _reallocFn = vtable.realloc;
        _freeFn = vtable.free;
        _use_custom_allocator = true;
        _initialized = true;
    }
    @extern("C") @export("chic_rt_allocator_reset") public static void chic_rt_allocator_reset() {
        DefaultVTable();
        _initialized = true;
    }
    public static ChicAllocatorVTable TestAllocatorVTable() {
        return new ChicAllocatorVTable {
            context = NativePtr.NullMut(), alloc = TestAllocHook, alloc_zeroed = TestAllocZeroedHook, realloc = TestReallocHook, free = TestFreeHook,
        }
        ;
    }
    public static void ResetTestAllocatorCalls() {
        _testAllocCalls = 0;
        _testFreeCalls = 0;
    }
    public static usize TestAllocatorAllocCalls() {
        return _testAllocCalls;
    }
    public static usize TestAllocatorFreeCalls() {
        return _testFreeCalls;
    }
    @extern("C") @export("chic_rt_alloc") public unsafe static ValueMutPtr chic_rt_alloc(usize size, usize align) {
        var table = EnsureAllocatorVTable();
        let alignment = align == 0 ?1 : align;
        if (size == 0)
        {
            return new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = alignment
            }
            ;
        }
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = alignment
        }
        ;
        let alloc_fn = table.alloc;
        if (_use_custom_allocator && alloc_fn != null)
        {
            result = alloc_fn(table.context, size, alignment);
        }
        else
        {
            result = DefaultAlloc(size, alignment);
        }
        if (!NativePtr.IsNull (result.Pointer))
        {
            AllocTelemetry.RecordAlloc(size, false);
        }
        return result;
    }
    @extern("C") @export("chic_rt_alloc_zeroed") public unsafe static ValueMutPtr chic_rt_alloc_zeroed(usize size, usize align) {
        var table = EnsureAllocatorVTable();
        let alignment = align == 0 ?1 : align;
        if (size == 0)
        {
            return new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = alignment
            }
            ;
        }
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = size, Alignment = alignment
        }
        ;
        let alloc_zero_fn = table.alloc_zeroed;
        if (_use_custom_allocator && alloc_zero_fn != null)
        {
            result = alloc_zero_fn(table.context, size, alignment);
        }
        else
        {
            result = DefaultAllocZeroed(size, alignment);
        }
        if (!NativePtr.IsNull (result.Pointer))
        {
            AllocTelemetry.RecordAlloc(size, true);
        }
        return result;
    }
    @extern("C") @export("chic_rt_realloc") public unsafe static ValueMutPtr chic_rt_realloc(ValueMutPtr ptr, usize oldSize,
    usize newSize, usize align) {
        var table = EnsureAllocatorVTable();
        let alignment = align == 0 ?1 : align;
        if (newSize == 0)
        {
            if (!NativePtr.IsNull (ptr.Pointer) && oldSize != 0)
            {
                chic_rt_free(ptr);
            }
            return new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0, Alignment = alignment
            }
            ;
        }
        var result = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = newSize, Alignment = alignment
        }
        ;
        let realloc_fn = table.realloc;
        if (_use_custom_allocator && realloc_fn != null)
        {
            result = realloc_fn(table.context, ptr, oldSize, newSize, alignment);
        }
        else
        {
            result = DefaultRealloc(ptr, oldSize, newSize, alignment);
        }
        if (!NativePtr.IsNull (result.Pointer))
        {
            AllocTelemetry.RecordRealloc(newSize, oldSize);
        }
        return result;
    }
    @extern("C") @export("chic_rt_free") public unsafe static void chic_rt_free(ValueMutPtr ptr) {
        var table = EnsureAllocatorVTable();
        if (NativePtr.IsNull (ptr.Pointer))
        {
            return;
        }
        AllocTelemetry.RecordFree(ptr.Size);
        let free_fn = table.free;
        if (_use_custom_allocator && free_fn != null)
        {
            free_fn(table.context, ptr);
        }
        else
        {
            DefaultFree(ptr);
        }
    }
    @extern("C") @export("chic_rt_alloc_stats") public static AllocationTelemetry chic_rt_alloc_stats() {
        return AllocTelemetry.Snapshot();
    }
    @extern("C") @export("chic_rt_alloc_stats_fill") public unsafe static void chic_rt_alloc_stats_fill(* mut AllocationTelemetry out_stats) {
        if (out_stats == null)
        {
            return;
        }
        * out_stats = AllocTelemetry.Snapshot();
    }
    @extern("C") @export("chic_rt_reset_alloc_stats") public static void chic_rt_reset_alloc_stats() {
        AllocTelemetry.Reset();
    }
    @extern("C") @export("chic_rt_memcpy") public unsafe static void chic_rt_memcpy(ValueMutPtr dst, ValueConstPtr src, usize len) {
        NativeAlloc.Copy(dst, src, len);
    }
    @extern("C") @export("chic_rt_memmove") public unsafe static void chic_rt_memmove(ValueMutPtr dst, ValueMutPtr src, usize len) {
        let src_const = new ValueConstPtr {
            Pointer = NativePtr.AsConstPtr(src.Pointer), Size = src.Size, Alignment = src.Alignment,
        }
        ;
        NativeAlloc.Move(dst, src_const, len);
    }
    @extern("C") @export("chic_rt_memset") public unsafe static void chic_rt_memset(ValueMutPtr dst, byte value, usize len) {
        NativeAlloc.Set(dst, value, len);
    }
    public unsafe static void TestCoverageHelpers() {
        AllocTelemetry.Reset();
        let _ = DefaultVTable();
        let _ = EnsureAllocatorVTable();
        let _ = DefaultAlloc(0usize, 0usize);
        let _ = DefaultAllocZeroed(0usize, 0usize);
        DefaultFree(new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize,
        }
        );
        var zeroAlign = chic_rt_alloc(1usize, 0usize);
        if (!NativePtr.IsNull (zeroAlign.Pointer))
        {
            chic_rt_free(zeroAlign);
        }
        var zeroAlignZeroed = chic_rt_alloc_zeroed(1usize, 0usize);
        if (!NativePtr.IsNull (zeroAlignZeroed.Pointer))
        {
            chic_rt_free(zeroAlignZeroed);
        }
        var block = DefaultAlloc(8usize, 1usize);
        var zeroed = DefaultAllocZeroed(8usize, 1usize);
        var resized = DefaultRealloc(block, 8usize, 16usize, 1usize);
        if (!NativePtr.IsNull (resized.Pointer))
        {
            DefaultFree(resized);
        }
        else if (!NativePtr.IsNull (block.Pointer))
        {
            DefaultFree(block);
        }
        if (!NativePtr.IsNull (zeroed.Pointer))
        {
            DefaultFree(zeroed);
        }
        let _ = DefaultAllocHook(NativePtr.NullMut(), 0usize, 1usize);
        let _ = DefaultAllocZeroedHook(NativePtr.NullMut(), 0usize, 1usize);
        let _ = DefaultReallocHook(NativePtr.NullMut(), new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize,
        }
        , 0usize, 0usize, 1usize);
        DefaultFreeHook(NativePtr.NullMut(), new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize,
        }
        );
    }
}
