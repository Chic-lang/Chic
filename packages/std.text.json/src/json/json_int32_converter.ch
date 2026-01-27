namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonInt32Converter : JsonConverter <int >
{
    public override void Write(Utf8JsonWriter writer, int value, JsonSerializerOptions options) {
    }
    public override int Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return 0;
    }
}
testcase Given_json_int_converter_reads_default_zero_When_executed_Then_json_int_converter_reads_default_zero()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let intConv = new JsonInt32Converter();
    intConv.Write(writer, 3, options);
    Assert.That(intConv.Read(ref reader, options)).IsEqualTo(0);
}
