namespace Std.IO;
import Std.Span;
import Std.Testing;

testcase Given_buffered_stream_reader_first_refill_returns_true_When_executed_Then_buffered_stream_reader_first_refill_returns_true()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    Assert.That(reader.Refill()).IsTrue();
}

testcase Given_buffered_stream_reader_first_span_length_When_executed_Then_buffered_stream_reader_first_span_length()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let first = reader.CurrentSpan();
    Assert.That(first.Length).IsEqualTo(2usize);
}

testcase Given_buffered_stream_reader_first_span_first_byte_When_executed_Then_buffered_stream_reader_first_span_first_byte()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let first = reader.CurrentSpan();
    Assert.That(first[0usize]).IsEqualTo(10u8);
}

testcase Given_buffered_stream_reader_first_span_second_byte_When_executed_Then_buffered_stream_reader_first_span_second_byte()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let first = reader.CurrentSpan();
    Assert.That(first[1usize]).IsEqualTo(20u8);
}

testcase Given_buffered_stream_reader_second_refill_returns_true_When_executed_Then_buffered_stream_reader_second_refill_returns_true()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    Assert.That(reader.Refill()).IsTrue();
}

testcase Given_buffered_stream_reader_second_span_length_When_executed_Then_buffered_stream_reader_second_span_length()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let _ = reader.Refill();
    let second = reader.CurrentSpan();
    Assert.That(second.Length).IsEqualTo(1usize);
}

testcase Given_buffered_stream_reader_second_span_byte_When_executed_Then_buffered_stream_reader_second_span_byte()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let _ = reader.Refill();
    let second = reader.CurrentSpan();
    Assert.That(second[0usize]).IsEqualTo(30u8);
}

testcase Given_buffered_stream_reader_final_refill_returns_false_When_executed_Then_buffered_stream_reader_final_refill_returns_false()
{
    var data = new byte[3];
    data[0] = 10u8;
    data[1] = 20u8;
    data[2] = 30u8;
    var stream = new MemoryStream(ReadOnlySpan<byte>.FromArray(in data));
    var reader = new BufferedStreamReader(stream, 2);
    let _ = reader.Refill();
    let _ = reader.Refill();
    Assert.That(reader.Refill()).IsFalse();
}
