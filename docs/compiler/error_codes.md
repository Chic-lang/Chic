# Compiler Error Codes

This catalog records Chic compiler error codes that are surfaced during
type checking and lowering. Codes are stable and intended for tooling and CI
gating.

## Effects and randomness

- `TCK100` — a function may throw an effect that is not listed in its
  `throws` clause.
- `RND100` — a function uses randomness without declaring `effects(random)`;
  add the declaration to make nondeterminism explicit.
- `RND101` — an RNG handle was duplicated without using `split`, violating the
  linearity rule for deterministic streams.
- `NET100` — a function performs networking without declaring
  `effects(network)`; networked actors and collectives must explicitly opt in.
