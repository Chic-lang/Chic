use super::fixtures::simd_fma_module;
use super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::codegen::{CodegenOptions, CpuIsaConfig};
use crate::perf::PerfMetadata;

#[test]
fn simd_fma_multiversion_includes_intrinsic_and_fallback() {
    if std::env::var("CHIC_ENABLE_SIMD_TESTS").is_err() {
        eprintln!(
            "skipping simd_fma_multiversion_includes_intrinsic_and_fallback (set CHIC_ENABLE_SIMD_TESTS=1 to enable)"
        );
        return;
    }
    let module = simd_fma_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::auto();
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

    let baseline = function_ir(&ir, "Root__SimdFma__baseline");
    assert!(baseline.contains("fmul <8 x float>"));
    assert!(baseline.contains("fadd <8 x float>"));

    let avx2 = function_ir(&ir, "Root__SimdFma__avx2");
    assert!(
        avx2.contains("@llvm.fma.v8f32"),
        "expected avx2 variant to call llvm.fma intrinsic"
    );
}
