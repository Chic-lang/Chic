# Std.Compiler.Lsp.Types

Chic-native types for the subset of the Language Server Protocol (LSP) used by the Chic tooling today (`impact-lsp` + `chic-vscode`).

## Scope (initial subset)

- JSON-RPC envelope fields that matter for Chic tooling (request id, method string, params/result payload).
- Lifecycle: `initialize`, `shutdown`, `exit`.
- Documents: `textDocument/didOpen`, `textDocument/didChange`, `textDocument/didClose`.
- Diagnostics: `textDocument/publishDiagnostics`.
- Navigation: `textDocument/hover`, `textDocument/definition`.

## Diagnostics mapping

This package does not define Chic diagnostics; it models LSP shapes. Chic diagnostics follow `SPEC.md` (severity taxonomy, codes/categories, spans via `FileCache`), and are mapped to LSP diagnostics by the LSP host layer.

