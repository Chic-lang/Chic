# Chic C Header Generation

Chic must export C-compatible headers so other languages (C, C++, Rust, Swift, etc.) can link against Chic libraries. This document outlines the planned `chic header` workflow.

## Goals

- Emit ANSI C headers (`.h`) describing all `public` (and opt-in `internal`) APIs.
- Preserve calling conventions, visibility, and type aliases (including 64-bit integer aliases and future SIMD types).
- Respect attributes: `@extern`, `@link`, `@repr`, `@align`, `@no_mangle`, etc.
- Support namespace flattening into C-style prefixes (e.g., `Geometry_Point`).
- Generate guard macros and `extern "C"` blocks for C++ consumers.
- Allow per-target header emission (`--target x86_64-apple-darwin` vs `aarch64-unknown-linux-gnu`).

## Proposed CLI

```bash
chic header src/library.ch \
  --target x86_64-apple-darwin \
  --out include/geometry.h \
  --include-internals # optional
```

## Implementation Sketch

1. **AST traversal**: Re-use `CompilerDriver::check` to gather `Item::Function`, `Struct`, `Enum`, `Class` (public static members only), etc.
2. **Type lowering**: Map Chic types to C equivalents.
   - primitives → `int32_t`, `uint64_t`, `double`, `int128_t` (via compiler extensions), etc.
   - `string` → `const chic_string*` wrapper (future runtime type).
   - `Task`/`Task<T>` not exported (async APIs must expose explicit future handles).
3. **Name mangling**: convert namespaces (`namespace Geometry;`) into prefixes (`Geometry_`). Provide `@export_name` override.
4. **Attributes**: honor `@extern("C")`, `@link("lib")`, `@cimport("header.h")`, `@repr(c)`; warn when layout cannot be represented in C.
5. **Header formatting**:
   - Include guard: `#ifndef GEOMETRY_H` / `#define GEOMETRY_H`.
   - `#ifdef __cplusplus` guard for `extern "C"`.
   - Emit documentation comments (convert `///` XML into Doxygen-style tags).
6. **Integration tests**: compile generated headers with Clang/GCC (`-Wall -Werror`) in CI.

## Open Questions

- How to map Chic generics to C? (Likely unsupported; require specialized exports.)
- Handling `struct`/`class` with methods: export only static/free functions; record fields as `typedef struct`.
- Async exports: need FFI-safe future/handle type in `std.async`.

## Deliverables

- `chic header` CLI subcommand.
- Library code that walks AST and prints headers.
- CI job verifying emitted headers compile with Clang (macOS) and GCC (Linux).
- Documentation updates in `SPEC.md` and `docs/manifest_manifest.md`.

When behavior changes, keep this document and the relevant CLI help text in sync.
