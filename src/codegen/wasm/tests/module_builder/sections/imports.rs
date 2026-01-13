#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;
use crate::codegen::wasm::tests::module_builder::common::*;
use crate::codegen::wasm::{
    LINEAR_MEMORY_MIN_PAGES, STACK_BASE, ValueType, push_i32_const_expr, write_u32,
};
use crate::mir::*;

#[test]
fn module_builder_emits_memory_and_global_sections() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Entry",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Entry"), ChicKind::Executable)
        .unwrap();

    let memory = builder
        .emit_memory_section()
        .expect("memory section result")
        .expect("memory section should be present");
    assert_eq!(memory.id(), 5);
    let mut memory_expected = Vec::new();
    write_u32(&mut memory_expected, 1);
    memory_expected.push(0x00);
    write_u32(&mut memory_expected, LINEAR_MEMORY_MIN_PAGES);
    assert_eq!(memory.payload_bytes(), memory_expected);

    let global = builder
        .emit_global_section()
        .expect("global section result")
        .expect("global section should be present");
    assert_eq!(global.id(), 6);
    let mut expected = Vec::new();
    write_u32(&mut expected, 1);
    expected.push(ValueType::I32.to_byte());
    expected.push(0x01);
    push_i32_const_expr(&mut expected, STACK_BASE as i32);
    assert_eq!(global.payload_bytes(), expected);
}

#[test]
fn module_builder_emits_runtime_imports() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Entry",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Entry"), ChicKind::Executable)
        .unwrap();

    let state = compute_signature_state(harness.module(), false);
    let import_section = builder
        .emit_import_section()
        .expect("import section should emit");
    assert_eq!(import_section.id(), 2);

    let payload = import_section.payload_bytes();
    let mut cursor = 0usize;
    let count = read_uleb(payload, &mut cursor);
    let runtime_hooks = runtime_hooks(false);
    assert_eq!(
        count,
        runtime_hooks.len() as u32,
        "expected runtime imports for panic/abort/throw/await/yield/cancel/borrow/drop/string formatting, string slicing, and mmio hooks"
    );

    for hook in &runtime_hooks {
        let module = read_string(payload, &mut cursor);
        assert_eq!(module, hook.module());
        let name = read_string(payload, &mut cursor);
        assert_eq!(name, hook.name());
        assert_eq!(payload[cursor], 0x00);
        cursor += 1;
        let ty = read_uleb(payload, &mut cursor);
        let expected_index = state
            .runtime_hook_indices
            .get(hook)
            .copied()
            .expect("runtime hook index should be recorded");
        assert_eq!(ty, expected_index);
    }
}

#[test]
fn module_builder_populates_function_table_and_elements() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Entry",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Entry"), ChicKind::Executable)
        .unwrap();
    let module_ref = harness.module();

    let table_section = builder
        .emit_table_section()
        .expect("table section result")
        .expect("table section should be present");
    assert_eq!(table_section.id(), 4);
    let mut cursor = 0usize;
    let table_payload = table_section.payload_bytes();
    let table_count = read_uleb(table_payload, &mut cursor);
    assert_eq!(table_count, 1);
    assert_eq!(table_payload[cursor], 0x70);
    cursor += 1;
    assert_eq!(table_payload[cursor], 0x00);
    cursor += 1;
    let table_min = read_uleb(table_payload, &mut cursor);
    assert!(table_min >= module_ref.functions.len() as u32);

    let element_section = builder
        .emit_element_section()
        .expect("element section result")
        .expect("element section should be present");
    assert_eq!(element_section.id(), 9);
    cursor = 0;
    let elements = element_section.payload_bytes();
    let segment_count = read_uleb(elements, &mut cursor);
    assert_eq!(segment_count, 1);
    assert_eq!(elements[cursor], 0x00);
    cursor += 1;
    assert_eq!(elements[cursor], 0x41);
    cursor += 1;
    assert_eq!(elements[cursor], 0x00);
    cursor += 1;
    assert_eq!(elements[cursor], 0x0B);
    cursor += 1;
    let element_count = read_uleb(elements, &mut cursor);
    assert_eq!(element_count, table_min);
    for expected_index in 0..element_count {
        let func_index = read_uleb(elements, &mut cursor);
        assert_eq!(func_index, expected_index);
    }
}
