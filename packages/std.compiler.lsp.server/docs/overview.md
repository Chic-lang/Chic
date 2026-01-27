# Std.Compiler.Lsp.Server

Chic-native JSON-RPC/LSP transport primitives (framing + routing) intended to replace the current Rust dependencies (`lsp-server`, `lsp-types`) over time.

## Scope (initial subset)

- JSON-RPC 2.0 wire framing used by LSP: `Content-Length: <n>\\r\\n\\r\\n<body>`.
- Request id correlation helpers (track pending requests, match responses).
- Notification routing by `method` string.

This package is deliberately small and only targets the LSP subset used by Chic tooling today.

