use super::super::fixtures::flag_enum_module;
use super::super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::perf::PerfMetadata;

#[test]
fn llvm_emitter_lowers_flag_bitwise_ops() {
    let module = flag_enum_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();

    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let function = function_ir(&ir, "Flags__Combine");
    assert!(
        function.contains("or i32 1, 2"),
        "expected bitwise or lowering, got:\n{function}"
    );
    assert!(
        function.contains("and i32"),
        "expected bitwise and lowering, got:\n{function}"
    );
}
