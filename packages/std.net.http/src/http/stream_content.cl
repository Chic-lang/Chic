namespace Std.Net.Http;
import Std.Span;
/// <summary>
/// HttpContent wrapping a byte buffer to simulate stream payloads.
/// </summary>
public sealed class StreamContent : HttpContent
{
    private readonly byte[] _buffer;
    public init(byte[] buffer) : base() {
        if (buffer == null)
        {
            throw new Std.ArgumentNullException("buffer");
        }
        _buffer = buffer;
        Headers.Set("Content-Length", buffer.Length.ToString());
    }
    internal override byte[] GetBytes() {
        return _buffer;
    }
}
