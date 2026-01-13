# JsonSerializer

`JsonSerializer` provides object-level serialization on top of the UTF-8 reader/writer. The surface follows common JSON serializer conventions with Chic-specific constraints.

## Options

- `WriteIndented`: pretty-print output with two-space indentation.
- `PropertyNameCaseInsensitive`: permit case-insensitive property matching during deserialization.
- `PropertyNamingPolicy`: apply a naming policy (for example, `JsonNamingPolicy.CamelCase`) to property names.
- `IgnoreNullValues`: skip null reference properties during serialization.
- `Converters`: register custom `JsonConverter` instances.
- `DefaultBufferSize`: streaming read buffer length; defaults to 4096 bytes.

## Type metadata

`JsonTypeInfo<T>` and `JsonSerializerContext` describe how to serialize object graphs without runtime reflection. Use `JsonTypeInfo<T>.CreateObject(options)` to build metadata, add properties via `AddProperty`, and register the type info with a context:

```chic
var options = new JsonSerializerOptions();
var info = JsonTypeInfo<User>.CreateObject(options);
info.AddProperty("Id", (ref User u) => u.Id, (ref User u, int v) => u.Id = v);
info.AddProperty("Name", (ref User u) => u.Name, (ref User u, string v) => u.Name = v);
var ctx = new JsonSerializerContext(options);
ctx.AddTypeInfo(info);
options.TypeInfoResolver = ctx;
```

The serializer consults the context first; if no converter is found it falls back to built-in converters for primitives and selected arrays. Unknown types throw `NotSupportedException` rather than silently emitting invalid JSON.

## Streaming usage

- `Serialize(Stream utf8Json, T value, JsonSerializerOptions? options = null)` writes UTF-8 to any `Std.IO.Stream`.
- `Deserialize<T>(Stream utf8Json, JsonSerializerOptions? options = null)` reads from a stream in buffered chunks; partial reads are handled deterministically.

## Converters

Built-in converters handle:

- Primitives: `bool`, `string`, `int`, `long`, `double`
- Arrays: `byte[]`, `string[]`, `int[]`, `long[]`, `double[]`

Register additional converters through `options.Converters.Add(...)`. Custom converters implement `JsonConverter<T>` and provide `Read`/`Write` methods over `Utf8JsonReader`/`Utf8JsonWriter`.

## Error model

- Malformed JSON raises `JsonException`.
- Unsupported types raise `NotSupportedException`.
- Buffer growth failures or invalid numeric formats surface deterministic exceptions to ease debugging across platforms.
