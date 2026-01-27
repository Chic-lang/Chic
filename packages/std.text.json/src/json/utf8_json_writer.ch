namespace Std.Text.Json;
import Std.Span;
import Std.Strings;
import Std.Testing;
/// <summary>Lightweight stub writer to keep JSON surface compiling.</summary>
public sealed class Utf8JsonWriter
{
    private string _buffer;
    public init(JsonSerializerOptions ?options = null) {
        _buffer = "";
    }
    public void dispose(ref this) {
    }
    public ReadOnlySpan <byte >WrittenSpan => _buffer.AsUtf8Span();
    public byte[] ToArray() {
        let span = _buffer.AsUtf8Span();
        var result = new byte[span.Length];
        if (span.Length >0usize)
        {
            Span <byte >.FromArray(ref result).Slice(0, span.Length).CopyFrom(span);
        }
        return result;
    }
    public void WriteStartObject() {
    }
    public void WriteEndObject() {
    }
    public void WriteStartArray() {
    }
    public void WriteEndArray() {
    }
    public void WritePropertyName(string name) {
    }
    public void WriteStringValue(string value) {
        _buffer = value;
    }
    public void WriteNumberValue(int value) {
        _buffer = value.ToString();
    }
    public void WriteNumberValue(long value) {
        _buffer = value.ToString();
    }
    public void WriteNumberValue(double value) {
        _buffer = value.ToString();
    }
    public void WriteBooleanValue(bool value) {
        _buffer = value ?"true" : "false";
    }
    public void WriteNullValue() {
        _buffer = "null";
    }
}

testcase Given_json_writer_emits_string_When_executed_Then_json_writer_emits_string()
{
    var writer = new Utf8JsonWriter();
    writer.WriteStringValue("value");
    let text = Utf8String.FromSpan(writer.WrittenSpan);
    Assert.That(text).IsEqualTo("value");
}

testcase Given_json_writer_emits_boolean_When_executed_Then_json_writer_emits_boolean()
{
    var writer = new Utf8JsonWriter();
    writer.WriteBooleanValue(true);
    let boolText = Utf8String.FromSpan(writer.WrittenSpan);
    Assert.That(boolText).IsEqualTo("true");
}

testcase Given_json_writer_emits_null_When_executed_Then_json_writer_emits_null()
{
    var writer = new Utf8JsonWriter();
    writer.WriteNullValue();
    let nullText = Utf8String.FromSpan(writer.WrittenSpan);
    Assert.That(nullText).IsEqualTo("null");
}
