namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonByteArrayConverter : JsonConverter <byte[] >
{
    public override void Write(Utf8JsonWriter writer, byte[] value, JsonSerializerOptions options) {
    }
    public override byte[] Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return new byte[0];
    }
}
testcase Given_json_byte_array_converter_reads_empty_When_executed_Then_json_byte_array_converter_reads_empty()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let byteConv = new JsonByteArrayConverter();
    byteConv.Write(writer, new byte[0], options);
    let bytes = byteConv.Read(ref reader, options);
    Assert.That(bytes.Length).IsEqualTo(0usize);
}
