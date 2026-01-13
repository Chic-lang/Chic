# Inline Assembly (`asm!`)

Chic follows Rust’s [`asm!`](https://doc.rust-lang.org/reference/inline-assembly.html) surface while keeping the bootstrapper’s MIR and backend contracts explicit.

## Syntax

- Form: `asm!(template, operands..., options(...)? , clobber(regs...)?);`
- Templates accept raw string literals only; `{{`/`}}` escape braces, and placeholders reference operands by position (`{0}`) or name (`{dst}`) with optional modifiers (`{0:e}`).
- Operands:
  - `in(<reg>) expr`
  - `out(<reg>) place` / `lateout(<reg>) place`
  - `inout(<reg>) expr [=> place]?` / `inlateout(<reg>) ...`
  - `const expr` (compile-time constant)
  - `sym ident` (symbol reference)
- Options: `volatile`, `alignstack`, `intel`/`att_syntax`, `nomem`, `nostack`, `preserves_flags`, `pure`, `readonly`, `noreturn`.
- Clobbers: `clobber("xmm0", "r11")` or register classes (`clobber(reg)`).

## Register support

- Classes: `reg`, `reg8/16/32/64`, `xmm/ymm/zmm`, `vreg`, `kreg`, plus explicit registers (`"{rax}"`, `"x0"`, `"xmm0"`).
- LLVM backend accepts x86_64 and aarch64; unsupported classes or targets raise diagnostics. WASM rejects inline assembly with a targeted error.

## Safety

- Inline assembly requires an `unsafe` context (block or function). Lowering injects `EnterUnsafe`/`ExitUnsafe` markers and emits diagnostics when `asm!` appears in safe code.
- `const` operands must be compile-time constants; outputs require writable places.

## MIR shape

- `StatementKind::InlineAsm` carries:
  - `template: Vec<InlineAsmTemplatePiece>` with resolved operand indices and optional modifiers.
  - `operands: Vec<InlineAsmOperand>` (In/Out/InOut/Const/Sym) with register specs and spans.
  - `clobbers: Vec<InlineAsmRegister>` and `options: InlineAsmOptions`.
- Placeholders are validated against operand names/positions during lowering; missing/duplicate operands surface as diagnostics.

## LLVM lowering

- Templates translate to `llvm::InlineAsm` strings (`{0}` → `$0`), honouring `intel_syntax` and `alignstack`.
- Constraints mirror operand modes (`=r`/`+r`/& early-clobber for `out`/`inout`, `i` for `const`, `s` for `sym`) and append clobbers; default clobbers include memory plus flags unless `preserves_flags`/`nomem`/`pure`/`readonly` opt out.
- Attributes: `volatile` → `sideeffect`; `nomem`/`pure` → `readnone`; `readonly` → `readonly`; `noreturn` → call attribute.
- Multi-output operands return structs; results are written back to the recorded places.

## Tests

- Parser coverage (`parses_inline_asm_with_operands_and_options`, interpolation rejection).
- MIR lowering (`lowers_inline_asm_inside_unsafe_block`).
- Backend parity: LLVM lowers to inline asm IR; WASM rejects with a dedicated diagnostic.
