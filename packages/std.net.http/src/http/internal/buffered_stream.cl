namespace Std.Net.Http.Internal;
import Std.Span;
import Std.Numeric;
public sealed class BufferedStream
{
    private readonly byte[] _buffer;
    private usize _position;
    public init(byte[] buffer) {
        if (buffer == null)
        {
            throw new Std.ArgumentNullException("buffer");
        }
        _buffer = buffer;
        _position = 0usize;
    }
    public int Read(Span <byte >destination) {
        if (destination.Length == 0)
        {
            return 0;
        }
        let remaining = _buffer.Length - NumericUnchecked.ToInt32(_position);
        if (remaining <= 0)
        {
            return 0;
        }
        let toCopy = remaining;
        if (toCopy >destination.Length)
        {
            toCopy = destination.Length;
        }
        var sourceArray = _buffer;
        let sourceSpan = ReadOnlySpan <byte >.FromArray(in sourceArray);
        let slice = sourceSpan.Slice(_position, NumericUnchecked.ToUSize(toCopy));
        destination.Slice(0, NumericUnchecked.ToUSize(toCopy)).CopyFrom(slice);
        _position += NumericUnchecked.ToUSize(toCopy);
        return toCopy;
    }
    public byte[] ToArray() {
        return _buffer;
    }
}
