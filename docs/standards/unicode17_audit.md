# Unicode 17.0.0 Support – Audit & Requirements

Context: Standards compliance note for Unicode 17.0.0. This document captures current behaviour across lexer/parser/runtime, gaps versus Unicode 17.0.0, and acceptance criteria for implementation and testing.

## Current Behaviour

- **Identifier classification**
  - The lexer uses `unicode-xid` 0.2.6 (`UnicodeXID::is_xid_start` / `is_xid_continue`) with an underscore fallback; the bundled tables are older than Unicode 17.
  - No normalization is applied; identifier spelling is kept byte-for-byte from source. Symbol resolution compares raw lexemes, so canonically equivalent spellings are distinct.
  - There is no policy to reject pattern whitespace, default ignorable, or bidi edge cases. Diagnostics report byte offsets and are not grapheme-aware.

- **Literals and escapes**
  - String/char escapes are decoded via `unicode::escapes` tables plus `char::from_u32`; this rejects out-of-range code points and surrogate halves but does not enforce normalization.
  - Interpolated/verbatim strings accept doubled quotes per C# rules; normalization is not performed on the resulting payloads.

- **Runtime / CLI**
  - Diagnostics and pretty-printers operate on raw UTF-8 slices without grapheme-aware slicing; multi-codepoint user text can be split mid-grapheme in spans/snippets.
  - There is no runtime normalization helper or API for inspecting identifier/literal normalization.

## Gaps vs Unicode 17.0.0

- Identifier rules rely on `unicode-xid` data predating Unicode 17 and do not follow UAX #31 with the recommended Pattern_Syntax/Pattern_White_Space exclusions.
- No normalization policy is defined or enforced for identifiers (NFC/NFKC), string literals, or diagnostics display.
- No generated Unicode 17 property/normalization tables are present in-tree; lexer/parser/symbol tables rely on the external crate only.
- Diagnostics are not grapheme-aware; spans can slice inside extended grapheme clusters.

## Requirements / Acceptance Criteria

- Adopt UAX #31 for identifiers using Unicode 17.0.0 ID_Start/ID_Continue + underscore, excluding Pattern_Syntax/Pattern_White_Space.
- Define and apply a normalization policy (recommended: NFC) for identifiers during interning/resolution; retain source spelling for diagnostics but normalize comparison/storage.
- Generate and check in Unicode 17 tables (identifier properties + normalization data) with a documented regeneration tool; avoid runtime Rust shims beyond generation tooling.
- Extend literal handling to validate escapes against Unicode 17 ranges and apply the agreed normalization policy where required.
- Make diagnostics/CLI output grapheme-aware for slicing/underline rendering to avoid breaking clusters.
- Add conformance tests: positive/negative identifier cases (mixed scripts, bidi edge cases), normalization round-trips, escape handling (including surrogate rejection), and rendering checks for diagnostics.

## Implementation Snapshot (2025-12-06)

- Added `cargo xtask unicode17`, which downloads Unicode 17.0.0 UCD data (Core/Normalization/Composition exclusions + Grapheme/Emoji tables) and regenerates `generated/unicode17/{ident.rs,normalization.rs,grapheme.rs}`. See `docs/runtime/unicode_tables.md` for regeneration steps.
- Identifier handling now uses the generated ID_Start/ID_Continue sets with Pattern_Syntax/Pattern_White_Space exclusions, normalises to NFC during lexing, and emits diagnostics when code points are disallowed or source spellings are not NFC.
- String literal parsing normalises cooked text/format segments to NFC; diagnostics and CLI rendering use grapheme-aware caret positioning so multi-scalar graphemes are underlined once.
- Conformance coverage added for identifier normalisation/invalid code points, string literal NFC folding, and grapheme-aware diagnostic underlines.

## Next Steps

- Implement the generator/tooling for Unicode 17 data (properties + normalization) and wire lexer/parser/symbol tables to the generated tables.
- Normalize identifiers during symbol resolution; reject non-UAX #31 code points with precise diagnostics.
- Add grapheme-aware slicing helpers for diagnostics/CLI output and migrate existing span renderers.
- Document the policy in the language reference and tooling guides once behaviour is implemented.
