namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

public static class ThreadTestSupport
{
    public unsafe static ValueMutPtr AllocBytes(byte b0, byte b1, byte b2) {
        var handle = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 3usize, Alignment = 1usize
        }
        ;
        let status = NativeAlloc.AllocZeroed(3usize, 1usize, out handle);
        if (status == NativeAllocationError.Success)
        {
            * handle.Pointer = b0;
            * NativePtr.OffsetMut(handle.Pointer, 1isize) = b1;
            * NativePtr.OffsetMut(handle.Pointer, 2isize) = b2;
        }
        return handle;
    }
}

testcase Given_thread_spawn_invalid_start_When_executed_Then_thread_spawn_invalid_start()
{
    unsafe {
        let status = chic_rt_thread_spawn((* const ThreadStart) NativePtr.NullConst(), (* mut ThreadHandle) NativePtr.NullMut());
        Assert.That((int) status == (int) ThreadStatus.Invalid).IsTrue();
    }
}

testcase Given_thread_spawn_invalid_context_When_executed_Then_thread_spawn_invalid_context()
{
    unsafe {
        var badCtx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = 1usize, Alignment = 1usize
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(1usize, 1usize, out badCtx);
        var start = new ThreadStart {
            Context = badCtx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            , HasName = false, UseThreadIdName = false,
        }
        ;
        let status = chic_rt_thread_spawn(& start, (* mut ThreadHandle) NativePtr.NullMut());
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Invalid;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(badCtx);
    }
}

testcase Given_thread_spawn_join_and_detach_When_executed_Then_thread_spawn_join_and_detach()
{
    unsafe {
        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        let name = ThreadTestSupport.AllocBytes(116u8, 49u8, 0u8);
        var start = new ThreadStart {
            Context = ctx, Name = name, HasName = true, UseThreadIdName = true,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status = chic_rt_thread_spawn(& start, & handle);
        let joined = chic_rt_thread_join(& handle);
        NativeAlloc.Free(ctx);

        var ctx2 = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc2 = NativeAlloc.AllocZeroed(ctx2.Size, ctx2.Alignment, out ctx2);
        let name2 = ThreadTestSupport.AllocBytes(116u8, 50u8, 0u8);
        var start2 = new ThreadStart {
            Context = ctx2, Name = name2, HasName = true, UseThreadIdName = true,
        }
        ;
        var handle2 = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status2 = chic_rt_thread_spawn(& start2, & handle2);
        let state = (* mut HostThreadState) handle2.Raw;
        if (state != null)
        {
            (* state).completed = true;
        }
        let detached = chic_rt_thread_detach(& handle2);
        NativeAlloc.Free(ctx2);

        chic_rt_thread_sleep_ms(0u64);
        chic_rt_thread_yield();
        chic_rt_thread_spin_wait(10u32);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Success
            && (int) joined == (int) ThreadStatus.Success
            && (int) alloc2 == (int) NativeAllocationError.Success
            && (int) status2 == (int) ThreadStatus.Success
            && (int) detached == (int) ThreadStatus.Success;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_thread_join_and_detach_invalid_handles_When_executed_Then_thread_join_and_detach_invalid_handles()
{
    unsafe {
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let joinStatus = chic_rt_thread_join(& handle);
        let detachStatus = chic_rt_thread_detach(& handle);
        Assert.That((int) joinStatus == (int) ThreadStatus.Invalid
            && (int) detachStatus == (int) ThreadStatus.Invalid).IsTrue();
    }
}

testcase Given_thread_spawn_without_name_When_executed_Then_thread_spawn_without_name()
{
    unsafe {
        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = false,
            UseThreadIdName = false,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status = chic_rt_thread_spawn(& start, & handle);
        let joined = chic_rt_thread_join(& handle);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Success
            && (int) joined == (int) ThreadStatus.Success;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(ctx);
    }
}

testcase Given_thread_spawn_invalid_name_pointer_When_executed_Then_thread_spawn_invalid_name_pointer()
{
    unsafe {
        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = true,
            UseThreadIdName = false,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status = chic_rt_thread_spawn(& start, & handle);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Invalid;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(ctx);
    }
}

testcase Given_thread_spawn_detached_handle_null_When_executed_Then_thread_spawn_detached_handle_null()
{
    unsafe {
        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = false,
            UseThreadIdName = false,
        }
        ;
        let status = chic_rt_thread_spawn(& start, (* mut ThreadHandle) NativePtr.NullMut());
        Assert.That((int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Success).IsTrue();
        chic_rt_thread_sleep_ms(0u64);
    }
}

testcase Given_thread_entry_paths_and_helpers_When_executed_Then_thread_entry_paths_and_helpers()
{
    unsafe {
        let ctxAlign = (usize) __alignof<ChicArc>();
        let ctxSize = (usize) __sizeof<ChicArc>();
        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = ctxSize, Alignment = ctxAlign
        }
        ;
        let ctxStatus = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var badCtx = new ValueMutPtr {
            Pointer = ctx.Pointer, Size = 1usize, Alignment = 1usize
        }
        ;
        let ctxOk = (int) ctxStatus == (int) NativeAllocationError.Success
            && TestContextLayout(ctx)
            && !TestContextLayout(badCtx);

        let name = ThreadTestSupport.AllocBytes(116u8, 101u8, 0u8);
        let stateSize = (usize) __sizeof<HostThreadState>();
        let stateAlign = (usize) __alignof<HostThreadState>();
        var stateMem = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = stateSize, Alignment = stateAlign
        }
        ;
        let stateStatus = NativeAlloc.AllocZeroed(stateSize, stateAlign, out stateMem);
        var state = (* mut HostThreadState) stateMem.Pointer;
        (* state).ctx = ctx;
        (* state).name = name;
        (* state).hasName = true;
        (* state).useThreadIdName = false;
        (* state).detached = false;
        (* state).completed = false;
        TestRunEntry(state);
        let completed = (* state).completed;
        TestFreeName(state);
        (* state).hasName = false;
        TestFreeName(state);
        NativeAlloc.Free(stateMem);

        let name2 = ThreadTestSupport.AllocBytes(116u8, 50u8, 0u8);
        var stateMem2 = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = stateSize, Alignment = stateAlign
        }
        ;
        let stateStatus2 = NativeAlloc.AllocZeroed(stateSize, stateAlign, out stateMem2);
        var state2 = (* mut HostThreadState) stateMem2.Pointer;
        (* state2).ctx = ctx;
        (* state2).name = name2;
        (* state2).hasName = true;
        (* state2).useThreadIdName = true;
        (* state2).detached = true;
        (* state2).completed = false;
        TestRunEntry(state2);

        let ok = ctxOk
            && (int) stateStatus == (int) NativeAllocationError.Success
            && completed
            && (int) stateStatus2 == (int) NativeAllocationError.Success;
        Assert.That(ok).IsTrue();
        NativeAlloc.Free(ctx);
    }
}

testcase Given_thread_failure_and_null_paths_When_executed_Then_thread_failure_and_null_paths()
{
    unsafe {
        TestRunEntry((* mut HostThreadState) NativePtr.NullMut());
        TestFreeName((* mut HostThreadState) NativePtr.NullMut());

        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = false,
            UseThreadIdName = false,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        NativeAlloc.TestFailAllocAfter(0);
        let status = chic_rt_thread_spawn(& start, & handle);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.SpawnFailed
            && NativePtr.IsNull(handle.Raw);
        Assert.That(ok).IsTrue();
        NativeAlloc.TestReset();
        NativeAlloc.Free(ctx);
    }
}

testcase Given_thread_fake_thread_results_create_failure_When_executed_Then_thread_fake_thread_results_create_failure()
{
    unsafe {
        TestUseFakeThreads(true);
        TestSetFakeCreateResult(1);
        TestSetFakeJoinResult(0);
        TestSetFakeDetachResult(0);

        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        let name = ThreadTestSupport.AllocBytes(102u8, 49u8, 0u8);
        var start = new ThreadStart {
            Context = ctx, Name = name, HasName = true, UseThreadIdName = true,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let failed = chic_rt_thread_spawn(& start, & handle);
        let firstOk = (int) alloc == (int) NativeAllocationError.Success
            && (int) failed == (int) ThreadStatus.SpawnFailed
            && NativePtr.IsNull(handle.Raw);
        NativeAlloc.Free(ctx);

        TestSetFakeDetachResult(0);
        TestUseFakeThreads(false);
        Assert.That(firstOk).IsTrue();
    }
}

testcase Given_thread_fake_thread_results_join_failure_When_executed_Then_thread_fake_thread_results_join_failure()
{
    unsafe {
        TestUseFakeThreads(true);
        TestSetFakeCreateResult(0);
        TestSetFakeJoinResult(1);
        TestSetFakeDetachResult(0);

        var ctx2 = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc2 = NativeAlloc.AllocZeroed(ctx2.Size, ctx2.Alignment, out ctx2);
        var start2 = new ThreadStart {
            Context = ctx2,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            , HasName = false, UseThreadIdName = false,
        }
        ;
        var handle2 = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status2 = chic_rt_thread_spawn(& start2, & handle2);
        let joinStatus = chic_rt_thread_join(& handle2);
        let secondOk = (int) alloc2 == (int) NativeAllocationError.Success
            && (int) status2 == (int) ThreadStatus.Success
            && (int) joinStatus == (int) ThreadStatus.SpawnFailed
            && NativePtr.IsNull(handle2.Raw);
        NativeAlloc.Free(ctx2);

        TestSetFakeJoinResult(0);
        TestUseFakeThreads(false);
        Assert.That(secondOk).IsTrue();
    }
}

testcase Given_thread_fake_thread_results_detach_failure_When_executed_Then_thread_fake_thread_results_detach_failure()
{
    unsafe {
        TestUseFakeThreads(true);
        TestSetFakeCreateResult(0);
        TestSetFakeJoinResult(0);
        TestSetFakeDetachResult(1);

        var ctx3 = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc3 = NativeAlloc.AllocZeroed(ctx3.Size, ctx3.Alignment, out ctx3);
        var start3 = new ThreadStart {
            Context = ctx3,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            , HasName = false, UseThreadIdName = false,
        }
        ;
        var handle3 = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status3 = chic_rt_thread_spawn(& start3, & handle3);
        let detachStatus = chic_rt_thread_detach(& handle3);
        let thirdOk = (int) alloc3 == (int) NativeAllocationError.Success
            && (int) status3 == (int) ThreadStatus.Success
            && (int) detachStatus == (int) ThreadStatus.SpawnFailed
            && NativePtr.IsNull(handle3.Raw);
        NativeAlloc.Free(ctx3);

        TestSetFakeDetachResult(0);
        TestUseFakeThreads(false);
        Assert.That(thirdOk).IsTrue();
    }
}

testcase Given_thread_fake_spawn_detached_handle_null_When_executed_Then_thread_fake_spawn_detached_handle_null()
{
    unsafe {
        TestUseFakeThreads(true);
        TestSetFakeCreateResult(0);

        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = false,
            UseThreadIdName = false,
        }
        ;
        let status = chic_rt_thread_spawn(& start, (* mut ThreadHandle) NativePtr.NullMut());
        NativeAlloc.Free(ctx);
        TestUseFakeThreads(false);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Success;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_thread_fake_detach_incomplete_state_When_executed_Then_thread_fake_detach_incomplete_state()
{
    unsafe {
        TestUseFakeThreads(true);
        TestSetFakeCreateResult(0);
        TestSetFakeDetachResult(0);

        var ctx = new ValueMutPtr {
            Pointer = NativePtr.NullMut(), Size = sizeof(ChicArc), Alignment = __alignof <ChicArc >()
        }
        ;
        let alloc = NativeAlloc.AllocZeroed(ctx.Size, ctx.Alignment, out ctx);
        var start = new ThreadStart {
            Context = ctx,
            Name = new ValueMutPtr {
                Pointer = NativePtr.NullMut(), Size = 0usize, Alignment = 1usize
            }
            ,
            HasName = false,
            UseThreadIdName = false,
        }
        ;
        var handle = new ThreadHandle {
            Raw = NativePtr.NullMut()
        }
        ;
        let status = chic_rt_thread_spawn(& start, & handle);
        let raw = handle.Raw;
        let detached = chic_rt_thread_detach(& handle);
        if (!NativePtr.IsNull(raw))
        {
            NativeAlloc.Free(new ValueMutPtr {
                Pointer = raw, Size = (usize) __sizeof<HostThreadState>(), Alignment = (usize) __alignof<HostThreadState>()
            }
            );
        }
        NativeAlloc.Free(ctx);
        TestUseFakeThreads(false);
        let ok = (int) alloc == (int) NativeAllocationError.Success
            && (int) status == (int) ThreadStatus.Success
            && (int) detached == (int) ThreadStatus.Success;
        Assert.That(ok).IsTrue();
    }
}
