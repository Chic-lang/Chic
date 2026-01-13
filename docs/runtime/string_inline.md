# Runtime String Inline Storage

The runtime string representation now includes a 32-byte inline buffer. Strings whose UTF-8 payload
fits in 32 bytes remain in-place without allocating on the heap. Longer strings transparently
promote to the heap-backed representation used previously.

## Behaviour

- Inline strings set the high bit of `ChicString::cap` and keep the payload inside the
  embedded `[u8; 32]` buffer. Extern callers should obtain access through
  `chic_rt_string_as_slice`, which exposes the correct pointer regardless of storage mode.
- When mutation pushes the payload past 32 bytes we migrate into a `Vec<u8>`, preserving the public
  FFI surface. Subsequent shrink operations can re-inline the payload when it fits again.
- Unit tests cover both behaviours (`inline_capacity_handles_small_appends`,
  `exceeding_inline_capacity_promotes_to_heap`) and ensure the inline state remains observable via
  `ChicString::is_inline()`.

## Impact

- Strings ≤ 32 bytes now avoid heap allocation across creation, cloning, and mutation paths. Before
  this change each such operation incurred at least one allocation.
- `cargo test runtime::string` validates the behaviour; no benchmark regressions are expected for
  large strings because the heap-backed path is unchanged.
- SIMD copy paths automatically select AVX2/SSE2/NEON when available via `support::cpu::features()`;
  the behaviour is documented in `docs/runtime/string_simd.md`.

## Benchmarks

Benchmarks captured with `cargo bench --bench runtime_string_push` on the current implementation:

- `runtime_string_push_inline`: 19.93 ns (mean)
- `runtime_string_push_heap`: 57.12 ns (mean)
- `runtime_string_append_unsigned`: 38.88 ns (mean)
