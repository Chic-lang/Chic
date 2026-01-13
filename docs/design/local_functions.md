# Local Functions – Design Analysis

Status: Draft  
Last updated: 2025‑11‑11  

This note captures the groundwork for adding local functions. It
documents the current parser/typeck/MIR/codegen behaviour, the gaps we must
close to support nested function declarations, and the files/modules that need
touches when we implement the feature.

## 1. Current Behaviour & Limitations

* Only namespace/type members can declare `FunctionDecl`. Blocks may contain
  statements (`StatementKind::VariableDeclaration`, expression statements,
  etc.) but there is no syntax that recognises `function Foo() { ... }`
  inside another function.
* The parser (`src/frontend/parser/statements`) treats any `Keyword::Function`
  token in statement position as an error. `parse_statement` delegates to the
  `simple`, `loops`, `selection`, etc. modules, none of which recognise nested
  declarations.
* `StatementKind` lacks a variant for local functions; downstream passes have
  no data structure to represent them.
* Type checking (`src/typeck/registry.rs`, `typeck/arena`) only registers
  top-level members. There is no scope object for nested function symbols.
* MIR lowering (`src/mir/builder/module_lowering/*.rs`,
  `src/mir/builder/functions.rs`) assumes each `FunctionDecl` becomes a
  separate `MirFunction` attached to a module. There is no handling for a
  function defined inside another function’s body, nor for capturing locals.
* Codegen/executors treat every `MirFunction` as either a free function, a
  method, a constructor, or a testcase (see `FunctionKind` in
  `src/mir/data/definitions/strings/functions.rs`). There is no `Local`
  kind nor linkage strategy for nested functions.
* Documentation/spec make no mention of local functions.

## 2. Parser & AST Requirements

Relevant files:

- `src/frontend/parser/statements/mod.rs` (`parse_statement`,
  `parse_embedded_statement`)
- `src/frontend/parser/statements/simple.rs`
- `src/frontend/parser/core/module.rs` (namespace handling)
- `src/frontend/ast/statements.rs` & `src/frontend/ast/items/functions.rs`

### Gaps to Close

1. **Grammar acceptance.** We must extend the statement grammar so a block can
   contain a function declaration. Suggested syntax mirrors namespace
   functions:

   ```chic
   function Helper<T>(int value) where T : Copy
   {
       // ...
   }
   ```

   The parser should accept modifiers (`async`, `constexpr`, `unsafe`), generic
   parameters/constraints, attributes, and access to the surrounding lexical
   scope.

2. **AST representation.** Introduce a new `StatementKind::LocalFunction` that
   stores a `FunctionDecl` along with captured scope metadata (e.g. an id or
   the inferred capture list). The AST arena currently owns `FunctionDecl`
   instances for namespace/type members; we can reuse the same structure but
   mark it as `is_local`.

3. **Scope plumbing.** Nested functions must inherit the surrounding namespace
   and type context (for generics, visibility, diagnostics). The parser should
   record the lexical parent (likely via a `LocalFunctionScope` on
   `FunctionDecl`), so typeck can resolve references correctly.

4. **Error recovery.** `statements/recovery.rs` needs new synchronisation
   points so malformed local functions do not poison the enclosing block.

## 3. Type Checking & Borrowing

Relevant files:

- `src/typeck/registry.rs`
- `src/typeck/arena/*`
- `src/typeck/diagnostics.rs`
- `src/mir/builder/body_builder/closures/*` (capture analysis)
- `src/mir/borrow/context/*`

### Requirements

1. **Symbol registration.** The type checker must register local functions in a
   scope tied to their parent function. We likely need a `LocalFunctionTable`
   keyed by block id, so references in the enclosing body can resolve them.

2. **Capture analysis.** Local functions behave like named closures—they may
   reference locals from the parent scope. We should reuse the closure capture
   machinery (`mir::builder::body_builder::closures`) to infer captured
   variables, generate env structs, and enforce borrow/lifetime rules.

3. **Generics & constraints.** Local functions can be generic. We need to
   ensure the type checker carries generic parameters and constraints that may
   reference the parent scope (`T` in the enclosing function, for example).

4. **Async/await & effects.** Local functions should support `async`,
   `generator`, and effect annotations. Typeck must check that a local async
   function either gets awaited or returned; diagnostics should mirror the
   behaviour of top-level async functions.

5. **Borrow checking.** Captured references must respect lifetimes. The borrow
   checker needs to treat local functions similarly to closures: moves into the
   environment consume the parent locals; `ref` captures enforce the same
   alias rules.

## 4. MIR Lowering

Relevant files:

- `src/mir/builder/functions.rs`
- `src/mir/builder/module_lowering/*.rs`
- `src/mir/builder/body_builder/mod.rs`
- `src/mir/data/definitions/strings/functions.rs`

### Requirements

1. **New `FunctionKind`.** Introduce `FunctionKind::Local` so MIR/pretty/codegen
   can distinguish nested functions. Local functions will still be compiled as
   standalone MIR bodies, but they need linkage to the parent context.

2. **Environment struct.** Lower captured values into an explicit environment,
   similar to closures. When the parent body references the local function, it
   should produce a function pointer plus an environment pointer (if captures
   exist). We can reuse `ClosureBuilder` infrastructure.

3. **Call sites.** `mir::builder::body_builder::expressions` must emit calls to
   local functions via either direct calls (if no captures) or thunk calls (if
   captures). Needs to integrate with existing `CallOperand` logic.

4. **Visibility & naming.** Generate stable symbol names for local functions,
   e.g. `Parent$local$1`. The name must include a disambiguator (block index,
   source location) so incremental builds remain deterministic.

5. **MirModule exports.** Ensure `MirModule::functions` includes local
   functions and that `MirModule::exports` and metadata skip them (local
   functions are not externally visible).

## 5. Codegen & Executors

Relevant files:

- `src/codegen/llvm/emitter/function/*`
- `src/codegen/wasm/emitter/function/*`
- `src/codegen/llvm/emitter/module.rs`
- `src/codegen/wasm/emitter/module/*`
- `runtime_adapter` (if executors need metadata)

### Requirements

1. **LLVM backend.** Map `FunctionKind::Local` to internal linkage symbols
   (`linkonce_odr` or `internal`). Ensure captured environment pointers are
   threaded into generated thunks. Stack allocations inside local functions
   follow the same ABI as top-level functions.

2. **WASM backend.** Emit local functions as additional entries in the function
   section. When captures exist, pass the environment pointer as the first
   parameter (mirroring the closure lowering strategy).

3. **Metadata/executors.** Local functions should not appear in reflection or
   module export tables. Test executors (`tests/runtime_*`) need coverage to
   ensure local functions execute correctly on both backends.

4. **Debug info.** Update MIR pretty-printer and any debugging aids to show
   local functions with their lexical parent for readability.

## 6. Documentation & Testing

Docs to update:

- `SPEC.md` (new subsection explaining local function
  syntax, captures, limitations)
- `docs/frontend/parser.md` / `docs/mir/layout.md` if we keep design docs.
- New developer note (this file) referenced from the relevant tracking issue(s).

Tests to add:

- Parser grammar tests in `tests/frontend/parser/tests/grammar/statements.rs`.
- Typeck arena tests covering capturing, generics, async locals.
- MIR builder tests verifying environment lowering.
- Codegen integration tests (LLVM + WASM sample programs).
- Runtime executor tests to ensure nested functions run correctly.

## 7. Next Steps

1. Open a tracking issue and link this analysis for the grammar/typeck/codegen/doc items.
2. Plan implementation phases:
   - Parser + AST support (introduce `StatementKind::LocalFunction`).
   - Typeck/MIR capture & environment handling.
   - Backend/codegen changes.
   - Documentation + examples.

This document will serve as the reference for those steps.
