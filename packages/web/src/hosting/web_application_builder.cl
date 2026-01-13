namespace Chic.Web;
import Std.Security.Tls;
import Std.IO;
import Std.Numeric;
import Std.Span;
/// <summary>Configures and constructs a <see cref="WebApplication"/>.</summary>
public sealed class WebApplicationBuilder
{
    private string _urls;
    private HttpProtocols _protocols;
    private TlsServerOptions ?_tlsOptions;
    public init() {
        _urls = "http://127.0.0.1:5000";
        _protocols = HttpProtocols.Http1;
    }
    public string Urls {
        get {
            return _urls;
        }
        set {
            if (value != null)
            {
                _urls = value;
            }
        }
    }
    public HttpProtocols Protocols {
        get {
            return _protocols;
        }
        set {
            _protocols = value;
        }
    }
    public TlsServerOptions ?TlsOptions {
        get {
            return _tlsOptions;
        }
        set {
            _tlsOptions = value;
        }
    }
    public WebApplication Build() {
        return new WebApplication(_urls, _protocols, _tlsOptions);
    }
    public void UseHttps(string certificatePath, string serverName) {
        if (certificatePath == null)
        {
            throw new Std.ArgumentNullException("certificatePath");
        }
        var options = _tlsOptions;
        if (options == null)
        {
            options = new TlsServerOptions();
        }
        options.CertificateChain = ReadFile(certificatePath);
        if (serverName != null && serverName.Length >0)
        {
            options.ServerName = serverName;
        }
        _tlsOptions = options;
    }
    private static byte[] ReadFile(string path) {
        var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        var buffer = new byte[NumericUnchecked.ToInt32(stream.Length)];
        var span = Std.Span.Span <byte >.FromArray(ref buffer);
        let read = stream.Read(span);
        stream.Dispose();
        if (read <buffer.Length)
        {
            var trimmed = new byte[read];
            Span <byte >.FromArray(ref trimmed).CopyFrom(ReadOnlySpan <byte >.FromArray(ref buffer).Slice(0usize, NumericUnchecked.ToUSize(read)));
            return trimmed;
        }
        return buffer;
    }
}
