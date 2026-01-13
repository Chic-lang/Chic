# Impact LSP Design

_Last updated: 2025-02-06_

Impact LSP brings Chic compiler features into editors via the Language Server Protocol while reusing the existing compiler crates and diagnostics model. This document captures the initial architecture, goals, and testing plan for the `impact-lsp` server and the VS Code client wiring.

## Goals and constraints

- **Single source of truth:** Reuse the Chic frontend, type checker, MIR lowering, and diagnostics pipeline. Do not fork logic inside the server.
- **Incremental correctness:** Edits invalidate only the affected modules; diagnostics and navigation refresh without full recompiles.
- **Determinism:** Given file contents, responses (diagnostics, hovers, definitions, semantic tokens) are stable across machines and sessions.
- **Shared diagnostics schema:** Preserve severity taxonomy, codes, categories, spans, and JSON schema `1.0.0` (see `docs/guides/diagnostics.md`).
- **Workspaces first:** Honour `manifest.yaml`/workspace roots, multi-file modules, and in-memory overlays from editors.
- **Performance:** No full rebuild per keystroke; target <150ms for small edits and sub-second refresh for medium modules. Background refresh tasks must be cancelable.

## Existing building blocks

- **File cache and spans:** `diagnostics::FileCache` tracks file IDs and line/column math for spans. Used by diagnostic formatters and will back LSP range conversion.
- **Diagnostics model:** `Diagnostic` + `DiagnosticCode` + `Suggestion` with renderers (`human`, `short`, `json`, `toon`) already shared by CLI/tests.
- **Frontend pipeline:** `driver::CompilerPipelineBuilder` orchestrates lex/parse → macro expansion → type checking → MIR lowering → borrow/reachability. It produces `FrontendReport` with MIR slices, diagnostics, and perf metadata.
- **Incremental caches:** `driver::incremental` snapshots files and artifacts for builds. While LSP uses in-memory overlays, the hashing and manifest fingerprinting are reusable for cache keys.
- **CLI workflows:** `chic check/build/test/run` with `--error-format` flag and incremental controls. LSP should invoke the same stages rather than reimplementing behaviour.

## Server architecture

- **Binary name:** `impact-lsp` (JSON-RPC 2.0 over stdin/stdout). Provides a thin transport layer plus a workspace session manager.
- **Workspace session:** One session per root URI/workspace folder. Tracks:
  - manifest/workspace config, target triple, and backend/runtime settings (mirrors CLI defaults);
  - document store (URI → version, contents, FileId) with in-memory overlays overriding disk;
  - module graph and dependency metadata from the compiler pipeline;
  - memoised analysis results (parse trees, HIR/type tables, MIR slices, symbol index).
- **Overlay model:** `didOpen`/`didChange` replace the source stored in `FileCache`; `didClose` can drop overlays while keeping last-known content for navigation. File IDs remain stable per URI to keep diagnostic ranges deterministic.
- **Incremental invalidation:** Changes mark the owning module/package dirty. Reuse existing pipeline hooks to re-run lex/parse for the changed file, then type-check/MIR-lower only the affected module and its dependants. Long-running work is cancellable per request token.
- **Concurrency:** Single-threaded request sequencing per session to preserve ordering, with a small task pool for background refresh (formatting, symbol index rebuild) that feeds results back through the main loop.

## Protocol surface (v1)

- **Lifecycle:** `initialize`/`initialized`/`shutdown`/`exit`. Advertise serverInfo.version and a stable capabilities block.
- **Documents:** `textDocument/didOpen`/`didChange`/`didClose`. `didSave` optional hook to trigger full validation.
- **Diagnostics:** Support the pull API (`textDocument/diagnostic`) when the client declares it; fall back to `textDocument/publishDiagnostics` otherwise. Diagnostics are sourced from the compiler pipeline and keep:
  - `code` = `DiagnosticCode.code`, `codeDescription` (when spec links exist), `source` = `"chic"`;
  - ranges mapped from `Span` → `LineCol` via `FileCache`;
  - related information from secondary labels; suggestions surfaced as CodeActions.
- **Phase parity:** Lexer/parser + type-check + MIR + borrow/reachability diagnostics flow through the same channel so LSP mirrors `chic check`.
- **Navigation:** `textDocument/hover`, `textDocument/definition`, `textDocument/documentSymbol`, and `workspace/symbol` (bounded result set). Reference search is optional in v1. Hover/definition resolve using MIR symbol spans when available and fall back to token-based navigation.
- **Edits:** `textDocument/formatting` delegates to `chic format` or an in-process formatter; `textDocument/codeAction` emits quick fixes from compiler suggestions and simple cleanups. Inlay hints and semantic tokens are surfaced when the semantic model is available.
- **Commands:** Expose build/test/run invocations (`chic build|test|run`) with streamed output in the LSP `window/logMessage` channel for the VS Code extension commands.

## Diagnostics and code actions

- **Mapping:** Severity → LSP severity (Error/Warning/Information/Hint). Category flows into `codeDescription` when present. Keep JSON schema `1.0.0` for consumers that request raw payloads.
- **Ranges:** Compute using the same `FileCache` logic as CLI renderers to avoid off-by-one drift; unknown file IDs are rendered without ranges.
- **Suggestions:** Each `Suggestion` with a span+replacement becomes a `CodeAction` edit; span-less suggestions become command-style actions with a description.
- **Stability:** Do not reorder diagnostics for a document within a run; sort by primary span start then code to keep snapshots stable.

## Incremental strategy

- **Parsing:** Maintain per-file parse trees; re-lex/re-parse only changed files. LL(1) parser hooks already collect diagnostics; reuse them directly.
- **Type checking:** Track module-level type tables. On change, invalidate the owning module and any dependants (imports, import directives). Re-run type checking and MIR lowering for the dirty set only.
- **MIR and borrow/reachability:** Run these passes lazily when a feature needs them (hover on a symbol inside a function, go-to-definition on a local). Cache MIR slices in the session for reuse across requests.
- **Perf guards:** Budget timers per stage with logging to the tracing backend; emit `"chic-lsp"` span metadata that mirrors CLI `--trace-pipeline` output for debugging.

## Testing and verification

- **Protocol snapshots:** JSON-RPC fixtures for `initialize` → `didOpen` → diagnostics; checked into `tests/lsp/` with stable IDs.
- **Incremental tests:** Open file with an error → diagnostic appears; apply change that fixes it → diagnostic clears without rebuilding other modules; introduce new error → new diagnostic appears in-order.
- **Navigation tests:** Hover/definition/documentSymbol against sample modules (including multi-file namespaces) to ensure spans resolve correctly.
- **Stability tests:** Capability negotiation and diagnostic schema snapshots to catch breaking changes before publishing the extension.
- **Perf tests:** Smoke test that rapid edits (<100ms apart) do not enqueue redundant full rebuilds; background tasks honour cancellation.

## Open items and future work

- Semantic tokens legend and inlay hint catalogue depend on the semantic model; design the token kinds once the MIR/type metadata surface is plumbed through the session.
- Debug/trace streaming for VS Code’s Output panel should share the CLI logging formatter to keep parity.
- Workspace watching for on-disk changes (created/deleted files) will land after the core request/response loop is stable.
