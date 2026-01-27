namespace Chic.Web;
import Std.Async;
/// <summary>HTTP/2 server placeholder until TLS/ALPN and frame handling land.</summary>
internal sealed class Http2Server
{
    private string _host;
    private int _port;
    public init(string host, int port) {
        _host = host;
        _port = port;
    }
    public void Run(RequestDelegate app, CancellationToken ct) {
        throw new Std.NotSupportedException("HTTP/2 server is not implemented yet; TLS/ALPN and framing are pending (see docs/web/protocols.md)");
    }
}
