#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;
use crate::mir::*;

#[test]
fn module_builder_metadata_section_includes_descriptor() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Entry",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Entry"), ChicKind::Executable)
        .expect("construct module builder");
    let section = builder
        .emit_metadata_section()
        .expect("metadata section result");
    assert_eq!(section.id(), 0);
    let payload = section.payload_bytes();
    assert!(
        payload
            .windows("chic.metadata".len())
            .any(|window| window == b"chic.metadata"),
        "metadata payload should include section label"
    );
    assert!(
        payload
            .windows("target=wasm32;kind=executable".len())
            .any(|window| window == b"target=wasm32;kind=executable"),
        "metadata payload should encode descriptor string"
    );
}

#[test]
fn module_builder_metadata_section_contains_target_and_kind() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(Vec::new()));
    let builder = harness
        .module_builder(None, ChicKind::DynamicLibrary)
        .expect("builder");
    let section = builder
        .emit_metadata_section()
        .expect("metadata section result");
    assert_eq!(section.id(), 0);
    let payload = section.payload_bytes();
    assert!(
        payload
            .windows("chic.metadata".len())
            .any(|window| window == b"chic.metadata"),
        "metadata section should include name label"
    );
    assert!(
        payload
            .windows("target=wasm32;kind=dynamic-library".len())
            .any(|window| window == b"target=wasm32;kind=dynamic-library"),
        "metadata should encode target/kind descriptor"
    );
}

#[test]
fn metadata_section_respects_payload_alignment() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::Executable)
        .expect("builder");
    let section = builder
        .emit_metadata_section()
        .expect("metadata section result");
    let payload = section.payload_bytes();
    assert!(
        payload.starts_with(&["chic.metadata".len() as u8]),
        "metadata section should start with length-prefixed name"
    );
}
