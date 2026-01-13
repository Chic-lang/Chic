# Deterministic RNG Runtime

This note records the design for Chic’s counter-based RNG (`effects(random)`), ensuring
deterministic behaviour across native and WASM targets.

## Goals

- Single portable algorithm (counter-based, 128-bit state).
- Deterministic `split` and `advance` operations so parallel workloads remain reproducible.
- Zero global state; RNG handles thread through API boundaries explicitly.

## Runtime Surfaces

- `packages/std/src/random/rng.cl` – exposes `RNG`, `split`, `advance`, and convenience sampling helpers.
- `runtime_adapter/native/random.rs` – native implementation using 128-bit counters.
- `runtime_adapter/wasm/random.rs` – WASM implementation mirroring native bit-for-bit.

## Diagnostics

- Builds fail with `RND100` if a function calls random helpers without declaring `effects(random)`.
- RNG handles are linear: duplication without `split` triggers `RND101`.

## Usage and determinism

- Seed via `RNG Seed(hi, lo)` (Chic surface) or `Rng::from_seed(u128)` in the adapters. Seeds are mapped through SplitMix64 to decorrelate the two 64-bit lanes.
- `split(ref RNG)` uses xoroshiro128++ jump constants to produce non-overlapping subsequences deterministically. `advance` steps the counter forward explicitly; there are no hidden jumps.
- `next_u32`/`next_u64`/`next_u128` are deterministic across LLVM/WASM. Cross-backend tests assert the bitstreams match for seeds, splits, and advances.
- RNGs are move-only; duplication requires `split`. Async captures should move the RNG into the task; cloning/aliasing is prohibited and will be diagnosed once the effect checks are wired.
- Set `CHIC_RUN_LOG=<path>` to record RNG stream IDs and event ordering; logs are embedded into
  `perf.json` and can be read via `chic seed --from-run <path> [--profile <name>]`.

## Replay

- Future work will capture RNG events into `perf.json`/run logs so `chic seed --from-run` can recreate runs exactly. The runtime will log stream IDs and event ordering to keep parallel RNG usage reproducible.
