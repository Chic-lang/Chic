namespace Std.Platform.Thread;
import Std;
import Std.Runtime.Collections;
import Std.Testing;

testcase Given_thread_function_start_adapter_invokes_callback_When_executed_Then_thread_function_start_adapter_invokes_callback()
{
    var called = false;
    var adapter = new ThreadFunctionStartAdapter(() => {
        called = true;
    });
    adapter.Run();
    Assert.That(called).IsTrue();
}

testcase Given_thread_function_runner_invokes_callback_When_executed_Then_thread_function_runner_invokes_callback()
{
    var called = false;
    var runner = new ThreadFunctionRunner(() => {
        called = true;
    });
    runner.Run();
    Assert.That(called).IsTrue();
}

testcase Given_thread_start_factory_rejects_null_inputs_When_executed_Then_thread_start_factory_rejects_null_inputs()
{
    Assert.Throws<InvalidOperationException>(() => {
        let _ = ThreadStartFactory.From<ThreadFunctionStartAdapter>(null);
    });
}

testcase Given_thread_start_factory_function_rejects_null_When_executed_Then_thread_start_factory_function_rejects_null()
{
    Assert.Throws<InvalidOperationException>(() => {
        let _ = ThreadStartFactory.Function(null);
    });
}

testcase Given_thread_status_to_string_success_When_executed_Then_thread_status_to_string_success()
{
    Assert.That(ThreadStatus.Success.ToString()).IsEqualTo("Success");
}

testcase Given_thread_status_to_string_not_supported_When_executed_Then_thread_status_to_string_not_supported()
{
    Assert.That(ThreadStatus.NotSupported.ToString()).IsEqualTo("NotSupported");
}

testcase Given_thread_status_to_string_invalid_When_executed_Then_thread_status_to_string_invalid()
{
    Assert.That(ThreadStatus.Invalid.ToString()).IsEqualTo("Invalid");
}

testcase Given_thread_status_to_string_spawn_failed_When_executed_Then_thread_status_to_string_spawn_failed()
{
    Assert.That(ThreadStatus.SpawnFailed.ToString()).IsEqualTo("SpawnFailed");
}

testcase Given_thread_handle_null_is_invalid_When_executed_Then_thread_handle_null_is_invalid()
{
    var handle = ThreadHandle.Null();
    Assert.That(handle.IsValid).IsFalse();
}

testcase Given_thread_handle_clear_keeps_invalid_When_executed_Then_thread_handle_clear_keeps_invalid()
{
    var handle = ThreadHandle.Null();
    handle.Clear();
    Assert.That(handle.IsValid).IsFalse();
}

testcase Given_thread_runtime_callbacks_reject_invalid_context_When_executed_Then_thread_runtime_callbacks_reject_invalid_context()
{
    let handle = ValuePointer.NullMut(0usize, 0usize);
    let valid = RuntimeCallbacks.ContextLayoutIsValid(handle);
    Assert.That(valid).IsFalse();
}

testcase Given_thread_runtime_exports_accept_null_context_When_executed_Then_thread_runtime_exports_accept_null_context()
{
    let handle = ValuePointer.NullMut(0usize, 0usize);
    var ok = true;
    try {
        ThreadRuntimeExports.chic_thread_invoke(handle);
        ThreadRuntimeExports.chic_thread_drop(handle);
    }
    catch(Exception) {
        ok = false;
    }
    Assert.That(ok).IsTrue();
}
