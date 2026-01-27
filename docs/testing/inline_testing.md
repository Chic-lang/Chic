# Inline Testcases

Chic ships first-class inline tests: declare `testcase` blocks next to the
code they verify and run them through `chic test`. Testcases are discovered at
MIR lowering time (no dynamic reflection) and carry stable ids, namespaces, and
categories for filtering.

## Declaring tests

- Synchronous: `testcase AddsNumbers() { Assert.That(2 + 2).IsEqualTo(4); }`
- Asynchronous: `async testcase FetchesData() { let payload = await Http.Get("/status"); Assert.That(payload.Code).IsEqualTo(200); }`
- Parameters: `testcase WithData(value: int) { ... }` (discovered; currently
  skipped with a diagnostic because data-driven execution is not wired up yet).
- Attributes:
  - Stable id override via `@id`, `@test_id`, or `@testid`.
  - Categories/tags via `@category`, `@categories`, `@tag`, `@group`,
    `@test_group`, or `@testgroup` (arguments are lowercased and de-duplicated).
  - Unsupported attributes produce lowering diagnostics.

Examples required by the spec:

```cl
testcase ComputesHypot() { Assert.That(Math.Hypot(3, 4)).IsEqualTo(5); }

async testcase FetchesData()
{
    let payload = await Http.Get("/status");
    Assert.That(payload.Code).IsEqualTo(200);
}
```

## Discovery and identifiers

- Each testcase lowers into `TestCaseMetadata` with:
  - fully-qualified name,
  - stable id (`t-<hash>`) derived from the qualified name unless overridden by
    an `@id` attribute,
  - namespace and short name split from the qualified symbol,
  - categories/tags, async flag, and captured parameter metadata.
- Discovery never executes user code; metadata is embedded into the MIR module
  and exported in both LLVM and WASM builds.
- Ordering is deterministic: discovery sorts by stable id, then filters.

## Running tests (`chic test`)

- Selection:
  - `--test <pattern>` / `CHIC_TEST` matches ids, fully-qualified names, or
    short names; `*` wildcards are allowed.
  - `--test-group <pattern>` / `CHIC_TEST_GROUP` matches categories or
    namespaces.
  - `--all` / `--test-all` clears filters (wins over env vars).
- Execution:
  - Sync cases run directly; async cases execute inside the async executor.
  - Parameterized cases are reported as `SKIP` until data sources are wired up.
  - Parallelism via `--test-parallel <N>` / `CHIC_TEST_PARALLELISM` (defaults to
    the host CPU count); ordering of reported results is still deterministic.
  - `--fail-fast` / `CHIC_TEST_FAIL_FAST` stops scheduling after the first
    failure.
- Watchdog / loop detection:
  - Wall-clock timeout per testcase via `--watchdog-timeout <ms>` or
    `CHIC_TEST_WATCHDOG_TIMEOUT_MS`; failures surface as `FAIL` with a clear
    message.
  - Step-limit flags are accepted but not enforced yet; timeouts are the current
    guardrail.

## Reporting and exit codes

- Per-test lines report `PASS`/`FAIL`/`SKIP`, async flag, id/namespace/groups,
  duration, and any failure message.
- Summary includes discovered/filtered counts and totals for passed/failed/
  skipped cases. Any failure yields a non-zero exit.

## Writing new tests

- Prefer inline `testcase` declarations (in product files or dedicated
  `*.ch` test files) and assert via `import Std.Testing;` and `Assert.That(...)`.
- Use async testcases when covering async APIs; add categories/ids to keep
  selection predictable in CI.
