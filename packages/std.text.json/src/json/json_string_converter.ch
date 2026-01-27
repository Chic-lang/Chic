namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonStringConverter : JsonConverter <string >
{
    public override void Write(Utf8JsonWriter writer, string value, JsonSerializerOptions options) {
    }
    public override string Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return Std.Runtime.StringRuntime.Create();
    }
}

testcase Given_json_string_converter_reads_default_empty_When_executed_Then_json_string_converter_reads_default_empty()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let stringConv = new JsonStringConverter();
    stringConv.Write(writer, "value", options);
    Assert.That(stringConv.Read(ref reader, options)).IsEqualTo("");
}
