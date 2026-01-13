#![cfg(test)]

use super::super::common::*;
use crate::chic_kind::ChicKind;
use crate::codegen::wasm::test_emit_module;
use crate::runtime::wasm_executor::execute_wasm;

#[test]
fn module_builder_guarded_match_executes_true_branch() {
    let function = guarded_match_function("Main", 1, true, 10, 20);
    let bytes = test_emit_module(vec![function], Some("Main".into()), ChicKind::Executable);
    let outcome = execute_wasm(&bytes, "chic_main").expect("guarded match wasm should execute");
    assert_eq!(
        outcome.exit_code, 10,
        "guard satisfied branch should return success value"
    );
}

#[test]
fn module_builder_guarded_match_falls_back_when_guard_false() {
    let function = guarded_match_function("Main", 1, false, 10, 20);
    let bytes = test_emit_module(vec![function], Some("Main".into()), ChicKind::Executable);
    let outcome = execute_wasm(&bytes, "chic_main").expect("guarded match wasm should execute");
    assert_eq!(
        outcome.exit_code, 20,
        "guard failure should take fallback branch"
    );
}

#[test]
fn module_builder_rejects_pending_terminator() {
    let harness =
        WasmFunctionHarness::from_module(module_with_functions(vec![function_with_pending()]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder");
    let err = builder
        .emit_code_section()
        .expect_err("pending terminator should fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("cannot lower pending terminators")
            || msg.contains("pending lowering operations"),
        "unexpected error message: {msg}"
    );
}

#[test]
fn module_builder_emit_propagates_function_emitter_failures() {
    let harness = WasmFunctionHarness::from_module(module_with_functions(vec![
        match_with_projection_function(),
    ]));
    let builder = harness
        .module_builder(None, ChicKind::Executable)
        .expect("builder");
    let err = builder
        .emit()
        .expect_err("projected match lowering should cause module emission to fail");
    let message = format!("{err}");
    assert!(
        message.contains("projected match values"),
        "unexpected error message for propagated emitter failure: {message}"
    );
}

#[test]
fn module_builder_emits_borrow_lowering() {
    let bytes = test_emit_module(vec![borrow_operand_function()], None, ChicKind::Executable);
    assert!(
        !bytes.is_empty(),
        "borrow lowering should succeed and emit a non-empty wasm module"
    );
}
