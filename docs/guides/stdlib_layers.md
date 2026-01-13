# Standard library layering

Chic ships multiple standard-library layers so projects can choose the smallest surface that fits their target and runtime requirements.

## Layers

- **core:** Always linked. Language primitives (`Option`, `Result`, spans, basic traits, intrinsics). No heap and no platform calls.
- **alloc:** Heap abstractions and allocator hooks. Linked only when allocation is enabled and an allocator is installed.
- **foundation:** Portable collections and utilities built on `core` + `alloc` under the `Foundation.*` namespace (no platform calls).
- **std:** Platform wrappers (IO, sockets, threading, async) under `Std` / `Std.Platform`. Not available in `#![no_std]` crates.

## Choosing a layer

- Firmware / freestanding targets: start with `core`, opt into `alloc`/`foundation` only when you need heap-backed collections.
- Hosted applications: use `std` when you need OS services (files, networking, threads).

## Related docs

- `docs/guides/no_std.md`
- `docs/runtime/allocators.md`
