namespace Std.Text.Json;
public class JsonSerializerContext
{
    public JsonSerializerOptions Options {
        get;
        set;
    }
    public init(JsonSerializerOptions options) {
        Options = options;
    }
    public void AddTypeInfo <T >(JsonTypeInfo <T >info) {
    }
}
