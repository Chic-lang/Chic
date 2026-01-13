namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonDoubleConverter : JsonConverter <double >
{
    public override void Write(Utf8JsonWriter writer, double value, JsonSerializerOptions options) {
    }
    public override double Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return 0.0;
    }
}

testcase Given_json_double_converter_reads_default_zero_When_executed_Then_json_double_converter_reads_default_zero()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let doubleConv = new JsonDoubleConverter();
    doubleConv.Write(writer, 3.5d, options);
    Assert.That(doubleConv.Read(ref reader, options)).IsEqualTo(0.0d);
}
