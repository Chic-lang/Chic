namespace Std.IO;
import Std;
import Std.Numeric;
import Std.Span;
import Std.Testing;

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
