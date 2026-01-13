namespace Std.IO;
import Std.Span;
import Std.Numeric;
/// <summary>Internal reusable buffer helper for stream readers.</summary>
internal sealed class BufferedStreamReader
{
    private Stream _stream;
    private byte[] _buffer;
    private int _count;
    public init(Stream stream, int bufferSize = 4096) {
        if (stream == null)
        {
            throw new Std.ArgumentNullException("stream");
        }
        if (bufferSize <= 0)
        {
            throw new Std.ArgumentOutOfRangeException("bufferSize");
        }
        _stream = stream;
        _buffer = new byte[bufferSize];
        _count = 0;
    }
    public ReadOnlySpan <byte >CurrentSpan() {
        return ReadOnlySpan <byte >.FromArray(in _buffer).Slice(0usize, NumericUnchecked.ToUSize(_count));
    }
    /// <summary>Refill the buffer; returns false on EOF.</summary>
    public bool Refill() {
        var buffer = _buffer;
        let span = Span <byte >.FromArray(ref buffer);
        let read = _stream.Read(span);
        _count = read;
        return read >0;
    }
}
