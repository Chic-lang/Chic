namespace Std.IO;
import Std;
import Std.Span;
import Std.Numeric;
import Std.Memory;
import Std.Testing;
/// <summary>Growable in-memory stream with span-based IO.</summary>
public sealed class MemoryStream : Stream
{
    private byte[] _buffer;
    private int _length;
    private long _position;
    private bool _writable;
    private bool _expandable;
    private bool _exposable;
    /// <summary>Initializes an empty expandable memory stream.</summary>
    public init() {
        let empty = 0;
        _buffer = new byte[empty];
        _length = 0;
        _position = 0;
        _writable = true;
        _expandable = true;
        _exposable = true;
    }
    /// <summary>Initializes a memory stream over an existing buffer.</summary>
    /// <param name="initial">Backing buffer.</param>
    /// <param name="writable">Whether the stream permits writing.</param>
    public init(byte[] ?initial, bool writable = true) {
        let empty = 0;
        var buffer = initial;
        if (buffer == null)
        {
            buffer = new byte[empty];
        }
        _buffer = buffer;
        _length = buffer.Length;
        _position = 0;
        _writable = writable;
        _expandable = false;
        _exposable = true;
    }
    /// <summary>Initializes a memory stream by copying an initial span.</summary>
    /// <param name="initial">Initial data to copy.</param>
    public init(ReadOnlySpan <byte >initial) {
        _buffer = new byte[initial.Length];
        _length = NumericUnchecked.ToInt32(initial.Length);
        _position = 0;
        _writable = true;
        _expandable = true;
        _exposable = true;
        if (initial.Length >0)
        {
            var buffer = _buffer;
            Span <byte >.FromArray(ref buffer).Slice(0usize, initial.Length).CopyFrom(initial);
        }
    }
    /// <inheritdoc />
    public override bool CanRead => true;
    /// <inheritdoc />
    public override bool CanWrite => _writable;
    /// <inheritdoc />
    public override bool CanSeek => true;
    /// <summary>Gets or sets the capacity of the underlying buffer.</summary>
    /// <exception cref="Std.NotSupportedException">Thrown when resizing is not permitted.</exception>
    /// <exception cref="Std.ArgumentOutOfRangeException">Thrown when attempting to shrink below the current length.</exception>
    public int Capacity {
        get {
            ThrowIfDisposed();
            return _buffer.Length;
        }
        set {
            ThrowIfDisposed();
            if (value <_length)
            {
                throw new Std.ArgumentOutOfRangeException("value");
            }
            if (! _expandable && value != _buffer.Length)
            {
                throw new Std.NotSupportedException("Stream is not expandable");
            }
            if (value == _buffer.Length)
            {
                return;
            }
            var newBuffer = new byte[value];
            if (_length >0)
            {
                Span <byte >.FromArray(ref newBuffer).Slice(0usize, NumericUnchecked.ToUSize(_length)).CopyFrom(ReadOnlySpan <byte >.FromArray(in _buffer).Slice(0usize,
                NumericUnchecked.ToUSize(_length)));
            }
            _buffer = newBuffer;
        }
    }
    /// <inheritdoc />
    public override long Length {
        get {
            ThrowIfDisposed();
            return _length;
        }
    }
    /// <inheritdoc />
    public override long Position {
        get {
            ThrowIfDisposed();
            return _position;
        }
        set {
            ThrowIfDisposed();
            if (value <0)
            {
                throw new Std.ArgumentOutOfRangeException("Position");
            }
            _position = value;
            EnsureCapacity(_position, false);
        }
    }
    /// <inheritdoc />
    public override int Read(Span <byte >buffer) {
        ThrowIfDisposed();
        if (buffer.Length == 0)
        {
            return 0;
        }
        if (_position >= _length)
        {
            return 0;
        }
        let available = _length - NumericUnchecked.ToInt32(_position);
        var toCopy = available;
        if (toCopy >buffer.Length)
        {
            toCopy = buffer.Length;
        }
        let source = ReadOnlySpan <byte >.FromArray(in _buffer);
        buffer.Slice(0usize, NumericUnchecked.ToUSize(toCopy)).CopyFrom(source.Slice(NumericUnchecked.ToUSize(_position),
        NumericUnchecked.ToUSize(toCopy)));
        _position += toCopy;
        return toCopy;
    }
    /// <inheritdoc />
    public override void Write(ReadOnlySpan <byte >buffer) {
        ThrowIfDisposed();
        if (! _writable)
        {
            throw new Std.NotSupportedException("Stream is not writable");
        }
        if (buffer.Length == 0)
        {
            return;
        }
        let newPos = _position + buffer.Length;
        if (newPos >int.MaxValue)
        {
            throw new Std.NotSupportedException("Stream capacity exceeded");
        }
        EnsureCapacity(newPos, true);
        var bufferRef = _buffer;
        var dest = Span <byte >.FromArray(ref bufferRef);
        dest.Slice(NumericUnchecked.ToUSize(_position), buffer.Length).CopyFrom(buffer);
        _position = newPos;
        if (_position >_length)
        {
            _length = NumericUnchecked.ToInt32(_position);
        }
    }
    /// <inheritdoc />
    public override void Flush() {
        ThrowIfDisposed();
    }
    /// <inheritdoc />
    public override long Seek(long offset, SeekOrigin origin) {
        ThrowIfDisposed();
        var target = _position;
        if (origin == SeekOrigin.Begin)
        {
            target = offset;
        }
        else if (origin == SeekOrigin.Current)
        {
            target = _position + offset;
        }
        else if (origin == SeekOrigin.End)
        {
            target = _length + offset;
        }
        if (target <0)
        {
            throw new Std.ArgumentOutOfRangeException("offset");
        }
        _position = target;
        EnsureCapacity(_position, false);
        return _position;
    }
    /// <inheritdoc />
    public override void SetLength(long value) {
        ThrowIfDisposed();
        if (! _writable)
        {
            throw new Std.NotSupportedException("Stream is not writable");
        }
        if (value <0)
        {
            throw new Std.ArgumentOutOfRangeException("value");
        }
        EnsureCapacity(value, true);
        _length = NumericUnchecked.ToInt32(value);
        if (_position >value)
        {
            _position = value;
        }
    }
    /// <summary>Resets the logical read position to the start.</summary>
    public void ResetPosition() {
        ThrowIfDisposed();
        _position = 0;
    }
    /// <summary>Exposes the underlying buffer when allowed without copying.</summary>
    /// <param name="buffer">Receives the underlying buffer when available.</param>
    /// <returns>True when the buffer was exposed; otherwise false.</returns>
    public bool TryGetBuffer(out byte[] ?buffer) {
        ThrowIfDisposed();
        if (! _exposable)
        {
            buffer = null;
            return false;
        }
        buffer = _buffer;
        return true;
    }
    /// <summary>Returns a read-only span over the valid bytes in the stream.</summary>
    public ReadOnlySpan <byte >GetSpan() {
        ThrowIfDisposed();
        return ReadOnlySpan <byte >.FromArray(in _buffer).Slice(0usize, NumericUnchecked.ToUSize(_length));
    }
    /// <summary>Copies the current contents into a new array.</summary>
    /// <returns>A new array containing the stream data.</returns>
    public byte[] ToArray() {
        ThrowIfDisposed();
        var arr = new byte[_length];
        if (_length >0)
        {
            Span <byte >.FromArray(ref arr).Slice(0usize, NumericUnchecked.ToUSize(_length)).CopyFrom(ReadOnlySpan <byte >.FromArray(in _buffer).Slice(0usize,
            NumericUnchecked.ToUSize(_length)));
        }
        return arr;
    }
    public override void CopyTo(Stream destination, int bufferSize = 81920) {
        ThrowIfDisposed();
        if (destination == null)
        {
            throw new Std.ArgumentNullException("destination");
        }
        if (_position >= _length)
        {
            return;
        }
        let span = ReadOnlySpan <byte >.FromArray(in _buffer).Slice(NumericUnchecked.ToUSize(_position), NumericUnchecked.ToUSize(_length - NumericUnchecked.ToInt32(_position)));
        destination.Write(span);
        _position = _length;
    }
    private void EnsureCapacity(long targetLength, bool extendLength) {
        if (targetLength <0 || targetLength >int.MaxValue)
        {
            throw new Std.ArgumentOutOfRangeException("targetLength");
        }
        let needed = NumericUnchecked.ToInt32(targetLength);
        if (needed <= _buffer.Length)
        {
            if (extendLength && needed >_length)
            {
                _length = needed;
            }
            return;
        }
        if (! _expandable)
        {
            throw new Std.NotSupportedException("Stream is not expandable");
        }
        var newCapacity = _buffer.Length == 0 ?needed : _buffer.Length * 2;
        if (newCapacity <needed)
        {
            newCapacity = needed;
        }
        var newBuffer = new byte[newCapacity];
        if (_length >0)
        {
            Span <byte >.FromArray(ref newBuffer).Slice(0usize, NumericUnchecked.ToUSize(_length)).CopyFrom(ReadOnlySpan <byte >.FromArray(in _buffer).Slice(0usize,
            NumericUnchecked.ToUSize(_length)));
        }
        _buffer = newBuffer;
        if (extendLength)
        {
            _length = needed;
        }
    }
    /// <inheritdoc />
    protected override void Dispose(bool disposing) {
        if (IsDisposed)
        {
            return;
        }
        if (disposing)
        {
            let empty = 0;
            _buffer = new byte[empty];
            _length = 0;
            _position = 0;
        }
        base.Dispose(disposing);
    }
}

testcase Given_memory_stream_write_sets_length_When_executed_Then_memory_stream_write_sets_length()
{
    var stream = new MemoryStream();
    var data = new byte[3];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    Assert.That(stream.Length).IsEqualTo(3);
}

testcase Given_memory_stream_read_returns_count_When_executed_Then_memory_stream_read_returns_count()
{
    var stream = new MemoryStream();
    var data = new byte[3];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    stream.Position = 0;
    var buffer = Span<byte>.StackAlloc(3usize);
    let read = stream.Read(buffer);
    Assert.That(read).IsEqualTo(3);
}

testcase Given_memory_stream_read_roundtrip_When_executed_Then_memory_stream_read_roundtrip()
{
    var stream = new MemoryStream();
    var data = new byte[3];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    stream.Position = 0;
    var buffer = Span<byte>.StackAlloc(3usize);
    let _ = stream.Read(buffer);
    Assert.That(buffer.AsReadOnly()).IsEqualTo(ReadOnlySpan<byte>.FromArray(in data));
}

testcase Given_memory_stream_set_length_truncates_length_When_executed_Then_memory_stream_set_length_truncates_length()
{
    var stream = new MemoryStream();
    var data = new byte[4];
    data[0] = 9u8;
    data[1] = 8u8;
    data[2] = 7u8;
    data[3] = 6u8;
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    stream.SetLength(2);
    Assert.That(stream.Length).IsEqualTo(2);
}

testcase Given_memory_stream_set_length_truncates_position_When_executed_Then_memory_stream_set_length_truncates_position()
{
    var stream = new MemoryStream();
    var data = new byte[4];
    data[0] = 9u8;
    data[1] = 8u8;
    data[2] = 7u8;
    data[3] = 6u8;
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    stream.SetLength(2);
    Assert.That(stream.Position).IsEqualTo(2);
}

testcase Given_memory_stream_capacity_shrink_throws_When_executed_Then_memory_stream_capacity_shrink_throws()
{
    var stream = new MemoryStream();
    var data = new byte[3];
    stream.Write(ReadOnlySpan<byte>.FromArray(in data));
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        stream.Capacity = 2;
    });
}

testcase Given_memory_stream_non_writable_throws_When_executed_Then_memory_stream_non_writable_throws()
{
    var data = new byte[2];
    var stream = new MemoryStream(data, false);
    Assert.Throws<NotSupportedException>(() => {
        var payload = new byte[1];
        stream.Write(ReadOnlySpan<byte>.FromArray(in payload));
    });
}

testcase Given_memory_stream_try_get_buffer_exposes_ok_When_executed_Then_memory_stream_try_get_buffer_exposes_ok()
{
    var stream = new MemoryStream();
    let ok = stream.TryGetBuffer(out var buffer);
    let _ = buffer;
    Assert.That(ok).IsTrue();
}

testcase Given_memory_stream_try_get_buffer_exposes_buffer_When_executed_Then_memory_stream_try_get_buffer_exposes_buffer()
{
    var stream = new MemoryStream();
    let _ = stream.TryGetBuffer(out var buffer);
    Assert.That(buffer).IsNotNull();
}

testcase Given_memory_stream_copy_to_output_length_When_executed_Then_memory_stream_copy_to_output_length()
{
    var data = new byte[4];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    data[3] = 4u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    stream.Position = 1;
    var dest = new MemoryStream();
    stream.CopyTo(dest, 2);
    let output = dest.ToArray();
    Assert.That(output.Length).IsEqualTo(3);
}

testcase Given_memory_stream_copy_to_output_first_byte_When_executed_Then_memory_stream_copy_to_output_first_byte()
{
    var data = new byte[4];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    data[3] = 4u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    stream.Position = 1;
    var dest = new MemoryStream();
    stream.CopyTo(dest, 2);
    let output = dest.ToArray();
    Assert.That(output[0]).IsEqualTo(2u8);
}

testcase Given_memory_stream_copy_to_output_last_byte_When_executed_Then_memory_stream_copy_to_output_last_byte()
{
    var data = new byte[4];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    data[3] = 4u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    stream.Position = 1;
    var dest = new MemoryStream();
    stream.CopyTo(dest, 2);
    let output = dest.ToArray();
    Assert.That(output[2]).IsEqualTo(4u8);
}

testcase Given_memory_stream_seek_begin_When_executed_Then_memory_stream_seek_begin()
{
    var data = new byte[5];
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    let pos1 = stream.Seek(2, SeekOrigin.Begin);
    Assert.That(pos1).IsEqualTo(2);
}

testcase Given_memory_stream_seek_current_When_executed_Then_memory_stream_seek_current()
{
    var data = new byte[5];
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    let _ = stream.Seek(2, SeekOrigin.Begin);
    let pos2 = stream.Seek(1, SeekOrigin.Current);
    Assert.That(pos2).IsEqualTo(3);
}

testcase Given_memory_stream_seek_end_When_executed_Then_memory_stream_seek_end()
{
    var data = new byte[5];
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    let pos3 = stream.Seek(-1, SeekOrigin.End);
    Assert.That(pos3).IsEqualTo(4);
}
