namespace Std.Runtime.Native;
// Chic-native region allocator that mirrors the Rust runtime ABI.
// Provides basic arena allocation with telemetry tracking. Implemented entirely
// in Chic so the Rust runtime can defer to the native archive when
// `chic_native_runtime` is linked.
@repr(c) public struct RegionTelemetry
{
    public ulong alloc_calls;
    public ulong alloc_zeroed_calls;
    public ulong alloc_bytes;
    public ulong alloc_zeroed_bytes;
    public ulong freed_bytes;
}
@repr(c) public struct RegionAllocation
{
    public ValueMutPtr Block;
    public * mut @expose_address byte Next;
}
@repr(c) public struct RegionArena
{
    public * mut @expose_address byte Head;
    public Std.Runtime.Native.RegionTelemetry Telemetry;
    public byte Freed;
    public ulong Profile;
}
// Align region metadata to pointer width; matches the Rust runtime layout.
private const usize REGION_ALIGN = sizeof(usize);
private unsafe static * mut RegionArena ArenaPtr(RegionHandle handle) {
    let addr = (isize) handle.Pointer;
    var * mut @expose_address byte raw = NativePtr.FromIsize(addr);
    return raw;
}
private unsafe static bool ArenaMissing(* const RegionArena arena) {
    return arena == null;
}
private unsafe static bool ArenaFreed(* const RegionArena arena) {
    return arena != null && (* arena).Freed != 0;
}
private unsafe static ValueMutPtr MakeNodeMut(* mut RegionAllocation node) {
    var * mut @expose_address byte raw = node;
    return new ValueMutPtr {
        Pointer = NativePtr.AsByteMut(raw), Size = sizeof(RegionAllocation), Alignment = REGION_ALIGN
    }
    ;
}
private unsafe static ValueMutPtr MakeArenaMut(* mut RegionArena arena) {
    var * mut @expose_address byte raw = arena;
    return new ValueMutPtr {
        Pointer = NativePtr.AsByteMut(raw), Size = sizeof(RegionArena), Alignment = REGION_ALIGN
    }
    ;
}
private static ValueMutPtr MakeFailed(usize size, usize align) {
    var realAlign = align == 0 ?1 : align;
    return new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = realAlign
    }
    ;
}
private unsafe static void FreeAllocations(* mut RegionArena arena) {
    var arenaPtr = arena;
    var head = (* arenaPtr).Head;
    while (! NativePtr.IsNull (head))
    {
        var * mut RegionAllocation node = head;
        let block = (* node).Block;
        if (! NativePtr.IsNull (block.Pointer) && block.Size >0)
        {
            (* arenaPtr).Telemetry.freed_bytes = (* arenaPtr).Telemetry.freed_bytes + (ulong) block.Size;
            NativeAlloc.Free(block);
        }
        let next = (* node).Next;
        NativeAlloc.Free(MakeNodeMut(node));
        head = next;
    }
    (* arenaPtr).Head = NativePtr.NullMut();
}
private unsafe static bool PushAllocation(* mut RegionArena arena, ValueMutPtr block) {
    var arenaPtr = arena;
    var node = MakeFailed(sizeof(RegionAllocation), REGION_ALIGN);
    let status = NativeAlloc.AllocZeroed(sizeof(RegionAllocation), REGION_ALIGN, out node);
    if (status != NativeAllocationError.Success)
    {
        return false;
    }
    var * mut RegionAllocation ptr = node.Pointer;
    (* ptr).Block = block;
    (* ptr).Next = (* arenaPtr).Head;
    (* arenaPtr).Head = node.Pointer;
    return true;
}
private unsafe static ValueMutPtr AllocBlock(usize size, usize align, bool zeroed) {
    var realAlign = align == 0 ?1 : align;
    var block = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = size, Alignment = realAlign
    }
    ;
    var status = NativeAllocationError.Success;
    if (zeroed)
    {
        status = NativeAlloc.AllocZeroed(size, realAlign, out block);
    }
    else
    {
        status = NativeAlloc.Alloc(size, realAlign, out block);
    }
    if (status != NativeAllocationError.Success)
    {
        return MakeFailed(size, align);
    }
    return block;
}
@extern("C") @export("chic_rt_region_enter") public unsafe static RegionHandle chic_rt_region_enter(ulong profile) {
    var arenaMem = MakeFailed(sizeof(RegionArena), REGION_ALIGN);
    if (NativeAlloc.AllocZeroed (sizeof(RegionArena), REGION_ALIGN, out arenaMem) != NativeAllocationError.Success) {
        return new RegionHandle {
            Pointer = 0ul,
            Profile = profile,
            Generation = 0ul
        }
        ;
    }
    var * mut RegionArena arena = arenaMem.Pointer;
    (* arena).Head = NativePtr.NullMut();
    (* arena).Telemetry = new RegionTelemetry {
        alloc_calls = 0, alloc_zeroed_calls = 0, alloc_bytes = 0, alloc_zeroed_bytes = 0, freed_bytes = 0,
    }
    ;
    (* arena).Freed = 0;
    (* arena).Profile = profile;
    return new RegionHandle {
        Pointer = (ulong) (nuint) arenaMem.Pointer,
        Profile = profile,
        Generation = 0ul
    }
    ;
}
@extern("C") @export("chic_rt_region_exit") public unsafe static void chic_rt_region_exit(RegionHandle handle) {
    var * mut RegionArena arena = ArenaPtr(handle);
    if (ArenaMissing (arena) || ArenaFreed (arena))
    {
        return;
    }
    FreeAllocations(arena);
    (* arena).Freed = 1;
}
@extern("C") @export("chic_rt_region_alloc") public unsafe static ValueMutPtr chic_rt_region_alloc(RegionHandle handle, usize size,
usize align) {
    var * mut RegionArena arena = ArenaPtr(handle);
    if (ArenaMissing (arena) || ArenaFreed (arena))
    {
        return MakeFailed(size, align);
    }
    var block = AllocBlock(size, align, false);
    if (NativePtr.IsNull (block.Pointer))
    {
        return block;
    }
    if (! PushAllocation (arena, block))
    {
        NativeAlloc.Free(block);
        return MakeFailed(size, align);
    }
    (* arena).Telemetry.alloc_calls = (* arena).Telemetry.alloc_calls + 1;
    (* arena).Telemetry.alloc_bytes = (* arena).Telemetry.alloc_bytes + (ulong) size;
    return block;
}
@extern("C") @export("chic_rt_region_alloc_zeroed") public unsafe static ValueMutPtr chic_rt_region_alloc_zeroed(RegionHandle handle,
usize size, usize align) {
    var * mut RegionArena arena = ArenaPtr(handle);
    if (ArenaMissing (arena) || ArenaFreed (arena))
    {
        return MakeFailed(size, align);
    }
    var block = AllocBlock(size, align, true);
    if (NativePtr.IsNull (block.Pointer))
    {
        return block;
    }
    if (! PushAllocation (arena, block))
    {
        NativeAlloc.Free(block);
        return MakeFailed(size, align);
    }
    (* arena).Telemetry.alloc_zeroed_calls = (* arena).Telemetry.alloc_zeroed_calls + 1;
    (* arena).Telemetry.alloc_zeroed_bytes = (* arena).Telemetry.alloc_zeroed_bytes + (ulong) size;
    return block;
}
@extern("C") @export("chic_rt_region_telemetry") public unsafe static RegionTelemetry chic_rt_region_telemetry(RegionHandle handle) {
    var * const RegionArena arena = ArenaPtr(handle);
    if (ArenaMissing (arena))
    {
        return new RegionTelemetry {
            alloc_calls = 0, alloc_zeroed_calls = 0, alloc_bytes = 0, alloc_zeroed_bytes = 0, freed_bytes = 0,
        }
        ;
    }
    return(* arena).Telemetry;
}
@extern("C") @export("chic_rt_region_reset_stats") public unsafe static void chic_rt_region_reset_stats(RegionHandle handle) {
    var * mut RegionArena arena = ArenaPtr(handle);
    if (ArenaMissing (arena) || ArenaFreed (arena))
    {
        return;
    }
    (* arena).Telemetry.alloc_calls = 0;
    (* arena).Telemetry.alloc_zeroed_calls = 0;
    (* arena).Telemetry.alloc_bytes = 0;
    (* arena).Telemetry.alloc_zeroed_bytes = 0;
    (* arena).Telemetry.freed_bytes = 0;
}

public unsafe static bool RegionTestCoverageSweep() {
    var ok = true;
    let missing = chic_rt_region_telemetry(new RegionHandle {
        Pointer = 0ul,
        Profile = 0ul,
        Generation = 0ul
    }
    );
    ok = ok && missing.alloc_calls == 0ul && missing.alloc_zeroed_calls == 0ul;

    let handle = chic_rt_region_enter(3ul);
    ok = ok && handle.Pointer != 0ul;

    var arena = ArenaPtr(handle);
    ok = ok && !ArenaMissing(arena);
    ok = ok && !ArenaFreed(arena);
    let _ = MakeArenaMut(arena);

    var node = new RegionAllocation {
        Block = MakeFailed(0usize, 0usize), Next = NativePtr.NullMut()
    }
    ;
    let _ = MakeNodeMut(& node);

    NativeAlloc.TestFailAllocAfter(0);
    let failedBlock = AllocBlock(4usize, 0usize, false);
    ok = ok && NativePtr.IsNull(failedBlock.Pointer);
    NativeAlloc.TestReset();

    let block = chic_rt_region_alloc(handle, 8usize, 0usize);
    ok = ok && !NativePtr.IsNull(block.Pointer);
    let zeroed = chic_rt_region_alloc_zeroed(handle, 16usize, 4usize);
    ok = ok && !NativePtr.IsNull(zeroed.Pointer);

    NativeAlloc.TestFailAllocAfter(1);
    let pushFail = chic_rt_region_alloc(handle, 4usize, 1usize);
    ok = ok && NativePtr.IsNull(pushFail.Pointer);
    NativeAlloc.TestReset();

    NativeAlloc.TestFailAllocAfter(0);
    let allocFail = chic_rt_region_alloc(handle, 4usize, 1usize);
    ok = ok && NativePtr.IsNull(allocFail.Pointer);
    NativeAlloc.TestReset();

    chic_rt_region_reset_stats(handle);
    let afterReset = chic_rt_region_telemetry(handle);
    ok = ok && afterReset.alloc_calls == 0ul && afterReset.alloc_zeroed_calls == 0ul;

    chic_rt_region_exit(handle);
    ok = ok && ArenaFreed(arena);
    let afterExitAlloc = chic_rt_region_alloc(handle, 1usize, 1usize);
    ok = ok && NativePtr.IsNull(afterExitAlloc.Pointer);
    chic_rt_region_exit(handle);
    return ok;
}

public unsafe static void RegionTestCoverageHelpers() {
    let _ = ArenaMissing((* const RegionArena) NativePtr.NullConst());
    var arena = new RegionArena {
        Head = NativePtr.NullMut(), Telemetry = new RegionTelemetry {
            alloc_calls = 0, alloc_zeroed_calls = 0, alloc_bytes = 0, alloc_zeroed_bytes = 0, freed_bytes = 0,
        }, Freed = 0, Profile = 0ul,
    }
    ;
    let _ = ArenaFreed(& arena);
    let _ = MakeArenaMut(& arena);
    var node = new RegionAllocation {
        Block = MakeFailed(0usize, 0usize), Next = NativePtr.NullMut(),
    }
    ;
    let _ = MakeNodeMut(& node);
    let failed = MakeFailed(8usize, 0usize);
    let _ = failed;
    let block = AllocBlock(0usize, 0usize, false);
    if (! NativePtr.IsNull (block.Pointer))
    {
        NativeAlloc.Free(block);
    }
    NativeAlloc.TestFailAllocAfter(0);
    let _ = PushAllocation(& arena, failed);
    NativeAlloc.TestReset();
    FreeAllocations(& arena);
}
