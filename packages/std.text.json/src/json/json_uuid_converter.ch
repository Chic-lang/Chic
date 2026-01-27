namespace Std.Text.Json;
import Std.Span;
import Std.Testing;
public class JsonUuidConverter : JsonConverter <Std.Uuid >
{
    public override void Write(Utf8JsonWriter writer, Std.Uuid value, JsonSerializerOptions options) {
    }
    public override Std.Uuid Read(ref Utf8JsonReader reader, JsonSerializerOptions options) {
        return Std.Core.CoreIntrinsics.DefaultValue <Std.Uuid >();
    }
}
testcase Given_json_uuid_converter_reads_empty_When_executed_Then_json_uuid_converter_reads_empty()
{
    var options = new JsonSerializerOptions();
    var writer = new Utf8JsonWriter(options);
    var reader = new Utf8JsonReader(ReadOnlySpan.FromString("true"));
    let uuidConv = new JsonUuidConverter();
    uuidConv.Write(writer, Std.Uuid.Empty, options);
    let uuid = uuidConv.Read(ref reader, options);
    Assert.That(uuid == Std.Uuid.Empty).IsTrue();
}
