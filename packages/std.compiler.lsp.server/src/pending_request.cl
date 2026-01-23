namespace Std.Compiler.Lsp.Server;
public struct PendingRequest
{
    public long Id;
    public string Method;
    public init(long id, string method) {
        Id = id;
        Method = method;
    }
}
