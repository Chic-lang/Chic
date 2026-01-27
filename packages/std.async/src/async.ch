namespace Std.Async;
import Std.Core;
internal static class RuntimeIntrinsics
{
    @extern("C") public static extern void chic_rt_async_register_future(* mut FutureHeader header);
    @extern("C") public static extern void chic_rt_async_spawn(* mut FutureHeader header);
    @extern("C") public static extern void chic_rt_async_block_on(* mut FutureHeader header);
    @extern("C") public static extern uint chic_rt_async_scope(* mut FutureHeader header);
    @extern("C") public static extern uint chic_rt_async_spawn_local(* mut FutureHeader header);
    @extern("C") public static extern uint chic_rt_await(* mut RuntimeContext context, * mut FutureHeader awaited);
    @extern("C") public static extern uint chic_rt_yield(* mut RuntimeContext context);
    @extern("C") public static extern uint chic_rt_async_cancel(* mut FutureHeader header);
    @extern("C") public static extern uint chic_rt_async_task_result(* mut byte src, * mut byte outPtr, uint outLen);
    @extern("C") public static extern uint chic_rt_async_token_state(* mut bool state_ptr);
    @extern("C") public static extern * mut bool chic_rt_async_token_new();
    @extern("C") public static extern uint chic_rt_async_token_cancel(* mut bool state_ptr);
}
/// <summary>
/// Flag bits used by the runtime to communicate future/task state.
/// </summary>
internal static class FutureFlags
{
    public const uint Pending = 0x00000000u;
    public const uint Ready = 0x00000001u;
    public const uint Completed = 0x00000002u;
    public const uint Cancelled = 0x00000004u;
    public const uint Faulted = 0x80000000u;
}
/// <summary>
/// Runtime metadata associated with a Chic future state machine.
/// </summary>
@repr(c) internal struct FutureHeader
{
    /// <summary>
    /// Pointer to the state machine instance that owns this header. The value is opaque to user
    /// code but allows the runtime to resume or destroy the frame.
    /// </summary>
    internal isize StatePointer;
    /// <summary>
    /// Pointer to the runtime vtable describing how to poll and drop the state machine.
    /// </summary>
    internal isize VTablePointer;
    /// <summary>
    /// Executor-defined context pointer (waker, scheduler state, etc.).
    /// </summary>
    internal isize ExecutorContext;
    /// <summary>
    /// Current runtime flags (see <see cref="FutureFlags"/>).
    /// </summary>
    internal uint Flags;
}
/// <summary>
/// Function pointers supplied by the compiler for each lowered future.
/// </summary>
@repr(c) internal struct FutureVTable
{
    internal isize PollFunction;
    internal isize DropFunction;
}
/// <summary>
/// Untyped future handle. Generic futures embed this header as their first field so the runtime
/// can downcast when necessary.
/// </summary>
@repr(c) public struct Future
{
    internal FutureHeader Header;
}
/// <summary>
/// Represents an awaitable Chic future that yields a value of type <typeparamref name="T"/>.
/// The runtime stores the eventual result inside the struct once the state machine reports
/// completion via its vtable.
/// </summary>
@repr(c) public struct Future <T >
{
    internal FutureHeader Header;
    internal bool Completed;
    internal T Result;
    public bool IsCompleted() {
        return(Header.Flags & FutureFlags.Completed) != 0u;
    }
}
/// <summary>
/// Base task object tracked by the async scheduler. Individual tasks embed a future header so the
/// runtime can poll/drop the underlying state machine.
/// </summary>
@repr(c) public class Task
{
    internal FutureHeader Header;
    internal uint Flags;
    public static void SpawnLocal(Task task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_spawn_local(RuntimeExports.TaskHeader(task));
        }
    }
    public static void Scope(Task task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_scope(RuntimeExports.TaskHeader(task));
        }
    }
}
/// <summary>
/// Strongly-typed task that wraps a <see cref="Future{T}"/> and exposes the eventual result.
/// </summary>
@repr(c) public class Task <T >: Task
{
    internal Future <T >InnerFuture;
    public static Task <T >SpawnLocal(Task <T >task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_spawn_local(RuntimeExports.TaskHeader(task));
        }
        return task;
    }
    public static T Scope(Task <T >task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_scope(RuntimeExports.TaskHeader(task));
        }
        return task.InnerFuture.Result;
    }
}
public struct CancellationTokenSource
{
    // Runtime-owned cancellation state pointer; kept public so layout is retained in metadata.
    public * mut bool StatePtr;
    public static CancellationTokenSource Create() {
        var source = CoreIntrinsics.DefaultValue <CancellationTokenSource >();
        unsafe {
            source.StatePtr = RuntimeIntrinsics.chic_rt_async_token_new();
        }
        return source;
    }
    public CancellationToken Token() {
        var token = CoreIntrinsics.DefaultValue <CancellationToken >();
        unsafe {
            token.IsCanceledPtr = StatePtr;
        }
        return token;
    }
    public void Cancel() {
        unsafe {
            if (StatePtr == null)
            {
                return;
            }
            RuntimeIntrinsics.chic_rt_async_token_cancel(StatePtr);
        }
    }
    public bool IsCanceled {
        get {
            unsafe {
                if (StatePtr == null)
                {
                    return false;
                }
                let status = RuntimeIntrinsics.chic_rt_async_token_state(StatePtr);
                return status != 0u;
            }
        }
    }
}
public struct CancellationToken
{
    // Points into the source's cancellation state.
    public * mut bool IsCanceledPtr;
    public bool IsCancellationRequested() {
        unsafe {
            if (IsCanceledPtr == null)
            {
                return false;
            }
            var status = RuntimeIntrinsics.chic_rt_async_token_state(IsCanceledPtr);
            return status != 0u;
        }
    }
}
public static class Runtime
{
    public static void Spawn(Task task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_spawn(RuntimeExports.TaskHeader(task));
        }
    }
    public static void BlockOn(Task task) {
        unsafe {
            RuntimeIntrinsics.chic_rt_async_block_on(RuntimeExports.TaskHeader(task));
        }
    }
    public static bool Cancel(Task task) {
        unsafe {
            var status = RuntimeIntrinsics.chic_rt_async_cancel(RuntimeExports.TaskHeader(task));
            return status == FutureFlags.Ready;
        }
    }
}
internal static class RuntimeExports
{
    public static * mut FutureHeader TaskHeader(Task task) {
        unsafe {
            return & task.Header;
        }
    }
    public static bool TaskBoolResult(Task <bool >task) {
        return task.InnerFuture.Result;
    }
    public static int TaskIntResult(Task <int >task) {
        return task.InnerFuture.Result;
    }
}
internal struct RuntimeContext
{
    internal isize Inner;
}
