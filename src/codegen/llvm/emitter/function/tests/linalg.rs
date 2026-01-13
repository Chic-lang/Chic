use super::fixtures::linalg_dpbusd_module;
use super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::codegen::{CodegenOptions, CpuIsaConfig, CpuIsaTier};
use crate::perf::PerfMetadata;

#[test]
fn linalg_dpbusd_requires_avx512_config() {
    if std::env::var("CHIC_ENABLE_X86_DPBUSD_TESTS").is_err() {
        eprintln!(
            "skipping linalg_dpbusd_requires_avx512_config (set CHIC_ENABLE_X86_DPBUSD_TESTS=1 to enable)"
        );
        return;
    }
    let module = linalg_dpbusd_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::Avx2,
        CpuIsaTier::Avx512,
    ]);
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
    )
    .expect("emit module");

    let baseline = function_ir(&result, "Root__Dpbusd__baseline");
    assert!(
        baseline.contains("llvm.trap"),
        "baseline tier should trap without dpbusd support"
    );
}

#[test]
fn linalg_dpbusd_generates_intrinsic_and_trap_fallback() {
    if std::env::var("CHIC_ENABLE_X86_DPBUSD_TESTS").is_err() {
        eprintln!(
            "skipping linalg_dpbusd_generates_intrinsic_and_trap_fallback (set CHIC_ENABLE_X86_DPBUSD_TESTS=1 to enable)"
        );
        return;
    }
    let module = linalg_dpbusd_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::Avx2,
        CpuIsaTier::Avx512,
    ]);
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
    )
    .expect("emit module");

    let avx512 = function_ir(&result, "Root__Dpbusd__avx512");
    assert!(
        avx512.contains("@llvm.x86.avx512.vpdpbusd.512"),
        "expected AVX512 variant to call vpdpbusd intrinsic"
    );

    let baseline = function_ir(&result, "Root__Dpbusd__baseline");
    assert!(
        baseline.contains("llvm.trap"),
        "baseline variant should trap when intrinsic unavailable"
    );
}
