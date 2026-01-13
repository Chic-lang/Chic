# TLS (Std.Security.Tls)

`Std.Security.Tls` now provides Chic-native TLS 1.2/1.3 over any `Std.IO.Stream`, with AES-GCM record protection and X25519 key exchange. The handshake is lightweight but verifies Finished messages and integrates with `Std.Net.Sockets.NetworkStream`.

## What works
- Protocols: TLS 1.3 (preferred) and TLS 1.2 negotiation.
- Cipher suites: `TLS_AES_128_GCM_SHA256` (default) and `TLS_AES_256_GCM_SHA384` via `TlsRecordAead`.
- Key exchange: X25519 with HKDF-based traffic key derivation; Finished MACs validated on both sides.
- Streams: `TlsStream.AuthenticateAsClientAsync/AuthenticateAsServerAsync` wrap any `Stream` and expose plaintext `Read`/`Write` once the handshake completes.
- Exceptions: handshake/alert/certificate/protocol failures surface as `TlsHandshakeException`, `TlsAlertException`, `TlsCertificateException`, or `TlsProtocolException`.

## Certificate handling
- ServerHello carries a raw certificate chain plus the advertised host name. Clients validate the hostname against `TlsClientOptions.ServerName`.
- Trust: if `AllowUntrustedCertificates` is false, the first certificate must byte-match one of the files in `TrustedRootFiles`. Set `AllowUntrustedCertificates = true` for development.
- Signature/OCSP/CRL/path validation are not implemented in this bootstrap; trust is equality-based and should be hardened before production use.

## Usage (client)
```chic
import Std.Security.Tls;
import Std.Net.Sockets;
import Std.IO;

var socket = new Socket(AddressFamily.InterNetwork, SocketType.Stream, ProtocolType.Tcp);
socket.Connect(IPAddress.Parse("93.184.216.34"), 443); // example.com
var network = new NetworkStream(socket, true);
var tls = new TlsStream(network);
var opts = new TlsClientOptions();
opts.ServerName = "example.com";
opts.AllowUntrustedCertificates = true; // configure roots for real deployments
tls.AuthenticateAsClientAsync(opts, CoreIntrinsics.DefaultValue<CancellationToken>());
tls.Write("ping".AsUtf8Span());
var buffer = new byte[4];
let read = tls.Read(Span<byte>.FromArray(ref buffer));
```

## Tests
- Loopback TLS 1.3/1.2 handshakes over TCP with plaintext echo.
- Certificate validation success (trusted root) and hostname mismatch rejection.
- Protocol mismatch rejection (no common version).
- Ciphertext tamper rejection (corrupted record triggers alert).

## Limitations
- Handshake messages remain plaintext; no ALPN/0-RTT/client-auth yet.
- Certificate validation is minimal (hostname + root equality); signature/path/OCSP are not verified.
- HttpClient still uses the bootstrap handler; HTTPS wiring will be layered over `TlsStream`.
- WASM network integration is still gated; the TLS engine itself compiles but sockets may not be available.
