namespace Std.Net.Http;
import Std.Span;
import Std.Strings;
import Std.Numeric;
/// <summary>
/// HttpContent representing a UTF-8 encoded string.
/// </summary>
public sealed class StringContent : HttpContent
{
    private readonly string _content;
    private readonly byte[] _buffer;
    public init(string content) : base() {
        if (content == null)
        {
            throw new Std.ArgumentNullException("content");
        }
        _content = content;
        let utf8 = _content.AsUtf8Span();
        _buffer = new byte[utf8.Length];
        var span = Span <byte >.FromArray(ref _buffer);
        span.CopyFrom(utf8);
        Headers.Set("Content-Type", "text/plain; charset=utf-8");
        Headers.Set("Content-Length", _buffer.Length.ToString());
    }
    internal override byte[] GetBytes() {
        return _buffer;
    }
}
