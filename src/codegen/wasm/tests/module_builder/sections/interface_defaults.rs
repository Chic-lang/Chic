#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;
use crate::mir::*;

#[test]
fn interface_defaults_section_omitted_when_empty() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Geometry::Entry",
        FunctionKind::Function,
        Ty::Unit,
    )]));
    let builder = harness
        .module_builder(Some("Geometry::Entry"), ChicKind::Executable)
        .expect("construct module builder");
    let section = builder
        .emit_interface_defaults_section()
        .expect("section result");
    assert!(
        section.is_none(),
        "expected no interface default section when module does not record bindings"
    );
}

#[test]
fn interface_defaults_section_lists_entries() {
    let mut module = module_with_functions(vec![simple_function(
        "Geometry::IRenderable::Draw",
        FunctionKind::Function,
        Ty::Unit,
    )]);
    module.interface_defaults.push(InterfaceDefaultImpl {
        implementer: "Geometry::Circle".into(),
        interface: "Geometry::IRenderable".into(),
        method: "Draw".into(),
        symbol: "Geometry::IRenderable::Draw".into(),
    });

    let harness = WasmFunctionHarness::from_module(module);
    let builder = harness
        .module_builder(None, ChicKind::StaticLibrary)
        .expect("construct module builder");
    let section = builder
        .emit_interface_defaults_section()
        .expect("section result")
        .expect("section should be emitted when defaults exist");
    assert_eq!(section.id(), 0);
    let payload = section.payload_bytes();
    assert!(
        contains_bytes(payload, b"chic.iface.defaults"),
        "payload should include section label"
    );
    assert!(
        contains_bytes(payload, b"Geometry::Circle"),
        "implementer name should be encoded"
    );
    assert!(
        contains_bytes(payload, b"Geometry::IRenderable"),
        "interface name should be encoded"
    );
    assert!(
        contains_bytes(payload, b"Draw"),
        "method name should be encoded"
    );
}
