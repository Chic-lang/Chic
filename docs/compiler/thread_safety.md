# Thread Safety Enforcement

Chic’s bootstrap compiler ships auto-trait based concurrency guarantees that forbid data races across async suspension points and thread boundaries. The implementation combines type-layout driven trait inference, constraint emission during MIR construction, and borrow-checking guards so values only cross concurrency frontiers when they implement the required traits.

## Auto-trait inference

- `TypeLayoutTable::finalize_auto_traits` walks every nominal layout, collapsing the auto-trait status of each field, variant, or view into a summarised `AutoTraitSet` so later passes can query `ThreadSafe` / `Shareable` directly (`src/mir/layout/auto_traits.rs:48` – `src/mir/layout/auto_traits.rs:147`).
- Attribute overrides (e.g., `@thread_safe(false)`) and stdlib primitives (`Arc<T>`, guards) participate in the lattice via `AutoTraitOverride`, letting the runtime opt types into or out of the traits while preserving downstream inference (`src/mir/layout/auto_traits.rs:73` – `src/mir/layout/auto_traits.rs:155`).
- Standard synchronisation wrappers under `std.sync` provide trait-safe escape hatches; `MutexGuard<T>` and friends are documented as satisfying both auto traits so locking code can cross awaits without additional annotations (`packages/std/src/std_sync.cl:1` – `packages/std/src/std_sync.cl:60`).
- Generic constraints (`where T : @thread_safe`) now flow through the same inference path:
  the parser records the annotations, `TypeChecker::validate_generic_arguments`
  enforces them (raising `[TCK035]` / `[TCK037]`), and the regression suite
  contains representative coverage in
  `typeck::arena::tests::diagnostics::auto_trait_constraint_*`.
- Traits annotated with `@thread_safe` / `@shareable` propagate those requirements to every
  implementation. The trait solver consults the trait declaration, emits a
  `ConstraintKind::RequiresAutoTrait` with origin `Generic`, and surfaces
  `[TCK035]` / `[TCK037]` if the target type fails the check; see
  `typeck::arena::tests::diagnostics::trait_impl_requires_thread_safe_when_trait_marked_thread_safe`.

## Constraint emission at async boundaries

- During MIR lowering the async builder records locals that survive across suspension points. For each surviving local it emits a `ConstraintKind::RequiresAutoTrait` pointing to the owning function, capture site, and the trait that must hold (`src/mir/builder/body_builder/async_support.rs:193` – `src/mir/builder/body_builder/async_support.rs:205`).
- `TypeChecker::check_constraints` consumes those requirements and defers to `ensure_auto_trait`, which resolves the target’s layout, inspects the derived `AutoTraitSet`, and reports trait violations with actionable guidance (`src/typeck/coercions.rs:24` – `src/typeck/coercions.rs:75`, `src/typeck/traits.rs:248` – `src/typeck/traits.rs:296`).
- When a captured local’s type is a generic parameter (e.g., `T` in `Runner<T>`), the async builder now falls back to the canonical type name if no layout exists. The type checker consults the surrounding type/function generics before emitting `[TCK035]`/`[TCK037]`, so `where T : @thread_safe` or `where T : @shareable` clauses satisfy the constraint even though no concrete layout is available (`typeck::arena::tests::diagnostics::auto_trait_constraint_respected_for_async_generic_context`).

## Diagnostics surfaced today

- `TCK035` (`AUTO_TRAIT_REQUIRED`) fires when a captured value definitively lacks the requested trait, while `TCK037` (`AUTO_TRAIT_UNPROVEN`) covers indeterminate cases; both diagnostics suggest wrapping the value in `std.sync::Mutex`, `std.sync::RwLock`, or atomics to make intent explicit (`src/typeck/traits.rs:265` – `src/typeck/traits.rs:296`).
- The MIR borrow checker double-checks pinned async locals. If a pinned unique borrow crosses an `await` without satisfying `ThreadSafe`, it raises an error tagged with `AwaitRequiresThreadSafe` so the diagnostic points at the offending suspension site (`src/mir/borrow/context/checker/terminators.rs:257` – `src/mir/borrow/context/checker/terminators.rs:291`).

| Code    | Trigger | Surfaces | Fix guidance |
|---------|---------|----------|--------------|
| `TCK022` | Variance misuse (covariant parameter in setter, variance on classes, etc.). | Parser/type checker. | Remove the modifier or restructure the signature; see `SPEC.md#generic-parameter-variance`. |
| `TCK035` | Definitive auto-trait violation when a value crosses an async/thread boundary. | Type checker, async lowering, MIR builders. | Add `where T : @thread_safe` / `@shareable`, wrap payloads in `std.sync` guards, or refactor the type’s fields. |
| `TCK037` | Auto-trait requirement cannot be proven (typically unconstrained generics). | Type checker. | Constrain the type parameter, bubble the requirement upward, or provide a safe wrapper type. |
| `MM0102` | Attempting to spawn a thread/task with a non-`ThreadSafe` payload. | MIR lowering for `std.thread::spawn`/`Task.Run`. | Guard the call with `if (!std.thread::IsSupported)` or wrap payloads so the trait holds. |

## Regression coverage

- Type-checker arena tests validate that missing auto traits produce the correct diagnostics and suggestions (`src/typeck/arena/tests/diagnostics.rs:136` – `src/typeck/arena/tests/diagnostics.rs:183`).
- Borrow-checker async harnesses demonstrate both the accepted and rejected execution paths, including the pinned `await` failure that must mention `ThreadSafe` (`src/mir/borrow/tests/async/pinned.rs:78` – `src/mir/borrow/tests/async/pinned.rs:159`).

## Working the diagnostics

1. Reproduce the failure with `cargo test typeck::arena::tests::diagnostics::requires_thread_safe_trait_for_async_locals` or the borrow harnesses to confirm behaviour locally.
2. Wrap the captured value in `std.sync::Mutex` / `RwLock` or refactor the type so every field implements the desired trait; the type checker re-derives auto traits from the updated layout.
3. Re-run the relevant unit suites and async borrow tests to ensure no regression remains.

For a narrative overview of the concurrency model see the updated specification section in `SPEC.md`.

## Tooling & lint integrations

- `src/typeck/spec_link_table.in` maps every diagnostic code above to a spec anchor.
- Editor adapters and linters reuse the same metadata, including the spec links emitted for `TCK022`, `TCK035`, `TCK037`, and `MM0102`.
- `cargo xtask lint-sizes` now tracks the concurrency docs so future contributors keep each section focused (<200 LOC). Update the allowlist if major expansions are required, but prefer splitting into targeted guides (e.g., `docs/guides/concurrency.md`) instead.

## Guidance for container and async authors

1. **Audit payload fields:** Use the layout dump instrumentation (`TypeLayoutTable::debug_string` or the upcoming `chic layout` CLI) to inspect derived auto traits before wiring a type into channels/executors. Mark intentionally unsafe fields with `@not_thread_safe` and document the rationale.
2. **Bubble constraints:** Prefer `where T : @thread_safe, @shareable` on containers (`Task<T>`, `Channel<T>`, pools) so callers see the requirement at the boundary instead of failing deep inside async lowering.
3. **Wrap mutable state:** When the requirement would make the container unusable (e.g., storing `SpanWriter` inside an async struct), expose helper builders that wrap the state in `std.sync::Mutex`/`RwLock` and return guard types that already satisfy both traits.
4. **Test both happy and failure paths:** Extend `typeck::arena::tests::diagnostics` or add MIR async fixtures whenever a new container introduces auto-trait contracts. Each regression should cover (a) missing trait, (b) satisfied trait, and (c) guidance message text so IDEs can mirror the spec.
5. **Document the boundary:** Update the relevant spec/tutorial section when you introduce a new `@thread_safe`/`@shareable` requirement. Link to this file plus the spec section so downstream authors have a clear reference.
