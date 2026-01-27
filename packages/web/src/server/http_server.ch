namespace Chic.Web;
import Std.Async;
import Std.Net;
import Std.Net.Sockets;
import Std.IO;
import Std.Security.Tls;
import NetSocketError = Std.Net.Sockets.SocketError;
/// <summary>TCP listener hosting the HTTP request loop.</summary>
public sealed class HttpServer
{
    private string _host;
    private int _port;
    private HttpProtocols _protocols;
    private bool _useTls;
    private TlsServerOptions ?_tlsOptions;
    public init(string host, int port, HttpProtocols protocols, bool useTls, TlsServerOptions ?options) {
        _host = host;
        if (_host == null || _host.Length == 0)
        {
            _host = "127.0.0.1";
        }
        _port = port;
        _protocols = protocols;
        _useTls = useTls;
        _tlsOptions = options;
    }
    public Task Run(RequestDelegate app, CancellationToken ct) {
        if (_useTls && (_tlsOptions == null || _tlsOptions.CertificateChain == null || _tlsOptions.CertificateChain.Length == 0))
        {
            throw new Std.InvalidOperationException("TLS requires a configured certificate chain");
        }
        if (UsesHttp3 (_protocols))
        {
            var http3 = new Http3Server(_host, _port);
            http3.Run(app, ct);
            return TaskRuntime.CompletedTask();
        }
        if (UsesHttp2 (_protocols))
        {
            var http2 = new Http2Server(_host, _port);
            http2.Run(app, ct);
            return TaskRuntime.CompletedTask();
        }
        RunHttp1(app, ct);
        return TaskRuntime.CompletedTask();
    }
    private void RunHttp1(RequestDelegate app, CancellationToken ct) {
        var listener = new Std.Net.Sockets.Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
        let bindStatus = listener.Bind(IPAddress.Parse(_host), _port);
        if (bindStatus != NetSocketError.Success)
        {
            throw new Std.IOException("failed to bind HTTP server socket");
        }
        let listenStatus = listener.Listen(128);
        if (listenStatus != NetSocketError.Success)
        {
            throw new Std.IOException("failed to listen on HTTP server socket");
        }
        try {
            while (!ct.IsCancellationRequested ())
            {
                var client = (Std.Net.Sockets.Socket ?) null;
                try {
                    client = listener.Accept();
                }
                catch(Std.Exception) {
                    if (ct.IsCancellationRequested ())
                    {
                        break;
                    }
                    continue;
                }
                if (client == null)
                {
                    continue;
                }
                ProcessClient(app, ct, client);
            }
        }
        finally {
            listener.Close();
        }
    }
    private void ProcessClient(RequestDelegate app, CancellationToken ct, Std.Net.Sockets.Socket client) {
        var stream = (NetworkStream ?) null;
        try {
            stream = new NetworkStream(client, true);
            var active = (Stream) stream;
            if (_useTls)
            {
                var tls = new Std.Security.Tls.TlsStream(active, false);
                var opts = _tlsOptions;
                if (opts == null)
                {
                    opts = new TlsServerOptions();
                }
                if (opts.ServerName == null || opts.ServerName.Length == 0)
                {
                    opts.ServerName = _host;
                }
                tls.AuthenticateAsServerAsync(opts, ct);
                active = tls;
            }
            var connection = new Http1Connection(active, app);
            connection.Process(ct);
        }
        catch(Std.Exception) {
            // Swallow connection-level failures to keep the listener alive.
        }
        finally {
            if (stream != null)
            {
                stream.Dispose();
            }
            else
            {
                client.Close();
            }
        }
    }
    private static bool UsesHttp2(HttpProtocols protocols) {
        return protocols == HttpProtocols.Http2 || protocols == HttpProtocols.Http1AndHttp2 || protocols == HttpProtocols.Http2AndHttp3 || protocols == HttpProtocols.Http1AndHttp2AndHttp3;
    }
    private static bool UsesHttp3(HttpProtocols protocols) {
        return protocols == HttpProtocols.Http3 || protocols == HttpProtocols.Http1AndHttp3 || protocols == HttpProtocols.Http2AndHttp3 || protocols == HttpProtocols.Http1AndHttp2AndHttp3;
    }
}
