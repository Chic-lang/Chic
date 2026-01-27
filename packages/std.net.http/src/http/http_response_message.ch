namespace Std.Net.Http;
/// <summary>
/// Represents an HTTP response with status, headers, and optional content.
/// </summary>
public sealed class HttpResponseMessage
{
    public HttpStatusCode StatusCode {
        get;
        set;
    }
    public string ?ReasonPhrase {
        get;
        set;
    }
    public HttpResponseHeaders Headers {
        get;
        set;
    }
    public HttpContent ?Content {
        get;
        set;
    }
    public Std.Version Version {
        get;
        set;
    }
    public HttpRequestMessage ?RequestMessage {
        get;
        set;
    }
    public init() {
        Headers = new HttpResponseHeaders();
        Version = new Std.Version(1, 1);
        StatusCode = HttpStatusCode.OK;
        ReasonPhrase = null;
    }
    public bool IsSuccessStatusCode() {
        let code = (int) StatusCode;
        return code >= 200 && code <= 299;
    }
}
