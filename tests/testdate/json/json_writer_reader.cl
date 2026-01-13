namespace Exec;

import Std.Text.Json;
import Std.Span;

public static class Program
{
    public static int Main()
    {
        var writer = new Utf8JsonWriter(new JsonSerializerOptions());
        writer.WriteStartArray();
        writer.WriteNumberValue(3);
        writer.WriteNumberValue(4);
        writer.WriteNumberValue(5);
        writer.WriteEndArray();
        let bytes = writer.ToArray();
        writer.dispose(ref writer);

        var reader = new Utf8JsonReader(ReadOnlySpan<byte>.FromArray(in bytes));
        var sum = 0;
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.Number)
            {
                sum += reader.GetInt32();
            }
        }
        reader.dispose(ref reader);
        Std.Console.WriteLine(sum.ToString());
        return 0;
    }
}
