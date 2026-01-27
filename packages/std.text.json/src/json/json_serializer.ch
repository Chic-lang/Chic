namespace Std.Text.Json;
import Std.Runtime;
import Std.Strings;
import Std.Testing;
/// <summary>Stub serializer for compilation.</summary>
public static class JsonSerializer
{
    public static string Serialize <T >(T value, JsonSerializerOptions ?options = null) {
        return StringRuntime.Create();
    }
    public static string Serialize(object ?value, Std.Type returnType, JsonSerializerOptions ?options = null) {
        return StringRuntime.Create();
    }
    public static T Deserialize <T >(string text, JsonSerializerOptions ?options = null) {
        return Std.Core.CoreIntrinsics.DefaultValue <T >();
    }
    public static object ?Deserialize(string text, Std.Type returnType, JsonSerializerOptions ?options = null) {
        return null;
    }
}

testcase Given_json_serializer_serialize_returns_empty_When_executed_Then_json_serializer_serialize_returns_empty()
{
    let text = JsonSerializer.Serialize(42);
    Assert.That(text.Length).IsEqualTo(0);
}

testcase Given_json_serializer_deserialize_returns_default_When_executed_Then_json_serializer_deserialize_returns_default()
{
    let value = JsonSerializer.Deserialize<int>("123");
    Assert.That(value).IsEqualTo(0);
}
