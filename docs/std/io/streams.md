# Std.IO.Streams

`Std.IO.Stream` is the span-first, async-friendly base abstraction for Chic I/O. Streams expose span-based `Read`/`Write`, async counterparts, seeking, and deterministic disposal.

Key points:
- `Read(Span<byte>)`/`Write(ReadOnlySpan<byte>)` are the primary APIs; async wrappers honor `CancellationToken` and throw `TaskCanceledException` when requested.
- `Seek`/`Position`/`Length` are available when `CanSeek` is true; non-seekable streams throw `NotSupportedException`.
- `CopyTo`/`CopyToAsync` copy between streams using reusable buffers (default 81,920 bytes).
- `ReadByte`/`WriteByte` helpers are convenience wrappers.

Example:

```chic
var source = new Std.IO.MemoryStream();
var data = new byte[3]; data[0] = 1; data[1] = 2; data[2] = 3;
source.Write(ReadOnlySpan<byte>.FromArray(ref data));
source.Position = 0;
var dest = new Std.IO.MemoryStream();
source.CopyTo(dest);
```

Cancellation and disposal:
- Async methods check cancellation before performing work.
- Disposing a stream (or letting `dispose` run) closes underlying resources; further use throws `ObjectDisposedException`.
