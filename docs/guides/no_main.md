# `#![no_main]` crate attribute

Use `#![no_main]` when you need to supply a custom entry point (bootloader,
firmware, host-managed runtime) instead of the default Chic `Main`.

## Behaviour

- Disables the implicit `Main` requirement for executables. The compiler no
  longer errors when `Main` is absent.
- Suppresses the generated startup descriptor so native/WASM backends stop
  emitting the default entry shims.
- Works with both `#![no_std]` and `#![std]` crates; the `Std` prelude import is
  still present.
- You must provide a start symbol that your loader understands (e.g.,
  `@extern("C") start` or an exported function the host calls explicitly).

## Examples

### Bare-metal firmware (`#![no_std]`)

```chic
#![no_std]
#![no_main]

@extern("C")
public void start()
{
    // Initialise hardware, then jump to your scheduler.
    while (true) { /* ... */ }
}
```

Build with a freestanding target:

```
CHIC_ENABLE_ALLOC=0 chic build firmware.cl --target aarch64-unknown-none
```

### Hosted runtime override (`#![std]`)

```chic
#![no_main]

namespace Custom;

@extern("C")
public int chic_entry(int argc, *const *const char argv, *const *const char envp)
{
    // Bridge to your runtime; return process exit code.
    return 0;
}
```

Load the resulting symbol from your host or linker script instead of `Main`.

## Notes and diagnostics

- Aliasing `Std` is still rejected (`IMPORT0002`); the implicit prelude remains.
- Backends will not invent a `Main`. If you try to run a `#![no_main]` WASM
  module without exporting a start symbol, the CLI reports the missing entry
  instead of executing.
- Combine with `@suppress_startup_descriptor` only if you also need to disable
  namespace-level startup markers; `#![no_main]` already disables the default
  descriptor.
