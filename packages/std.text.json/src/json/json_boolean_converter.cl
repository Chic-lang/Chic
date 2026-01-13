namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonBooleanConverter : JsonConverter <bool >
{
    public override void Write(Utf8JsonWriter writer, bool value, JsonSerializerOptions options) {
    }
    public override bool Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return false;
    }
}

testcase Given_json_boolean_converter_reads_default_false_When_executed_Then_json_boolean_converter_reads_default_false()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let boolConv = new JsonBooleanConverter();
    boolConv.Write(writer, true, options);
    Assert.That(boolConv.Read(ref reader, options)).IsFalse();
}
