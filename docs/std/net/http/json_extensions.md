# HttpClient JSON extensions

`Std.Net.Http.Json` provides `HttpClient` helpers for sending and receiving JSON payloads on top of `Std.Text.Json`.

## Reading JSON

- `GetFromJsonAsync<T>(uri, CancellationToken)`: issues a GET and deserializes the body.
- Overloads accept `JsonSerializerOptions`, `JsonTypeInfo<T>`, `Std.Uri`, and runtime `Std.Type`.
- `DeleteFromJsonAsync<T>` mirrors the GET helpers for DELETE responses.

Requests are issued with `HttpCompletionOption.ResponseContentRead` and run through the standard handler pipeline (timeouts, cancellation, buffering).

## Writing JSON

- `PostAsJsonAsync<T>`, `PutAsJsonAsync<T>`, and `PatchAsJsonAsync<T>` serialize the supplied value and send it with `Content-Type: application/json; charset=utf-8`.
- Overloads accept either `JsonSerializerOptions` or `JsonTypeInfo<T>` to control serialization.

## Type metadata and options

When a `JsonTypeInfo<T>` is supplied, the helper creates an options instance that registers the type info in a `JsonSerializerContext`. If you pass a `JsonSerializerOptions` instance with an existing context, it is reused so caller-provided converters remain active.

## Cancellation and errors

All helpers honor the provided `CancellationToken` and propagate `HttpRequestException` if the response content is missing or deserialization fails.
