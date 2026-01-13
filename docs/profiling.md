# Impact Profiling (Instrumentation + Sampling)

Chic ships a native profiler that combines deterministic tracepoints with lightweight sampling. Builds tagged for profiling inject trace hooks into every lowered function/test, collect resource summaries, and optionally emit flamegraphs from folded stack samples.

## Quick Start

1. Profile any program with the built-in command (auto-instruments all functions/tests):

   ```bash
   chic profile profiling/fixtures/wasm_hot_switch.cl --backend wasm
   # or: chic run main.cl --profile
   ```

2. Inspect the outputs (default base: `profiling/latest/perf.json`):
   - `perf.json`: trace metrics + embedded summary.
   - `perf.summary.json`: wall/CPU/IO/allocation snapshot.
   - `perf.folded`: folded stacks from instrumentation+sampling.
   - `perf.svg`: flamegraph (when `--profile-flamegraph` or `chic profile` is used).

3. Open the flamegraph:

   ```bash
   open profiling/latest/perf.svg        # macOS
   xdg-open profiling/latest/perf.svg    # Linux
   ```

## Commands and Flags

- `chic profile <inputs> [--backend wasm|llvm] [--profile-out <path>] [--profile-sample-ms <ms>] [--profile-flamegraph]`
  - Enables auto-instrumentation (`CHIC_PROFILE_AUTO_TRACE=1`), defaults to 1 ms sampling, and renders `perf.svg`.
- `chic run|test --profile [--profile-out <path>] [--profile-sample-ms <ms>] [--profile-flamegraph]`
  - Same pipeline but preserves the original command verb (run/tests still emit their usual output).
- The base path set via `--profile-out` controls all artefacts: `.json`, `.summary.json`, `.folded`, and optional `.svg`.

## Configuration Knobs

- `CHIC_PROFILE_AUTO_TRACE=1` – force tracepoints on all lowered functions/tests (set automatically by the commands above).
- `CHIC_TRACE_OUTPUT=<path>` – override the base path for perf outputs.
- `CHIC_TRACE_PROFILE` / `CHIC_TRACE_TARGET` – override the recorded profile name or target triple.
- `CHIC_TRACE_SAMPLE_MS` / `CHIC_TRACE_SAMPLE_HZ` / `CHIC_TRACE_SAMPLE_NS` – sampling cadence (defaults to 1 ms when auto trace is enabled).
- `CHIC_TRACE_FAKE_CLOCK=1` – deterministic timestamps for tests.

## Reading the Outputs

- `perf.json` → `runs[*].metrics` holds tracepoint timings (µs) keyed by stable IDs; `summary` carries wall time, CPU user/system nanoseconds, IO block counts, allocation counters, and the sampling interval.
- `perf.folded` aggregates both instrumentation durations and sampled stacks. Idle time appears as `[idle]` when no tracepoints are active so flamegraphs line up with wall time.
- `perf.svg` renders the folded stacks; the CLI skips generation if no samples are present.

## Troubleshooting

- Empty folded stacks or missing metrics: ensure profiling flags were set (`--profile` or `chic profile`) so `CHIC_PROFILE_AUTO_TRACE` is active, and confirm the program reached its entrypoint.
- No flamegraph produced: `perf.folded` must exist and be non-empty; rerun with a longer workload or lower `--profile-sample-ms`.
- Zeroed allocation counters: expected when no runtime allocations occur between `chic_rt_reset_alloc_stats` and flush. Long-lived services can periodically call `chic_rt_trace_flush` to snapshot live runs.

## Samples

- `profiling/fixtures/wasm_hot_switch.cl` – hotspot example for WASM lowering.
- `tests/testdate/wasm_simple_add.cl` – minimal executable used by the profiling CLI test.
