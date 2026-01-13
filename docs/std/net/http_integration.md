# HttpClient Integration

`Std.Net.Http.HttpClient` now routes through the shared networking stack:
- Host resolution uses `Std.Net.Dns` (literal IPs short-circuit).
- TCP connections are established with `Std.Net.Sockets.Socket` (IPv4 in the bootstrap).
- Response bodies stream over `Std.IO.NetworkStream` semantics; `StreamContent`/`ByteArrayContent` build on top.

Diagnostics and determinism:
- Cancellation propagates via `CancellationToken` from `HttpClient` to the transport handler and per-request content readers.
- Unsupported targets (e.g., WASM without socket hooks or DNS resolvers) throw `NotSupportedException` rather than hanging.
- Keep-alive sockets are pooled per `(host, port, version, policy)` key; pools are flushed when `CancelPendingRequests` is invoked.

Example:
```chic
var client = new Std.Net.Http.HttpClient();
var response = client.GetAsync("http://127.0.0.1:8080").Result;
var stream = await response.Content.ReadAsStreamAsync();
var buffer = new byte[256];
while (true) {
    let read = stream.Read(Span<byte>.FromArray(ref buffer));
    if (read == 0) break;
    // process chunk...
}
```
