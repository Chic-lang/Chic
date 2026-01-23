namespace Std.Compiler.Lsp.Server;
import Std.Testing;
static class RouterTestState
{
    public static int Calls;
    public static void Reset() {
        Calls = 0;
    }
    public static void Handler(string method, string payload) {
        Calls = Calls + 1;
    }
}
testcase Given_notification_router_When_dispatched_Then_handler_called()
{
    RouterTestState.Reset();
    var router = new NotificationRouter();
    router.Register("textDocument/didOpen", RouterTestState.Handler);
    let _ = router.Dispatch("textDocument/didOpen", "{}");
    Assert.That(RouterTestState.Calls).IsEqualTo(1);
}
