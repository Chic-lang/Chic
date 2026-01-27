namespace Std.Text.Json;
import Std.Numeric;
import Std.Span;
import Std.Strings;
import Std.Testing;
/// <summary>Minimal stub reader to satisfy current callers.</summary>
public struct Utf8JsonReader
{
    private ReadOnlySpan <byte >_data;
    private long _idx;
    private JsonTokenType _tokenType;
    private ReadOnlySpan <byte >_valueSpan;
    public init(ReadOnlySpan <byte >utf8Json) {
        _data = utf8Json;
        _idx = 0;
        _tokenType = JsonTokenType.None;
        _valueSpan = ReadOnlySpan <byte >.Empty;
    }
    public JsonTokenType TokenType => _tokenType;
    public JsonValueKind ValueKind {
        get {
            if (_tokenType == JsonTokenType.String || _tokenType == JsonTokenType.PropertyName)
            {
                return JsonValueKind.String;
            }
            if (_tokenType == JsonTokenType.True)
            {
                return JsonValueKind.True;
            }
            if (_tokenType == JsonTokenType.False)
            {
                return JsonValueKind.False;
            }
            return JsonValueKind.Undefined;
        }
    }
    public ReadOnlySpan <byte >ValueSpan => _valueSpan;
    public bool HasValueSequence => false;
    public long BytesConsumed => _idx;
    public void dispose(ref this) {
    }
    public bool Read() {
        if (_tokenType != JsonTokenType.None)
        {
            _tokenType = JsonTokenType.None;
            _valueSpan = ReadOnlySpan <byte >.Empty;
            return false;
        }
        _valueSpan = _data;
        _tokenType = JsonTokenType.String;
        _idx = NumericUnchecked.ToInt64(_data.Length);
        return true;
    }
    public string GetString() {
        return Utf8String.FromSpan(_valueSpan);
    }
    public int GetInt32() {
        return 0;
    }
    public long GetInt64() {
        return 0L;
    }
    public double GetDouble() {
        return 0.0d;
    }
    public bool GetBoolean() {
        return false;
    }
    public static void Skip(ref Utf8JsonReader reader) {
        reader.Read();
    }
}

testcase Given_json_reader_first_read_is_true_When_executed_Then_json_reader_first_read_is_true()
{
    let input = ReadOnlySpan.FromString("hi");
    var reader = new Utf8JsonReader(input);
    let first = reader.Read();
    Assert.That(first).IsTrue();
}

testcase Given_json_reader_token_type_is_string_When_executed_Then_json_reader_token_type_is_string()
{
    let input = ReadOnlySpan.FromString("hi");
    var reader = new Utf8JsonReader(input);
    let _ = reader.Read();
    Assert.That(reader.TokenType).IsEqualTo(JsonTokenType.String);
}

testcase Given_json_reader_value_span_matches_length_When_executed_Then_json_reader_value_span_matches_length()
{
    let input = ReadOnlySpan.FromString("hi");
    var reader = new Utf8JsonReader(input);
    let _ = reader.Read();
    Assert.That(reader.ValueSpan.Length).IsEqualTo(input.Length);
}

testcase Given_json_reader_get_string_matches_input_When_executed_Then_json_reader_get_string_matches_input()
{
    let input = ReadOnlySpan.FromString("hi");
    var reader = new Utf8JsonReader(input);
    let _ = reader.Read();
    Assert.That(reader.GetString()).IsEqualTo("hi");
}

testcase Given_json_reader_second_read_is_false_When_executed_Then_json_reader_second_read_is_false()
{
    let input = ReadOnlySpan.FromString("hi");
    var reader = new Utf8JsonReader(input);
    let _ = reader.Read();
    let second = reader.Read();
    Assert.That(second).IsFalse();
}
