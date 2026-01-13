#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;

#[test]
fn module_builder_emits_async_await_function() {
    let harness =
        WasmFunctionHarness::from_module(module_with_functions(
            vec![async_single_await_function()],
        ));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder");
    let section = builder
        .emit_code_section()
        .expect("await lowering should succeed");
    assert_eq!(section.id(), 10);
    assert!(
        !section.payload_bytes().is_empty(),
        "code section payload expected"
    );
}

#[test]
fn module_builder_emits_generator_function() {
    let harness =
        WasmFunctionHarness::from_module(module_with_functions(vec![function_with_yield()]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder");
    let section = builder
        .emit_code_section()
        .expect("yield lowering should succeed");
    assert_eq!(section.id(), 10);
    assert!(
        !section.payload_bytes().is_empty(),
        "code section payload should not be empty"
    );
}
