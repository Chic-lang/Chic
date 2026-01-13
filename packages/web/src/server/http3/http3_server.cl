namespace Chic.Web;
import Std.Async;
/// <summary>HTTP/3 over QUIC placeholder until the transport and QPACK stack are ready.</summary>
internal sealed class Http3Server
{
    private string _host;
    private int _port;
    public init(string host, int port) {
        _host = host;
        _port = port;
    }
    public void Run(RequestDelegate app, CancellationToken ct) {
        throw new Std.NotSupportedException("HTTP/3 (QUIC) server is not implemented yet; QUIC/TLS 1.3 support is pending (see docs/web/protocols.md)");
    }
}
