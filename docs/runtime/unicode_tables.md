# Unicode Escape Tables

The runtime and frontend share a single set of basic escape mappings. These mappings cover the
standard C# escape sequences (``\n``, ``\r``, ``\t``, etc.) and are consumed by both
`src/frontend/literals/escape.rs` and the runtime string infrastructure via the shared
`crate::unicode::escapes` module.

## Regenerating the tables

Run the generator whenever we add or adjust a basic escape:

```sh
rustc scripts/gen_unicode.rs -o target/gen_unicode
./target/gen_unicode
```

The script emits `generated/unicode/escapes.rs` with deterministic ordering so source control
changes only when the data set changes. The generator can also be invoked via
`cargo xtask gen-unicode` once that helper lands; for now we keep the direct `rustc` invocation to
avoid bootstrapping friction.

## Generated artefacts

- `generated/unicode/escapes.rs` – data file containing:
  - `BASIC_ESCAPE_MAPPINGS`: list of `(marker, codepoint, description)` tuples.
  - `BASIC_ESCAPE_BY_MARKER`: indexed lookup from ASCII escape marker → decoded `char`.
  - `BASIC_ESCAPE_BY_CODEPOINT`: reverse lookup from ASCII control codepoint → canonical marker.

`src/unicode/escapes.rs` wraps the generated data with helpers (`decode_basic_escape`,
`encode_basic_escape`, `basic_escape_mappings`) and unit tests that guard the invariants.

## Unicode 17 property tables

Run the Unicode property generator whenever upgrading Unicode or refreshing identifier/grapheme/normalisation data:

```sh
cargo xtask unicode17
```

This downloads the Unicode 17.0.0 UCD/emoji datasets and regenerates:

- `generated/unicode17/ident.rs` – ID_Start/ID_Continue + Pattern_White_Space/Pattern_Syntax ranges.
- `generated/unicode17/normalization.rs` – canonical combining classes, decompositions, and compositions for NFC.
- `generated/unicode17/grapheme.rs` – grapheme break properties and Extended_Pictographic ranges.

The frontend/runtime `crate::unicode::{identifier,normalization,grapheme}` modules consume these tables directly; no runtime Rust dependencies are required beyond the generated data.
