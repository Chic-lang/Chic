# Conditional compilation & optimization hints

This guide explains how Chic’s conditional compilation and optimization hints work end-to-end.

## Define Map & Evaluation Order

1) Textual directives (`#if` / `#elif` / `#else` / `#endif`) run before lexing. Inactive regions are replaced with whitespace so source offsets stay stable.  
2) Structural filters (`@cfg(...)`) run after parsing and again after macro expansion. Inactive items/statements vanish before name resolution, MIR lowering, and codegen.

Both layers share the same define map (case-insensitive keys):

- `DEBUG` / `RELEASE` (mutually exclusive) plus `PROFILE` = `debug` or `release`.
- `TRACE` defaults to `true` for both debug and release; override with `--define TRACE=false` to strip tracing calls.
- Target data: `TARGET` / `TARGET_TRIPLE`, `TARGET_ARCH`, `TARGET_OS`, `TARGET_ENV`.
- Backend and artifact: `BACKEND` (`llvm` / `wasm`), `KIND` (`executable` / `static-library` / `dynamic-library`).
- Feature toggles: `feature_<name>` from `--define feature=a,b,c` (non-alphanumerics normalize to `_`).
- CLI overrides: `--define KEY` sets a boolean; `--define KEY=value` sets a string or boolean literal. Overriding only `DEBUG` or `RELEASE` flips the counterpart automatically.

Functions marked `@conditional("FLAG")` are removed entirely when `FLAG` is unset/false; their arguments are not evaluated, so they are safe to leave in production builds alongside `DEBUG`/`TRACE` guards.

Expressions accept `&&`, `||`, `!`, parentheses, `==`/`=`/`!=` comparisons against string or boolean literals; unknown identifiers evaluate to `false`. Mixing string/boolean in a comparison raises an error.

## `@cfg` Usage Patterns

- Attach `@cfg` to namespaces, types, members, free functions/methods/constructors, properties/accessors, testcases, local functions, or statements. Example:

```chic
@cfg(target_os == "linux" && BACKEND == "llvm")
public extern void EnableSyscallFastPath();

public void Configure() {
    @cfg(DEBUG) var sink = Std.Diagnostics.MakeVerbose();
    @cfg(!DEBUG) var sink = Std.Diagnostics.MakeStructured();
}
```

- When guarding an `if` or loop, inactive branches are stripped and the remaining branch is hoisted where possible (`else` replaces the whole statement when the `if` arm is inactive).
- Apply the pass to stdlib and workspace code so backend- or OS-specific shims live alongside portable implementations instead of duplicating modules.
- Use `feature_<flag>` for optional capabilities (`@cfg(feature_simd && TARGET_ARCH == "x86_64")`).

## Optimisation Hints

Decorate functions, methods, constructors, or testcases with advisory hints:

- `@hot` / `@cold` flag likely or rare paths.
- `@always_inline` / `@never_inline` request or forbid inlining.
- Duplicates and conflicts (`@hot`+`@cold`, `@always_inline`+`@never_inline`) are rejected during lowering. Hints never change borrow rules or determinism.

Backend mapping:

- **LLVM:** emits `hot` / `cold` / `alwaysinline` / `noinline` attributes on the function.
- **WASM:** writes a custom section `chic.hints` containing `symbol:hint|hint` entries for tooling and engines.

## Practical guidance

1. Replace runtime platform checks with `@cfg` guards so unreachable branches disappear before codegen (`@cfg(target_os == "windows")` instead of `if Std.Platform.IsWindows()`).
2. Keep `DEBUG`/`RELEASE` branches symmetric: ensure the surviving branch still binds required locals or initialises resources.
3. Centralise feature names: use `--define feature=simd,gpu` and gate with `feature_simd`, `feature_gpu` rather than introducing bespoke flags per file.
4. Add optimisation hints only to validated hot/cold paths backed by profiling. Pair every hint with a benchmark or perf test to prove the win.
5. For libraries, prefer `@hot` on inlineable helpers and `@never_inline` on slow-path validation routines; avoid hinting ABI boundary thunks that hosts may override.
