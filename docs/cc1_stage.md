# cc1 Compilation Stage

Status: Draft  
Last updated: 2025-10-26  
Owners: Bootstrap compiler team

The bootstrap toolchain now exposes a `cc1`-style assembly stage used to
bridge Chic code generation with the Clang driver. This document captures
the supported workflows, configuration flags, and integration points.

## Overview

`clang -cc1` is Clang's internal compilation entry point. It consumes a
preprocessed C translation unit (`.i`) and emits an assembly listing (`.s`).
We wrap this behaviour in the Impact CLI for two scenarios:

1. **Direct invocation** – Convert preprocessed C produced by external tools
   into assembly without going through the full Clang driver.
2. **Chic assembly backend** – Translate Chic MIR into a restricted
   preprocessed C dialect, run `clang -cc1`, and surface the generated
   assembly to downstream toolchains.

The stage is intentionally minimal: it focuses on deterministic output for
simple integer workloads and exists so we can validate multi-stage pipelines
ahead of broader backend support.

## CLI Surface

### `chic cc1`

```
chic cc1 <file> [options]

Options:
  -o, --output <path>      Write the assembly listing to <path> (default: swap
                           `.i` for `.s`).
  -t, --target <triple>    Assemble for the given target triple (defaults to
                           the host triple).
  --cc1-arg <arg>          Forward a raw flag directly to `clang -cc1`. May be
                           repeated.
```

The command requires a preprocessed C input (`.i`) and forwards the request to
`clang -cc1 -S`. The wrapper ensures the output directory exists and reports
the generated path on success.

### `chic build --backend cc1`

Invoking `chic build` with `--backend cc1` lowers Chic MIR into a
preprocessed C buffer, then drives the same `clang -cc1` stage. Additional
flags:

- `--cc1-arg <arg>` repeats the forwarding behaviour for build invocations.
- `--cc1-keep-input` instructs the driver to retain the generated `.i` file
  instead of deleting it after assembly.

The backend currently supports a single input module composed of synchronous
functions returning integer or boolean scalars. Unsupported constructs produce
deterministic diagnostics so we can expand coverage incrementally.

## Determinism and Validation

The tests under `tests/cc1_stage.rs` exercise both the direct command and the
Chic backend. Each test assembles the emitted `.s` with the system Clang
to ensure we always produce syntactically valid output. The translator also
rejects multi-block MIR bodies or unsupported rvalues with structured error
messages.

## Forward Work

- Broaden MIR coverage beyond simple arithmetic and constants.
- Integrate richer type support (structs, references) once lowering is stable.
- Allow cc1 output to feed directly into Impact's packaging pipeline alongside
  objects for mixed-language builds.
- Expose optimisation level plumbing (`-O0`/`-O2`, etc.) to the CLI once the
  translator supports corresponding semantics.

For now the stage acts as a deterministic bridge to the system assembler and a
foundation for more advanced C-style pipelines.
