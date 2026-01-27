namespace Std.Compiler.Lsp.Server;
public struct NotificationRoute
{
    public string Method;
    public NotificationHandler Handler;
    public init(string method, NotificationHandler handler) {
        Method = method;
        Handler = handler;
    }
}
