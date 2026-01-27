# `#![no_std]` smoke samples

Use these snippets to validate freestanding builds. They avoid platform calls
and rely only on `core`/`alloc` (when enabled).

## Bare-metal (host default target)

```chic
#![no_std]

namespace Firmware;

public int Main()
{
    let value = Std.Option<int>.Some(1);
    var buf = Std.Span.Span<int>.StackAlloc(1);
    buf[0] = value.Expect("missing");
    return buf[0];
}
```

Run: `chic check firmware.ch` (defaults to host `*-unknown-none`). The pipeline
loads `core` + the no_std runtime shim; `std` is omitted.

## Heap-enabled (`alloc`/`foundation`)

```chic
#![no_std]
import Foundation.Collections;

namespace Firmware;

public int Main()
{
    var vec = Vec.New<int>();
    Vec.Push(ref vec, 7);
    return Vec.Len(in vec) == 1 ? 0 : 1;
}
```

Run with `CHIC_ENABLE_ALLOC=1 chic check firmware.ch` to pull in `alloc` +
`foundation`.

## WASM-embedded

Use the same snippets with `chic::codegen::wasm::compile` (tests exercise
this path) to emit `wasm32-unknown-unknown` modules without `Std.Platform`
symbols. Ensure `#![no_main]` when supplying a custom start export.
