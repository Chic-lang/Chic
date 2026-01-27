namespace Std.Net.Http;
import Std.Span;
import Std.Strings;
import Std.Numeric;
/// <summary>
/// Base HTTP content abstraction capable of producing serialized bytes.
/// </summary>
public abstract class HttpContent
{
    private readonly HttpContentHeaders _headers;
    protected init() {
        _headers = new HttpContentHeaders();
    }
    public HttpContentHeaders Headers => _headers;
    internal virtual byte[] GetBytes() {
        let empty = 0;
        return new byte[empty];
    }
    public virtual byte[] ReadAsByteArray() {
        return GetBytes();
    }
    public virtual string ReadAsString() {
        let bytes = GetBytes();
        var array = bytes;
        let span = ReadOnlySpan <byte >.FromArray(in array);
        return Utf8String.FromSpan(span);
    }
    public virtual void Dispose() {
    }
    public void dispose(ref this) {
        Dispose();
    }
}
