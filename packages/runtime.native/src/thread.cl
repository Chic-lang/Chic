namespace Std.Runtime.Native;
@repr(c) public struct ThreadStart
{
    public ValueMutPtr Context;
    public ValueMutPtr Name;
    public bool HasName;
    public bool UseThreadIdName;
}
@repr(c) public struct ThreadHandle
{
    public * mut @expose_address byte Raw;
}
@repr(c) public enum ThreadStatus
{
    Success = 0, NotSupported = 1, Invalid = 2, SpawnFailed = 3,
}
@repr(c) internal struct HostThreadState
{
    public usize thread;
    public ValueMutPtr ctx;
    public ValueMutPtr name;
    public bool hasName;
    public bool useThreadIdName;
    public bool nameFreed;
    public bool detached;
    public bool completed;
}
@repr(c) internal struct Timespec
{
    public i64 tv_sec;
    public i64 tv_nsec;
}
@extern("C") private unsafe static extern int pthread_create(* mut usize thread, * const @readonly @expose_address byte attrs,
fn @extern("C")(* mut @expose_address byte) -> * mut @expose_address byte entry, * mut @expose_address byte arg);
@extern("C") private unsafe static extern int pthread_join(usize thread, * mut @expose_address byte retval);
@extern("C") private unsafe static extern int pthread_detach(usize thread);
@extern("C") private unsafe static extern int sched_yield();
@extern("C") private unsafe static extern int nanosleep(* const @readonly @expose_address Timespec req, * mut @expose_address Timespec rem);
	// Linux exposes the two-argument signature. We gate usage at runtime so non-Linux targets
	// avoid invoking an incompatible ABI and simply treat naming as a no-op.
	@extern("C") private unsafe static extern int pthread_setname_np(usize thread, * const @readonly @expose_address byte name);
	// Chic-owned thread callbacks implemented in Std.Platform.Thread.RuntimeCallbacks.
	//
	// The native runtime package is built and tested in isolation, so we provide weak
	// no-op fallbacks here. When `std.platform` is linked, it exports strong definitions
	// that override these stubs.
	@extern("C") @weak @export("chic_thread_invoke") public static void chic_thread_invoke(ValueMutPtr context) {
	    (void) context;
	}
	@extern("C") @weak @export("chic_thread_drop") public static void chic_thread_drop(ValueMutPtr context) {
	    (void) context;
	}
internal static class ThreadTestState
{
    public static bool UseFakeThreads = false;
    public static int FakeCreateResult = 0;
    public static int FakeJoinResult = 0;
    public static int FakeDetachResult = 0;
}
private static bool ContextLayoutValid(ValueMutPtr context) {
    let expectedSize = sizeof(ChicArc);
    let expectedAlign = __alignof <ChicArc >();
    if (NativePtr.IsNull (context.Pointer))
    {
        return false;
    }
    return context.Size == expectedSize && context.Alignment == expectedAlign;
}
private unsafe static * mut * mut @expose_address byte HandleRawSlot(* mut ThreadHandle handle) {
    return(* mut * mut @expose_address byte) handle;
}
private unsafe static * mut @expose_address byte ReadHandleRaw(* mut ThreadHandle handle) {
    return handle == null ?NativePtr.NullMut() : * HandleRawSlot(handle);
}
private unsafe static void WriteHandleRaw(* mut ThreadHandle handle, * mut @expose_address byte value) {
    if (handle != null)
    {
        * HandleRawSlot(handle) = value;
    }
}
private unsafe static void FreeName(* mut HostThreadState state) {
    var statePtr = state;
    if (statePtr == null)
    {
        return;
    }
    if ( (* statePtr).nameFreed || ! (* statePtr).hasName)
    {
        return;
    }
    if (! NativePtr.IsNull ( (* statePtr).name.Pointer))
    {
        NativeAlloc.Free((* statePtr).name);
    }
    (* statePtr).name.Pointer = NativePtr.NullMut();
    (* statePtr).nameFreed = true;
    (* statePtr).hasName = false;
}
private unsafe static void TrySetThreadName(* mut HostThreadState state) {
    var statePtr = state;
    if (statePtr == null)
    {
        return;
    }
    if (! (* statePtr).hasName || NativePtr.IsNull ( (* statePtr).name.Pointer))
    {
        return;
    }
    if (! (* statePtr).useThreadIdName)
    {
        return;
    }
    // Best-effort: some platforms may reject longer names. We ignore the return
    // value but still free the buffer.
    let namePtr = (* const @readonly @expose_address byte)(* statePtr).name.Pointer;
    let _ = pthread_setname_np((* statePtr).thread, namePtr);
}
@export("chic_rt_thread_spawn") public unsafe static ThreadStatus chic_rt_thread_spawn(* const ThreadStart start,
* mut ThreadHandle handle) {
    if (start == null)
    {
        return ThreadStatus.Invalid;
    }
    let ctx = (* start).Context;
    if (! ContextLayoutValid (ctx))
    {
        if ( (* start).HasName && ! NativePtr.IsNull ( (* start).Name.Pointer))
        {
            NativeAlloc.Free((* start).Name);
        }
        return ThreadStatus.Invalid;
    }
    if ( (* start).HasName && NativePtr.IsNull ( (* start).Name.Pointer))
    {
        chic_thread_drop(ctx);
        return ThreadStatus.Invalid;
    }
    let stateSize = sizeof(HostThreadState);
    let stateAlign = __alignof <HostThreadState >();
    var stateMem = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = stateSize, Alignment = stateAlign
    }
    ;
    if (NativeAlloc.AllocZeroed (stateSize, stateAlign, out stateMem) != NativeAllocationError.Success) {
        chic_thread_drop(ctx);
        WriteHandleRaw(handle, NativePtr.NullMut());
        return ThreadStatus.SpawnFailed;
    }
    var statePtr = (* mut HostThreadState) stateMem.Pointer;
    (* statePtr).thread = 0;
    (* statePtr).ctx = ctx;
    (* statePtr).name = (* start).Name;
    (* statePtr).hasName = (* start).HasName && ! NativePtr.IsNull((* start).Name.Pointer);
    (* statePtr).useThreadIdName = (* start).UseThreadIdName;
    (* statePtr).nameFreed = false;
    (* statePtr).detached = handle == null;
    (* statePtr).completed = false;
    if (ThreadTestState.UseFakeThreads)
    {
        if (ThreadTestState.FakeCreateResult != 0)
        {
            chic_thread_drop(ctx);
            FreeName(statePtr);
            NativeAlloc.Free(stateMem);
            WriteHandleRaw(handle, NativePtr.NullMut());
            return ThreadStatus.SpawnFailed;
        }
        (* statePtr).thread = 1usize;
        if (handle == null)
        {
            let _ = ThreadEntry(stateMem.Pointer);
        }
        else
        {
            WriteHandleRaw(handle, stateMem.Pointer);
        }
        return ThreadStatus.Success;
    }
    let threadPtr = (* mut usize) stateMem.Pointer;
    let rc = pthread_create(threadPtr, NativePtr.NullConst(), ThreadEntry, stateMem.Pointer);
    if (rc != 0)
    {
        chic_thread_drop(ctx);
        FreeName(statePtr);
        NativeAlloc.Free(stateMem);
        WriteHandleRaw(handle, NativePtr.NullMut());
        return ThreadStatus.SpawnFailed;
    }
    if (handle == null)
    {
        let _ = pthread_detach((* statePtr).thread);
    }
    else
    {
        WriteHandleRaw(handle, stateMem.Pointer);
    }
    return ThreadStatus.Success;
}
@export("chic_rt_thread_join") public unsafe static ThreadStatus chic_rt_thread_join(* mut ThreadHandle handle) {
    let rawHandle = ReadHandleRaw(handle);
    if (NativePtr.IsNull (rawHandle))
    {
        return ThreadStatus.Invalid;
    }
    var statePtr = (* mut HostThreadState) rawHandle;
    let thread = (* statePtr).thread;
    let rc = ThreadTestState.UseFakeThreads ? ThreadTestState.FakeJoinResult : pthread_join(thread, NativePtr.NullMut());
    FreeName(statePtr);
    (* statePtr).thread = 0;
    NativeAlloc.Free(new ValueMutPtr {
        Pointer = rawHandle, Size = sizeof(HostThreadState), Alignment = __alignof <HostThreadState >()
    }
    );
    WriteHandleRaw(handle, NativePtr.NullMut());
    return rc == 0 ?ThreadStatus.Success : ThreadStatus.SpawnFailed;
}
@export("chic_rt_thread_detach") public unsafe static ThreadStatus chic_rt_thread_detach(* mut ThreadHandle handle) {
    let rawHandle = ReadHandleRaw(handle);
    if (NativePtr.IsNull (rawHandle))
    {
        return ThreadStatus.Invalid;
    }
    var statePtr = (* mut HostThreadState) rawHandle;
    (* statePtr).detached = true;
    let completed = (* statePtr).completed;
    let thread = (* statePtr).thread;
    let rc = ThreadTestState.UseFakeThreads ? ThreadTestState.FakeDetachResult : pthread_detach(thread);
    if (completed)
    {
        FreeName(statePtr);
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = rawHandle, Size = sizeof(HostThreadState), Alignment = __alignof <HostThreadState >(),
        }
        );
    }
    WriteHandleRaw(handle, NativePtr.NullMut());
    return rc == 0 ?ThreadStatus.Success : ThreadStatus.SpawnFailed;
}
@export("chic_rt_thread_sleep_ms") public unsafe static void chic_rt_thread_sleep_ms(u64 millis) {
    var ts = new Timespec {
        tv_sec = (i64)(millis / 1000u64), tv_nsec = (i64)((millis % 1000u64) * 1000000u64),
    }
    ;
    let _ = nanosleep(& ts, (* mut @expose_address Timespec) NativePtr.NullMut());
}
@export("chic_rt_thread_yield") public unsafe static void chic_rt_thread_yield() {
    let _ = sched_yield();
}
@export("chic_rt_thread_spin_wait") public static void chic_rt_thread_spin_wait(u32 iterations) {
    let _ = iterations;
}
@extern("C") private unsafe static * mut @expose_address byte ThreadEntry(* mut @expose_address byte arg) {
    if (NativePtr.IsNull (arg))
    {
        return NativePtr.NullMut();
    }
    var state = (* mut HostThreadState) arg;
    TrySetThreadName(state);
    chic_thread_invoke((* state).ctx);
    (* state).completed = true;
    if ( (* state).detached)
    {
        FreeName(state);
        NativeAlloc.Free(new ValueMutPtr {
            Pointer = (* mut @expose_address byte) state, Size = sizeof(HostThreadState), Alignment = __alignof <HostThreadState >(),
        }
        );
    }
    return NativePtr.NullMut();
}

public unsafe static void TestRunEntry(* mut HostThreadState state) {
    let _ = ThreadEntry((* mut @expose_address byte) state);
}

public unsafe static void TestFreeName(* mut HostThreadState state) {
    FreeName(state);
}

public static bool TestContextLayout(ValueMutPtr context) {
    return ContextLayoutValid(context);
}

public static void TestUseFakeThreads(bool value) {
    ThreadTestState.UseFakeThreads = value;
}

public static void TestSetFakeCreateResult(int value) {
    ThreadTestState.FakeCreateResult = value;
}

public static void TestSetFakeJoinResult(int value) {
    ThreadTestState.FakeJoinResult = value;
}

public static void TestSetFakeDetachResult(int value) {
    ThreadTestState.FakeDetachResult = value;
}

public unsafe static bool ThreadTestCoverageSweep() {
    var ok = true;
    var dummyArc = new ChicArc {
        header = null
    }
    ;
    var dummyPtr = new ValueMutPtr {
        Pointer = NativePtr.AsByteMut(& dummyArc), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
    }
    ;
    var invalidLayout = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 0usize
    }
    ;
    ok = ok && ContextLayoutValid(dummyPtr);
    ok = ok && !ContextLayoutValid(invalidLayout);

    var tmpHandle = new ThreadHandle {
        Raw = NativePtr.NullMut()
    }
    ;
    WriteHandleRaw(& tmpHandle, dummyPtr.Pointer);
    ok = ok && !NativePtr.IsNull(ReadHandleRaw(& tmpHandle));
    WriteHandleRaw(& tmpHandle, NativePtr.NullMut());
    ok = ok && NativePtr.IsNull(ReadHandleRaw(& tmpHandle));

    let nullStatus = chic_rt_thread_spawn((* const ThreadStart) NativePtr.NullConst(),
    (* mut ThreadHandle) NativePtr.NullMut());
    ok = ok && (int) nullStatus == (int) ThreadStatus.Invalid;

    var invalidName = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 3usize, Alignment = 1usize
    }
    ;
    let invalidNameAlloc = NativeAlloc.AllocZeroed(3usize, 1usize, out invalidName);
    var invalidStart = new ThreadStart {
        Context = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        , Name = invalidName, HasName = true, UseThreadIdName = false,
    }
    ;
    let invalidStatus = chic_rt_thread_spawn(& invalidStart, (* mut ThreadHandle) NativePtr.NullMut());
    ok = ok && (int) invalidNameAlloc == (int) NativeAllocationError.Success
        && (int) invalidStatus == (int) ThreadStatus.Invalid;

    var ctxInvalidName = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
    }
    ;
    let ctxInvalidAlloc = NativeAlloc.AllocZeroed(ctxInvalidName.Size, ctxInvalidName.Alignment, out ctxInvalidName);
    var badNameStart = new ThreadStart {
        Context = ctxInvalidName,
        Name = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
        }
        , HasName = true, UseThreadIdName = false,
    }
    ;
    let badNameStatus = chic_rt_thread_spawn(& badNameStart, (* mut ThreadHandle) NativePtr.NullMut());
    ok = ok && (int) ctxInvalidAlloc == (int) NativeAllocationError.Success
        && (int) badNameStatus == (int) ThreadStatus.Invalid;

    TestUseFakeThreads(true);
    TestSetFakeCreateResult(1);
    TestSetFakeJoinResult(0);
    TestSetFakeDetachResult(0);

    var ctxFail = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
    }
    ;
    let ctxFailAlloc = NativeAlloc.AllocZeroed(ctxFail.Size, ctxFail.Alignment, out ctxFail);
    var failName = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 3usize, Alignment = 1usize
    }
    ;
    let failNameAlloc = NativeAlloc.AllocZeroed(3usize, 1usize, out failName);
    var failStart = new ThreadStart {
        Context = ctxFail, Name = failName, HasName = true, UseThreadIdName = false,
    }
    ;
    var failHandle = new ThreadHandle {
        Raw = NativePtr.NullMut()
    }
    ;
    let failStatus = chic_rt_thread_spawn(& failStart, & failHandle);
    ok = ok && (int) ctxFailAlloc == (int) NativeAllocationError.Success
        && (int) failNameAlloc == (int) NativeAllocationError.Success
        && (int) failStatus == (int) ThreadStatus.SpawnFailed
        && NativePtr.IsNull(failHandle.Raw);

    TestSetFakeCreateResult(0);
    var ctxDetach = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
    }
    ;
    let ctxDetachAlloc = NativeAlloc.AllocZeroed(ctxDetach.Size, ctxDetach.Alignment, out ctxDetach);
    var detachName = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 4usize, Alignment = 1usize
    }
    ;
    let detachNameAlloc = NativeAlloc.AllocZeroed(4usize, 1usize, out detachName);
    var detachStart = new ThreadStart {
        Context = ctxDetach, Name = detachName, HasName = true, UseThreadIdName = true,
    }
    ;
    var detachHandle = new ThreadHandle {
        Raw = NativePtr.NullMut()
    }
    ;
    let detachStatus = chic_rt_thread_spawn(& detachStart, & detachHandle);
    let detachRaw = detachHandle.Raw;
    if (! NativePtr.IsNull (detachRaw))
    {
        (* (* mut HostThreadState) detachRaw).completed = true;
    }
    let detachResult = chic_rt_thread_detach(& detachHandle);
    if ((int) ctxDetachAlloc == (int) NativeAllocationError.Success)
    {
        NativeAlloc.Free(ctxDetach);
    }
    ok = ok && (int) ctxDetachAlloc == (int) NativeAllocationError.Success
        && (int) detachNameAlloc == (int) NativeAllocationError.Success
        && (int) detachStatus == (int) ThreadStatus.Success
        && (int) detachResult == (int) ThreadStatus.Success;

    TestUseFakeThreads(false);
    var ctxReal = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
    }
    ;
    let ctxRealAlloc = NativeAlloc.AllocZeroed(ctxReal.Size, ctxReal.Alignment, out ctxReal);
    var realName = new ValueMutPtr {
        Pointer = NativePtr.NullMut(), Size = 5usize, Alignment = 1usize
    }
    ;
    let realNameAlloc = NativeAlloc.AllocZeroed(5usize, 1usize, out realName);
    var realStart = new ThreadStart {
        Context = ctxReal, Name = realName, HasName = true, UseThreadIdName = true,
    }
    ;
    var realHandle = new ThreadHandle {
        Raw = NativePtr.NullMut()
    }
    ;
    let realStatus = chic_rt_thread_spawn(& realStart, & realHandle);
    let joinStatus = chic_rt_thread_join(& realHandle);
    if ((int) ctxRealAlloc == (int) NativeAllocationError.Success)
    {
        NativeAlloc.Free(ctxReal);
    }
    ok = ok && (int) ctxRealAlloc == (int) NativeAllocationError.Success
        && (int) realNameAlloc == (int) NativeAllocationError.Success
        && (int) realStatus == (int) ThreadStatus.Success
        && (int) joinStatus == (int) ThreadStatus.Success;

    return ok;
}
