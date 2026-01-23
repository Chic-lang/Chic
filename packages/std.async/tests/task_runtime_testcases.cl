namespace Std.Async;
import Std.Testing;
testcase Given_task_runtime_from_result_get_result_When_executed_Then_task_runtime_from_result_get_result()
{
    var task = TaskRuntime.FromResult <int >(42);
    let result = TaskRuntime.GetResult <int >(task);
    Assert.That(result).IsEqualTo(42);
}
testcase Given_task_runtime_from_result_completed_flag_When_executed_Then_task_runtime_from_result_completed_flag()
{
    var task = TaskRuntime.FromResult <int >(42);
    Assert.That(task.InnerFuture.IsCompleted()).IsTrue();
}
testcase Given_task_runtime_from_result_flags_completed_set_When_executed_Then_task_runtime_from_result_flags_completed_set()
{
    var task = TaskRuntime.FromResult <int >(42);
    let flags = task.Flags;
    Assert.That((flags & FutureFlags.Completed) != 0u).IsTrue();
}
testcase Given_task_runtime_from_result_flags_ready_set_When_executed_Then_task_runtime_from_result_flags_ready_set()
{
    var task = TaskRuntime.FromResult <int >(42);
    let flags = task.Flags;
    Assert.That((flags & FutureFlags.Ready) != 0u).IsTrue();
}
testcase Given_task_runtime_completed_task_completed_flag_When_executed_Then_task_runtime_completed_task_completed_flag()
{
    var task = TaskRuntime.CompletedTask();
    let flags = task.Flags;
    Assert.That((flags & FutureFlags.Completed) != 0u).IsTrue();
}
testcase Given_task_runtime_completed_task_ready_flag_When_executed_Then_task_runtime_completed_task_ready_flag()
{
    var task = TaskRuntime.CompletedTask();
    let flags = task.Flags;
    Assert.That((flags & FutureFlags.Ready) != 0u).IsTrue();
}
testcase Given_cancellation_token_source_default_not_canceled_When_executed_Then_cancellation_token_source_default_not_canceled()
{
    var source = Std.Core.CoreIntrinsics.DefaultValue <CancellationTokenSource >();
    Assert.That(source.IsCanceled).IsFalse();
}
testcase Given_cancellation_token_source_default_token_not_canceled_When_executed_Then_cancellation_token_source_default_token_not_canceled()
{
    var source = Std.Core.CoreIntrinsics.DefaultValue <CancellationTokenSource >();
    let token = source.Token();
    Assert.That(token.IsCancellationRequested()).IsFalse();
}
testcase Given_cancellation_token_source_default_cancel_no_effect_When_executed_Then_cancellation_token_source_default_cancel_no_effect()
{
    var source = Std.Core.CoreIntrinsics.DefaultValue <CancellationTokenSource >();
    source.Cancel();
    Assert.That(source.IsCanceled).IsFalse();
}
testcase Given_cancellation_token_source_create_token_not_canceled_When_executed_Then_cancellation_token_source_create_token_not_canceled()
{
    var source = CancellationTokenSource.Create();
    let token = source.Token();
    Assert.That(token.IsCancellationRequested()).IsFalse();
}
testcase Given_cancellation_token_source_create_cancel_sets_source_When_executed_Then_cancellation_token_source_create_cancel_sets_source()
{
    var source = CancellationTokenSource.Create();
    source.Cancel();
    Assert.That(source.IsCanceled).IsTrue();
}
testcase Given_cancellation_token_source_create_cancel_sets_token_When_executed_Then_cancellation_token_source_create_cancel_sets_token()
{
    var source = CancellationTokenSource.Create();
    let token = source.Token();
    source.Cancel();
    Assert.That(token.IsCancellationRequested()).IsTrue();
}
