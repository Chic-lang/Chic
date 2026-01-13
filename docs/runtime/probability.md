# Probabilistic Runtime

Deterministic probabilistic helpers built on top of the core RNG.

## Surfaces

- `Std.Probability.Uniform` and `LogProb` types expose surface-level metadata for codegen.
- Chic-native implementations live in `packages/std/src/probability/dist.cl` and use the shared `Std.Random.RNG` surface for deterministic sampling/log-prob accumulation across backends.

## Determinism

- Sampling draws use the shared RNG bitstream; identical seeds produce identical samples across
  native and WASM.
- Log-prob accumulation is purely additive; no hidden allocations.

## Testing

- `chic test packages/std/src/probability/dist.cl` exercises deterministic sampling and log-prob accumulation.
