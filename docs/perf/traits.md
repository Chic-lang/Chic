# Trait Dispatch Benchmarks

These Criterion benchmarks stress a few representative patterns to ensure our
trait-centric designs stay zero-cost compared to hand-written alternatives.

## Benches

| Group | Description |
|-------|-------------|
| `traits::iter::*` | Compares summing a slice manually vs. a monomorphised trait iterator vs. a trait object iterator. |
| `traits::collections::*` | Measures pushing data into a `Vec` directly, via a generic `Bag` trait, and via a `dyn Bag`. |
| `traits::async::*` | Simulates a simple async state machine using a hand-written loop, a generic `PollJob`, and a trait object. |

Run them with:

```bash
cargo bench --bench traits -- traits::iter
cargo bench --bench traits
```

## CI Integration

`cargo xtask metrics --bench traits` runs the `cargo bench --bench traits`
workload and compares the runtime against the recorded baseline. Regressions
larger than 10% trigger a failure so we notice abstraction overhead early.

## Results Snapshot

| Benchmark | Manual | Generic Trait | Trait Object |
|-----------|--------|---------------|--------------|
| `traits::iter` | ~1.00x | ~1.01x | ~1.05x |
| `traits::collections` | ~1.00x | ~1.02x | ~1.07x |
| `traits::async` | ~1.00x | ~1.01x | ~1.04x |

(Values are relative to the manual baseline on an M3 Max; run the benches on
your hardware to capture precise timings.)

When making changes that affect trait lowering/runtime dispatch, re-run the
benches and update this table if the ratios shift.
