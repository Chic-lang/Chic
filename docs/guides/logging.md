# Chic Logging Guide

This guide explains how to configure Chic’s compiler logging so build, run, and test invocations emit the level of detail you need for debugging.

## Quick Start
- The CLI accepts `--log-level {error,warn,info,debug,trace}` and `--log-format {auto,text,json}` on commands that execute the compiler pipeline (`check`, `build`, `run`, `test`, `mir-dump`, `header`).
- `auto` format selects compact text today and will switch to JSON when future heuristics (e.g., piping to files) warrant it. Use `--log-format json` to force structured output.
- Environment variables provide defaults: set `CHIC_LOG_LEVEL` or `CHIC_LOG_FORMAT` to configure all invocations launched from your shell.

```bash
# Verbose text logs
chic build main.cl --log-level debug

# Emit JSON suitable for log aggregation
CHIC_LOG_FORMAT=json chic test suite.cl --backend wasm
```

## Log Levels
`LogLevel` controls which tracing spans Chic emits:

| Level  | Intended Use                                                                                   |
|--------|------------------------------------------------------------------------------------------------|
| error  | Only fatal pipeline failures (missing files, IR verification errors).                          |
| warn   | Adds non-fatal warnings (e.g., stdlib discovery fallbacks).                                    |
| info   | Default. Includes stage transition markers such as `driver.build.start` and completion status. |
| debug  | Enables intermediate pipeline diagnostics (frontend module counts, backend options).           |
| trace  | Reserved for exhaustive tracing once MIR/LLVM/WASM emitters expose fine-grained spans.         |

When `--trace-pipeline` is enabled the effective level is promoted to `trace` so downstream code can emit full pipeline spans.

## Formats
Two output modes are currently supported:

- **Text** (default) — compact `tracing` formatter that prints `LEVEL target{key=value}` entries. This keeps human-readable summaries alongside diagnostics.
- **JSON** — machine-readable payload with the same fields (`stage`, `status`, `command`, `elapsed_ms`, etc.) for ingestion by log tooling or snapshot tests.

Future work will add per-command headers reflecting inputs, targets, and artifact paths. The CLI resolves `auto` to text for now so existing developer workflows are stable.

## Propagation across the Pipeline
Command-line parsing produces a `LogOptions` structure that is stored on `Cli`. `main` resolves environment overrides, applies `--trace-pipeline` promotion, and calls `init_logging`. The selected `LogLevel` is propagated to every driver entry point. Backend modules use `resolve_trace_enabled(trace_flag, log_level)` to decide whether to emit span events, ensuring JSON/text parity.

## Debug Stack Traces
Debug builds automatically capture `std::backtrace::Backtrace` data whenever the compiler surfaces an internal or codegen error. When a failure bubbles up, the CLI prints the stack trace beneath the error message, showing the Rust source locations that triggered the failure. Release binaries skip this capture to avoid overhead; use a debug build (the default when running `cargo build`/`cargo run`) when you need call-site diagnostics.

## Deterministic output
For stable logs in automation, prefer `--log-format json --log-level info` so the structure remains consistent even if text wording changes.

## Trait Solver Telemetry

- Pass `--trait-solver-metrics` to `chic check/build/run/test/mir-dump` to print a concise summary (`traits`, `impls`, `overlaps`, `cycles`, `elapsed`) after the command finishes. The flag also forces the `frontend.trait_solver` span to be emitted even when `--trace-pipeline` is off.
- Set `CHIC_TRAIT_SOLVER_METRICS=1` to enable the same telemetry for every invocation in a shell session. This is useful for CI dashboards that monitor solver regressions.
- When `--trace-pipeline` is present the telemetry is automatically enabled; the dedicated flag is only necessary when you need the metrics without full trace noise.

 
