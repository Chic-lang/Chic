# Foundation Library (no_std surface)

The `Foundation` crate is the portable layer that sits on top of `core` (and
`alloc` when enabled) to provide heap-backed collections and utilities without
pulling in platform wrappers.

- **Location:** `src/foundation`
- **Namespace:** `Foundation.*` (collections, trait helpers, reflection metadata).
- **Loading:** Pulled in automatically when `load_stdlib` is set and
  `CHIC_ENABLE_ALLOC=1`; skipped for `#![no_std]` builds that keep the heap
  disabled.
- **Contents:** `Collections` (Vec/Array views and span slicing helpers),
  `Traits.Debug`, and `Meta` reflection enums. All APIs depend only on `core`
  runtime intrinsics and allocator hooks.

## Collections (Vec/Array)

- `Foundation.Collections.Vec`/`Array` expose span-oriented views and borrow the
  runtime vector layout (`Std.Runtime.Collections::VecPtr`/`ArrayPtr`).
- Inline/heap growth is still driven by the allocator hooks defined in `alloc`;
  no platform IO is required.

## Tips

- Prefer `Foundation.Collections` for heap-backed data in freestanding builds.
- Keep platform wrappers (`Std.Platform.IO`, threading, async) gated in a separate module
  so `#![no_std]` crates can compile with only `core`/`alloc`/`foundation`.
- See `docs/guides/stdlib_layers.md` and `docs/runtime/allocators.md` for allocator configuration details.
- Feature availability across layers is summarised in
  `docs/runtime/no_std_feature_matrix.md`.
