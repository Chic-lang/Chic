namespace Exec;

import Std.IO;
import Std.Text.Json;

public struct BoxedNumber
{
    public int Value;
}

public static class Program
{
    public static int Main()
    {
        var options = new JsonSerializerOptions();
        var info = JsonTypeInfo<BoxedNumber>.CreateObject(options);
        info.AddProperty("Value", (ref BoxedNumber b) => b.Value, (ref BoxedNumber b, int v) => b.Value = v);
        var ctx = new JsonSerializerContext(options);
        ctx.AddTypeInfo(info);
        options.TypeInfoResolver = ctx;

        let payload = "{\"Value\":42}";
        var stream = new MemoryStream();
        let bytes = payload.AsUtf8Span();
        // Write in two chunks to exercise streaming read.
        stream.Write(bytes.Slice(0usize, 5usize));
        stream.Write(bytes.Slice(5usize));
        stream.ResetPosition();

        var parsed = JsonSerializer.Deserialize<BoxedNumber>(stream, options);
        Std.Console.WriteLine(parsed.Value.ToString());
        return 0;
    }
}
