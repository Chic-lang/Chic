namespace Std.IO;
import Std;
import Std.Async;
import Std.Numeric;
import Std.Span;
import Std.Testing;

private sealed class TestStream : Stream
{
    private byte[] _data;
    private int _readIndex;
    private byte[] _written;
    private int _writtenLength;
    public init(byte[] data) {
        _data = data;
        _readIndex = 0;
        _written = new byte[0];
        _writtenLength = 0;
    }
    public override bool CanRead => true;
    public override bool CanWrite => true;
    public override bool CanSeek => false;
    public override int Read(Span<byte> buffer) {
        ThrowIfDisposed();
        if (buffer.Length == 0)
        {
            return 0;
        }
        let remaining = _data.Length - _readIndex;
        if (remaining <= 0)
        {
            return 0;
        }
        var toCopy = remaining;
        if (toCopy > buffer.Length)
        {
            toCopy = buffer.Length;
        }
        let src = ReadOnlySpan<byte>.FromArray(in _data).Slice(NumericUnchecked.ToUSize(_readIndex), NumericUnchecked.ToUSize(toCopy));
        buffer.Slice(0usize, NumericUnchecked.ToUSize(toCopy)).CopyFrom(src);
        _readIndex += toCopy;
        return toCopy;
    }
    public override void Write(ReadOnlySpan<byte> buffer) {
        ThrowIfDisposed();
        if (buffer.Length == 0)
        {
            return;
        }
        let additional = NumericUnchecked.ToInt32(buffer.Length);
        EnsureWriteCapacity(additional);
        var written = _written;
        let dest = Span<byte>.FromArray(ref written).Slice(NumericUnchecked.ToUSize(_writtenLength), NumericUnchecked.ToUSize(additional));
        dest.CopyFrom(buffer);
        _writtenLength += additional;
    }
    public override void Flush() {
        ThrowIfDisposed();
    }
    public byte[] WrittenBytes() {
        var result = new byte[_writtenLength];
        if (_writtenLength > 0)
        {
            var written = _written;
            let source = ReadOnlySpan<byte>.FromArray(in written).Slice(0usize, NumericUnchecked.ToUSize(_writtenLength));
            Span<byte>.FromArray(ref result).Slice(0usize, NumericUnchecked.ToUSize(_writtenLength)).CopyFrom(source);
        }
        return result;
    }
    private void EnsureWriteCapacity(int additional) {
        let needed = _writtenLength + additional;
        if (needed <= _written.Length)
        {
            return;
        }
        var newSize = _written.Length == 0 ?needed : _written.Length * 2;
        if (newSize < needed)
        {
            newSize = needed;
        }
        var newBuf = new byte[newSize];
        if (_writtenLength > 0)
        {
            var old = _written;
            let source = ReadOnlySpan<byte>.FromArray(in old).Slice(0usize, NumericUnchecked.ToUSize(_writtenLength));
            Span<byte>.FromArray(ref newBuf).Slice(0usize, NumericUnchecked.ToUSize(_writtenLength)).CopyFrom(source);
        }
        _written = newBuf;
    }
}

testcase Given_stream_read_byte_returns_first_When_executed_Then_stream_read_byte_returns_first()
{
    var data = new byte[2];
    data[0] = 0x2Au8;
    data[1] = 0x2Bu8;
    var stream = new TestStream(data);
    Assert.That(stream.ReadByte()).IsEqualTo(0x2A);
}

testcase Given_stream_read_byte_returns_second_When_executed_Then_stream_read_byte_returns_second()
{
    var data = new byte[2];
    data[0] = 0x2Au8;
    data[1] = 0x2Bu8;
    var stream = new TestStream(data);
    let _ = stream.ReadByte();
    Assert.That(stream.ReadByte()).IsEqualTo(0x2B);
}

testcase Given_stream_read_byte_returns_minus_one_on_end_When_executed_Then_stream_read_byte_returns_minus_one_on_end()
{
    var data = new byte[2];
    data[0] = 0x2Au8;
    data[1] = 0x2Bu8;
    var stream = new TestStream(data);
    let _ = stream.ReadByte();
    let _ = stream.ReadByte();
    Assert.That(stream.ReadByte()).IsEqualTo(-1);
}

testcase Given_stream_write_byte_writes_single_When_executed_Then_stream_write_byte_writes_single()
{
    var stream = new TestStream(new byte[0]);
    stream.WriteByte(0x7Fu8);
    let written = stream.WrittenBytes();
    Assert.That(written.Length).IsEqualTo(1);
}

testcase Given_stream_write_byte_writes_value_When_executed_Then_stream_write_byte_writes_value()
{
    var stream = new TestStream(new byte[0]);
    stream.WriteByte(0x7Fu8);
    let written = stream.WrittenBytes();
    Assert.That(written[0]).IsEqualTo(0x7Fu8);
}

testcase Given_stream_copy_to_copies_all_bytes_When_executed_Then_stream_copy_to_copies_all_bytes()
{
    var data = new byte[3];
    data[0] = 1u8;
    data[1] = 2u8;
    data[2] = 3u8;
    var source = new TestStream(data);
    var destination = new MemoryStream();
    source.CopyTo(destination, 2);
    let output = destination.ToArray();
    let expected = ReadOnlySpan<byte>.FromArray(in data);
    Assert.That(ReadOnlySpan<byte>.FromArray(in output)).IsEqualTo(expected);
}

testcase Given_stream_copy_to_rejects_null_destination_When_executed_Then_stream_copy_to_rejects_null_destination()
{
    var source = new TestStream(new byte[0]);
    Assert.Throws<ArgumentNullException>(() => {
        source.CopyTo(null);
    });
}

testcase Given_stream_copy_to_rejects_invalid_buffer_size_When_executed_Then_stream_copy_to_rejects_invalid_buffer_size()
{
    var source = new TestStream(new byte[0]);
    var dest = new MemoryStream();
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        source.CopyTo(dest, 0);
    });
}

testcase Given_stream_read_async_reads_from_sync_stream_When_executed_Then_stream_read_async_reads_from_sync_stream()
{
    var data = new byte[2];
    data[0] = 9u8;
    data[1] = 8u8;
    var source = new TestStream(data);
    var buf = new byte[2];
    let mem = new Memory<byte>(buf);
    let task = source.ReadAsync(mem);
    let read = TaskRuntime.GetResult<int>(task);
    Assert.That(read).IsEqualTo(2);
}

testcase Given_stream_read_async_reads_first_byte_When_executed_Then_stream_read_async_reads_first_byte()
{
    var data = new byte[2];
    data[0] = 9u8;
    data[1] = 8u8;
    var source = new TestStream(data);
    var buf = new byte[2];
    let mem = new Memory<byte>(buf);
    let task = source.ReadAsync(mem);
    let _ = TaskRuntime.GetResult<int>(task);
    Assert.That(buf[0]).IsEqualTo(9u8);
}

testcase Given_stream_read_async_reads_second_byte_When_executed_Then_stream_read_async_reads_second_byte()
{
    var data = new byte[2];
    data[0] = 9u8;
    data[1] = 8u8;
    var source = new TestStream(data);
    var buf = new byte[2];
    let mem = new Memory<byte>(buf);
    let task = source.ReadAsync(mem);
    let _ = TaskRuntime.GetResult<int>(task);
    Assert.That(buf[1]).IsEqualTo(8u8);
}

testcase Given_stream_write_async_writes_to_sync_stream_When_executed_Then_stream_write_async_writes_to_sync_stream()
{
    var source = new TestStream(new byte[0]);
    var data = new byte[2];
    data[0] = 4u8;
    data[1] = 5u8;
    let mem = new ReadOnlyMemory<byte>(data);
    source.WriteAsync(mem);
    let written = source.WrittenBytes();
    Assert.That(written.Length).IsEqualTo(2);
}

testcase Given_stream_write_async_writes_first_byte_When_executed_Then_stream_write_async_writes_first_byte()
{
    var source = new TestStream(new byte[0]);
    var data = new byte[2];
    data[0] = 4u8;
    data[1] = 5u8;
    let mem = new ReadOnlyMemory<byte>(data);
    source.WriteAsync(mem);
    let written = source.WrittenBytes();
    Assert.That(written[0]).IsEqualTo(4u8);
}

testcase Given_stream_write_async_writes_second_byte_When_executed_Then_stream_write_async_writes_second_byte()
{
    var source = new TestStream(new byte[0]);
    var data = new byte[2];
    data[0] = 4u8;
    data[1] = 5u8;
    let mem = new ReadOnlyMemory<byte>(data);
    source.WriteAsync(mem);
    let written = source.WrittenBytes();
    Assert.That(written[1]).IsEqualTo(5u8);
}

testcase Given_stream_read_all_bytes_consumes_stream_When_executed_Then_stream_read_all_bytes_consumes_stream()
{
    var data = new byte[3];
    data[0] = 7u8;
    data[1] = 6u8;
    data[2] = 5u8;
    var source = new TestStream(data);
    let all = source.ReadAllBytes();
    let expected = ReadOnlySpan<byte>.FromArray(in data);
    Assert.That(ReadOnlySpan<byte>.FromArray(in all)).IsEqualTo(expected);
}

testcase Given_stream_copy_to_async_cancels_when_requested_When_executed_Then_stream_copy_to_async_cancels_when_requested()
{
    var source = new TestStream(new byte[1]);
    var dest = new MemoryStream();
    var cts = CancellationTokenSource.Create();
    cts.Cancel();
    Assert.Throws<TaskCanceledException>(() => {
        let _ = source.CopyToAsync(dest, 4, cts.Token());
    });
}

testcase Given_stream_read_throws_after_dispose_When_executed_Then_stream_read_throws_after_dispose()
{
    var source = new TestStream(new byte[1]);
    source.Dispose();
    Assert.Throws<ObjectDisposedException>(() => {
        var tmp = Span<byte>.StackAlloc(1usize);
        let _ = source.Read(tmp);
    });
}
