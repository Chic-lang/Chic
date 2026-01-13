namespace Std.Text.Json;
public abstract class JsonConverter <T >: JsonConverter
{
    public abstract void Write(Utf8JsonWriter writer, T value, JsonSerializerOptions options);
    public abstract T Read(ref Utf8JsonReader reader, JsonSerializerOptions options);
}
