# TLS and certificates

- The current HTTP/1.1 listener runs over plaintext TCP. TLS bindings in `Std` are not available yet.
- HTTP/2 and HTTP/3 require TLS/ALPN; protocol selection is gated with `NotSupportedException` until the TLS/QUIC stack lands.
- Planned surface:
  - Configure certificates on the `WebApplicationBuilder` (file paths or in-memory cert chains).
  - ALPN negotiation for `h2` and `h3`.
  - Deterministic fallback diagnostics when TLS is unavailable on the target.
- Until TLS support ships, prefer running behind a terminating proxy for encrypted traffic.
