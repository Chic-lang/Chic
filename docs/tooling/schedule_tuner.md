# Schedule Profiles and Tuner

Profiles capture tuned schedules for graphs and are consumed at build time to enforce deterministic execution.

## Profile Sidecars

- Stored under `profiling/profiles/*.json`.
- Schema: `{ "graph": "<id>", "hash": "<profile-hash>", "params": { ... } }`.
- `profile_loader` fails the build when the hash does not match the expected value from `@use_schedule`.

## Tuner (stub)

- `cargo xtask schedule-tune` is reserved for an offline tuner that will run candidate schedules and emit updated profile sidecars.
- Deterministic winner selection should be based on recorded perf counters; ties resolve by lexical order of candidate IDs.

## Validation

- Tests should compare profile hash behaviour and ensure missing/incorrect profiles surface deterministic diagnostics.
- Metadata is exported through `mir.json` so external planners can verify which profile hash was consumed.
