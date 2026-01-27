// Minimal async surface used by integration tests without loading the full standard library.
// Mirrors `packages/std/src/async.ch` layouts so the native runtime can introspect task/future headers.
@allow(dead_code)
@allow(unused_param)
@allow(unreachable_code)
namespace Std.Async;

import Std.Runtime;
import Std.Numeric;

internal static class RuntimeIntrinsics
{
    @extern("C")
    public static extern void chic_rt_async_register_future(*mut FutureHeader header);

    @extern("C")
    public static extern void chic_rt_async_spawn(*mut FutureHeader header);

    @extern("C")
    public static extern void chic_rt_async_block_on(*mut FutureHeader header);

    @extern("C")
    public static extern uint chic_rt_async_scope(*mut FutureHeader header);

    @extern("C")
    public static extern uint chic_rt_async_spawn_local(*mut FutureHeader header);

    @extern("C")
    public static extern uint chic_rt_await(
        *mut RuntimeContext context,
        *mut FutureHeader awaited
    );

    @extern("C")
    public static extern uint chic_rt_yield(*mut RuntimeContext context);

    @extern("C")
    public static extern uint chic_rt_async_cancel(*mut FutureHeader header);

    @extern("C")
    public static extern uint chic_rt_async_task_result(*mut byte src, *mut byte outPtr, uint outLen);

    @extern("C")
    public static extern uint chic_rt_async_token_state(*mut bool state_ptr);

    @extern("C")
    public static extern *mut bool chic_rt_async_token_new();

    @extern("C")
    public static extern uint chic_rt_async_token_cancel(*mut bool state_ptr);

}

internal static class FutureFlags
{
    public const uint Pending = 0x00000000u;
    public const uint Ready = 0x00000001u;
    public const uint Completed = 0x00000002u;
    public const uint Cancelled = 0x00000004u;
    public const uint Faulted = 0x80000000u;
}

internal struct FutureHeader
{
    internal isize StatePointer;
    internal isize VTablePointer;
    internal isize ExecutorContext;
    internal uint Flags;
}

internal struct FutureVTable
{
    internal isize PollFunction;
    internal isize DropFunction;
}

public struct Future
{
    internal FutureHeader Header;
}

public struct Future<T>
{
    internal FutureHeader Header;
    internal bool Completed;
    internal T Result;
}

public class Task
{
    internal FutureHeader Header;
    internal uint Flags;

    public static void SpawnLocal(Task task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_spawn_local(RuntimeExports.TaskHeader(task));
        }
    }

    public static void Scope(Task task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_scope(RuntimeExports.TaskHeader(task));
        }
    }
}

public class Task<T> : Task
{
    internal Future<T> InnerFuture;

    public static T Scope(Task<T> task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_scope(RuntimeExports.TaskHeader(task));
        }
        return task.InnerFuture.Result;
    }
}

internal static class RuntimeExports
{
    @export("chic_rt_async_task_header")
    public static *mut FutureHeader TaskHeader(Task task)
    {
        return &task.Header;
    }

    // These mirror the real stdlib result exports so codegen can load task payloads.
    @export("chic_rt_async_task_bool_result")
    public static bool TaskBoolResult(Task<bool> task)
    {
        return task.InnerFuture.Result;
    }

    @export("chic_rt_async_task_int_result")
    public static int TaskIntResult(Task<int> task)
    {
        return task.InnerFuture.Result;
    }
}

public struct CancellationTokenSource
{
    public bool Flag;

    @allow(unreachable_code)
    public static CancellationTokenSource Create()
    {
        var source = new CancellationTokenSource { Flag = false };
        return source;
    }

    public CancellationToken Token()
    {
        var token = new CancellationToken { IsCanceled = Flag };
        return token;
    }

    public void Cancel()
    {
        Flag = true;
    }

    @allow(unreachable_code)
    public bool IsCanceled
    {
        get
        {
            return Flag;
        }
    }

    public int StubMarker()
    {
        return 1234;
    }
}

public struct CancellationToken
{
    public bool IsCanceled;

    @allow(unreachable_code)
    public bool IsCancellationRequested()
    {
        return IsCanceled;
    }
}

public static class Runtime
{
    public static void Register(Task task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_register_future(RuntimeExports.TaskHeader(task));
        }
    }

    public static void RegisterAndBlockOn(Task task)
    {
        unsafe
        {
            let header = RuntimeExports.TaskHeader(task);
            RuntimeIntrinsics.chic_rt_async_register_future(header);
            RuntimeIntrinsics.chic_rt_async_block_on(header);
        }
    }

    public static void Spawn(Task task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_spawn(RuntimeExports.TaskHeader(task));
        }
    }

    public static void BlockOn(Task task)
    {
        unsafe
        {
            RuntimeIntrinsics.chic_rt_async_block_on(RuntimeExports.TaskHeader(task));
        }
    }

    public static int TaskIntResult(Task<int> task)
    {
        return task.InnerFuture.Result;
    }

    public static bool Cancel(Task task)
    {
        var status = RuntimeIntrinsics.chic_rt_async_cancel(RuntimeExports.TaskHeader(task));
        return status == FutureFlags.Ready;
    }

    public static bool CancelInt(ref Task<int> task)
    {
        var cancelled = FutureFlags.Ready | FutureFlags.Completed | FutureFlags.Cancelled;
        task.Header.Flags = cancelled;
        task.Flags = cancelled;
        task.InnerFuture.Header.Flags = cancelled;
        task.InnerFuture.Completed = true;
        task.InnerFuture.Result = 0;
        RuntimeIntrinsics.chic_rt_async_cancel(RuntimeExports.TaskHeader(task));
        return true;
    }
}

internal struct RuntimeContext
{
    internal isize Inner;
}
