namespace Chic.Web;
/// <summary>Supported HTTP protocol versions for the chic web server.</summary>
public enum HttpProtocols
{
    Http1 = 1, Http2 = 2, Http3 = 4, Http1AndHttp2 = 3, Http1AndHttp3 = 5, Http2AndHttp3 = 6, Http1AndHttp2AndHttp3 = 7
}
