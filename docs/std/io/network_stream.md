# NetworkStream

`Std.IO.NetworkStream` wraps a connected `Std.Net.Sockets.Socket` to expose span-based stream semantics.

Basics:
- Construct with an existing connected socket: `new NetworkStream(socket, ownsSocket: true)`.
- `CanRead`/`CanWrite` are true; `CanSeek` is false. `Seek`/`SetLength`/`Length` throw `NotSupportedException`.
- `Read`/`Write` delegate to the underlying socket span APIs. Async methods are synchronous wrappers that honor cancellation tokens.

Lifecycle:
- If `ownsSocket` is true, disposing the stream closes the socket; otherwise the caller retains ownership.
- Flush is a no-op (sockets write immediately).

Target notes:
- Available on native targets where `Std.Net.Sockets.Socket` is supported; unsupported targets should gate usage and raise `NotSupportedException`.
