# POSIX/Apple IO Wrappers

Chic ships lightweight `Std.Platform.IO.File` wrappers over the C stdio layer to
provide safe file access without pulling in platform-specific Rust shims.

- **Surface:** `Std.Platform.IO.File` (`OpenRead`, `OpenWrite`, `Read`, `Write`, `Flush`, `Close`), `Std.Platform.IO.Socket` (TCP/IPv4 connect/send/recv/shutdown/close), and `Std.Platform.Time` (`MonotonicNanoseconds`, `SleepMillis`).
- **Backend:** `fopen`/`fread`/`fwrite`/`fflush`/`fclose` via `@extern("C")` on native targets; the WASM executor imports `env.fopen`/`env.fread`/`env.fwrite`/`env.fflush`/`env.fclose` and routes them through `WasmExecutionOptions.io_hooks` (or the built-in host filesystem shim when hooks are `None`).
- **Encoding:** Paths/modes are UTF-8 encoded with stack-allocated buffers and
  null terminators; callers supply `string` values.
- **Errors:** Return `IoError` codes (`InvalidPointer`, `Eof`, `Unknown`) with
  best-effort mapping from stdio results.
- **Sockets:** IPv4-only today. `Socket.CreateTcp` opens an AF_INET/SOCK_STREAM
  descriptor, `Connect`/`Send`/`Receive` wrap `connect`/`send`/`recv`, and
  `ShutdownWrite`/`Close` map to the corresponding POSIX syscalls. Errors map to
  `SocketError` values; more detailed errno mapping is planned. WASM builds import
  `env.socket`/`env.connect`/`env.send`/`env.recv`/`env.shutdown`/`env.close`
  plus `env.htons`/`env.inet_pton`, and route them through host hooks or the
  built-in loopback shim.
- **Time:** `Time.MonotonicNanoseconds` uses `clock_gettime(CLOCK_MONOTONIC)` on
  native targets and `env.monotonic_nanos` in the WASM executor; `SleepMillis`
  uses `nanosleep`/`env.sleep_millis` with hook overrides available for
  deterministic tests.
- **WASM:** WASM builds now include `Std.Platform.IO.File`, `Std.Platform.IO.Socket`, and
  `Std.Platform.Time`; `WasmExecutionOptions.io_hooks` can override filesystem/sockets/time
  behaviour while the executor provides a default host-backed implementation for
  local runs.

## Examples

```chic
import Std.Platform.IO;
import Std.Span;
import Std.Strings;

IoError err;
var file = File.OpenRead("/tmp/data.txt", out err);
var buf = Span<byte>.StackAlloc(256);
usize read;
if (file.Read(buf, out read, out err) && err == IoError.Success) {
    let text = Utf8String.FromSpan(buf.AsReadOnly().Slice(0, read));
}
file.Close(out err);
```

```chic
import Std.Platform.IO;
import Std.Strings;

Socket sock;
if (Socket.CreateTcp(out sock) == SocketError.Success)
{
    var addr = Ipv4Address.Loopback();
    var err = sock.Connect(addr, 8080);
    if (err == SocketError.Success)
    {
        sock.Send("ping".AsUtf8Span(), out var _);
        var buf = Span<byte>.StackAlloc(16);
        sock.Receive(buf, out var _);
    }
    sock.Close();
}
```
