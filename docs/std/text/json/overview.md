# Std.Text.Json overview

Std.Text.Json is a Chic-native JSON stack modeled after `System.Text.Json`. It includes:

- `Utf8JsonReader`: forward-only tokenization of UTF-8 buffers for low-allocation parsing.
- `Utf8JsonWriter`: incremental writer that emits UTF-8 with optional indentation.
- `JsonSerializer`: high-level serialization and deserialization over strings, UTF-8 bytes, and streams.
- `JsonSerializerOptions`: controls indentation, naming policies, null handling, and registered converters.
- `JsonTypeInfo<T>` / `JsonSerializerContext`: compile-time friendly metadata for object graphs.

The stack is deterministic across LLVM/WASM backends and avoids platform JSON shims; parsing and formatting are implemented in Chic.

## Quick examples

Serialize a simple type:

```chic
var person = new Person();
person.Id = 1;
person.Name = "Ada";

var options = new JsonSerializerOptions();
var info = JsonTypeInfo<Person>.CreateObject(options);
info.AddProperty("Id", (ref Person p) => p.Id, (ref Person p, int v) => p.Id = v);
info.AddProperty("Name", (ref Person p) => p.Name, (ref Person p, string v) => p.Name = v);
var ctx = new JsonSerializerContext(options);
ctx.AddTypeInfo(info);
options.TypeInfoResolver = ctx;

let json = JsonSerializer.Serialize(person, options);
```

Parse tokens directly:

```chic
var reader = new Utf8JsonReader("{\"value\":4}".AsUtf8Span());
while (reader.Read())
{
    if (reader.TokenType == JsonTokenType.Number)
    {
        Std.Console.WriteLine(reader.GetInt32().ToString());
    }
}
reader.dispose(ref reader);
```

## Streaming

`JsonSerializer.Deserialize<T>(Std.IO.Stream)` reads from any `Std.IO.Stream` (e.g., `MemoryStream`) in buffered chunks. Writer output can be sent to streams by calling `Serialize(stream, value, options)` or by writing the result of `Utf8JsonWriter.ToArray()` to your target.
