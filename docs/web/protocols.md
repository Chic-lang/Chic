# Protocol support

## HTTP/1.1 (implemented)

- TCP listener with keep-alive connection reuse.
- Request parser: request line, headers (case-insensitive), `Content-Length`, and `Transfer-Encoding: chunked`.
- Body handling supports fixed-length and chunked requests; responses stream from the response body stream with automatic `Content-Length`.
- Minimal routing and middleware pipeline run per request; exceptions are caught and surfaced as `500` responses.

## HTTP/2 (gated)

- Not implemented yet. Selecting `HttpProtocols.Http2` or any combination that includes HTTP/2 throws `NotSupportedException` with a deterministic message.
- Pending work: TLS + ALPN negotiation, HTTP/2 framing, HPACK, and request stream mapping.

## HTTP/3 (gated)

- Not implemented yet. Selecting `HttpProtocols.Http3` (or combinations that include HTTP/3) throws `NotSupportedException` with a deterministic message.
- Pending work: QUIC transport, TLS 1.3, HTTP/3 framing, and QPACK header compression.

## Protocol selection

- `WebApplicationBuilder.Protocols` defaults to `HttpProtocols.Http1`.
- Setting unsupported protocols fails fast during `WebApplication.RunAsync(...)` to avoid silent downgrades.
- HTTP/2 and HTTP/3 will require TLS once implemented; plaintext upgrades are not planned.
