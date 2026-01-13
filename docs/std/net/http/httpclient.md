# Std.Net.Http.HttpClient (bootstrap)

This bootstrap implementation provides a minimal HTTP/1.1 client in Chic. It follows a familiar `HttpClient`-style surface (constructors, properties, send/convenience overloads) but is intentionally limited:

- Transport: the default handler still returns stub responses while the real pipeline is being wired. TCP sockets are available, and TLS is now exposed via `Std.Security.Tls.TlsStream`, but HTTPS requests are not yet streamed over the wire.
- Version/policy: requests default to HTTP/1.1; other versions/policies are accepted but ignored by the transport.
- Completion: `ResponseHeadersRead` returns immediately with a streaming content; buffering is used only if `ResponseContentRead` is requested.
- Buffering: `MaxResponseContentBufferSize` is enforced; content exceeding the limit fails the request. Set to -1 for “no limit” (still subject to memory).
- Timeout/cancellation: `Timeout` and `CancellationToken` (plus `CancelPendingRequests`) are checked between IO operations; blocking sockets mean cancellation granularity is coarse but still deterministic. Set `Timeout = Std.Datetime.Duration.Infinite` to disable.
- JSON helpers: the `Std.Net.Http.Json` extensions are backed by `Std.Text.Json` and provide common convenience APIs (GetFromJsonAsync, PostAsJsonAsync, etc.).
- Connection reuse: a simple pool keeps sockets alive per host/port/policy/timeout/buffer settings when responses are fully read; streaming responses return sockets to the pool once consumption completes.

## Usage

```cl
import Std.Net.Http;

public int Main()
{
    var client = new HttpClient();
    client.BaseAddress = new Std.Uri("http://127.0.0.1:8080", Std.UriKind.Absolute);

    var response = client.GetAsync("/ping").Scope(); // synchronous scope helper
    var body = response.Content.ReadAsString();
    Std.Console.WriteLine(body);
    client.Dispose();
    return 0;
}
```

## Handlers and disposal

- `HttpClient()` creates and owns a default `HttpClientHandler`; disposing the client disposes the handler.
- The `HttpClient(HttpMessageHandler, bool disposeHandler)` constructor lets callers supply custom handlers and control disposal.

## Properties

- `Timeout` (default 100 seconds) is enforced across connect/send/receive operations; `Std.Datetime.Duration.Infinite` disables it. `CancelPendingRequests()` cancels in-flight operations and clears the connection pool.
- `BaseAddress`, `DefaultRequestHeaders`, `DefaultRequestVersion`, and `DefaultVersionPolicy` are applied to new requests.
- Convenience helpers cover GET/POST/PUT/PATCH/DELETE as well as HEAD/OPTIONS, and all delegate to the canonical send pipeline.

## TLS

- `Std.Security.Tls.TlsStream` provides TLS 1.2/1.3 over any `Stream`. The HTTP transport will adopt it for `https://` once the real socket-backed pipeline replaces the bootstrap handler.

## Compression

- Responses with `Content-Encoding: gzip` or `Content-Encoding: deflate` are transparently decoded. The handler replaces the content with the decompressed payload and rewrites `Content-Length` after decoding.

## JSON extensions

The `Std.Net.Http.Json` namespace wires the Chic serializer into HTTP helpers. Pass `JsonSerializerOptions` to control naming policies or indentation, or supply `JsonTypeInfo<T>` metadata when using compiled contexts. All helpers honor cancellation tokens and emit `HttpRequestException` if the response payload is missing or malformed.
