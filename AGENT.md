# Chic Bootstrap Agent

This repository hosts the **temporary Rust implementation** of the Chic compiler. The library crate (`chic`) powers the tooling, while the end-user CLI ships as the `chic` executable until the self-hosting rewrite lands. Every change should move us closer to deleting the Rust runtime entirely and running the full stack in Chic.

## Build Verification (Non-Negotiable) + Tests (Best-effort)

- **Mandatory verification loop (release-ready):** `cargo fmt -- --check` → `cargo build --all --all-targets` → `cargo test --all --all-targets --no-run` → repeat until green.
- **Runtime/Std test execution is best-effort:** `chic test` is useful during development, but may be temporarily gated/disabled in CI while the native runtime is being stabilized. Do not block releases on flaky or currently-broken runtime tests unless the task is explicitly “fix tests”.
- **Coverage is opt-in:** run `chic coverage --workspace` only when explicitly working on coverage/instrumentation. Do not block builds/releases on coverage percent unless a specific task requires it.
- **Testing policy (Chic code):** do **not** write Rust unit tests for Chic library behavior (Std or other Chic packages). Chic behavior is validated via **Chic `testcase`** executed by `chic test`. Rust tests are allowed only for Rust code itself, or compiler/codegen harness tests that compile/run Chic programs (language feature validation / codegen harness).
- **Constructor style:** use `new TypeName(...)` (including generics like `new HashSet<int>()`); do not introduce or rely on `TypeName.New(...)` patterns.
- **Assertion style:** tests use fluent assertions via `import Std.Testing;` and then `Assert.That(...).Is...`. For `std.core`, use `import Std.Core.Testing;` and `Assert.That(...).Is...` (avoid fully-qualified assertion calls; prefer imports).
- **No MSTest-style asserts:** avoid `Ensure(...)`/`AreEqual(...)`-style test APIs for Chic code. Fix the fluent assertion library or add missing fluent matchers instead.
- **Determinism + targets:** avoid flaky tests; gate platform-only behavior with deterministic `NotSupported` (or equivalent) and/or explicit `@group("native")` / `@group("wasm")` tags or compiler directives.

## Cache + Artifact Hygiene (Non-Negotiable)

- **Do not check in build outputs:** never commit `obj/`, `bin/`, `tmp/`, `coverage/` outputs, logs, `.o`, `.ll`, `.profraw`, or cache manifests (`*.cache.json`). Prefer per-invocation output dirs (`--artifacts-path`) and project `bin_dir`/`obj_dir` plumbing.
- **Cache correctness must not depend on timestamps:** incremental caching must key off deterministic fingerprints/content hashes, not filesystem mtimes or wall-clock time. Timestamps may be recorded for telemetry only and must never affect reuse decisions.
- **Timestamp/caching bugs are critical:** if cache reuse or invalidation appears wrong, fix it immediately (start at `src/codegen/cache.rs` and any build graph timestamp comparisons) and add a regression test/gate.

## Zero-Tolerance Rust Shim Policy

- Treat the Rust runtime as a temporary bootstrap layer. Do **not** add new numeric helpers, parsing routines, or standard-library features to Rust just because it is convenient—the work belongs in Chic, even if that means adding language/runtime support first.
- **Do not modify Rust runtime/object crates** beyond strictly necessary build fixes; implement replacements in `Std.Compiler.*` packages under `/packages` and migrate callers until Rust is deletable.
- **New compiler/runtime infrastructure work goes in `Std.Compiler.*`** (under `/packages`), explicitly separated from user-facing `Std.*` APIs.
- `runtime_adapter/**` has been deleted. Do not reintroduce Rust runtime semantics anywhere else; implement behaviour in Chic Std modules or, if unavoidable, minimal C/ABI shims under `Std.Platform`.
- If you discover an area that still leans on a Rust shim (strings, numerics, decimal kernels, containers, etc.), either delete it outright or open a tracked subtask with a near-term expiry describing how it will be replaced. Features that land without a Chic-native implementation are considered regressions.
- `Std.Numeric.*` (including `IntPtr`/`UIntPtr` and `Std.Numeric.Decimal.*`) must always call Chic code. Pointer width, overflow/bit operations, formatting, and parser logic now run entirely in the standard library; reintroducing the old `chic_rt_numeric_*` exports is forbidden.
- The bootstrap runtime must stay “Rust runtime–small”: startup glue, host bindings, and temporary allocator hooks are acceptable; everything else needs a Chic-native plan with tests and docs (and a linked issue/PR for tracking).

## Responsibilities

- Treat `/packages` as the canonical package workspace. Every non-compiler library lives under `packages/<name>` with its own `manifest.yaml` declaring `package.name`, `namespace`, `build.kind`, `sources`, and explicit `dependencies`.
- Manifests must live at `packages/<name>/manifest.yaml`; nested manifests under `src/` (or other subfolders) are not supported.
- Dependency resolution is manifest-only. `@package` directives are disallowed; missing dependencies must be added to `manifest.yaml` (including explicit `std`/`std.*` packages when used).
- Namespace imports use `import` directives (`import Foo.Bar;`, `global import Foo;`, `import static Foo.Bar;`). `using` directives are not supported.
- Std is modular: keep portable pieces in minimal std packages (e.g., `std.core`, `std.alloc`, `std.foundation`, `std.runtime`, `std.platform`, `std.net`, `std.security`, `std.text`, plus additional std.* leaf packages) instead of a monolith, and declare dependencies between Std packages explicitly.
- Package placement (where to put code):
  - `packages/std.core`: language primitives, spans, numerics; no IO or networking.
  - `packages/std.async`: futures/task runtime and cancellation primitives.
  - `packages/std.io`: Stream hierarchy, IO exceptions, file/memory streams; platform IO glue lives in `std.platform`.
  - `packages/std.data`: ADO-style contracts and mapping helpers only; drivers live elsewhere.
  - `packages/std.security`: cryptography + TLS; consumes `Std.IO` streams but keep protocol/crypto here.
  - `packages/std.net`: sockets, HTTP stacks, and network stream wrapper (uses `std.io`).
  - `packages/std.text`: text encoders, JSON, XML.
  - `packages/std`: transitional umbrella; do not put new IO/data/crypto/net code here.
- Build outputs and cache keys must respect the manifest-driven package graph; no implicit Std loading or package auto-includes.
- Internalise the project goals by skimming `docs/mission.md` at the start of every work session before tackling new work or proposing changes.
- Keep the language definition authoritative in `SPEC.md`. Update the specification **before** (or together with) any change in behaviour or syntax.

## Changelog Policy

- Update `CHANGELOG.md` only for **breaking, user-visible changes** to the Chic language and core runtime contract.
- Do not add entries for routine refactors, internal compiler changes, or standard-library/package churn unless they break existing programs/artifacts.
- When you add an entry, include a short rationale and link to the relevant spec/design doc where possible.
- Deterministic destruction uses `dispose(ref this)`. `deinit` is forbidden in Chic code.
- Keep the Rust bootstrap compiler functional only as long as necessary for the self-hosting effort—new runtime work must land in Chic first, with any Rust changes restricted to obvious shims slated for deletion.
- Treat Rust shims as a *last resort*. If you cannot avoid introducing one, open a tracking issue with an explicit expiry, document the intended Chic replacement, and prioritise deleting the shim immediately after the missing Chic feature or runtime helper exists.
- Enforce one type per file for Chic sources: each class, struct, interface, or static extension lives in its own `.cl` file. Split existing multi-type files before adding new work.
- When you hit a missing language or runtime capability, spec it, open a tracking issue, and implement it in Chic. Do not route around gaps with Rust hacks—the work belongs in Chic.
- Never declare a task complete until the relevant test suites finish cleanly; if any test fails, fix or revert before closing the work item.

## Mandatory Verification Loop

- **Required loop order:** clean → build → `chic test` → repeat until green.
- **Clean (Chic artifacts):** `chic clean --all`
- **Clean (Rust artifacts, when needed):** `cargo clean`
- **Build (full workspace):** `cargo build --all --all-targets`
- **Chic tests (workspace):** `chic test --workspace`
- **Chic coverage (opt-in):** `chic coverage --workspace` (optionally with `--min <n>` when explicitly required)
- Run the loop in this order after every change and repeat until green. Never mark a task done until these steps pass.
- **Rust-only validation:** when you touch Rust code, run `cargo test --all` after the loop.

## Chic Testcase Authoring Standards (Non-Negotiable)

- **Testing policy:** Do **not** write Rust tests for Chic code. Chic behaviour must be tested via Chic `testcase` and executed by `chic test`.
- **Allowed Rust tests:** Rust tests may only validate Rust code directly, or act as compiler/codegen harnesses that compile and execute Chic programs (language feature validation), without re-implementing Chic library logic in Rust.
- **Coverage is optional:** coverage tooling exists (`chic coverage`, `chic test --coverage`) but is not a default correctness gate for tests/builds.
- **Test discovery:** Every Chic package must declare a manifest `tests:` section and must ship real Chic `testcase` coverage.
- **Test location:** Prefer inline `testcase` blocks next to the code under test, but use `packages/<pkg>/tests/` for multi-module or end-to-end scenarios when it keeps implementation files readable.
- **Determinism:** Tests must be deterministic. Platform-specific tests must be gated with deterministic NotSupported diagnostics and/or explicit tags (e.g. `@group("native")`, `@group("wasm")`).
- **One behavior per testcase:** Each `testcase` validates exactly one behavior/contract and contains a single assertion chain/call. Split scenarios into separate testcases rather than stacking multiple independent assertions.
- **Fluent assertion style required:** Use fluent assertion helpers via imports (e.g. `import Std.Testing;` then `Assert.That(...)`) so failures report expected/actual values and read as a linear chain. If an assertion API is broken (e.g. `Assert.That(x).IsTrue()`), fix `std.testing`—do not rewrite tests into ad-hoc checks.
- **Canonical pattern:** `Assert.That(threw).IsTrue()` / `Assert.That(value).IsEqualTo(expected)` is the intended style; keep tests fluent and fix the assertion contexts when the chain breaks.
- **Do not redesign tests into MSTest/xUnit:** Keep the Chic testing surface as fluent assertions (`Assert.That(...).Is...`) with clear expected/actual failures, not attribute-heavy test frameworks.
- **`std.core` exception (dependency-cycle):** `packages/std.core` must not depend on `std.testing`; use `import Std.Core.Testing;` then `Assert.That(...)` in `std.core` testcases. All other Chic packages must use fluent assertions via `import Std.Testing;` then `Assert.That(...)`—do not “fix” failures by replacing `Assert.That(threw).IsTrue()` with `Ensure(threw)`.
- **Testcases mandatory:** All new Chic code must ship `testcase` coverage. No exceptions.
- **Arrange → Act → Assert:** Test bodies must read as a linear flow that uses fluent assertion helpers.
- **Given/When/Then names:** Every testcase name must follow `Given <context> When <action> Then <single expected result>` (encoded as an identifier, e.g. `Given_context_When_action_Then_result`).
- **Diagnostics quality bar:** Test failures must include testcase name, package, file, line/column, expected vs actual, and relevant notes. Compiler errors must include primary span, labels, notes/help, and clear actionable guidance.

## Local variable style (let vs var)

- Chic locals **must** start with `let` (immutable) or `var` (mutable). The parser rejects `Type name = expr` forms.
- Default to `let` and only use `var` when the binding will be reassigned/mutated (e.g., loop counters, accumulators).
- Type annotations are optional but, when needed, use `let name: Type = expr;`—never `Type name = expr;`.
- Keep this convention consistent across core/std/runtime code so parser diagnostics stay quiet (see LCL0001).

## Quick Orientation (read these first)

- **Runtime Policy (Hard Requirement):** Chic runtime work must be implemented in Chic. The Rust runtime exists solely as a temporary shim for validation and must not gain new features. When in doubt, delete Rust runtime code instead of extending it. If Chic is missing the capability you need, spec it, open a tracking issue, and implement it in Chic—do **not** route new behaviour through Rust.
- **Runtime Footprint:** The bootstrap runtime must stay “Rust runtime–small”: the minimum glue necessary to bring up the compiler. Every feature that currently lives in Rust should have a Chic replacement plan, and additions to the runtime should default to Chic code unless the host integration absolutely forbids it.
- **Language Specification:** `SPEC.md` — authoritative syntax/semantics, including async, testing, grammar, and the “64-bit targets only” constraint. Run `chic spec` to display the embedded summary.
- **Mission Statement (skim every session):** `docs/mission.md` — revisit it quickly to stay aligned with the project’s purpose, strategic pillars, and how every change should reinforce them.
- **Package Manifest Guide:** `docs/manifest_manifest.md` — details the `manifest.yaml` schema (sources, targets, runtime, tests).
- **Runtime selection:** `docs/tooling/runtime_selection.md` — manifest-driven runtime packaging, ABI requirements, and cache partitioning rules.
- **MIR Reference:** `docs/mir_design.md` — canonical description of the mid-level IR (CFG, ownership, borrow checking, slices).
- **Coverage Workflow:** `docs/coverage.md` — how to generate LCOV/HTML reports and run optional coverage checks (`cargo xtask coverage`, `chic coverage`).
- **Embedded MMIO Guide:** `docs/embedded.md` — memory-mapped register patterns, safety rules, and sample driver walkthroughs.
- **Profiling**: `docs/profiling.md` — step-by-step flame graph capture (WASM backend hotspot analysis, `cargo-flamegraph` helper script).
- **Borrow Checker:** `src/mir/borrow.rs` — enforces `in/ref/out` semantics, region tracking, definite assignment, and `await` borrow guards.
- **Backlog & Tracking:** Prefer GitHub issues/PRs for work tracking; keep `todos.md` small and focused on near-term items.
- **Codegen Backend:** `src/codegen/` — LLVM remains the default backend and the Cranelift prototype has been retired. The in-house WASM plugin handles fast iteration; keep the spec and design docs aligned whenever backend behaviour or targets change.
- **WASM Goldens:** If you ever adjust the underlying MIR fixtures that drive the `function_emitter` or `module_builder` snapshots, rerun the same tests with `UPDATE_WASM_GOLDENS=1` to refresh `tests/golden/wasm/*.hex`.
- **C Header Generation Plan:** `docs/header_generation.md` — outlines the forthcoming `chic header` command for producing C-compatible headers.
- **Compiler Entrypoint:** `src/main.rs`, with orchestration logic in `src/driver.rs`.
- **Frontend Modules:** `src/frontend/` (lexer, parser, diagnostics, AST) — keep changes in sync with the spec.
- **Parser Reliability:** If any parsing regression is uncovered while implementing a task, **fix it immediately** (or add the missing coverage) as long as the resolution matches `SPEC.md`. Do not defer parser errors—address them before moving to downstream work.
- **Error Model:** `src/error.rs` — unified error handling; extend when adding new failure modes (tests, codegen, etc.).
- **Commit Rhythm:** Keep commits atomic and reviewable; each commit should build and keep tests green (or clearly explain why a temporary exception is required).
- **Chic compiler binary:** the compiler is the `chic` executable produced by this crate (`cargo build --bin chic` or `cargo build -p chic --bin chic`). Tooling auto-detects the workspace compiler (`target/debug/chic`, then `target/release/chic`) so you should not need to set `CHIC_COMPILER` locally; override only for nonstandard host layouts.
- **Compiler entrypoint:** invoke the compiler via `chic build|run|test` rather than `cargo build -p chic`; the `cargo build` command only builds the Rust driver binary.
- **Runtime packages + selection:** runtime code lives under `packages/runtime.native` and `packages/runtime.no_std`. Every manifest must declare `toolchain.runtime` (kind, package, ABI tag, optional path); builds fail if runtime selection is omitted. There is no implicit `src/runtime` fallback.
- **Native runtime enforcement:** the build fails unless the selected Chic runtime archive links. `build.rs` resolves the runtime package and links `target/runtime/<kind>/<runtime.identity>/libchic_rt_<kind>.a`, otherwise it builds the archive via the workspace compiler. There is no Rust shim fallback or `CHIC_BUILD_NATIVE_RUNTIME=0` escape.
- **Runtime selection:** the Rust runtime backend has been removed. CLI parsing rejects `--runtime-backend rust`/`--runtime rust`, and only the Chic runtime is accepted. CI/local builds must keep it that way; `cargo xtask lint-runtime-shim` guards against reintroducing the shim.

## Iteration Expectations

1. Extend the parser/AST coverage beyond the current function/struct/enum support (classes, interfaces, extensions, attributes).
2. Mirror every semantic addition or adjustment in the specification.
3. Prefer tight, well-factored Rust modules—the goal is to make the later Chic rewrite straightforward.
4. Check in changes with a commit after wrapping each scoped backlog item; this keeps progress auditable and keeps reviews manageable.
5. Practice “Boy Scout" refactoring: whenever you touch a large file, leave it smaller and cleaner—extract helper modules or files when a source grows beyond a few hundred lines.
6. Maintain single-responsibility functions: split logic until each branch can be unit-tested; write tests for every new or modified control path.
7. Avoid reintroducing monoliths: if a file balloons past ~1k LOC during work, break it apart before merging to keep the bootstrapper maintainable.
8. Track non-trivial refactors in a GitHub issue and keep changes small, testable, and deletion-oriented.

When the team is ready to build the self-hosted compiler, this bootstrapper should serve as the reference implementation and regression oracle.

## Refactor & Unit-Test Enforcement (Rust)

**Guardrails**
- Functions over 50 lines must be split or explicitly annotated with `#[expect(clippy::too_many_lines)]` along with a one-line rationale.
- Nesting depth above two should be flattened via early returns, `match`, or `let-else`.
- Loop bodies belong in dedicated `step(..)` helpers that can be unit-tested directly.
- Avoid `unwrap`/`expect` in core paths—prefer returning `Result<_, _>` and propagate errors.
- Prefer module-level unit tests written in the Arrange-Act-Assert style; cover every branch and error path.
- Enforce a 2 s slow-test marker and 30 s hard timeout; never raise the limits.

**Tooling expectations**
- Clippy thresholds are enforced through `.clippy.toml` (size, argument count, complexity caps).
- Test timeouts are enforced via `.config/nextest.toml` (2 s slow marker, 30 s termination).
- CI must fail on clippy warnings or threshold violations.

**Working style**
- Before declaring work complete, run the Mandatory Verification Loop above (Rust + Chic).
- Keep diffs small and auditable; avoid large generated blobs.
- Apply Boy Scout refactoring so touched code becomes smaller, clearer, and easier to test.
- Uphold single responsibility across modules and functions so changes are easy to test and reason about.
