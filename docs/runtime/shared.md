# Runtime shared module layout

The `runtime/shared` module now owns three focused responsibilities:

1. `config.rs` – runtime configuration builder + install helpers used by the RC/ARC helpers and other runtime subsystems. The config exposes guardrails for allocation sizes, leak handling, and resource-table capacity, and it can be overridden in tests via `RuntimeConfig::builder()` plus `install_runtime_config`.
2. `resources.rs` – resource table that observes RC/ARC allocations when tracking is enabled. The table performs leak detection at teardown via `teardown_tracked_allocations` and honours the chosen `LeakStrategy` (`Ignore`, `Warn`, or `Panic`).
3. `handles.rs` – the unsafe FFI surface for Chic-style `Rc`/`Arc` handles and `chic_rt_object_new`, now instrumented to consult `RuntimeConfig` limits and record allocations in the resource table when enabled.

## Extending the module

- Adding a new shared handle or helper should live in `handles.rs`. Guard allocation-heavy entry points with `ensure_allocation_allowed` so oversized buffers respect the runtime config, and register any heap allocation with `register_shared_allocation(ResourceKind::...)` so leak tracking can observe it.
- Runtime-wide toggles (max allocation bytes, leak handling strategy, table sizes, etc.) belong in `config.rs`. Use `RuntimeConfigBuilder` when introducing new knobs and back them with validation + targeted unit tests.
- Whenever new resources are tracked, update `resources.rs` so `teardown_tracked_allocations` can summarise them, and add regression tests that exercise register, release, and teardown flows (including capacity/failure injection cases).

## Tests & coverage

- Unit tests for `handles.rs` live inline and now cover the RC→Weak downgrade/upgrade flow, allocation guardrails, and resource tracking. Use `ConfigGuard` in tests to temporarily install configs without leaking state across cases.
- `config.rs` and `resources.rs` include dedicated unit suites covering builder validation, installation churn, capacity limits, and leak detection. Run the focused suite with:

  ```bash
  cargo test --lib runtime::shared::config::tests:: runtime::shared::resources::tests:: runtime::shared::handles::tests::
  ```

- Coverage for the shared runtime module can be refreshed locally via:

  ```bash
  cargo llvm-cov --lib --json --output-path coverage/runtime_shared_local.json -- runtime::shared::
  ```

  The refactor recorded `handles.rs`, `config.rs`, and `resources.rs` coverage >85%, so CI can enforce the new baseline.
