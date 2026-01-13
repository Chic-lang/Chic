# Parser Guardrails

Chic’s grammar is required to stay LL(1) so the parser can dispatch exclusively on the current token and report deterministic diagnostics. The Rust bootstrap parser occasionally needs a narrowly scoped escape hatch (contextual keywords, constructor detection, etc.). Every such case must be documented, justified, and covered by CI so new grammar features cannot quietly add multi-token lookahead.

## LL(1) lint

- `cargo xtask lint-ll1` scans `src/frontend/parser` for calls to `peek_n`, `peek_keyword_n`, `peek_punctuation_n`, and `try_type_expr_from`.
- Any use that requires more than one-token lookahead **must** be preceded by a dedicated comment line in the format:

  ```rust
  // LL1_ALLOW: <why this lookahead is unavoidable> (docs/compiler/parser.md#ll1-allowances)
  if self.peek_identifier("record") && self.peek_keyword_n(1, Keyword::Struct) { ... }
  ```

- The lint fails if a lookahead call lacks the comment or if the comment omits a reason. CI (\[.github/workflows/ci.yml](../../.github/workflows/ci.yml)) runs the lint on every build, so violations block merges.

## LL(1) allowances

| Allowance | Files | Notes |
| --- | --- | --- |
| `global using` prefix | `src/frontend/parser/mod.rs` | `using` directives optionally start with `global`, so we peek one token to decide whether `global` belongs to a directive or an identifier. |
| Contextual `record struct` sugar | `src/frontend/parser/item_dispatch.rs`, `src/frontend/parser/declarations.rs`, `src/frontend/parser/members/union.rs` | `record struct` reuses the `record` identifier without turning it into a reserved keyword; we peek for `struct` to keep the syntax without growing the keyword set. |
| Named constructors | `src/frontend/parser/declarations.rs`, `src/frontend/parser/members/class.rs` | Constructors reuse the enclosing type name, so a `(` lookahead disambiguates constructors from fields/properties. |
| Using aliases (`using Alias = Target;`) | `src/frontend/parser/usings.rs` | Alias declarations must detect `=` before consuming the identifier to remain LL(1) with namespace imports. |
| Labeled statements | `src/frontend/parser/statements/mod.rs` | Distinguishing `label:` from expression statements requires peeking for `:` immediately after an identifier. |
| Local function modifiers | `src/frontend/parser/statements/local.rs` | Local functions accept modifier prefixes (`static`, `async`, etc.), so we scan ahead until `function` appears. |
| Typed local declarations | `src/frontend/parser/core/locals.rs` | We still peek for a leading type token to emit the targeted `LCL0001` diagnostic (“locals must use `let`/`var`”) instead of misparsing `Type name = …` as an expression. |

## Adding a new allowance

1. Confirm that the grammar cannot be expressed with a single-token lookahead without regressing existing programs.
2. Add a `// LL1_ALLOW:` comment immediately above the relevant code, referencing this document and summarising the reason.
3. Update the table above with the new allowance (files + rationale).
4. Run `cargo xtask lint-ll1` locally and ensure CI stays green.
