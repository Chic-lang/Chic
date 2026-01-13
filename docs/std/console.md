# Std.Console

`Std.Console` provides basic console input/output utilities while keeping behavior Chic-native and portable across LLVM and WASM builds.

- **Streams & redirection**: `Console.In/Out/Error` expose `TextReader`/`TextWriter` instances. `SetIn/SetOut/SetError` swap them atomically; `Is*Redirected` reports `true` when a non-terminal or custom stream is attached.
- **Newlines & flushing**: `Console.NewLine` defaults to `Std.Environment.NewLine` (falls back to `"\n"`). `AutoFlush` flushes after each logical write; formatting and WriteLine calls remain atomic under a global lock.
- **Input**: `ReadLine` returns `null` on EOF, never an empty string sentinel. `Read` returns the next code unit or `-1` on exhaustion. `ReadKey`/`KeyAvailable` throw `NotSupportedException` until a portable key decoder lands.
- **Output**: Overloads cover strings/str, numbers, chars, bool and object. Composite formatting supports `{0}` placeholders plus `{{`/`}}` escaping; alignment/format specifiers are ignored for now but parsed deterministically.
- **Terminal features**: Color, cursor visibility/positioning and clear are offered only when a terminal is detected. When unsupported (non-TTY or host opts out), the APIs throw `NotSupportedException` rather than silently no-op. Sizing APIs currently throw on all targets. `NO_COLOR`/`CHIC_NO_COLOR` disable ANSI colour even on terminals.
- **Capability detection**: `Std.Console` relies on `Std.Platform.IO` TTY probes for both LLVM and WASM shims so observable behaviour matches across backends.

`StringReader`/`StringWriter` live alongside Console for lightweight redirection helpers. Behavior is deterministic; the only host-specific pieces are the basic I/O syscalls/bridges exposed via `Std.Platform.IO`.
