namespace Std.Compiler.Lsp.Server;
import Std.Testing;
static class OutParamTestHelpers
{
    public static bool TryGet(out string value) {
        value = "initialize";
        return true;
    }
}
testcase Given_request_tracker_When_completed_Then_returns_true()
{
    var tracker = new RequestTracker();
    tracker.Track(99L, "initialize");
    let ok = tracker.TryComplete(99L, out var method);
    Assert.That(ok).IsTrue();
}
testcase Given_request_tracker_When_completed_Then_returns_method()
{
    var tracker = new RequestTracker();
    tracker.Track(99L, "initialize");
    let _ = tracker.TryComplete(99L, out var completed);
    Assert.That(completed).IsEqualTo("initialize");
}
testcase Given_out_param_When_set_Then_roundtrips()
{
    let _ = OutParamTestHelpers.TryGet(out var value);
    Assert.That(value).IsEqualTo("initialize");
}
testcase Given_pending_request_When_constructed_Then_method_roundtrips()
{
    let pending = new PendingRequest(99L, "initialize");
    Assert.That(pending.Method).IsEqualTo("initialize");
}
testcase Given_pending_request_array_When_assigned_Then_method_roundtrips()
{
    let pending = new PendingRequest(99L, "initialize");
    let arr = new PendingRequest[1];
    arr[0] = pending;
    Assert.That(arr[0].Method).IsEqualTo("initialize");
}
