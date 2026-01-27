# Logging snapshot suites

This document tracks the refactored logging snapshot tests, their owners, and how to refresh snapshots safely.

## Snapshot catalogue

| Subsystem | Module path | Test names | Notes / Owner |
| --- | --- | --- | --- |
| CLI lifecycle | `tests/logging/cli` | `logging::cli::text_cli_pipeline`, `logging::cli::json_cli_pipeline` | Ensures CLI header/start/footer stages stay stable (Owner: CLI/Driver) |
| Driver orchestration | `tests/logging/driver` | `logging::driver::text_driver_pipeline`, `logging::driver::json_driver_pipeline` | Exercises driver.* stages for all commands (Owner: Driver team) |
| Frontend stdlib loading | `tests/logging/frontend` | `logging::frontend::text_frontend_pipeline`, `logging::frontend::json_frontend_pipeline` | Covers frontend.* stages while loading the stdlib pack (Owner: Frontend) |
| Diagnostics surfacing | `tests/logging/diagnostics` | `logging::diagnostics::text_diagnostics_snapshot`, `logging::diagnostics::json_diagnostics_snapshot` | Verifies `[Error]` formatting + footer summaries (Owner: Diagnostics/IDE) |

Each subsystem lives in its own directory under `tests/logging/` and imports the shared harness + `log_snapshot_test!` macro to keep the boilerplate uniform.

## Harness helpers

- `tests/logging/harness.rs` owns the shared fixture that writes `sample.ch`, runs commands, sanitizes both text + JSON logs, and exposes the stage/diagnostic filters.
- `log_snapshot_test!` builds the filtered snapshot for all four commands (`check`, `build`, `test`, `run`) and pipes the result into an `expect!` block.
- Stage filters keep the snapshots focused on a tiny slice of stderr (`cli.*`, `driver.*`, `frontend.*`) instead of recording the entire pipeline trace.

When adding a new snapshot family, create a sibling directory (e.g., `tests/logging/runtime`) and invoke `log_snapshot_test!` with the appropriate filter (either `FilterKind::stage("runtime.")` or a custom helper).

## Regenerating snapshots

1. Run `cargo test --test logging` to ensure the harness builds and the current snapshots pass.
2. If the output changed intentionally, re-run with `UPDATE_EXPECT=1 cargo test --test logging` to rewrite the `expect!` blocks automatically.
3. Review the diff, paying close attention to the per-subsystem modules so that unrelated snapshots are not re-recorded accidentally.

Individual tests can be targeted as well, e.g.:

```
cargo test --test logging logging::frontend::text_frontend_pipeline
```

The sanitizers strip ANSI escapes, timestamps, elapsed time, and temp paths, so re-running the suite only records semantic changes to the logs.
