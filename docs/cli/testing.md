# Chic Test Command

`chic test` now ships a first-class selection and watchdog UX that works the same
across LLVM and WASM backends. Testcases carry namespace/category/id metadata at
MIR time so the runner can filter without rescanning every function on each
invocation. The command is project-aware: pass `manifest.yaml` explicitly or
run inside a project directory (`chic test path/to/project` or just `chic test`)
and the CLI will discover the project file and include `tests/` automatically.
Author tests inline with `testcase` (sync) or `async testcase` (async) and assert
via `import Std.Testing;` and `Assert.That(...)`. Categories and ids come from attributes
(`@category`, `@id`, etc.) and are honored by CLI filters.

## Selection UX (CLI + env)

- `--test <pattern>`: run a single testcase or a wildcard match. Patterns match
  the stable id, fully-qualified name, or short name (e.g.
  `--test Suite::Math::Adds` or `--test Suite::Math::*`). Multiple flags stack.
- `--test-group <pattern>`: filter by category/tag or namespace prefix; accepts
  wildcards (e.g. `--test-group smoke`, `--test-group async*`). Multiple flags
  stack.
- `--all`/`--test-all`: clear filters and run the full suite even if env vars are
  present.
- Environment overrides (applied only when the CLI flag is absent):
  - `CHIC_TEST="Suite::Math::Adds;Suite::Math::Subtracts"`
  - `CHIC_TEST_GROUP="smoke,perf"` (comma/semicolon delimited)
  - `CHIC_TEST_ALL=1` (wins over other selectors)
  - `CHIC_TEST_PARALLELISM=<N>` (0/absent = auto via CPU count)
  - `CHIC_TEST_FAIL_FAST=1` (stop scheduling after the first failure)
  - `CHIC_TEST_WATCHDOG_ENABLE_RELEASE=1` (enable watchdog in release builds
    and apply the configured thresholds)
- Defaults: if no selector is provided, the runner executes every discovered
  testcase; selection/filtered-out counts are surfaced in CLI output.

## Test discovery metadata

- MIR lowering records `TestCaseMetadata` for every `testcase`:
  - Stable id (`t-<hash>`) derived from the fully-qualified name, or an explicit
    override via `@id`, `@test_id`, or `@testid`.
  - Namespace + short name split from the qualified symbol (e.g. `Suite::Math`,
    `Adds`).
  - Categories/tags via `@category`, `@categories`, `@tag`, `@group`,
    `@test_group`, or `@testgroup` attributes. Arguments are lowercased and
    deduped; invalid/empty values emit diagnostics.
  - Unsupported attributes on `testcase` emit diagnostics during lowering.
- The metadata is stored on `MirModule::test_cases` and reused by both backends
  (no additional scanning required) and by the CLI for reporting (discovered vs
  filtered-out counts).

## Runtime execution and parallelism

- The runner applies selectors before scheduling work. It uses
  `--test-parallel <N>`/`CHIC_TEST_PARALLELISM` or the host CPU count to size
  the worker pool (minimum 1).
- `--fail-fast`/`CHIC_TEST_FAIL_FAST` short-circuits scheduling once a failure
  is observed while keeping per-case timings accurate for already-running
  cases.
- Each `TestCaseResult` now carries `id`, `qualified_name`, `namespace`,
  `categories`, and `duration`. The CLI prints discovered/selected/filtered-out
  counts so filtered runs are visible.
- WASM execution uses the same metadata/selection path as the native interpreter
  and reports failures when an export is missing or panics inside WASM.

## Debug/Release Watchdog and loop detection

- The runner supports a per-test watchdog:
  - Wall-clock timeout via `--watchdog-timeout <ms>` or
    `CHIC_TEST_WATCHDOG_TIMEOUT_MS` (recommended when guarding against hangs).
  - Step-limit flags (`--watchdog` / `CHIC_TEST_WATCHDOG_LIMIT[_FORCE]`) are
    accepted but enforcement is pending; timeout remains the active guardrail.
  - `--watchdog-timeout` or `CHIC_TEST_WATCHDOG_ENABLE_RELEASE=1` applies the
    timeout in release builds as well.
- Failures surface as test failures with messages indicating the timeout and the
  environment variable to override; the watchdog is applied after selection so
  it targets the filtered subset.

## Examples

- Project-aware runs: `chic test` (current directory) or `chic test manifest.yaml --test-group smoke`.
- Single test (exact or wildcard): `chic test --test Suite::Math::Adds` or
  `chic test --test Suite::Math::*`.
- Tag/group selection: `chic test --test-group smoke --test-group async*`.
- Full suite despite env filters: `CHIC_TEST_GROUP=smoke chic test --all`.
- Parallel, fail-fast, watchdog: `chic test --test-group smoke --test-parallel 4 --fail-fast --watchdog 50000 --watchdog-timeout 2000`.
