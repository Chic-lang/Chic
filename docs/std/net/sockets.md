# Std.Net.Sockets

`Std.Net.Sockets` provides span-first socket APIs for TCP/UDP/RAW scenarios. The bootstrap targets IPv4; IPv6 and advanced options are gated.

Key types:
- `Socket`: created with `(AddressFamily, SocketType, ProtocolType)`; exposes span-based `Send`/`Receive`, async wrappers, `Connect`/`Bind`/`Listen`/`Accept`, and datagram helpers `SendTo`/`ReceiveFrom`.
- `IPAddress`, `IPEndPoint`, `AddressFamily` complete the addressing surface.
- `SocketFlags`/`SocketShutdown` provide common socket flags and shutdown options.

Usage (TCP echo):
```chic
var server = new Std.Net.Sockets.Socket(Std.Net.AddressFamily.InterNetwork, Std.Net.Sockets.SocketType.Stream, Std.Net.Sockets.ProtocolType.Tcp);
server.Bind(new Std.Net.IPEndPoint(Std.Net.IPAddress.Parse("127.0.0.1"), 8080));
server.Listen(1);
var clientTask = Std.Async.TaskRuntime.FromResult(new Std.Net.Sockets.Socket(Std.Net.AddressFamily.InterNetwork, Std.Net.Sockets.SocketType.Stream, Std.Net.Sockets.ProtocolType.Tcp));
let client = clientTask.InnerFuture.Result;
client.Connect(Std.Net.IPAddress.Parse("127.0.0.1"), 8080);
let accepted = server.Accept();
var buffer = new byte[3]; buffer[0] = 1; buffer[1] = 2; buffer[2] = 3;
client.Send(ReadOnlySpan<byte>.FromArray(ref buffer));
var recv = Span<byte>.StackAlloc(3);
accepted.Receive(recv);
```

Target notes:
- IPv4 sockets are supported on native targets via libc; IPv6 and dual-mode sockets are currently gated.
- Raw sockets may require elevated privileges; creation failures return `SocketError.PermissionDenied`/`SocketError.Unsupported`.
- WASM and hosts without socket plumbing should throw `NotSupportedException` deterministically.
