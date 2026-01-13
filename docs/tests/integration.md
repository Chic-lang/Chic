# Large Integration Tests Strategy

_updated: 2025-11-02_

This document defines how we organise, author, and execute Chic’s
large-scale integration tests. “Large” in this context covers scenarios that
exercise whole subsystems (frontend → MIR → LLVM/WASM, runtime harnesses,
CLI workflows) rather than unit-level behaviour.

## Scope & Taxonomy

We split integration tests into three categories:

| Category           | Location                     | Examples                                   | Runtime goal |
|-------------------|------------------------------|---------------------------------------------|--------------|
| **End-to-end**     | `tests/e2e/`                  | `chic build/run/test`, stdlib builds      | ≤ 60 s       |
| **Subsystem**      | `tests/subsystem/<area>/`     | MIR borrow suites, WASM executor smoke     | ≤ 20 s       |
| **Golden outputs** | `tests/golden/<area>/`        | LLVM/WASM textual emitters, diagnostics     | ≤ 10 s       |

Each directory contains:

- `PLAN.md` describing scenarios and rationale.
- `fixtures/` with source modules or runtime inputs.
- `helpers.rs` for shared setup (use `tests/support/` for cross-area helpers).

## Authoring Guidelines

1. **Hermetic execution** – Tests must avoid mutating the developer machine.
   Use `tempfile`, mocked toolchains, and the in-tree runtimes.
2. **Single behaviour per test** – Keep individual tests focused to simplify
   debugging. For multi-step scenarios, use helper functions that log
   boundaries.
3. **Deterministic outputs** – Golden tests should normalise paths and pointer
   values. Provide canonical fixtures under `fixtures/`.
4. **Time budgets** – If a test exceeds the runtime goal, add an explicit
   justification in `PLAN.md` and flag it with `#[ignore]` if it cannot run on
   every `cargo test` invocation (schedule ignored tests in CI nightly jobs).
5. **Resource tagging** – Long-running tests must emit a `[long]` tag in their
   name or use the `LONG_TESTS` env gate.
6. **Snapshot strategy** – Textual snapshots live in
   `tests/golden/<area>/snapshots/`. Use `insta` for updating.

## Execution Entry Points

- `cargo test --test <name>` for individual suites.
- `cargo test --features integration-harness` to run the full set (used in CI).
- `cargo xtask integration smoke` (planned) to orchestrate per-category runs and
  enforce runtime budgets.

CI runs the subsystem and golden suites on every PR. The end-to-end suite runs
nightly and before releases. Failures gate merges.

## Environment Configuration

| Setting                 | Description                                               |
|-------------------------|-----------------------------------------------------------|
| `CHIC_LONG_TESTS`  | When set, enables `#[ignore]` long tests locally.         |
| `CHIC_WASM_TRACE`  | Grants verbose logging for WASM executor smoke tests.     |
| `CHIC_GOLDEN_OVERWRITE` | Regenerates golden outputs (off by default).        |

Test code must respect these flags.

## Fixture Ownership

- `tests/support/fs.rs` – tempdir helpers, manifest builders.
- `tests/support/runtime.rs` – managed runtime values (string/span/vec).
- `tests/support/wasm.rs` – module constructors for the WASM executor tests.

Fixtures should be reusable across suites. Avoid ad-hoc embedded strings in the
main test files; keep them in `fixtures/` with descriptive names.

### MIR Borrow Checker Harness

The MIR borrow checker subsystem now lives under `src/mir/borrow/tests/` and is
organised into topical shards:

- `moves.rs` – move/borrow conflict scenarios.
- `borrows.rs` (+ `borrows/fixtures.rs`) – span/unique/shared borrow
  interactions.
- `async/{basic.rs,pinned.rs}` – async/pinned await coverage with shared helpers
  in `async/common.rs`.
- `diagnostics.rs` – in/out parameter and union diagnostics exercising
  `parse_module` pipelines.

All suites share `BorrowTestHarness` (`tests/util.rs`) to build MIR functions
without repeating boilerplate. New tests should instantiate a harness, grab a
`BorrowTestCase` via `case()`, and reuse helper builders where possible:

```rust
let mut case = BorrowTestHarness::new("Borrow::Example").case();
let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);
// build blocks...
case.run().expect_message("conflicting borrow");
```

Keep each shard under 350 LOC by extracting shared fixtures into
`borrows/fixtures.rs` or `async/common.rs`. When adding new helpers, prefer to
extend `BorrowTestHarness`/`BorrowTestCase` instead of cloning MIR setup logic
inside individual tests.

## Adding New Suites

1. Create the directory + `PLAN.md` under the appropriate category.
2. Add fixtures/helpers.
3. Update `cargo xtask integration` (if needed) and `docs/tests/integration.md`.
4. Document the suite in the changelog if it adds significant coverage.

### Concurrency Litmus Suite

- **Location:** `tests/concurrency/litmus/` (Chic sources) with the Rust harness at
  `tests/concurrency.rs` → `mod litmus`.
- **Purpose:** Exercise the memory-model guarantees (Store/Load buffering, IRIW, message passing)
  end-to-end by spawning real `Std.Platform.Thread` workloads and asserting that forbidden outcomes never
  occur under the documented `Std.Sync::MemoryOrder` semantics.
- **Execution:** `tests/concurrency/litmus/mod.rs` compiles the Chic sources via
  `CompilerDriver::run_tests`, then runs the generated testcases on both the LLVM and WASM backends.
  CI treats any failing testcase as a hard failure so the guarantees stay aligned across targets.

## Review Checklist

- [ ] Test is hermetic and deterministic.
- [ ] Runtime budget recorded in `PLAN.md`.
- [ ] Dependencies documented (toolchains, env vars).
- [ ] Golden outputs normalised.
- [ ] CI job updated if runtime increases.
