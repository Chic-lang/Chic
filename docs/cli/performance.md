# CLI Performance & Instrumentation

Pipeline orchestration in the `chic` CLI now emits structured tracing so developers can inspect end-to-end latency and stage behaviour without attaching a debugger. The spans are backed by the `tracing` crate and are enabled automatically in debug builds, or on demand via CLI flag or environment variable.

## Enabling Pipeline Tracing

- Pass `--trace-pipeline` to any pipeline-aware command (`chic check`, `chic build`, `chic run`, `chic test`, `chic mir-dump`).  
  Example: `chic build examples/hello.ch --backend wasm --trace-pipeline`.
- Set `CHIC_TRACE_PIPELINE=1` to opt into tracing without modifying CLI invocations (useful for scripted runs).
- Debug binaries initialise a JSON subscriber by default; release binaries honour the flag or environment variable before installing the subscriber. Use `RUST_LOG`/`RUST_TRACING` to override the default filter (`pipeline=info`).

## Sample Output

Tracing is emitted as newline-delimited JSON. Each line captures the target, stage, elapsed time, and contextual fields the driver records.

```json
{"timestamp":"2025-11-02T03:50:31.123456Z","level":"INFO","fields":{"stage":"driver.build.start","backend":"Llvm","kind":"Executable","input_count":1},"target":"pipeline","spans":["driver.build"]}
{"timestamp":"2025-11-02T03:50:31.458902Z","level":"INFO","fields":{"stage":"frontend.lower_module","elapsed_ms":142},"target":"pipeline","spans":["frontend.pipeline"]}
{"timestamp":"2025-11-02T03:50:32.004871Z","level":"INFO","fields":{"stage":"driver.build.complete","backend":"Llvm","kind":"Executable","artifact":"target/debug/examples/hello.clbin","elapsed_ms":881},"target":"pipeline","spans":["driver.build"]}
```

The span list shows the active context (`driver.build`, `frontend.pipeline`, …). Stages emitted by the WASM helpers report testcase names and statuses to simplify debugging discovery failures.

## Troubleshooting

- **No logs in release builds:** Ensure either `--trace-pipeline` or `CHIC_TRACE_PIPELINE=1` is set; release binaries stay silent otherwise.
- **Logs suppressed by filters:** Provide `RUST_LOG=pipeline=info` (or a broader filter) to combine pipeline spans with other crate targets.
- **JSON hard to read interactively:** Pipe output through `jq` (e.g., `chic test suite.ch --trace-pipeline 2> >(jq .)`) for pretty-printing.
- **CI artefact noise:** Redirect `stderr` to a dedicated log when running with tracing enabled so the JSON stream does not interleave with user-facing output.

## Related Coverage

- CLI parsing guarantees flag wiring: `src/cli/tests.rs`.
- Driver smoke tests exercise instrumentation toggles: `src/driver/tests.rs`.
- End-to-end CLI validation checks for JSON spans: `tests/backend_validation.rs`.
