namespace Std.Compiler.Lsp.Types;
public struct InitializeResult
{
    public ServerCapabilities Capabilities;
    public bool HasServerInfo;
    public ServerInfo ServerInfo;
    public init(ServerCapabilities capabilities) {
        Capabilities = capabilities;
        HasServerInfo = false;
        ServerInfo = new ServerInfo("");
    }
    public init(ServerCapabilities capabilities, ServerInfo serverInfo) {
        Capabilities = capabilities;
        HasServerInfo = true;
        ServerInfo = serverInfo;
    }
}
