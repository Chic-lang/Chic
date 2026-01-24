namespace Std.Runtime.Native;
// Minimal Chic-native async runtime shims used by the bootstrap compiler to link async-capable
// programs. The current implementation drives async state machines synchronously via their
// generated poll vtables. This is a stopgap until a Chic scheduler lands.
@repr(c) public struct NativeFutureHeader
{
    public isize StatePointer;
    public isize VTablePointer;
    public isize ExecutorContext;
    public uint Flags;
}
@repr(c) public struct NativeFutureVTable
{
    public isize PollFunction;
    public isize DropFunction;
}
@repr(c) public struct NativeRuntimeContext
{
    public isize Inner;
}
internal static class AsyncFlags
{
    public const uint Ready = 0x0000_0001u;
    public const uint Completed = 0x0000_0002u;
    public const uint Cancelled = 0x0000_0004u;
}
private unsafe static uint PollOnce(* mut NativeFutureHeader header, * mut NativeRuntimeContext ctx) {
    if (header == null)
    {
        return 1u;
    }
    if ( (* header).VTablePointer == 0isize)
    {
        return 1u;
    }
    let vtable = (* const @readonly @expose_address NativeFutureVTable) NativePtr.FromIsizeConst((* header).VTablePointer);
    if (vtable == null || (* vtable).PollFunction == 0isize)
    {
        return 1u;
    }
    let pollPtr = NativePtr.FromIsizeConst((* vtable).PollFunction);
    let pollFn = (fn @extern("C")(* mut NativeFutureHeader, * mut NativeRuntimeContext) -> uint) pollPtr;
    return pollFn(header, ctx);
}
private unsafe static void DriveToCompletion(* mut NativeFutureHeader header) {
    if (header == null)
    {
        return;
    }
    var ctx = new NativeRuntimeContext {
        Inner = 0isize
    }
    ;
    var spins = 0;
    while (spins <1024)
    {
        let status = PollOnce(header, & ctx);
        if (status == 1u)
        {
            (* header).Flags = (* header).Flags | AsyncFlags.Ready | AsyncFlags.Completed;
            return;
        }
        spins += 1;
    }
}
@export("chic_rt_async_register_future") public static void chic_rt_async_register_future(* mut NativeFutureHeader header) {
    let _ = header;
}
@export("chic_rt_async_spawn") public static void chic_rt_async_spawn(* mut NativeFutureHeader header) {
    unsafe {
        DriveToCompletion(header);
    }
}
@export("chic_rt_async_block_on") public static void chic_rt_async_block_on(* mut NativeFutureHeader header) {
    unsafe {
        DriveToCompletion(header);
    }
}
@export("chic_rt_async_spawn_local") public static uint chic_rt_async_spawn_local(* mut NativeFutureHeader header) {
    chic_rt_async_spawn(header);
    return 1u;
}
@export("chic_rt_async_scope") public static uint chic_rt_async_scope(* mut NativeFutureHeader header) {
    chic_rt_async_spawn(header);
    return 1u;
}
@export("chic_rt_await") public static uint chic_rt_await(* mut NativeRuntimeContext _ctx, * mut NativeFutureHeader header) {
    chic_rt_async_spawn(header);
    return 1u;
}
@export("chic_rt_yield") public static uint chic_rt_yield(* mut NativeRuntimeContext _ctx) {
    return 1u;
}
@export("chic_rt_async_cancel") public static uint chic_rt_async_cancel(* mut NativeFutureHeader header) {
    if (header == null)
    {
        return 1u;
    }
    unsafe {
        (* header).Flags = (* header).Flags | AsyncFlags.Cancelled | AsyncFlags.Completed;
    }
    return 1u;
}
@export("chic_rt_async_task_result") public static uint chic_rt_async_task_result(* mut byte src, * mut byte outPtr, uint outLen) {
    if (src == null || outPtr == null)
    {
        return 0u;
    }
    unsafe {
        var i = 0u;
        while (i <outLen)
        {
            var dst = NativePtr.OffsetMut(outPtr, (isize) i);
            let from = NativePtr.OffsetConst(src, (isize) i);
            * dst = * from;
            i = i + 1u;
        }
    }
    return 1u;
}
@export("chic_rt_async_token_new") public unsafe static * mut bool chic_rt_async_token_new() {
    var handle = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = sizeof(bool)
    }
    ;
    let status = NativeAlloc.AllocZeroed(sizeof(bool), sizeof(bool), out handle);
    if (status != NativeAllocationError.Success)
    {
        return (* mut bool) NativePtr.NullMut();
    }
    return (* mut bool) handle.Pointer;
}
@export("chic_rt_async_token_state") public unsafe static uint chic_rt_async_token_state(* mut bool state_ptr) {
    if (state_ptr == null)
    {
        return 0u;
    }
    return * state_ptr ?1u : 0u;
}
@export("chic_rt_async_token_cancel") public unsafe static uint chic_rt_async_token_cancel(* mut bool state_ptr) {
    if (state_ptr == null)
    {
        return 0u;
    }
    * state_ptr = true;
    return 1u;
}
