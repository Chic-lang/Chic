# Static member codegen (LLVM & WASM)

Chic static fields, properties, and module-level `static` items lower to backend globals. The pipeline reuses the MIR `StaticVar` table produced during module lowering, so every backend sees the same set of storage slots and initialised values.

- **LLVM:** each `StaticVar` becomes an LLVM global symbol named after its qualified Chic path, sanitised for LLVM (`Demo::Config::Version` -> `@Demo__Config__Version`). Public statics use `dso_local global`; non-public statics use `internal global`. Initialisers are emitted as constants; missing initialisers zero-initialise the slot. Alignment comes from the type layout table. `StaticLoad` lowers to a simple `load` from the global; `StaticStore` lowers to `store`.
- **WASM:** statics are placed in linear memory. The module builder assigns an aligned offset per `StaticVar` and emits a data segment containing either the folded initialiser bytes or zeroes. Function code loads and stores via `i32.load`/`i32.store` (or width-appropriate variants) from the recorded offset. Visibility remains module-local because the wasm MVP lacks symbol visibility; export/import plumbing is deferred to the runtime sidecars.
- **Runtime metadata:** public statics are captured in reflection tables (TypeKind `static`) with mutability and type recorded. Runtime consumers can locate statics via the reflection sidecar once loader support lands.
- **Safety:** mutable statics (`static mut`) are considered unsafe; the MIR lowering emits diagnostics when accessed outside `unsafe` blocks. Immutable statics (`static const`) may be read freely.

Tests: `codegen::llvm::emitter::module::tests::emits_module_static_global`, `codegen::wasm::tests::function_emitter::statics::module_static_load_emits_memory_read`, `codegen::wasm::tests::function_emitter::statics::module_static_store_emits_memory_write`, `mir::builder::tests::statics::module_level_statics_lower_to_static_ops`.
