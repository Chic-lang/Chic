# Test Coverage Workflow

Chic supports optional coverage reporting for both the Rust compiler and Chic packages.

- Rust coverage: `cargo xtask coverage` (via `cargo-llvm-cov`)
- Chic coverage: `chic test --coverage` or `chic coverage`

## Prerequisites

1. Install the `cargo-llvm-cov` subcommand:

   ```bash
   cargo install cargo-llvm-cov
   ```

2. Ensure the LLVM tools component is available:

   ```bash
   rustup component add llvm-tools-preview
   ```

Both are one-time steps per toolchain.

## Running Coverage (Rust)

Run the canonical command from the repo root:

```bash
cargo xtask coverage
```

This cleans prior coverage artifacts, runs `cargo llvm-cov --workspace --json --summary-only`, and writes `coverage/coverage.json`. Flags:

- `--min <percent>`: enforce a minimum line coverage percentage.
- `--output <path>`: choose the JSON output location (default `coverage/coverage.json`).

For detailed drill-downs, you can additionally run:

```bash
cargo llvm-cov --workspace --html --output-path coverage/html
cargo llvm-cov --workspace --lcov --output-path coverage/coverage.lcov
```

## Running Coverage (Chic)

Run Chic coverage from the repo root:

```bash
chic coverage --workspace
```

Equivalent:

```bash
chic test --workspace --coverage
```

Outputs (deterministic JSON):

- `coverage/chic/<package>.json` per package
- `coverage/chic/workspace.json` aggregate workspace summary

Optional flag:

- `--min <percent>`: fail the command if coverage drops below the requested percent.

Chic coverage is statement-based and currently supported via the WASM backend. When coverage is requested, manifest coverage settings (`coverage.min_percent`, `coverage.enforce`, `coverage.backend`, `coverage.scope`) can define enforced minimums per package and at the workspace level (via `manifest.workspace.yaml`).

### Coverage Model

- Coverage points are MIR statements recorded at runtime by the WASM executor.
- Coverage is reported at function granularity: if any statement in a function executes, all statements in that function are counted as covered; branch coverage is not yet tracked.
- Reports include per-file statement totals and an aggregate percentage.

## CI Expectations

- CI may publish coverage artifacts, but coverage percent is not treated as a release blocker by default. Use `--min` or task-specific gates when needed.

## Interpreting Results

- `coverage/coverage.json` contains the Rust workspace summary; inspect `data[0].totals.lines.percent` for the aggregate line percentage.
- `coverage/chic/workspace.json` contains Chic workspace totals and per-package summaries.
- When coverage regresses unexpectedly, add targeted tests for the uncovered paths before re-running the loop (clean → build → test → coverage).

## Policy

- Prefer adding tests with every change and keep coverage trending upward, but do not block builds/releases on coverage percent unless a specific task requires it.
