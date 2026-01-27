namespace Std.Text.Json;
import Std.Collections;
import Std.Testing;
public class JsonSerializerOptions
{
    public init() {
        Converters = new HashSet <JsonConverter >();
        WriteIndented = false;
        TypeInfoResolver = null;
    }
    public HashSet <JsonConverter >Converters {
        get;
        set;
    }
    public bool WriteIndented {
        get;
        set;
    }
    public JsonSerializerContext ?TypeInfoResolver {
        get;
        set;
    }
}
testcase Given_json_serializer_options_write_indented_default_false_When_executed_Then_json_serializer_options_write_indented_default_false()
{
    var options = new JsonSerializerOptions();
    Assert.That(options.WriteIndented).IsFalse();
}
testcase Given_json_serializer_options_converters_not_null_When_executed_Then_json_serializer_options_converters_not_null()
{
    var options = new JsonSerializerOptions();
    Assert.That(options.Converters).IsNotNull();
}
