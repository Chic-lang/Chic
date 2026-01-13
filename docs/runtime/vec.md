# Runtime Vec Inline Optimisation

The Chic runtime `Vec` now embeds a 64-byte inline buffer that handles the
common “small vector” case without touching the global allocator. When the
requested capacity fits inside that buffer (`elem_size <= 64` and
`elem_align <= 16`), mutation APIs keep elements in-place; once the buffer is
exhausted we transparently promote to the configured `VecAllocator`
(`GlobalVecAllocator` by default).

## Drop Glue Registration

`Vec<T>` instances created through the standard library façade (`Vec.New<T>()`,
`Vec.WithCapacity<T>()`) resolve the compiler-generated drop glue for `T`
via the `__drop_glue_of<T>()` intrinsic. The LLVM backend emits a per-module
drop table that is registered at startup by calling
`chic_rt_install_drop_table`; the runtime keeps the table alongside any
entries registered dynamically (e.g., host shims) and hands the resolved
callback to `chic_rt_vec_*` APIs. When `T` is trivially droppable the
intrinsic returns `null` and the façade substitutes the shared `__drop_noop`
function (exposed via `Std.Runtime.DropRuntime.DropNoopPtr()`) so container
code remains branch-free.

- Drop glue resolution happens once per element type; subsequent vectors reuse
  the cached pointers without re-querying the intrinsic.
- Runtime tests cover both explicit registration (`chic_rt_drop_register`)
  and table installation to ensure CLI commands (`chic check/run/test`)
  observe the same behaviour.

## Behaviour and Limits

- Inline capacity per element: `inline_cap = 64 / elem_size` (clamped to ≥ 1).
- Alignment support: up to 16 bytes (via `#[repr(C, align(16))]` inline storage).
- Zero-sized types continue to report `cap = usize::MAX`.
- Heap promotion/demotion is automatic; shrinking back to an inline-fit size
  migrates the payload into the embedded buffer and releases heap storage.

### Observing Inline Usage

`ChicVec::uses_inline_storage()` returns `true` whenever the inline buffer
is active. New helper accessors (`storage_ptr`, `inline_capacity`) allow
call-sites to inspect storage without poking internal fields.

## Benchmarks (Apple M1 Pro, Criterion 0.5)

| Benchmark                     | Mean time | Notes                       |
|------------------------------|-----------|-----------------------------|
| `runtime_vec_inline_push_pop`| 62.6 ns   | push/pop within inline cap  |
| `runtime_vec_heap_push_pop`  | 818 ns    | pushes beyond inline buffer |
| `runtime_vec_inline_clear`   | 98.5 ns   | clear inline vector         |

Inline push/pop is ~13× faster than the heap-backed path for the same workload.
Bench artefacts live under `target/criterion/runtime_vec_*`.

## Tuning and Follow-ups

- Bump `INLINE_BYTES` if future workloads benefit (keep alignment ≥ largest
  supported `elem_align`).
- Alternative allocators plug in by implementing `VecAllocator` and threading
  them through the existing trait-based growth APIs.
- Bench suite: `cargo bench --bench runtime_vec_push`.
