# `#![no_std]` crate mode

Chic supports crate-level `#![no_std]`/`#![std]` attributes that
select the standard-library surface and drive the build/link pipeline. The
attributes must appear before any items; conflicts are diagnosed immediately.

## Quick start

```chic
#![no_std]

namespace Kernel;

public void Start() { }
```

- Add `#![no_std]` to the root file of the crate.
- Build a freestanding target, e.g. `chic build kernel.ch --target aarch64-unknown-none`.
- Executables skip the implicit `Main` requirement in `#![no_std]` mode; provide your own entrypoint or runtime hooks.

## Library layering

- `core` always loads (primitives: `Option`, `Result`, `Span<T>`, `Copy`/`Drop`).
- `alloc` is optional for `#![no_std]` crates; set `CHIC_ENABLE_ALLOC=1` at build
  time to link it. Without the flag the heap stays disconnected.
- `foundation` loads alongside `alloc` and exposes the no_std-friendly surface
  (`Foundation.Collections.Vec`/`Array` views, span slicing helpers, reflection
  metadata, trait utilities) without platform dependencies.
- `std` (platform wrappers, async runtime, POSIX/Apple shims) never loads under
  `#![no_std]`; use `#![std]` or remove the crate attribute to pull it in. Platform
  wrappers remain under `Std`/`Std.Platform` so freestanding code can stay on
  `core`/`foundation`.
- Emitted metadata records `profile=no_std` so backends/linkers and tooling can track the profile.
- For custom entry flows, pair `#![no_std]` with `#![no_main]` (see
  `docs/guides/no_main.md`) to supply your own `start` symbol without pulling in
  the default runtime shims.
- Capability matrix: see `docs/runtime/no_std_feature_matrix.md` for which
  features are available in `core` vs `alloc`/`foundation` vs `std`.
- Smoke samples: see `docs/runtime/no_std_samples.md` for minimal programs and
  build invocations (heapless vs alloc-enabled, WASM notes).
- Layering and allocator selection guidance live in `docs/guides/stdlib_layers.md` and `docs/runtime/allocators.md`.

## Allocators

- `alloc` becomes usable only after an allocator is installed (e.g.,
  `@global_allocator` or `Std.Alloc.Hooks.Install`). On freestanding targets the heap
  remains inert until you provide one.
- Telemetry remains available via `Std.Alloc.Hooks.Telemetry()` even in `#![no_std]`
  builds once `alloc` is linked.

## Targets and examples

- Bare metal: `chic build firmware.ch --target aarch64-unknown-none` (set
  `CHIC_ENABLE_ALLOC=1` if heap types are required).
- WASM-embedded: `chic build module.ch --target wasm32-unknown-unknown` with host
  hooks installed through `WasmExecutionOptions`.

## Diagnostics

- Duplicate or conflicting `#![no_std]`/`#![std]` attributes emit parse errors.
- Namespace-scoped `@no_std` is not supported; use the crate attribute.
- Executables without `Main` are accepted under `#![no_std]`; standard crates still require a `Main` entry.
