# Attribute Macro & Meta-Programming Pipeline

Updated: 2025-11-18

This document describes the staged attribute pipeline that runs before MIR lowering. The pipeline is split into three explicit stages so language frontends, LSP tooling, and backends share the same ordering, hygiene, and diagnostic rules.

## Stages

1. **Collection (parser)**
   - `frontend::parser::attributes` records every surface attribute as an AST `Attribute` and annotates macro candidates with `AttributeMacroMetadata { expandable, tokens }`.
   - `expandable` is **true** only for attributes the parser classified as macros (`AttributeKind::Macro`), letting tooling distinguish intrinsic/builtin annotations from user-defined macros.
   - `tokens` captures the raw, whitespace/comment-stripped token stream for the attribute span so LSP/formatters can present precise ranges and macro executors can read the original surface syntax when needed.
2. **Expansion (macro expander)**
   - `frontend::macro_expander::engine::expand_module` walks the AST, drains expandable attributes into `MacroInvocation`s, and executes registered derives/attribute handlers.
   - Expansion runs in deterministic lexical order within a pass, stamps newly generated items with the originating attribute span (via `stamp_items_with_origin`), and re-runs until a fixed point or `MAX_MACRO_PASSES` (32) is reached.
   - Each invocation receives a `HygieneId` (composed from the pass number and a per-pass counter) and the captured token stream so handlers can build hygienic identifiers or diagnostics without reparsing source text.
3. **Evaluation (staged attributes & diagnostics)**
   - After macro expansion stabilises, `frontend::attributes::stage_builtin_attributes` re-applies builtin semantics (DI/module metadata, layout hints, etc.) to the expanded tree.
   - Diagnostics from collection, expansion, and staging are merged so the driver surfaces a single ordered list before MIR lowering.

## Ordering & Hygiene Guarantees

- Attributes are evaluated in lexical order on each target. Derive macros execute before procedural attributes on the same item; unsupported placements emit diagnostics without short-circuiting siblings.
- New items emitted by macros are appended immediately after the source item and are eligible for further passes, enabling chained expansions while keeping ordering stable across runs.
- Macro annotations are stripped from the AST once collected, preventing double execution and ensuring subsequent compiler phases never observe macro-only attributes.
- `HygieneId` is stable per pass and invocation, allowing memoisation keys to stay deterministic while still differentiating distinct attribute uses.

## Diagnostics & Failure Modes

- Unknown or unsupported macros produce errors at the attribute span; cached expansions ensure duplicate derives emit diagnostics once.
- A runaway guard (`macro expansion exceeded 32 passes`) halts recursive expansions deterministically instead of hanging the compiler.
- Generated items inherit the originating attribute span on blocks/statements/expressions, so downstream type checking and lowering map failures back to the macro site instead of anonymous `<generated>` spans.

## Tooling & LSP Hooks

- `Attribute.macro_metadata.tokens` exposes the raw token stream for LSP hovers and formatter hints; tooling can also consult `MacroInvocation.hygiene` to render stable IDs in logs.
- `MacroExpansionResult` reports cache hits/misses and pass counts so editors can surface macro churn or runaway risks.
- Because staged builtin interpretation runs **after** expansion, LSP servers can rely on the expanded tree for DI/layout metadata, ensuring parity with MIR/type checking.

## Authoring/Consuming Macros

Minimal attribute macro handler (Rust-side) that rewrites a function body:

```rust
fn trace_attribute(input: AttributeInput<'_>) -> AttributeOutput {
    let func = match input.target {
        AttributeTarget::Function(func) | AttributeTarget::Method { function: func, .. } => func,
    };
    let origin_span = input.invocation.span;
    func.body = Some(Block {
        span: None,
        statements: vec![Statement::new(origin_span, StatementKind::Expression(
            Expression::new("Std.Debug.trace()", origin_span),
        ))],
    });
    AttributeOutput::empty()
}
```

Registering the handler:

```rust
let mut registry = MacroRegistry::with_builtins();
registry.register_attribute("trace", trace_attribute);
```

Consumers annotate code with `@trace` and the expander will inject the traced body before type checking/MIR lowering. The stamped spans ensure any downstream diagnostics point to the `@trace` site.
