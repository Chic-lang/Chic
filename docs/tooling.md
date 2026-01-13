# Tooling Overview

This index links the Chic tooling design notes that remain active.

## Impact LSP architecture

The `impact-lsp` server brings Chic diagnostics and navigation into editors over JSON-RPC. It reuses the full compiler pipeline (lex/parse, type checking, MIR lowering, borrow/reachability), `FileCache` span math, and the shared diagnostics schema, with an in-memory overlay model for open documents. The `chic-vscode` extension ships a client pre-wired to launch `impact-lsp` (configurable via `chic.lsp.path`) and expose build/test/run commands. See `docs/tooling/impact_lsp.md` for the full design, protocol surface, incremental strategy, and testing plan.

## Package-first layout

- Libraries live under `packages/<name>/manifest.yaml`; `manifest.yaml` declares `package.name`, `namespace`, `build.kind`, `sources`, and `dependencies`.
- Std is modeled as a package (`packages/std/manifest.yaml`). Any manifest-driven build that uses the standard library must declare `std` in its dependencies (e.g. `std: { path: ../std }`); missing entries surface `PKG0103`.
- Downstream packages (like `packages/web`) declare explicit dependencies—no implicit Std loading for manifest builds.
- Package visibility is manifest-only. `@package` source directives are disallowed; missing dependencies must be added to `manifest.yaml`.

## Runtime selection

- Runtime selection is explicit and manifest-driven. Every manifest must declare `toolchain.runtime`
  (kind/package/ABI/path) and builds fail if the runtime is omitted. See
  `docs/tooling/runtime_selection.md` for the current rules and artifact partitioning.

## Mandatory verification loop

- For release-ready changes, keep the loop build-first and deterministic:
  1. `cargo fmt -- --check`
  2. `cargo build --all --all-targets`
  3. `cargo test --all --all-targets --no-run` (compile-only)
  4. Optional: `./target/debug/chic test` for targeted scenarios while runtime stabilization is in progress
  5. Optional: `cargo xtask coverage --min <percent>` and/or `./target/debug/chic coverage --workspace --min <percent>`
- Coverage and runtime test execution are optional unless a task or CI gate explicitly requires them.

## Namespace composition

- Namespaces are open and composable across files and packages. Contributions to `Std.IO` and `Std.IO.Compression` merge into the same logical tree instead of shadowing each other.
- Resolution only considers the manifest dependency closure—no implicit packages participate in lookup. Adding a new dependency can extend a namespace but cannot hide existing parent namespaces.
- `import` directives (including aliases and `import static`) import namespaces without replacing parent or child resolution; fully qualified paths stay stable (`Std.IO.Stream`, `Std.IO.Compression.GZipStream`).
- Duplicate fully qualified symbols across visible packages are deterministic errors (`TCK400: <kind> <name> conflicts with a previous declaration`); the compiler reports both declarations instead of silently picking one.
- Std is split into subpackages (`Std.IO`, `Std.IO.Compression`, `Std.Net`, etc.). Consumers can import any subset without breaking access to parent namespaces, and new subpackages must not shadow existing parents.
