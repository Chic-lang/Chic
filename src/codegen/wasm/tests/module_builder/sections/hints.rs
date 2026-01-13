#![cfg(test)]

use crate::chic_kind::ChicKind;
use crate::codegen::wasm::tests::common::*;
use crate::mir::{FunctionKind, Ty};

#[test]
fn hint_section_encodes_function_hints() {
    let mut hot_func = simple_function("Hot", FunctionKind::Function, Ty::Unit);
    hot_func.optimization_hints.hot = true;
    hot_func.optimization_hints.always_inline = true;
    let mut cold_func = simple_function("Cold", FunctionKind::Function, Ty::Unit);
    cold_func.optimization_hints.cold = true;
    cold_func.optimization_hints.never_inline = true;

    let harness =
        WasmFunctionHarness::from_module(module_with_functions(vec![hot_func, cold_func]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder creation");
    let section = builder
        .emit_hint_section()
        .expect("hint section generation");
    let section = section.expect("hint section should be present");
    assert_eq!(section.id(), 0);
    let payload = section.payload_bytes();
    assert!(
        payload
            .windows("chic.hints".len())
            .any(|window| window == b"chic.hints"),
        "hint section should include name label"
    );
    assert!(
        payload
            .windows("Hot:hot|always_inline".len())
            .any(|window| window == b"Hot:hot|always_inline"),
        "hot entry should be encoded"
    );
    assert!(
        payload
            .windows("Cold:cold|never_inline".len())
            .any(|window| window == b"Cold:cold|never_inline"),
        "cold entry should be encoded"
    );
}
