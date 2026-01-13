# MemoryStream

`Std.IO.MemoryStream` is a fast, growable, in-memory stream with zero-copy span access.

Capabilities:
- `CanRead`/`CanWrite`/`CanSeek` are always true; `Position` and `Length` track the logical buffer.
- Constructors:
  - `MemoryStream()` creates an empty, writable buffer.
  - `MemoryStream(byte[] buffer, bool writable = true)` wraps an existing byte array.
  - `MemoryStream(ReadOnlySpan<byte> initial)` copies the provided data.
- `TryGetBuffer(out byte[])` exposes the current contents when allowed; `ToArray()` clones the buffer.

Behavior notes:
- Seeking beyond the current length does not grow the logical length until data is written.
- Writing past the end zero-fills the gap.
- `SetLength` truncates or extends (zero-initializing new regions).

Example:

```chic
var ms = new Std.IO.MemoryStream();
ms.WriteByte(1);
var data = new byte[2]; data[0] = 2; data[1] = 3;
ms.Write(ReadOnlySpan<byte>.FromArray(ref data));
ms.Position = 0;
var buffer = new byte[3];
ms.Read(Span<byte>.FromArray(ref buffer));
// buffer is [1,2,3]
```
