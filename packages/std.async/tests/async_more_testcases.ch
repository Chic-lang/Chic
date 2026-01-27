namespace Std.Async;
import Std.Core;
import Std.Testing;
testcase Given_future_is_completed_When_flags_set_Then_is_completed_true()
{
    var fut = CoreIntrinsics.DefaultValue <Future <int >> ();
    fut.Header.Flags = FutureFlags.Completed;
    Assert.That(fut.IsCompleted()).IsTrue();
}
testcase Given_future_is_completed_When_flags_not_set_Then_is_completed_false()
{
    var fut = CoreIntrinsics.DefaultValue <Future <int >> ();
    fut.Header.Flags = FutureFlags.Ready;
    Assert.That(fut.IsCompleted()).IsFalse();
}
testcase Given_task_runtime_void_from_result_When_executed_Then_flags_completed_ready_set()
{
    var task = TaskRuntime.FromResult();
    Assert.That((task.Flags & FutureFlags.Completed) != 0u).IsTrue();
    Assert.That((task.Flags & FutureFlags.Ready) != 0u).IsTrue();
}
testcase Given_task_static_spawn_local_When_task_completed_Then_does_not_throw()
{
    var task = TaskRuntime.CompletedTask();
    Task.SpawnLocal(task);
    Assert.That(true).IsTrue();
}
testcase Given_task_static_scope_When_task_completed_Then_does_not_throw()
{
    var task = TaskRuntime.CompletedTask();
    Task.Scope(task);
    Assert.That(true).IsTrue();
}
testcase Given_task_generic_spawn_local_When_task_completed_Then_returns_same_task()
{
    var task = TaskRuntime.FromResult <int >(123);
    let returned = Task <int >.SpawnLocal(task);
    Assert.That(TaskRuntime.GetResult <int >(returned)).IsEqualTo(123);
}
testcase Given_task_generic_scope_When_task_completed_Then_returns_result()
{
    var task = TaskRuntime.FromResult <int >(123);
    let result = Task <int >.Scope(task);
    Assert.That(result).IsEqualTo(123);
}
testcase Given_runtime_spawn_When_task_completed_Then_does_not_throw()
{
    var task = TaskRuntime.CompletedTask();
    Runtime.Spawn(task);
    Assert.That(true).IsTrue();
}
testcase Given_runtime_block_on_When_task_completed_Then_does_not_throw()
{
    var task = TaskRuntime.CompletedTask();
    Runtime.BlockOn(task);
    Assert.That(true).IsTrue();
}
testcase Given_runtime_cancel_When_task_completed_Then_returns_true()
{
    var task = TaskRuntime.CompletedTask();
    let ok = Runtime.Cancel(task);
    Assert.That(ok).IsTrue();
}
testcase Given_runtime_exports_task_header_When_called_Then_is_not_null()
{
    var task = TaskRuntime.CompletedTask();
    let header = RuntimeExports.TaskHeader(task);
    Assert.That(header == null).IsFalse();
}
testcase Given_runtime_exports_task_bool_result_When_called_Then_returns_inner_future_result()
{
    var task = TaskRuntime.FromResult <bool >(true);
    Assert.That(RuntimeExports.TaskBoolResult(task)).IsTrue();
}
testcase Given_runtime_exports_task_int_result_When_called_Then_returns_inner_future_result()
{
    var task = TaskRuntime.FromResult <int >(42);
    Assert.That(RuntimeExports.TaskIntResult(task)).IsEqualTo(42);
}
testcase Given_cancellation_helper_new_cancel_source_When_executed_Then_token_budget_works()
{
    var source = Cancellation.NewCancelSource(2ul, 0ul);
    var token = source.Token();
    Assert.That(token.ConsumeBudget(0ul)).IsFalse();
    Assert.That(token.ConsumeBudget(1ul)).IsFalse();
    Assert.That(token.ConsumeBudget(1ul)).IsTrue();
}
testcase Given_cancellation_helper_token_When_executed_Then_token_starts_not_canceled()
{
    var token = Cancellation.Token();
    Assert.That(token.IsCanceled).IsFalse();
}
testcase Given_cancel_token_consume_budget_amount_zero_When_not_canceled_Then_returns_false()
{
    var source = new CancelSource(2ul, 0ul);
    var token = source.Token();
    Assert.That(token.ConsumeBudget(0ul)).IsFalse();
}
testcase Given_cancel_token_consume_budget_When_already_canceled_Then_returns_true()
{
    var source = new CancelSource(2ul, 0ul);
    source.Cancel();
    var token = source.Token();
    Assert.That(token.ConsumeBudget(1ul)).IsTrue();
}
testcase Given_cancel_token_check_deadline_When_no_deadline_Then_returns_false()
{
    var source = new CancelSource(0ul, 0ul);
    var token = source.Token();
    Assert.That(token.CheckDeadline(0ul)).IsFalse();
}
testcase Given_cancel_token_check_deadline_When_already_canceled_Then_returns_true()
{
    var source = new CancelSource(0ul, 0ul);
    source.Cancel();
    var token = source.Token();
    Assert.That(token.CheckDeadline(0ul)).IsTrue();
}
testcase Given_cancel_source_force_deadline_When_executed_Then_token_is_canceled()
{
    var source = new CancelSource(0ul, 0ul);
    source.ForceDeadline();
    var token = source.Token();
    Assert.That(token.IsCanceled).IsTrue();
}
testcase Given_cancel_source_budget_zero_When_executed_Then_budget_is_max()
{
    var source = new CancelSource(0ul, 0ul);
    Assert.That(source.State.BudgetRemaining).IsEqualTo(ulong.MaxValue);
}
testcase Given_scope_tracker_cancel_all_When_executed_Then_finalize_succeeds()
{
    var tracker = new ScopeTracker();
    tracker.RegisterTask("a");
    tracker.RegisterTask("b");
    tracker.CancelAll();
    var message = "";
    var ok = tracker.TryFinalizeScope(out message);
    Assert.That(ok).IsTrue();
    Assert.That(message).IsEqualTo("");
}
testcase Given_scope_tracker_register_grows_capacity_When_executed_Then_finalize_succeeds()
{
    var tracker = new ScopeTracker();
    tracker.RegisterTask("a");
    tracker.RegisterTask("b");
    tracker.RegisterTask("c");
    tracker.RegisterTask("d");
    tracker.RegisterTask("e");
    tracker.MarkCompleted("a");
    tracker.MarkCompleted("b");
    tracker.MarkCompleted("c");
    tracker.MarkCompleted("d");
    tracker.MarkCompleted("e");
    var message = "";
    var ok = tracker.TryFinalizeScope(out message);
    Assert.That(ok).IsTrue();
    Assert.That(message).IsEqualTo("");
}
testcase Given_scope_tracker_mark_completed_unknown_When_executed_Then_finalize_includes_task_names()
{
    var tracker = new ScopeTracker();
    tracker.RegisterTask("a");
    tracker.RegisterTask("b");
    tracker.MarkCompleted("missing");
    var message = "";
    var ok = tracker.TryFinalizeScope(out message);
    Assert.That(ok).IsFalse();
    Assert.That(message).StartsWith("structured scope exited with incomplete tasks: ");
    Assert.That(message).Contains("a");
    Assert.That(message).Contains("b");
}
testcase Given_numeric_unchecked_to_int32_from_usize_When_small_Then_roundtrips()
{
    let value = 123usize;
    let narrowed = NumericUnchecked.ToInt32(value);
    Assert.That(narrowed).IsEqualTo(123);
}
testcase Given_numeric_unchecked_to_int32_from_isize_When_small_Then_roundtrips()
{
    let value = (isize) 123;
    let narrowed = NumericUnchecked.ToInt32(value);
    Assert.That(narrowed).IsEqualTo(123);
}
testcase Given_numeric_unchecked_to_usize_from_int_When_small_Then_roundtrips()
{
    let value = 123;
    let widened = NumericUnchecked.ToUSize(value);
    Assert.That(NumericUnchecked.ToInt32(widened)).IsEqualTo(123);
}
