namespace Std.Async;
/// <summary>
/// Minimal helpers to produce already-completed tasks for synchronous implementations.
/// </summary>
public static class TaskRuntime
{
    public static Task <T >FromResult <T >(T value) {
        var task = new Task <T >();
        var inner : Future <T >= task.InnerFuture;
        inner.Completed = true;
        inner.Result = value;
        inner.Header.Flags = FutureFlags.Completed | FutureFlags.Ready;
        task.InnerFuture = inner;
        task.Header = inner.Header;
        task.Flags = FutureFlags.Completed | FutureFlags.Ready;
        return task;
    }
    public static Task FromResult() {
        var task = new Task();
        task.Header.Flags = FutureFlags.Completed | FutureFlags.Ready;
        task.Flags = FutureFlags.Completed | FutureFlags.Ready;
        return task;
    }
    /// <summary>Returns a cached completed task for void-returning async APIs.</summary>
    public static Task CompletedTask() {
        return FromResult();
    }
    public static T GetResult <T >(Task <T >task) {
        return task.InnerFuture.Result;
    }
}
