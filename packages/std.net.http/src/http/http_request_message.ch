namespace Std.Net.Http;
/// <summary>
/// Represents an outgoing HTTP request including method, URI, headers, and content.
/// </summary>
public sealed class HttpRequestMessage
{
    public HttpMethod Method {
        get;
        set;
    }
    public Std.Uri ?RequestUri {
        get;
        set;
    }
    public Std.Version Version {
        get;
        set;
    }
    public HttpVersionPolicy VersionPolicy {
        get;
        set;
    }
    public HttpContent ?Content {
        get;
        set;
    }
    public HttpRequestHeaders Headers {
        get;
        set;
    }
    public init() {
        Method = HttpMethod.Get;
        Version = new Std.Version(1, 1);
        VersionPolicy = HttpVersionPolicy.RequestVersionOrLower;
        Headers = new HttpRequestHeaders();
    }
    public init(HttpMethod method, Std.Uri requestUri) {
        Method = method;
        RequestUri = requestUri;
        Version = new Std.Version(1, 1);
        VersionPolicy = HttpVersionPolicy.RequestVersionOrLower;
        Headers = new HttpRequestHeaders();
    }
}
