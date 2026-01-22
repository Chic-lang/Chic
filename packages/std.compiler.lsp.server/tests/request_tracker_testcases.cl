namespace Std.Compiler.Lsp.Server;
import Std.Testing;

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
    let _ = tracker.TryComplete(99L, out var method);
    Assert.That(method).IsEqualTo("initialize");
}

