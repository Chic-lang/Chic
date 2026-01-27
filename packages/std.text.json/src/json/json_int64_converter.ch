namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonInt64Converter : JsonConverter <long >
{
    public override void Write(Utf8JsonWriter writer, long value, JsonSerializerOptions options) {
    }
    public override long Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return 0L;
    }
}

testcase Given_json_long_converter_reads_default_zero_When_executed_Then_json_long_converter_reads_default_zero()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let longConv = new JsonInt64Converter();
    longConv.Write(writer, 4L, options);
    Assert.That(longConv.Read(ref reader, options)).IsEqualTo(0L);
}
