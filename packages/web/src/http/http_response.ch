namespace Chic.Web;
import Std.Async;
import Std.IO;
import Std.Net.Http;
import Std.Span;
import Std.Strings;
/// <summary>Outgoing HTTP response with buffered body.</summary>
public sealed class HttpResponse
{
    private int _statusCode;
    private HttpHeaders _headers;
    private MemoryStream _body;
    private bool _hasStarted;
    private bool _hasContentLength;
    private long _contentLength;
    public init() {
        _statusCode = 200;
        _headers = new HttpHeaders();
        _body = new MemoryStream();
        _hasStarted = false;
        _hasContentLength = false;
        _contentLength = 0;
    }
    public int StatusCode {
        get {
            return _statusCode;
        }
        set {
            _statusCode = value;
        }
    }
    public HttpHeaders Headers => _headers;
    public Stream Body => _body;
    public bool HasStarted => _hasStarted;
    public bool HasContentLength => _hasContentLength;
    public long ContentLength {
        get {
            return _contentLength;
        }
        set {
            _contentLength = value;
            _hasContentLength = true;
        }
    }
    public Task WriteStringAsync(string text) {
        var value = text;
        if (value == null)
        {
            value = "";
        }
        let utf8 = value.AsUtf8Span();
        if (utf8.Length == 0)
        {
            return TaskRuntime.CompletedTask();
        }
        var buffer = new byte[utf8.Length];
        Span <byte >.FromArray(ref buffer).CopyFrom(utf8);
        Body.Write(ReadOnlySpan <byte >.FromArray(ref buffer));
        return TaskRuntime.CompletedTask();
    }
    public Task WriteJsonAsync <T >(T value) {
        let text = Std.Text.Json.JsonSerializer.Serialize(value);
        Headers.Set("Content-Type", "application/json");
        return WriteStringAsync(text);
    }
    internal MemoryStream BodyStream => _body;
    internal void MarkStarted() {
        _hasStarted = true;
    }
}
