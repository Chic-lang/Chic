# Arc<T> Design Notes

This note captures the normative behaviour of Chic’s atomic reference counting surface so the
runtime, standard library, compiler, and diagnostics stay aligned.

## Goals

- **Native ownership model.** `Std.Sync::Arc<T>` is the “shared ownership across threads/tasks” type.
  It must remain entirely Chic-native (no Rust shims) so the bootstrap compiler, runtime, and
  library evolve together.
- **Thread-safe by construction.** The runtime implementation uses explicit atomic orderings so clone
  and drop remain sound on every supported CPU (x86_64, AArch64, future SIMD targets). The type
  checker propagates auto traits (`ThreadSafe`, `Shareable`) from `T` through `Arc<T>` automatically.
- **Interior mutability hooks.** Helpers such as `get_mut`, `make_mut`, and `pin` emit Chic
  diagnostics when callers violate exclusivity rules or attempt to pin without an
  owned reference.
- **Weak references.** A `Weak<T>` companion type shares the allocation but does not participate in
  destruction. `Arc<T>::downgrade` produces a `Weak<T>`; `Weak<T>::upgrade` returns `Arc<T>` or `null`.
- **Async/thread integration.** `Arc<T>` must participate in async state machines, task schedulers,
  and thread payloads without extra glue. During lowering the compiler records the auto-trait
  requirements so `[MM0102]` continues to fire when payloads are not `ThreadSafe`.
- **Tooling coverage.** The design surfaces metrics (strong/weak counts), diagnostics, and spec/doc
  updates so CLI tests and documentation stay in sync.

## Runtime structure

```
ArcHeader {
    AtomicUsize strong;
    AtomicUsize weak;      // includes strong references (weak = strong + weak-only)
    usize size;            // payload size in bytes
    usize align;           // payload alignment
    usize drop_fn;         // Chic drop glue (or noop)
    u64   type_id;         // metadata handle for debugger/reflection
}
```

- `strong` counts the number of owning `Arc<T>` handles. When `strong` reaches zero the payload is
  dropped exactly once, but the allocation stays alive until `weak` also reaches zero.
- `weak` tracks outstanding `Weak<T>` handles plus the implicit entry that keeps the allocation alive
  while `strong > 0`. `Arc::downgrade` increments `weak`; `Weak::drop` and the final `Arc::drop`
  decrement it. When `weak` reaches zero the header/payload is deallocated.
- Every atomic operation uses explicit orderings:
  - `fetch_add(1, Relaxed)` for clone/downgrade.
  - `fetch_sub(1, Release)` on drop. When the count hits zero, a corresponding `Acquire`
    fence ensures payload drops observe prior writes.
  - Upgrades use `fetch_update` with `(Acquire, Relaxed)` semantics so readers see the payload after
    the fence.
- Allocation uses `TypeMetadata::Resolve<T>()` so the runtime can allocate aligned storage and record
  drop glue/type IDs for tooling.

## Standard library API

Arc is intentionally small; all operations are surfaced in Chic so tooling/tests remain introspectable.

- `Arc<T>::New(value)` allocates a header + payload, registers `__drop_glue_of<T>()`, and records
  `type_id` for reflection. Construction requires `T : ThreadSafe` when the value crosses thread
  boundaries (enforced via the existing auto-trait pipeline).
- `Arc<T>::Clone()` forwards to `chic_rt_arc_clone` and is `O(1)`—it never clones the payload.
  The method will additionally satisfy the upcoming `Clone` trait once trait impls for generics land.
- `Arc<T>::IntoRaw` / `Arc<T>::FromRaw` leak and rehydrate the handle. The raw value is a
  `ValueMutPtr` sized/aligned for `__StdSyncArcHandle`, so FFI boundaries retain typed metadata
  instead of erasing provenance into `usize`. Runtime trampolines (`Std.Platform.Thread.RuntimeCallbacks`)
  and native threads consume these typed handles directly; dropping a leaked raw handle without
  calling `FromRaw` is undefined behaviour.
- `Arc<T>::StrongCount` / `Arc<T>::WeakCount` expose the runtime counters for diagnostics and tests.
- `Arc<T>::Downgrade()` produces a `Weak<T>` handle that keeps the allocation alive but not the payload.
- `Weak<T>::Clone`, `Weak<T>::Drop`, `Weak<T>::Upgrade() -> Arc<T>?`, and the raw helpers live in
  `Std.Sync.Weak<T>` (see [docs/runtime/weak_pointers.md](weak_pointers.md) for the companion design).

### Interior mutability helpers

- `ArcMutableRef<T>` wraps the optional mutable borrow returned by `GetMut`. It exposes `HasValue`
  plus a `ref T Value` accessor so callers can gate mutation on uniqueness without branching on raw
  pointers.
- `Arc<T>::GetMut(inout self) -> ArcMutableRef<T>` succeeds only when `strong == 1` **and**
  `weak == 1` (i.e., no weak-only handles remain). On success it returns an `ArcMutableRef<T>` that
  reborrows the allocation; otherwise it returns an empty wrapper. The borrow checker treats the
  success branch as a unique borrow so writes through the reference remain safe inside async state
  machines.
- `Arc<T>::MakeMut(inout self) -> ref T` guarantees a mutable reference to the payload by cloning
  when necessary. When the counts indicate unique ownership, it forwards to `GetMut`. Otherwise it:
  1. Clones the underlying `T` by calling `Clone()` (which may dispatch through `__clone_glue_of<T>()`
     when invoked from type-erased contexts).
  2. Allocates a fresh header/payload pair.
  3. Drops the previous handle and swaps in the new allocation.
  4. Returns a unique `ref T` pointing at the new payload.
  `MakeMut` therefore requires `where T : Clone` and propagates the constraint through MIR lowering so
  diagnostics remain actionable.
- `Arc<T>::TryUnwrap(self) -> Std.Result<T, Arc<T>>` consumes the handle. When `strong == 1` and
  `weak == 1`, it moves the payload out (using `MaybeUninit<T>` to avoid double drops) and returns
  `Result<T, Arc<T>>.Ok`. Otherwise it returns `Result<T, Arc<T>>.Err(self)` so callers can fall
  back to cloning or retry later. This mirrors Rust’s `Arc::try_unwrap`.
- `Arc<T>::Pin(self) -> Pin<Arc<T>>` (future) will wrap the handle in the eventual `Std.Memory.Pin`
  façade so pinned async borrows can hold onto `Arc<T>` without extra allocations.

### Raw callbacks

The runtime exposes two exported Chic functions (`chic_thread_invoke` and
`chic_thread_drop`) that accept typed raw handles. These trampolines reconstruct `Arc<ThreadStart>`
values via `Arc<T>::FromRaw` and either run `ThreadStart::Run()` or drop the handle. All other host→Chic
interactions use the same typed contract: `ThreadStartDescriptor.Context` is a `ValueMutPtr` carrying
the arc handle pointer plus size/alignment, so the ABI remains stable across LLVM/WASM without ad-hoc
`usize` casts. The stdlib/runtime now validate those `ValueMutPtr` layouts (non-null, sized/aligned for
`__StdSyncArcHandle`) before dispatching, rejecting bad contexts with `ThreadStatus::Invalid` instead of
leaking or double-dropping.

## Compiler integration

- MIR lowering teaches async and thread builders that `Arc<T>` carries the same auto traits as `T`.
- Type metadata exposes `RuntimeArcDescriptor` so reflection/debuggers surface strong/weak counts.
- Borrow checker understands `Arc<T>::get_mut`/`make_mut` as unique borrows when the runtime returns
  success, preventing misuse inside async state machines.
- The auto-trait pipeline records that `Arc<T>` always implements `Copy = No`, `ThreadSafe = T`,
  `Shareable = T`. That metadata is threaded into MIR constraints so diagnostics such as `[MM0102]`
  remain precise.

## Tooling & Tests

- **Unit tests:** `runtime::shared::tests` gain concurrent clone/drop fuzzers using real threads plus
  deterministic harnesses that exercise all atomic orderings. Additional coverage now verifies
  downgrade/upgrade paths and strong/weak count tracking.
- **Language tests:** `tests/threading/thread_suite.cl` (and future async suites) exchange `Arc<T>`
  instances across threads/tasks, downgrade/upgrade loops, and verify final counts.
- **Specification:** `SPEC.md#threading-synchronisation-runtime` and this note
  stay in sync; `docs/guides/concurrency.md` gains an “Arc vs Rc vs Weak” section outlining usage.
