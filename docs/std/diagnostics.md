# Std.Diagnostics

`Std.Diagnostics` provides conditional tracing and assertion helpers while obeying Chic’s LL(1) + predictable MIR rules. Diagnostics output is implemented in Chic and routes through the platform I/O layer.

- **Defines:** `Debug.*` methods are marked `@conditional("DEBUG")`; `Trace.*` are `@conditional("TRACE")` (default `true` in both debug and release). When the symbol is unset/`false`, the call is removed during MIR lowering and arguments are not evaluated.
- **Shared core:** `AutoFlush`, `IndentLevel` (clamped `>=0`), `IndentSize` (clamped `>=0`), and a shared `TraceListenerCollection` back both Debug and Trace. Indentation is applied at the start of each logical line and respects multi-line writes.
- **Listeners:** `TraceListener` exposes `Write`/`WriteLine`/`Flush`/`Close`/`Fail` (idempotent close). The collection supports `Add`/`Remove`/`Clear` and snapshots safely for iteration. A `DefaultTraceListener` targeting stderr is installed by default; `ConsoleTraceListener` can target stdout/stderr; `FileTraceListener` appends to a file and flushes on demand. `AutoFlush = true` flushes every listener after each write.
- **Switches:** `BooleanSwitch` and `TraceSwitch` (`TraceLevel` = Off/Error/Warning/Info/Verbose) read from `Switches.SetOverride(name, value)` overrides first, then environment variables, then their constructor default.
- **Assertions:** `Debug.Assert`/`Fail` emit an “Assertion failed” record (with message/detail + placeholder stack trace) through the listeners, then throw `Std.Diagnostics.AssertFailedException` for deterministic failure handling.
- **Formatting helpers:** `Print`/`WriteLine` overloads accept `object?`, `str?`, or categories; `PrintFormat`/`WriteLineFormat` use the std console formatter rather than variadic params.

Recommended patterns:
- Keep `Trace` enabled in both debug and release, toggling via `--define TRACE=false` only when you must strip tracing.
- Use `Switches.SetOverride` or environment variables to steer `BooleanSwitch`/`TraceSwitch` values without recompiling.
- Clear/add listeners explicitly in tests to avoid stderr noise; Debug and Trace share the same listener collection.
