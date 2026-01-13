# Std.Math and Std.MathF

Std.Math and Std.MathF provide floating-point, integer, and decimal helpers modeled after
common math helper surfaces. The Chic implementation favors deterministic behavior and
shared runtime hooks so LLVM and WASM backends stay aligned.

## Surface overview

- `Std.Math`: double/decimal/integer helpers and constants `E`, `PI`, `Tau`.
- `Std.MathF`: float helpers and constants `E`, `PI`, `Tau`.
- `Std.MidpointRounding`: rounding modes used by `Round` overloads.

Core floating-point operations are exposed for both double and float: `Abs`, `Ceiling`,
`Floor`, `Truncate`, `Clamp`, `Max`, `Min`, `MaxMagnitude`, `MinMagnitude`, `CopySign`,
`BitIncrement`, `BitDecrement`, `ScaleB`, `ILogB`, `IEEERemainder`, `FusedMultiplyAdd`,
`Cbrt`, `Sqrt`, `Pow`, `ReciprocalEstimate`, `ReciprocalSqrtEstimate`, trig/hyperbolic
functions, logarithms, exponentials, and `SinCos` (out-parameter form).

## Rounding semantics

- `Round(value)` and `Round(value, digits)` use midpoint-to-even by default.
- `MidpointRounding` modes implemented:
  - `ToEven`
  - `AwayFromZero`
  - `ToZero`
  - `ToNegativeInfinity`
  - `ToPositiveInfinity`
- `Round` validates the mode and throws `ArgumentOutOfRangeException` when invalid.

## NaN, infinities, and signed zero

- `Max`/`Min` propagate NaN when any operand is NaN.
- `MaxMagnitude`/`MinMagnitude` propagate NaN and resolve ties via `Max`/`Min`.
- `Abs` clears the sign bit (so `Abs(-0.0)` yields `+0.0`).
- `Min(-0.0, +0.0)` returns `-0.0`; `Max(-0.0, +0.0)` returns `+0.0`.

## Determinism and portability

Chic routes floating-point Math/MathF operations through shared runtime hooks so both LLVM
and WASM backends execute the same algorithms.

- Tier 1 (integers/decimal): implemented in Std where possible. Decimal rounding currently
  uses a double-based bridge; keep this in mind for very large or highly precise values.
- Tier 2/3 (float/double): powered by runtime hooks that call the host math routines. This
  keeps LLVM/WASM aligned, but results can vary slightly across platforms when the host
  math library differs.

## Target support matrix

- LLVM native: supported for all Math/MathF APIs.
- WASM backend + in-house executor: supported for all Math/MathF APIs via runtime hooks.
- no_std: Math/MathF require runtime hooks for float operations; integer-only helpers are
  usable in no_std contexts.

## Notes on estimates

`ReciprocalEstimate` and `ReciprocalSqrtEstimate` currently return `1/x` and `1/sqrt(x)`,
respectively. These are deterministic but do not promise hardware-approximation behavior.
