namespace Std.Runtime.Native.Tests;
import Std.Runtime.Native;
import Std.Runtime.Native.Testing;

testcase Given_async_spawn_sets_flags_When_executed_Then_async_spawn_sets_flags()
{
    unsafe {
        var header = new NativeFutureHeader {
            StatePointer = 0, VTablePointer = 0, ExecutorContext = 0, Flags = 0u
        }
        ;
        chic_rt_async_spawn(& header);
        let spawned = (header.Flags & 0x0000_0001u) != 0u && (header.Flags & 0x0000_0002u) != 0u;
        Assert.That(spawned).IsTrue();
    }
}

testcase Given_async_cancel_sets_flag_When_executed_Then_async_cancel_sets_flag()
{
    unsafe {
        var header = new NativeFutureHeader {
            StatePointer = 0, VTablePointer = 0, ExecutorContext = 0, Flags = 0u
        }
        ;
        chic_rt_async_spawn(& header);
        chic_rt_async_cancel(& header);
        let canceled = (header.Flags & 0x0000_0004u) != 0u;
        chic_rt_async_block_on(& header);
        chic_rt_async_spawn_local(& header);
        chic_rt_async_scope(& header);
        chic_rt_await(new NativeRuntimeContext {
            Inner = 0isize
        }
        , & header);
        chic_rt_yield(new NativeRuntimeContext {
            Inner = 0isize
        }
        );
        Assert.That(canceled).IsTrue();
    }
}

testcase Given_async_task_result_and_token_When_executed_Then_async_task_result_and_token()
{
    unsafe {
        var source = MemoryRuntime.chic_rt_alloc(3usize, 1usize);
        var dest = MemoryRuntime.chic_rt_alloc(3usize, 1usize);
        let ptr0 = NativePtr.OffsetMut(source.Pointer, 0isize);
        let ptr1 = NativePtr.OffsetMut(source.Pointer, 1isize);
        let ptr2 = NativePtr.OffsetMut(source.Pointer, 2isize);
        * ptr0 = 1u8;
        * ptr1 = 2u8;
        * ptr2 = 3u8;
        let result = chic_rt_async_task_result(source.Pointer, dest.Pointer, 3u);
        let first = NativePtr.ReadByteMut(dest.Pointer);
        let second = NativePtr.ReadByteMut(NativePtr.OffsetMut(dest.Pointer, 1isize));
        let third = NativePtr.ReadByteMut(NativePtr.OffsetMut(dest.Pointer, 2isize));
        MemoryRuntime.chic_rt_free(source);
        MemoryRuntime.chic_rt_free(dest);
        let token = chic_rt_async_token_new();
        let initialState = chic_rt_async_token_state(token);
        let cancelState = chic_rt_async_token_cancel(token);
        let finalState = chic_rt_async_token_state(token);
        let ok = result == 1u
            && first == 1u8
            && second == 2u8
            && third == 3u8
            && initialState == 0u
            && cancelState == 1u
            && finalState == 1u;
        Assert.That(ok).IsTrue();
    }
}

testcase Given_async_null_inputs_and_token_state_When_executed_Then_async_null_inputs_and_token_state()
{
    unsafe {
        chic_rt_async_spawn((* mut NativeFutureHeader) NativePtr.NullMut());
        chic_rt_async_block_on((* mut NativeFutureHeader) NativePtr.NullMut());
        let cancelResult = chic_rt_async_cancel((* mut NativeFutureHeader) NativePtr.NullMut());

        let nullResult = chic_rt_async_task_result(NativePtr.NullMut(), NativePtr.NullMut(), 4u);
        var header = new NativeFutureHeader {
            StatePointer = 0, VTablePointer = 0, ExecutorContext = 0, Flags = 0u
        }
        ;
        let spawnLocal = chic_rt_async_spawn_local(& header);
        let scopeResult = chic_rt_async_scope(& header);
        let awaitResult = chic_rt_await(new NativeRuntimeContext {
            Inner = 0isize
        }
        , & header);
        let yieldResult = chic_rt_yield(new NativeRuntimeContext {
            Inner = 0isize
        }
        );

        let tokenState = chic_rt_async_token_state((* mut bool) NativePtr.NullMut());
        let tokenCancel = chic_rt_async_token_cancel((* mut bool) NativePtr.NullMut());
        let ok = cancelResult == 1u
            && nullResult == 0u
            && spawnLocal == 1u
            && scopeResult == 1u
            && awaitResult == 1u
            && yieldResult == 1u
            && tokenState == 0u
            && tokenCancel == 0u;
        Assert.That(ok).IsTrue();
    }
}
