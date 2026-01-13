use super::fixtures::{apple_dpbusd_module, simd_fma_module};
use super::helpers::linux_target;
use crate::chic_kind::ChicKind;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::codegen::{CodegenOptions, CpuIsaConfig, CpuIsaTier};
use crate::perf::PerfMetadata;

#[test]
fn linux_dispatch_uses_getauxval() {
    let module = simd_fma_module();
    let target = linux_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
        CpuIsaTier::Sve,
        CpuIsaTier::Sve2,
    ]);
    options.cpu_isa.set_sve_bits(256).expect("set sve bits");
    options.sve_vector_bits = options.cpu_isa.sve_vector_bits();
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
    assert!(
        ir.contains("declare i64 @getauxval"),
        "expected getauxval declaration in linux dispatch"
    );
    assert!(
        ir.contains("@chic_cpu_sve_bits = internal global i32 256"),
        "expected pinned SVE bits global"
    );
    assert!(
        ir.contains("call i64 @getauxval(i64 16)"),
        "expected hwcap probe"
    );
}

#[test]
fn linux_sve_lowering_emits_intrinsics() {
    if std::env::var("CHIC_ENABLE_LINUX_SVE_TESTS").is_err() {
        eprintln!(
            "skipping linux_sve_lowering_emits_intrinsics (set CHIC_ENABLE_LINUX_SVE_TESTS=1 to enable)"
        );
        return;
    }
    let module = apple_dpbusd_module();
    let target = linux_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
        CpuIsaTier::Sve,
        CpuIsaTier::Sve2,
    ]);
    options.cpu_isa.set_sve_bits(128).expect("set sve bits");
    options.sve_vector_bits = options.cpu_isa.sve_vector_bits();
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
    assert!(
        ir.contains("llvm.aarch64.sve.usmmla.nxv4i32"),
        "expected SVE usmmla intrinsic in lowering"
    );
}
