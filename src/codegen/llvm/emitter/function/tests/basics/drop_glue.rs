use super::super::fixtures::drop_with_deinit_module;
use super::super::helpers::test_target;
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::perf::PerfMetadata;

#[test]
fn llvm_emitter_invokes_deinit_for_drop_statements() {
    let module = drop_with_deinit_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();

    let result = emit_module(
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
    );
    let ir = result.expect("LLVM emission succeeded for drop glue test");
    assert!(
        ir.contains("call void @Demo__Disposable__dispose"),
        "expected dispose call in emitted IR: {ir}"
    );
    assert!(
        ir.contains("call void @__cl_drop__Demo__Disposable"),
        "expected drop glue call in emitted IR: {ir}"
    );
}
