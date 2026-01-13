#![cfg(test)]

use super::super::common::*;
use super::common::*;
use crate::chic_kind::ChicKind;
use crate::codegen::wasm::{WASM_MAGIC, WASM_VERSION, test_emit_module};
use crate::mir::{FunctionKind, Ty};
use crate::runtime::wasm_executor::parser::parse_module;

#[test]
fn emit_module_produces_wasm_header() {
    let bytes = test_emit_module(
        vec![simple_function("Main", FunctionKind::Function, Ty::Unit)],
        Some("Main".into()),
        ChicKind::Executable,
    );
    assert_eq!(
        &bytes[..WASM_MAGIC.len()],
        &WASM_MAGIC,
        "module should start with wasm magic"
    );
    assert_eq!(
        &bytes[WASM_MAGIC.len()..WASM_MAGIC.len() + WASM_VERSION.len()],
        &WASM_VERSION,
        "module should encode wasm version header"
    );
}

#[test]
fn module_builder_roundtrip_validates_module() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::Executable)
        .expect("construct module builder");
    let bytes = builder.emit().expect("emit wasm bytes");
    let parsed = parse_module(&bytes).expect("runtime parser should accept generated module");
    assert_eq!(parsed.functions.len(), harness.module().functions.len());
    assert_eq!(
        parsed.imports.len(),
        runtime_hooks(false).len(),
        "runtime imports should include the runtime hook set"
    );
    assert!(
        parsed.exports.contains_key("chic_main"),
        "entry export should be present"
    );
}

#[test]
fn module_builder_roundtrip_rejects_corruption() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![simple_function(
        "Main",
        FunctionKind::Function,
        Ty::named("int"),
    )]));
    let builder = harness
        .module_builder(Some("Main"), ChicKind::Executable)
        .expect("construct module builder");
    let mut bytes = builder.emit().expect("emit wasm bytes");
    bytes[0] = 0xFF;
    let err = parse_module(&bytes).expect_err("corrupted header should be rejected");
    assert!(
        err.message.contains("invalid wasm header"),
        "unexpected parser error message: {}",
        err.message
    );
}
