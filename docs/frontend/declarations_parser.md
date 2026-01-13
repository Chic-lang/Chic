# Declaration Parser Guide

| Syntax | Entry Function | Module |
| --- | --- | --- |
| `class`, `record struct`, `struct`, `enum`, `union`, `trait`, `interface` | `parse_class`, `parse_struct`, `parse_enum`, `parse_union`, `parse_trait`, `parse_interface` | `parser/declarations/types/{classes,interfaces,traits,structs,enums}` |
| `impl`, `testcase` declarations | `parse_impl`, `parse_testcase` | `parser/declarations/functions.rs`
| `extension` declarations and `when` constraints | `parse_extension`, `parse_extension_conditions` | `parser/declarations/resources.rs` |

Each module keeps the shape-specific helpers close to the entry-point so diagnostics and recovery code stay localized. When adding a new declaration form choose the module that best matches the surface syntax:

1. **Type declarations** – extend the relevant submodule under `types/` (classes/traits/structs/enums) and update the table above.
2. **Function-style declarations (impl/test)** – extend `functions.rs`.
3. **Resource/extension declarations** – add logic to `resources.rs` and document how constraints map to `ExtensionCondition`.

All entry-points are re-exported via `parser::declarations::mod.rs`, so call sites continue to use `self.parse_*` unchanged. The parser unit tests under `src/frontend/parser/tests/grammar/declarations/` cover the new modules.

## Test Coverage

- `src/frontend/parser/tests/grammar/declarations/types/` exercises struct/class/trait/enum/union parsing (see README for layout), including modifier/attribute diagnostics, nested declarations, and mmio edge cases.
- `src/frontend/parser/tests/grammar/declarations/functions.rs` drives the `impl` and `testcase` entry-points (trait vs inherent impls, async/sync testcases, parameter capture, and error recovery).
- `src/frontend/parser/tests/grammar/declarations/{classes,interfaces,extensions}.rs` contain focused suites that also hit the shared declaration helpers.
