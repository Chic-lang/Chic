# perf.json Artifact

`perf.json` records deterministic tracepoints emitted by `@trace`/`@cost` and the runtime collector.
Each metric is keyed by a stable `trace_id` derived from the MIR function name + label and carries
the observed CPU duration alongside declared budgets.

## File Layout

```json
{
  "version": "0.1.0",
  "target": "x86_64-unknown-linux-gnu",
  "runs": [
    {
      "profile": "debug",
      "metrics": [
        {
          "trace_id": 14073534039221174248,
          "mir_id": "Demo::Main::trace",
          "label": "Demo::Main::trace",
          "cpu_us": 12.4,
          "budget_cpu_us": 10,
          "budget_gpu_us": null,
          "budget_mem_bytes": 16384
        }
      ],
      "run_log": {
        "version": "0.1",
        "rng_streams": [
          {
            "id": 1,
            "seed": 81985529216486895,
            "events": [
              { "index": 0, "kind": "next", "bits": 64 }
            ]
          }
        }
        ]
      }
    }
  ]
}
```

- `trace_id` is deterministic (blake3(function, label)).
- `mir_id`/`label` come from the interned trace label; by default it is the qualified function name
  or `function::label` when the annotation supplies a label.
- Budgets are populated from `@cost` (and propagated onto `@trace` when only a cost is present).
- `run_log` captures RNG stream metadata; when `CHIC_RUN_LOG` is disabled the collector still emits
  an empty log with the current schema version so downstream tooling can rely on presence.

## Determinism & Capture

- Set `CHIC_TRACE_FAKE_CLOCK=1` to force a deterministic nanosecond counter in tests; otherwise a
  monotonic clock is used.
- `CHIC_TRACE_TARGET`/`CHIC_TRACE_PROFILE` override the defaults (host triple and `default`).
- `CHIC_TRACE_OUTPUT` controls the output path (default `perf.json` in the working directory).
- Native binaries flush traces after `Main`/test completion; the WASM executor flushes after export
  execution. Manual flush is available via `chic_rt_trace_flush`.
- `CHIC_RUN_LOG=<path>` enables RNG event capture; logs are embedded into `run_log` inside `perf.json`
  and also written as standalone files when the path points to a separate location.

## Related Commands

- `chic perf report perf.json` summarises cost drift and highlights regressions vs a baseline.
- `chic seed --from-run perf.json` extracts the RNG seeds and stream identifiers for replay.
