## Weak shared-pointer semantics

Weak pointers provide non-owning access to the allocation managed by `Arc<T>`/`Rc<T>`. They keep the
header alive but do **not** keep the payload alive. The standard `Weak<T>` (`Arc`) and `WeakRc<T>`
(`Rc`) surfaces are Chic-native so async tasks, MIR lowering, and diagnostics can reason about them
without Rust shims.

### Lifetime rules

- Downgrading increments the header’s weak count but leaves the strong count untouched.
- Dropping the final strong handle runs `dispose(ref this)` exactly once and then decrements the weak
  count. The allocation is freed only when both counts reach zero, ensuring weak handles cannot point
  to reclaimed memory.
- Upgrading returns `Arc<T>?` / `Rc<T>?` (sugar for `Option<Arc<T>>`). `null` indicates the payload
  has already been dropped; Chic code uses `if (let value? = weak.Upgrade())` or `??` to branch.
  There are no resurrection semantics: once the payload drops, upgrades permanently fail.

### Runtime hooks

| API | Arc | Rc |
|-----|-----|----|
| `Clone` | `chic_rt_arc_clone(dest, src)` | `chic_rt_rc_clone(dest, src)` |
| `Drop strong` | `chic_rt_arc_drop(handle)` | `chic_rt_rc_drop(handle)` |
| `Downgrade` | `chic_rt_arc_downgrade(destWeak, srcArc)` | `chic_rt_rc_downgrade(destWeak, srcRc)` |
| `Upgrade` | `chic_rt_weak_upgrade(destArc, srcWeak)` | `chic_rt_weak_rc_upgrade(destRc, srcWeak)` |
| `Weak clone` | `chic_rt_weak_clone(dest, src)` | `chic_rt_weak_rc_clone(dest, src)` |
| `Weak drop` | `chic_rt_weak_drop(handle)` | `chic_rt_weak_rc_drop(handle)` |
| `Counts` | `chic_rt_arc_strong_count`, `chic_rt_arc_weak_count` | `chic_rt_rc_strong_count`, `chic_rt_rc_weak_count` |

Codegen and MIR lowering call these helpers instead of touching the headers directly. This keeps the
atomic orderings (`Relaxed` for clone/downgrade, `Release`/`Acquire` for drops, `Acquire` on upgrade)
centralised in the runtime so LLVM/WASM stay in lockstep.

### Standard library surface

- `Arc<T>::Downgrade()` and `Rc<T>::Downgrade()` forward to the runtime downgrade hooks.
- `Weak<T>::Upgrade() -> Arc<T>?` / `WeakRc<T>::Upgrade() -> Rc<T>?` map runtime status codes to
  nullable results. The return type integrates with the language’s `T?` sugar so `if (let arc? =
  weak.Upgrade())` just works.
- `Weak<T>::Clone()` and `dispose` wrap the runtime clone/drop exports; failure paths throw
  `Std::InvalidOperationException` since allocation failure inside the runtime is fatal today.
- `Arc<T>`/`Rc<T>` raw conversions use typed `ValueMutPtr` handles sized/aligned for the runtime
  header so FFI boundaries can validate provenance instead of erasing it into `usize`.

### Auto traits and diagnostics

- Weak handles never carry ownership of `T`, so they inherit `ThreadSafe`/`Shareable`/`Copy = No` from
  the owning smart pointer type. MIR lowering records the same trait metadata so `[MM0102]` and
  borrow-check diagnostics mention the payload type instead of `Weak<T>`.
- Upgrading in unsupported contexts (e.g., WASM builds without threading) produces the same
  `MM0101`/`MM0102` diagnostics used for owning handles.

### Testing

- `runtime::shared::tests` cover downgrade/upgrade cycles, concurrent weak clones/drops, and racey
  upgrade/drop interleavings on both Arc and Rc. The suites assert that strong/weak counts converge
  to zero and that payload `dispose` runs exactly once.
- Integration tests in `tests/threading/thread_suite.cl` exercise downgrades across OS threads and
  ensure upgrading after the payload drops yields `null`.
