namespace Std.IO;
import Std;
import Std.Testing;

testcase Given_memory_null_array_is_empty_When_executed_Then_memory_null_array_is_empty()
{
    let mem = new Memory<int>(null);
    Assert.That(mem.Length).IsEqualTo(0);
}

testcase Given_memory_null_array_span_length_zero_When_executed_Then_memory_null_array_span_length_zero()
{
    let mem = new Memory<int>(null);
    Assert.That(mem.Span.Length).IsEqualTo(0usize);
}

testcase Given_memory_slice_length_two_When_executed_Then_memory_slice_length_two()
{
    var data = new int[4];
    data[0] = 10;
    data[1] = 20;
    data[2] = 30;
    data[3] = 40;
    let mem = new Memory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(mem.Length).IsEqualTo(2);
}

testcase Given_memory_slice_span_first_value_When_executed_Then_memory_slice_span_first_value()
{
    var data = new int[4];
    data[0] = 10;
    data[1] = 20;
    data[2] = 30;
    data[3] = 40;
    let mem = new Memory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(span[0usize]).IsEqualTo(20);
}

testcase Given_memory_slice_span_second_value_When_executed_Then_memory_slice_span_second_value()
{
    var data = new int[4];
    data[0] = 10;
    data[1] = 20;
    data[2] = 30;
    data[3] = 40;
    let mem = new Memory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(span[1usize]).IsEqualTo(30);
}

testcase Given_memory_invalid_range_null_throws_When_executed_Then_memory_invalid_range_null_throws()
{
    Assert.Throws<ArgumentNullException>(() => {
        let _ = new Memory<int>(null, 0, 1);
    });
}

testcase Given_memory_invalid_range_negative_start_throws_When_executed_Then_memory_invalid_range_negative_start_throws()
{
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        var data = new int[2];
        let _ = new Memory<int>(data, -1, 1);
    });
}

testcase Given_memory_invalid_range_length_throws_When_executed_Then_memory_invalid_range_length_throws()
{
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        var data = new int[2];
        let _ = new Memory<int>(data, 1, 2);
    });
}

testcase Given_read_only_memory_null_array_is_empty_When_executed_Then_read_only_memory_null_array_is_empty()
{
    let mem = new ReadOnlyMemory<int>(null);
    Assert.That(mem.Length).IsEqualTo(0);
}

testcase Given_read_only_memory_null_array_span_length_zero_When_executed_Then_read_only_memory_null_array_span_length_zero()
{
    let mem = new ReadOnlyMemory<int>(null);
    Assert.That(mem.Span.Length).IsEqualTo(0usize);
}

testcase Given_read_only_memory_slice_length_two_When_executed_Then_read_only_memory_slice_length_two()
{
    var data = new int[3];
    data[0] = 1;
    data[1] = 2;
    data[2] = 3;
    let mem = new ReadOnlyMemory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(mem.Length).IsEqualTo(2);
}

testcase Given_read_only_memory_slice_span_first_value_When_executed_Then_read_only_memory_slice_span_first_value()
{
    var data = new int[3];
    data[0] = 1;
    data[1] = 2;
    data[2] = 3;
    let mem = new ReadOnlyMemory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(span[0usize]).IsEqualTo(2);
}

testcase Given_read_only_memory_slice_span_second_value_When_executed_Then_read_only_memory_slice_span_second_value()
{
    var data = new int[3];
    data[0] = 1;
    data[1] = 2;
    data[2] = 3;
    let mem = new ReadOnlyMemory<int>(data, 1, 2);
    let span = mem.Span;
    Assert.That(span[1usize]).IsEqualTo(3);
}

testcase Given_read_only_memory_invalid_range_null_throws_When_executed_Then_read_only_memory_invalid_range_null_throws()
{
    Assert.Throws<ArgumentNullException>(() => {
        let _ = new ReadOnlyMemory<int>(null, 1, 1);
    });
}

testcase Given_read_only_memory_invalid_range_out_of_range_throws_When_executed_Then_read_only_memory_invalid_range_out_of_range_throws()
{
    Assert.Throws<ArgumentOutOfRangeException>(() => {
        var data = new int[2];
        let _ = new ReadOnlyMemory<int>(data, 2, 1);
    });
}
