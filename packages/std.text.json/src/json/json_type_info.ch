namespace Std.Text.Json;
public class JsonTypeInfo <T >
{
    public JsonSerializerOptions Options {
        get;
        set;
    }
    public init(JsonSerializerOptions options) {
        Options = options;
    }
}
