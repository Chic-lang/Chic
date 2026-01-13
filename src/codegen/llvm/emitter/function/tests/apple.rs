use super::fixtures::{
    apple_bf16_module, apple_bf16_sme_module, apple_dpbusd_module, apple_simd_fma_module,
};
use super::helpers::{apple_target, function_ir};
use crate::chic_kind::ChicKind;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::codegen::{CodegenOptions, CpuIsaConfig, CpuIsaTier};
use crate::perf::PerfMetadata;

#[test]
fn apple_dispatch_emits_sysctl_probes() {
    let module = apple_simd_fma_module();
    let target = apple_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
    ]);
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
        ir.contains("@sysctlbyname"),
        "expected Apple dispatch to call sysctlbyname"
    );
    assert!(
        ir.contains("@chic_arm_sysctl_flag"),
        "expected dispatch metadata helpers:\n{ir}"
    );
}

#[test]
fn apple_dpbusd_uses_udot_and_usmmla_variants() {
    if std::env::var("CHIC_ENABLE_APPLE_DOTPROD_TESTS").is_err() {
        eprintln!(
            "skipping apple_dpbusd_uses_udot_and_usmmla_variants (set CHIC_ENABLE_APPLE_DOTPROD_TESTS=1 to enable)"
        );
        return;
    }
    let module = apple_dpbusd_module();
    let target = apple_target();
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
        "arm64-apple-macosx15.0",
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");

    let dotprod_variant = function_ir(&ir, "Root__Dpbusd__dotprod");
    assert!(
        dotprod_variant.contains("@llvm.aarch64.neon.udot.v4i32.v16i8"),
        "DotProd variant should emit udot intrinsic"
    );

    let i8mm_variant = function_ir(&ir, "Root__Dpbusd__i8mm");
    assert!(
        i8mm_variant.contains("@llvm.aarch64.neon.usmmla.v4i32"),
        "I8MM variant should emit neon usmmla intrinsic"
    );
}

#[test]
fn apple_bf16_lowering_emits_bfmmla_and_sme_controls() {
    if std::env::var("CHIC_ENABLE_APPLE_SME_TESTS").is_err() {
        eprintln!(
            "skipping apple_bf16_lowering_emits_bfmmla_and_sme_controls (set CHIC_ENABLE_APPLE_SME_TESTS=1 to enable)"
        );
        return;
    }
    let module = apple_bf16_module();
    let target = apple_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::Bf16,
        CpuIsaTier::Sme,
    ]);
    let perf = PerfMetadata::default();

    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        "arm64-apple-macosx15.0",
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");

    let bf16_variant = function_ir(&ir, "Root__Bf16Mmla__bf16");
    assert!(
        bf16_variant.contains("@llvm.aarch64.neon.bfmmla.v4f32"),
        "BF16 tier should use bfmmla intrinsic"
    );

    let sme_variant = function_ir(&ir, "Root__Bf16Mmla__sme");
    assert!(
        sme_variant.contains("@llvm.aarch64.neon.bfmmla.v4f32"),
        "SME tier should also use bfmmla intrinsic"
    );
    assert!(
        sme_variant.contains("llvm.aarch64.sme.za.enable"),
        "SME tier should enable ZA streaming state"
    );
}

#[test]
fn apple_sme_mmla_requires_sme_tier() {
    if std::env::var("CHIC_ENABLE_APPLE_SME_TESTS").is_err() {
        eprintln!(
            "skipping apple_sme_mmla_requires_sme_tier (set CHIC_ENABLE_APPLE_SME_TESTS=1 to enable)"
        );
        return;
    }
    let module = apple_bf16_sme_module();
    let target = apple_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![CpuIsaTier::Baseline, CpuIsaTier::Bf16]);
    let perf = PerfMetadata::default();

    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        "arm64-apple-macosx15.0",
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");

    let baseline = function_ir(&ir, "Root__Bf16Sme__baseline");
    assert!(
        baseline.contains("llvm.trap"),
        "baseline variant should trap when SME tier missing"
    );
    let bf16 = function_ir(&ir, "Root__Bf16Sme__bf16");
    assert!(
        bf16.contains("llvm.trap"),
        "bf16 variant should trap without SME when sme_mmla requested"
    );
}

#[test]
fn apple_sme_mmla_enables_when_available() {
    if std::env::var("CHIC_ENABLE_APPLE_SME_TESTS").is_err() {
        eprintln!(
            "skipping apple_sme_mmla_enables_when_available (set CHIC_ENABLE_APPLE_SME_TESTS=1 to enable)"
        );
        return;
    }
    let module = apple_bf16_sme_module();
    let target = apple_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut options = CodegenOptions::default();
    options.cpu_isa = CpuIsaConfig::from_tiers(vec![
        CpuIsaTier::Baseline,
        CpuIsaTier::Bf16,
        CpuIsaTier::Sme,
    ]);
    let perf = PerfMetadata::default();

    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        "arm64-apple-macosx15.0",
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");

    let sme = function_ir(&ir, "Root__Bf16Sme__sme");
    assert!(
        sme.contains("llvm.aarch64.sme.za.enable"),
        "SME variant should enable ZA when sme_mmla requested"
    );
}
