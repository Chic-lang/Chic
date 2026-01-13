# FileStream

`Std.IO.FileStream` wraps native file handles (libc `FILE*`) with a span-first interface. It is available on native targets; WASM or platforms without file IO should gate usage and surface `NotSupportedException`.

Construction:

```chic
var fs = new Std.IO.FileStream("data.bin", Std.IO.FileMode.OpenOrCreate, Std.IO.FileAccess.ReadWrite);
```

Capabilities:
- `CanSeek` is true; `Seek`/`Position`/`Length` use `fseek`/`ftell`.
- Async methods delegate to synchronous operations and honor cancellation tokens.
- `SetLength` is not currently supported and throws `NotSupportedException`.

Target notes:
- The implementation uses libc buffered IO; host permissions may affect availability.
- Sharing flags are currently advisory; the platform decides enforcement.
