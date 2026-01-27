# IntelliJ Plugin (Plan + Scaffolding)

Tracking issue: `#42`

## Goal

Provide an IntelliJ IDEA plugin that uses the Chic LSP stack (`impact-lsp` over stdio JSON-RPC) to deliver editor features (diagnostics, hover, go-to-definition) in JetBrains IDEs without introducing new protocol surface.

## Constraints

- The language server entrypoint remains `impact-lsp` (stdio `Content-Length` framing).
- The protocol/types contracts are Chic-owned under `Std.Compiler.Lsp.*` (see `packages/std.compiler.lsp.types` and `packages/std.compiler.lsp.server`).
- Avoid over-scoping: this is a minimal, deletion-oriented starting point.

## Milestones

1) **Project scaffolding**
   - A standalone Gradle project under `intellij-plugin/` (not part of the Rust workspace).
   - Register `.cl` files as “Chic”.

2) **LSP client wiring**
   - Launch `impact-lsp` via stdio and connect using a JetBrains LSP client implementation (evaluate built-in LSP support vs. LSP4IJ).
   - Expose a single setting: `impact-lsp` binary path (default: `impact-lsp` on `$PATH`).

3) **Parity with `chic-vscode`**
   - Diagnostics update on edit (`didOpen`/`didChange`/`didClose`).
   - Hover + definition.

4) **Packaging + release**
   - Plugin metadata, versioning, and a minimal release process (manual zip or marketplace as a follow-up).

