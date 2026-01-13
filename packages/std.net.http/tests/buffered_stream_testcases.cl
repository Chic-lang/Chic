namespace Std.Net.Http.Internal;
import Std;
import Std.Span;
import Std.Testing;

testcase Given_buffered_stream_constructor_rejects_null_buffer_When_executed_Then_buffered_stream_constructor_rejects_null_buffer()
{
    Assert.Throws<ArgumentNullException>(() => {
        let _ = new BufferedStream(null);
    });
}

testcase Given_buffered_stream_read_empty_destination_returns_zero_When_executed_Then_buffered_stream_read_empty_destination_returns_zero()
{
    var data = new byte[1] { 0x05u8 };
    var stream = new BufferedStream(data);
    let read = stream.Read(Span<byte>.Empty);
    Assert.That(read).IsEqualTo(0);
}

testcase Given_buffered_stream_read_consumes_buffer_When_executed_Then_buffered_stream_read_consumes_buffer()
{
    var data = new byte[3] { 0x01u8, 0x02u8, 0x03u8 };
    var stream = new BufferedStream(data);

    var first = new byte[2];
    var read = stream.Read(Span<byte>.FromArray(ref first));
    var second = new byte[2];
    let read2 = stream.Read(Span<byte>.FromArray(ref second));
    let read3 = stream.Read(Span<byte>.FromArray(ref second));
    let ok = read == 2
        && first[0] == 0x01u8
        && first[1] == 0x02u8
        && read2 == 1
        && second[0] == 0x03u8
        && read3 == 0;
    Assert.That(ok).IsTrue();
}

testcase Given_buffered_stream_to_array_returns_buffer_When_executed_Then_buffered_stream_to_array_returns_buffer()
{
    var data = new byte[2] { 0x09u8, 0x0Au8 };
    var stream = new BufferedStream(data);
    let result = stream.ToArray();
    let ok = result.Length == 2 && result[0] == 0x09u8 && result[1] == 0x0Au8;
    Assert.That(ok).IsTrue();
}
