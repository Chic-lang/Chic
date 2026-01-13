# `#![no_std]` Feature Matrix

This matrix summarises which runtime features are available in `core`,
`alloc`/`foundation`, and `std` for freestanding builds. Use it to plan
portable code paths and avoid accidentally pulling in platform-dependent
symbols.

| Feature | core | alloc/foundation (`CHIC_ENABLE_ALLOC=1`) | std |
| --- | --- | --- | --- |
| Primitives (`Option`, `Result`, spans, ranges, `Copy`/`Drop`) | ✅ | ✅ | ✅ |
| Strings (`string`, span views) | ✅ (internals only) | ✅ | ✅ |
| Heap allocations (`Vec`, `Array`, `String`) | ❌ | ✅ | ✅ |
| Atomics | ✅ (stubs; map to runtime intrinsics) | ✅ | ✅ |
| SIMD stubs | ✅ (feature-detected in support/cpu) | ✅ | ✅ |
| Platform IO (files, sockets, threads, clocks, async runtime) | ❌ | ❌ | ✅ |
| Panic/abort handlers | ✅ (`core` calls `chic_rt_panic/abort`) | ✅ | ✅ |
| Startup descriptor | ❌ | ❌ | ✅ |

Notes:

- `core` never depends on `std`; it links with the no_std runtime shim that
  exports `chic_rt_panic`/`chic_rt_abort`.
- `foundation` requires `alloc` and is pulled in when `CHIC_ENABLE_ALLOC=1` is
  set at build time (even for `#![no_std]` crates).
- `std` is skipped entirely for `#![no_std]`; the implicit `Std` prelude remains
  in scope but `Std.Platform.*` symbols are unavailable.

References: `docs/guides/no_std.md`, `docs/guides/no_main.md`,
`docs/runtime/startup.md`.
