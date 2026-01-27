use std::collections::BTreeSet;
use std::fmt::Write;

use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::literals::{LLVM_STR_TYPE, LLVM_STRING_TYPE};

pub(crate) const LLVM_STARTUP_DESCRIPTOR_TYPE: &str =
    "{ i32, [4 x i8], { i64, i32, i32 }, { i64, i64 } }";
pub(super) fn emit_cpu_dispatch_helpers(
    out: &mut String,
    isa_tiers: &[CpuIsaTier],
    externals: &mut BTreeSet<&'static str>,
    sve_bits: Option<u32>,
) {
    if isa_tiers.len() <= 1 {
        return;
    }

    externals.insert("getauxval");

    writeln!(
        out,
        "@chic_cpu_active_tier = internal global i32 -1, align 4"
    )
    .ok();
    writeln!(out).ok();

    let uses_sve = isa_tiers
        .iter()
        .any(|tier| matches!(*tier, CpuIsaTier::Sve | CpuIsaTier::Sve2));
    if uses_sve {
        let bits = sve_bits.unwrap_or(128);
        writeln!(
            out,
            "@chic_cpu_sve_bits = internal global i32 {bits}, align 4"
        )
        .ok();
        writeln!(out).ok();
    }

    writeln!(out, "define internal i32 @chic_cpu_select_tier() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  %cached = load i32, ptr @chic_cpu_active_tier").ok();
    writeln!(out, "  %is_cached = icmp sge i32 %cached, 0").ok();
    writeln!(out, "  br i1 %is_cached, label %exit, label %detect").ok();

    writeln!(out, "detect:").ok();
    writeln!(out, "  %hwcap = call i64 @getauxval(i64 16)").ok();
    writeln!(out, "  %hwcap2 = call i64 @getauxval(i64 26)").ok();

    let mut temp_counter = 0usize;
    let mut next_temp = |prefix: &str| -> String {
        temp_counter += 1;
        format!("%{}{}", prefix, temp_counter)
    };

    let mut current_value = CpuIsaTier::Baseline.index().to_string();
    let hwcap_asimddp: u64 = 1 << 20;
    let hwcap_asimdfhm: u64 = 1 << 23;
    let hwcap_sve: u64 = 1 << 22;
    let hwcap2_i8mm: u64 = 1 << 13;
    let hwcap2_svei8mm: u64 = 1 << 9;
    let hwcap2_bf16: u64 = 1 << 14;
    let hwcap2_svebf16: u64 = 1 << 12;
    let hwcap2_sve2: u64 = 1 << 1;
    let hwcap2_sme: u64 = 1 << 23;

    for tier in isa_tiers.iter().rev() {
        match tier {
            CpuIsaTier::Baseline => {}
            CpuIsaTier::DotProd => {
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap, {}", hwcap_asimddp).ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Fp16Fml => {
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap, {}", hwcap_asimdfhm).ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Bf16 => {
                let mask = hwcap2_bf16 | hwcap2_svebf16;
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap2, {mask}").ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::I8mm => {
                let mask = hwcap2_i8mm | hwcap2_svei8mm;
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap2, {mask}").ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Sve => {
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap, {hwcap_sve}").ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Sve2 => {
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap2, {hwcap2_sve2}").ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Sme => {
                let and_tmp = next_temp("feat");
                writeln!(out, "  {and_tmp} = and i64 %hwcap2, {hwcap2_sme}").ok();
                let cmp_tmp = next_temp("cmp");
                writeln!(out, "  {cmp_tmp} = icmp ne i64 {and_tmp}, 0").ok();
                let sel_tmp = next_temp("sel");
                writeln!(
                    out,
                    "  {sel_tmp} = select i1 {cmp_tmp}, i32 {}, i32 {}",
                    tier.index(),
                    current_value
                )
                .ok();
                current_value = sel_tmp;
            }
            CpuIsaTier::Avx2
            | CpuIsaTier::Avx512
            | CpuIsaTier::Amx
            | CpuIsaTier::Crypto
            | CpuIsaTier::Pauth
            | CpuIsaTier::Bti => {}
        }
    }

    writeln!(
        out,
        "  store i32 {current_value}, ptr @chic_cpu_active_tier"
    )
    .ok();
    writeln!(out, "  br label %exit").ok();

    writeln!(out, "exit:").ok();
    writeln!(out, "  %result = load i32, ptr @chic_cpu_active_tier").ok();
    writeln!(out, "  ret i32 %result").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();
}

pub(crate) fn emit_external_declarations(out: &mut String, externals: &BTreeSet<&'static str>) {
    if externals.is_empty() {
        return;
    }

    for name in externals {
        match *name {
            "__cpu_indicator_init" => {
                writeln!(out, "declare void @__cpu_indicator_init()").ok();
            }
            "__cpu_model" => {
                writeln!(
                    out,
                    "@__cpu_model = external global {{ i32, i32, i32, [1 x i32] }}"
                )
                .ok();
            }
            "__cpu_features2" => {
                writeln!(out, "@__cpu_features2 = external global [3 x i32]").ok();
            }
            "llvm.fma.v8f32" => {
                writeln!(
                    out,
                    "declare <8 x float> @llvm.fma.v8f32(<8 x float>, <8 x float>, <8 x float>)"
                )
                .ok();
            }
            "llvm.x86.avx512.vpdpbusd.512" => {
                writeln!(
                    out,
                    "declare <16 x i32> @llvm.x86.avx512.vpdpbusd.512(<16 x i32>, <16 x i32>, <16 x i32>)"
                )
                .ok();
            }
            "llvm.aarch64.neon.fmla.v4f32" => {
                writeln!(
                    out,
                    "declare <4 x float> @llvm.aarch64.neon.fmla.v4f32(<4 x float>, <4 x float>, <4 x float>)"
                )
                .ok();
            }
            "llvm.memset.p0.i64" => {
                writeln!(out, "declare void @llvm.memset.p0.i64(ptr, i8, i64, i1)").ok();
            }
            "llvm.memcpy.p0.p0.i64" => {
                writeln!(
                    out,
                    "declare void @llvm.memcpy.p0.p0.i64(ptr, ptr, i64, i1)"
                )
                .ok();
            }
            "memcmp" => {
                writeln!(out, "declare i32 @memcmp(ptr, ptr, i64)").ok();
            }
            "llvm.aarch64.neon.fmla.v8f16" => {
                writeln!(
                    out,
                    "declare <8 x half> @llvm.aarch64.neon.fmla.v8f16(<8 x half>, <8 x half>, <8 x half>)"
                )
                .ok();
            }
            "llvm.aarch64.neon.udot.v4i32.v16i8" => {
                writeln!(
                    out,
                    "declare <4 x i32> @llvm.aarch64.neon.udot.v4i32.v16i8(<4 x i32>, <16 x i8>, <16 x i8>)"
                )
                .ok();
            }
            "llvm.aarch64.neon.usmmla.v4i32" => {
                writeln!(
                    out,
                    "declare <4 x i32> @llvm.aarch64.neon.usmmla.v4i32(<4 x i32>, <16 x i8>, <16 x i8>)"
                )
                .ok();
            }
            "llvm.aarch64.neon.bfmmla.v4f32" => {
                writeln!(
                    out,
                    "declare <4 x float> @llvm.aarch64.neon.bfmmla.v4f32(<4 x float>, <8 x bfloat>, <8 x bfloat>)"
                )
                .ok();
            }
            "llvm.aarch64.sve.usmmla.nxv4i32" => {
                writeln!(
                    out,
                    "declare <vscale x 4 x i32> @llvm.aarch64.sve.usmmla.nxv4i32(<vscale x 4 x i32>, <vscale x 16 x i8>, <vscale x 16 x i8>)"
                )
                .ok();
            }
            "llvm.aarch64.sve.bfmmla.nxv4f32" => {
                writeln!(
                    out,
                    "declare <vscale x 4 x float> @llvm.aarch64.sve.bfmmla.nxv4f32(<vscale x 4 x float>, <vscale x 8 x bfloat>, <vscale x 8 x bfloat>)"
                )
                .ok();
            }
            "getauxval" => {
                writeln!(out, "declare i64 @getauxval(i64)").ok();
            }
            "llvm.aarch64.sme.za.enable" => {
                writeln!(out, "declare void @llvm.aarch64.sme.za.enable()").ok();
            }
            "llvm.aarch64.sme.za.disable" => {
                writeln!(out, "declare void @llvm.aarch64.sme.za.disable()").ok();
            }
            "sysctlbyname" => {
                writeln!(out, "declare i32 @sysctlbyname(ptr, ptr, ptr, ptr, i64)").ok();
            }
            "llvm.trap" => {
                writeln!(out, "declare void @llvm.trap()").ok();
            }
            "chic_rt.mmio_read" => {
                writeln!(out, "declare i64 @chic_rt.mmio_read(i64, i32, i32)").ok();
            }
            "chic_rt.mmio_write" => {
                writeln!(out, "declare void @chic_rt.mmio_write(i64, i64, i32, i32)").ok();
            }
            "chic_rt_random_fill" => {
                writeln!(out, "declare i8 @chic_rt_random_fill(ptr, i64)").ok();
            }
            "chic_rt_panic" => {
                writeln!(out, "declare i32 @chic_rt_panic(i32)").ok();
            }
            value @ ("chic_rt_decimal_add"
            | "chic_rt_decimal_sub"
            | "chic_rt_decimal_mul"
            | "chic_rt_decimal_div"
            | "chic_rt_decimal_rem") => {
                writeln!(
                    out,
                    "declare void @{name}(ptr sret({DEC}) align 4, ptr, ptr, i32, i32)",
                    DEC = "{ i32, { i32, i32, i32, i32 } }",
                    name = value
                )
                .ok();
            }
            "chic_rt_decimal_fma" => {
                writeln!(
                    out,
                    "declare void @{name}(ptr sret({DEC}) align 4, ptr, ptr, ptr, i32, i32)",
                    DEC = "{ i32, { i32, i32, i32, i32 } }",
                    name = "chic_rt_decimal_fma"
                )
                .ok();
            }
            "chic_rt_decimal_sum" => {
                writeln!(
                    out,
                    "declare {{ i32, i128 }} @chic_rt_decimal_sum({{ i64 }}, i64, {{ i32 }}, i32)"
                )
                .ok();
            }
            "chic_rt_decimal_dot" => {
                writeln!(
                    out,
                    "declare {{ i32, i128 }} @chic_rt_decimal_dot({{ i64 }}, {{ i64 }}, i64, {{ i32 }}, i32)"
                )
                .ok();
            }
            "chic_rt_decimal_matmul" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_decimal_matmul({{ i64 }}, i64, i64, {{ i64 }}, i64, {{ i64 }}, {{ i32 }}, i32)"
                )
                .ok();
            }
            "chic_rt_decimal_clone" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_decimal_clone({{ i64 }}, {{ i64 }})"
                )
                .ok();
            }
            "chic_rt_startup_argv" => {
                writeln!(out, "declare i64 @chic_rt_startup_argv(i32)").ok();
            }
            "chic_rt_startup_env" => {
                writeln!(out, "declare i64 @chic_rt_startup_env(i32)").ok();
            }
            "chic_rt_throw" => {
                writeln!(out, "declare void @chic_rt_throw(i64, i64)").ok();
            }
            "chic_rt_string_clone" => {
                writeln!(out, "declare i32 @chic_rt_string_clone(ptr, ptr)").ok();
            }
            "chic_rt_string_clone_slice" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_clone_slice(ptr, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_from_slice" => {
                writeln!(
                    out,
                    "declare void @chic_rt_string_from_slice(ptr sret({LLVM_STRING_TYPE}) align 8, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_drop" => {
                writeln!(out, "declare void @chic_rt_string_drop(ptr)").ok();
            }
            "chic_rt_string_allocations" => {
                writeln!(out, "declare i64 @chic_rt_string_allocations()").ok();
            }
            "chic_rt_string_frees" => {
                writeln!(out, "declare i64 @chic_rt_string_frees()").ok();
            }
            "chic_rt_string_debug_ping" => {
                writeln!(out, "declare i32 @chic_rt_string_debug_ping()").ok();
            }
            "__drop_noop" => {
                writeln!(out, "declare void @__drop_noop(ptr)").ok();
            }
            "chic_rt_drop_noop_ptr" => {
                writeln!(out, "declare i64 @chic_rt_drop_noop_ptr()").ok();
            }
            "chic_rt_drop_register" => {
                writeln!(out, "declare void @chic_rt_drop_register(i64, ptr)").ok();
            }
            "chic_rt_drop_clear" => {
                writeln!(out, "declare void @chic_rt_drop_clear()").ok();
            }
            "chic_rt_drop_missing" => {
                writeln!(out, "declare void @chic_rt_drop_missing(ptr)").ok();
            }
            "chic_rt_has_pending_exception" => {
                writeln!(out, "declare i32 @chic_rt_has_pending_exception()").ok();
            }
            "chic_rt_take_pending_exception" => {
                writeln!(out, "declare i32 @chic_rt_take_pending_exception(ptr, ptr)").ok();
            }
            "chic_rt_closure_env_alloc" => {
                writeln!(out, "declare ptr @chic_rt_closure_env_alloc(i64, i64)").ok();
            }
            "chic_rt_closure_env_free" => {
                writeln!(out, "declare void @chic_rt_closure_env_free(ptr, i64, i64)").ok();
            }
            "chic_rt_closure_env_clone" => {
                writeln!(out, "declare ptr @chic_rt_closure_env_clone(ptr, i64, i64)").ok();
            }
            "chic_rt_drop_resolve" => {
                writeln!(out, "declare ptr @chic_rt_drop_resolve(i64)").ok();
            }
            "chic_rt_zero_init" => {
                writeln!(out, "declare void @chic_rt_zero_init(ptr, i64)").ok();
            }
            "chic_rt_ptr_offset" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_ptr_offset({{ ptr, i64, i64 }}, i64)"
                )
                .ok();
            }
            "chic_rt_ptr_offset_const" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_ptr_offset_const({{ ptr, i64, i64 }}, i64)"
                )
                .ok();
            }
            "chic_rt_region_enter" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_region_enter(i64)"
                )
                .ok();
            }
            "chic_rt_region_exit" => {
                writeln!(
                    out,
                    "declare void @chic_rt_region_exit({{ ptr, i64, i64 }})"
                )
                .ok();
            }
            "chic_rt_region_alloc" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_region_alloc({{ ptr, i64, i64 }}, i64, i64)"
                )
                .ok();
            }
            "chic_rt_region_alloc_zeroed" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_region_alloc_zeroed({{ ptr, i64, i64 }}, i64, i64)"
                )
                .ok();
            }
            "chic_rt_region_telemetry" => {
                writeln!(
                    out,
                    "declare {{ i64, i64, i64, i64, i64 }} @chic_rt_region_telemetry({{ ptr, i64, i64 }})"
                )
                .ok();
            }
            "chic_rt_region_reset_stats" => {
                writeln!(
                    out,
                    "declare void @chic_rt_region_reset_stats({{ ptr, i64, i64 }})"
                )
                .ok();
            }
            "chic_rt_alloc" => {
                writeln!(out, "declare {{ ptr, i64, i64 }} @chic_rt_alloc(i64, i64)").ok();
            }
            "chic_rt_alloc_zeroed" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_alloc_zeroed(i64, i64)"
                )
                .ok();
            }
            "chic_rt_realloc" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64, i64 }} @chic_rt_realloc({{ ptr, i64, i64 }}, i64, i64, i64)"
                )
                .ok();
            }
            "chic_rt_free" => {
                writeln!(out, "declare void @chic_rt_free({{ ptr, i64, i64 }})").ok();
            }
            "chic_rt_alloc_stats" => {
                writeln!(
                    out,
                    "declare {{ i64, i64, i64, i64, i64, i64, i64, i64 }} @chic_rt_alloc_stats()"
                )
                .ok();
            }
            "chic_rt_reset_alloc_stats" => {
                writeln!(out, "declare void @chic_rt_reset_alloc_stats()").ok();
            }
            "chic_rt_allocator_install" => {
                writeln!(
                    out,
                    "declare void @chic_rt_allocator_install({{ ptr, ptr, ptr, ptr, ptr }})"
                )
                .ok();
            }
            "chic_rt_allocator_reset" => {
                writeln!(out, "declare void @chic_rt_allocator_reset()").ok();
            }
            "chic_rt_memcpy" => {
                writeln!(
                    out,
                    "declare void @chic_rt_memcpy({{ ptr, i64, i64 }}, {{ ptr, i64, i64 }}, i64)"
                )
                .ok();
            }
            "chic_rt_memmove" => {
                writeln!(
                    out,
                    "declare void @chic_rt_memmove({{ ptr, i64, i64 }}, {{ ptr, i64, i64 }}, i64)"
                )
                .ok();
            }
            "chic_rt_memset" => {
                writeln!(
                    out,
                    "declare void @chic_rt_memset({{ ptr, i64, i64 }}, i8, i64)"
                )
                .ok();
            }
            "chic_rt_ptr_is_null_mut" => {
                writeln!(out, "declare i1 @chic_rt_ptr_is_null_mut(ptr)").ok();
            }
            "chic_rt_ptr_is_null_const" => {
                writeln!(out, "declare i1 @chic_rt_ptr_is_null_const(ptr)").ok();
            }
            "chic_rt_type_drop_glue" => {
                writeln!(out, "declare i64 @chic_rt_type_drop_glue(i64)").ok();
            }
            "chic_rt_type_clone_glue" => {
                writeln!(out, "declare i64 @chic_rt_type_clone_glue(i64)").ok();
            }
            "chic_rt_type_hash_glue" => {
                writeln!(out, "declare i64 @chic_rt_type_hash_glue(i64)").ok();
            }
            "chic_rt_type_eq_glue" => {
                writeln!(out, "declare i64 @chic_rt_type_eq_glue(i64)").ok();
            }
            "chic_rt_type_size" => {
                writeln!(out, "declare i64 @chic_rt_type_size(i64)").ok();
            }
            "chic_rt_type_align" => {
                writeln!(out, "declare i64 @chic_rt_type_align(i64)").ok();
            }
            "chic_rt_type_metadata" => {
                writeln!(out, "declare i32 @chic_rt_type_metadata(i64, ptr)").ok();
            }
            "chic_rt_type_metadata_register" => {
                writeln!(
                    out,
                    "declare void @chic_rt_type_metadata_register(i64, {{ i64, i64, i64, {{ ptr, i64 }}, i32 }})"
                )
                .ok();
            }
            "chic_rt_type_metadata_clear" => {
                writeln!(out, "declare void @chic_rt_type_metadata_clear()").ok();
            }
            "chic_rt_clone_invoke" => {
                writeln!(
                    out,
                    "declare void @chic_rt_clone_invoke(i64, {{ ptr, i64, i64 }}, {{ ptr, i64, i64 }})"
                )
                .ok();
            }
            "chic_rt_hash_invoke" => {
                writeln!(out, "declare i64 @chic_rt_hash_invoke(i64, ptr)").ok();
            }
            "chic_rt_eq_invoke" => {
                writeln!(out, "declare i32 @chic_rt_eq_invoke(i64, ptr, ptr)").ok();
            }
            "chic_rt_abort" => {
                writeln!(out, "declare i32 @chic_rt_abort(i32)").ok();
            }
            "chic_rt_install_drop_table" => {
                writeln!(out, "declare void @chic_rt_install_drop_table(ptr, i64)").ok();
            }
            "chic_rt_install_hash_table" => {
                writeln!(out, "declare void @chic_rt_install_hash_table(ptr, i64)").ok();
            }
            "chic_rt_install_eq_table" => {
                writeln!(out, "declare void @chic_rt_install_eq_table(ptr, i64)").ok();
            }
            "chic_rt_install_type_metadata" => {
                writeln!(out, "declare void @chic_rt_install_type_metadata(ptr, i64)").ok();
            }
            "chic_rt_span_from_raw_mut" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare {{ {vp}, i64, i64, i64 }} @chic_rt_span_from_raw_mut({vp}, i64)",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_span_from_raw_const" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare {{ {vp}, i64, i64, i64 }} @chic_rt_span_from_raw_const({vp}, i64)",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_span_copy_to" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_span_copy_to({{ {vp}, i64, i64, i64 }}, {{ {vp}, i64, i64, i64 }})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_span_fill" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_span_fill({{ {vp}, i64, i64, i64 }}, ptr)",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_span_slice_mut" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_span_slice_mut(ptr, i64, i64, ptr)"
                )
                .ok();
            }
            "chic_rt_span_slice_readonly" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_span_slice_readonly(ptr, i64, i64, ptr)"
                )
                .ok();
            }
            "chic_rt_span_ptr_at_mut" => {
                writeln!(out, "declare ptr @chic_rt_span_ptr_at_mut(ptr, i64)").ok();
            }
            "chic_rt_span_ptr_at_readonly" => {
                writeln!(out, "declare ptr @chic_rt_span_ptr_at_readonly(ptr, i64)").ok();
            }
            "chic_rt_span_to_readonly" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare {{ {vp}, i64, i64, i64 }} @chic_rt_span_to_readonly({{ {vp}, i64, i64, i64 }})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_str_as_chars" => {
                writeln!(
                    out,
                    "declare {{ ptr, i64 }} @chic_rt_str_as_chars({{ ptr, i64 }})"
                )
                .ok();
            }
            "chic_rt_string_as_chars" => {
                writeln!(out, "declare {{ ptr, i64 }} @chic_rt_string_as_chars(ptr)").ok();
            }
            "chic_rt_string_as_slice" => {
                writeln!(out, "declare {{ ptr, i64 }} @chic_rt_string_as_slice(ptr)").ok();
            }
            "chic_rt_string_new" => {
                writeln!(
                    out,
                    "declare void @chic_rt_string_new(ptr sret({LLVM_STRING_TYPE}) align 8)"
                )
                .ok();
            }
            "chic_rt_string_with_capacity" => {
                writeln!(
                    out,
                    "declare void @chic_rt_string_with_capacity(ptr sret({LLVM_STRING_TYPE}) align 8, i64)"
                )
                .ok();
            }
            "chic_rt_string_from_char" => {
                writeln!(
                    out,
                    "declare void @chic_rt_string_from_char(ptr sret({LLVM_STRING_TYPE}) align 8, i16)"
                )
                .ok();
            }
            "chic_rt_string_reserve" => {
                writeln!(out, "declare i32 @chic_rt_string_reserve(ptr, i64)").ok();
            }
            "chic_rt_string_truncate" => {
                writeln!(out, "declare i32 @chic_rt_string_truncate(ptr, i64)").ok();
            }
            "chic_rt_string_push_slice" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_push_slice(ptr, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_get_ptr" => {
                writeln!(out, "declare ptr @chic_rt_string_get_ptr(ptr)").ok();
            }
            "chic_rt_string_set_ptr" => {
                writeln!(out, "declare void @chic_rt_string_set_ptr(ptr, ptr)").ok();
            }
            "chic_rt_string_get_len" => {
                writeln!(out, "declare i64 @chic_rt_string_get_len(ptr)").ok();
            }
            "chic_rt_string_set_len" => {
                writeln!(out, "declare void @chic_rt_string_set_len(ptr, i64)").ok();
            }
            "chic_rt_string_get_cap" => {
                writeln!(out, "declare i64 @chic_rt_string_get_cap(ptr)").ok();
            }
            "chic_rt_string_set_cap" => {
                writeln!(out, "declare void @chic_rt_string_set_cap(ptr, i64)").ok();
            }
            "chic_rt_string_inline_ptr" => {
                writeln!(out, "declare ptr @chic_rt_string_inline_ptr(ptr)").ok();
            }
            "chic_rt_string_inline_capacity" => {
                writeln!(out, "declare i64 @chic_rt_string_inline_capacity()").ok();
            }
            "chic_rt_string_error_message" => {
                writeln!(
                    out,
                    "declare {LLVM_STR_TYPE} @chic_rt_string_error_message(i32)"
                )
                .ok();
            }
            "chic_rt_char_from_codepoint" => {
                writeln!(out, "declare i64 @chic_rt_char_from_codepoint(i32)").ok();
            }
            "chic_rt_char_is_digit" => {
                writeln!(out, "declare i32 @chic_rt_char_is_digit(i16)").ok();
            }
            "chic_rt_char_is_letter" => {
                writeln!(out, "declare i32 @chic_rt_char_is_letter(i16)").ok();
            }
            "chic_rt_char_is_scalar" => {
                writeln!(out, "declare i32 @chic_rt_char_is_scalar(i16)").ok();
            }
            "chic_rt_char_is_whitespace" => {
                writeln!(out, "declare i32 @chic_rt_char_is_whitespace(i16)").ok();
            }
            "chic_rt_char_status" => {
                writeln!(out, "declare i32 @chic_rt_char_status(i64)").ok();
            }
            "chic_rt_char_to_lower" => {
                writeln!(out, "declare i64 @chic_rt_char_to_lower(i16)").ok();
            }
            "chic_rt_char_to_upper" => {
                writeln!(out, "declare i64 @chic_rt_char_to_upper(i16)").ok();
            }
            "chic_rt_char_value" => {
                writeln!(out, "declare i16 @chic_rt_char_value(i64)").ok();
            }
            "chic_rt_thread_spawn" => {
                writeln!(out, "declare i32 @chic_rt_thread_spawn(ptr, ptr)").ok();
            }
            "chic_rt_thread_join" => {
                writeln!(out, "declare i32 @chic_rt_thread_join(ptr)").ok();
            }
            "chic_rt_thread_detach" => {
                writeln!(out, "declare i32 @chic_rt_thread_detach(ptr)").ok();
            }
            "chic_rt_thread_sleep_ms" => {
                writeln!(out, "declare void @chic_rt_thread_sleep_ms(i64)").ok();
            }
            "chic_rt_thread_yield" => {
                writeln!(out, "declare void @chic_rt_thread_yield()").ok();
            }
            "chic_rt_thread_spin_wait" => {
                writeln!(out, "declare void @chic_rt_thread_spin_wait(i32)").ok();
            }
            "chic_rt_vec_new" => {
                let vec_ty = "{ ptr, i64, i64, i64, i64, i64, { i64, i64, i64 } }";
                writeln!(
                    out,
                    "declare {vec} @chic_rt_vec_new(i64, i64, i64)",
                    vec = vec_ty
                )
                .ok();
            }
            "chic_rt_vec_with_capacity" => {
                let vec_ty = "{ ptr, i64, i64, i64, i64, i64, { i64, i64, i64 } }";
                writeln!(
                    out,
                    "declare {vec} @chic_rt_vec_with_capacity(i64, i64, i64, i64)",
                    vec = vec_ty
                )
                .ok();
            }
            "chic_rt_vec_reserve" => {
                writeln!(out, "declare i32 @chic_rt_vec_reserve(ptr, i64)").ok();
            }
            "chic_rt_vec_shrink_to_fit" => {
                writeln!(out, "declare i32 @chic_rt_vec_shrink_to_fit(ptr)").ok();
            }
            "chic_rt_vec_push" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(out, "declare i32 @chic_rt_vec_push(ptr, {vp})", vp = vp).ok();
            }
            "chic_rt_vec_pop" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(out, "declare i32 @chic_rt_vec_pop(ptr, {vp})", vp = vp).ok();
            }
            "chic_rt_vec_insert" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_vec_insert(ptr, i64, {vp})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_vec_remove" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_vec_remove(ptr, i64, {vp})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_vec_swap_remove" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_vec_swap_remove(ptr, i64, {vp})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_vec_truncate" => {
                writeln!(out, "declare i32 @chic_rt_vec_truncate(ptr, i64)").ok();
            }
            "chic_rt_vec_clear" => {
                writeln!(out, "declare i32 @chic_rt_vec_clear(ptr)").ok();
            }
            "chic_rt_vec_drop" => {
                writeln!(out, "declare void @chic_rt_vec_drop(ptr)").ok();
            }
            "chic_rt_vec_set_len" => {
                writeln!(out, "declare i32 @chic_rt_vec_set_len(ptr, i64)").ok();
            }
            "chic_rt_vec_len" => {
                writeln!(out, "declare i64 @chic_rt_vec_len(ptr)").ok();
            }
            "chic_rt_vec_capacity" => {
                writeln!(out, "declare i64 @chic_rt_vec_capacity(ptr)").ok();
            }
            "chic_rt_vec_clone" => {
                writeln!(out, "declare i32 @chic_rt_vec_clone(ptr, ptr)").ok();
            }
            "chic_rt_vec_data" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(out, "declare {vp} @chic_rt_vec_data(ptr)", vp = vp).ok();
            }
            "chic_rt_vec_data_mut" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(out, "declare {vp} @chic_rt_vec_data_mut(ptr)", vp = vp).ok();
            }
            "chic_rt_vec_view" => {
                let view_ty = "{ ptr, i64, i64, i64 }";
                writeln!(out, "declare {view} @chic_rt_vec_view(ptr)", view = view_ty).ok();
            }
            "chic_rt_vec_iter" => {
                let iter_ty = "{ ptr, i64, i64, i64 }";
                writeln!(out, "declare {iter} @chic_rt_vec_iter(ptr)", iter = iter_ty).ok();
            }
            "chic_rt_vec_iter_next" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(
                    out,
                    "declare i32 @chic_rt_vec_iter_next(ptr, {vp})",
                    vp = vp
                )
                .ok();
            }
            "chic_rt_vec_iter_next_ptr" => {
                let vp = "{ ptr, i64, i64 }";
                writeln!(out, "declare {vp} @chic_rt_vec_iter_next_ptr(ptr)", vp = vp).ok();
            }
            "chic_rt_vec_is_empty" => {
                writeln!(out, "declare i32 @chic_rt_vec_is_empty(ptr)").ok();
            }
            "chic_rt_vec_into_array" => {
                writeln!(out, "declare i32 @chic_rt_vec_into_array(ptr, ptr)").ok();
            }
            "chic_rt_vec_copy_to_array" => {
                writeln!(out, "declare i32 @chic_rt_vec_copy_to_array(ptr, ptr)").ok();
            }
            "chic_rt_array_into_vec" => {
                writeln!(out, "declare i32 @chic_rt_array_into_vec(ptr, ptr)").ok();
            }
            "chic_rt_array_copy_to_vec" => {
                writeln!(out, "declare i32 @chic_rt_array_copy_to_vec(ptr, ptr)").ok();
            }
            "chic_rt_async_block_on" => {
                writeln!(out, "declare void @chic_rt_async_block_on(ptr)").ok();
            }
            "chic_rt_async_spawn" => {
                writeln!(out, "declare void @chic_rt_async_spawn(ptr)").ok();
            }
            "chic_rt_async_register_future" => {
                writeln!(out, "declare void @chic_rt_async_register_future(ptr)").ok();
            }
            "chic_rt_async_cancel" => {
                writeln!(out, "declare i32 @chic_rt_async_cancel(ptr)").ok();
            }
            "chic_rt_await" => {
                writeln!(out, "declare i32 @chic_rt_await(ptr, ptr)").ok();
            }
            "chic_rt_async_task_result" => {
                writeln!(out, "declare i32 @chic_rt_async_task_result(ptr, ptr, i32)").ok();
            }
            "chic_rt_async_token_state" => {
                writeln!(out, "declare i32 @chic_rt_async_token_state(ptr)").ok();
            }
            "chic_rt_async_token_cancel" => {
                writeln!(out, "declare i32 @chic_rt_async_token_cancel(ptr)").ok();
            }
            "chic_rt_async_token_new" => {
                writeln!(out, "declare ptr @chic_rt_async_token_new()").ok();
            }
            "chic_rt_async_spawn_local" => {
                writeln!(out, "declare i32 @chic_rt_async_spawn_local(ptr)").ok();
            }
            "chic_rt_async_scope" => {
                writeln!(out, "declare i32 @chic_rt_async_scope(ptr)").ok();
            }
            "chic_rt_yield" => {
                writeln!(out, "declare i32 @chic_rt_yield(ptr)").ok();
            }
            "chic_rt_async_task_header" => {
                writeln!(out, "declare ptr @chic_rt_async_task_header(ptr)").ok();
            }
            "chic_rt_async_task_bool_result" => {
                writeln!(out, "declare i1 @chic_rt_async_task_bool_result(ptr)").ok();
            }
            "chic_rt_async_task_int_result" => {
                writeln!(out, "declare i32 @chic_rt_async_task_int_result(ptr)").ok();
            }
            "chic_rt_startup_store_state" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_store_state(i32, ptr, ptr)"
                )
                .ok();
            }
            "chic_rt_startup_raw_argc" => {
                writeln!(out, "declare i32 @chic_rt_startup_raw_argc()").ok();
            }
            "chic_rt_startup_raw_argv" => {
                writeln!(out, "declare ptr @chic_rt_startup_raw_argv()").ok();
            }
            "chic_rt_startup_raw_envp" => {
                writeln!(out, "declare ptr @chic_rt_startup_raw_envp()").ok();
            }
            "chic_rt_startup_has_run_tests_flag" => {
                writeln!(out, "declare i32 @chic_rt_startup_has_run_tests_flag()").ok();
            }
            "chic_rt_startup_descriptor_snapshot" => {
                writeln!(
                    out,
                    "declare {LLVM_STARTUP_DESCRIPTOR_TYPE} @chic_rt_startup_descriptor_snapshot()"
                )
                .ok();
            }
            "chic_rt_startup_test_descriptor" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_test_descriptor(ptr, i64)"
                )
                .ok();
            }
            "chic_rt_startup_call_entry" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_startup_call_entry(ptr, i32, i32, ptr, ptr)"
                )
                .ok();
            }
            "chic_rt_startup_call_entry_async" => {
                writeln!(
                    out,
                    "declare ptr @chic_rt_startup_call_entry_async(ptr, i32, i32, ptr, ptr)"
                )
                .ok();
            }
            "chic_rt_startup_complete_entry_async" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_startup_complete_entry_async(ptr, i32)"
                )
                .ok();
            }
            "chic_rt_startup_call_testcase" => {
                writeln!(out, "declare i32 @chic_rt_startup_call_testcase(ptr)").ok();
            }
            "chic_rt_startup_call_testcase_async" => {
                writeln!(out, "declare ptr @chic_rt_startup_call_testcase_async(ptr)").ok();
            }
            "chic_rt_startup_complete_testcase_async" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_startup_complete_testcase_async(ptr)"
                )
                .ok();
            }
            "chic_rt_startup_cstr_to_string" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_cstr_to_string(ptr sret({LLVM_STRING_TYPE}) align 8, ptr)"
                )
                .ok();
            }
            "chic_rt_startup_slice_to_string" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_slice_to_string(ptr sret({LLVM_STRING_TYPE}) align 8, ptr, i64)"
                )
                .ok();
            }
            "chic_rt_startup_i32_to_string" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_i32_to_string(ptr sret({LLVM_STRING_TYPE}) align 8, i32)"
                )
                .ok();
            }
            "chic_rt_startup_usize_to_string" => {
                writeln!(
                    out,
                    "declare void @chic_rt_startup_usize_to_string(ptr sret({LLVM_STRING_TYPE}) align 8, i64)"
                )
                .ok();
            }
            "chic_rt_startup_ptr_at" => {
                writeln!(out, "declare i64 @chic_rt_startup_ptr_at(i64, i32, i32)").ok();
            }
            "chic_rt_stdout_write_string" => {
                writeln!(out, "declare i32 @chic_rt_stdout_write_string(ptr)").ok();
            }
            "chic_rt_stdout_write_line_string" => {
                writeln!(out, "declare i32 @chic_rt_stdout_write_line_string(ptr)").ok();
            }
            "chic_rt_stdout_flush" => {
                writeln!(out, "declare i32 @chic_rt_stdout_flush()").ok();
            }
            "chic_rt_stderr_write_string" => {
                writeln!(out, "declare i32 @chic_rt_stderr_write_string(ptr)").ok();
            }
            "chic_rt_stderr_write_line_string" => {
                writeln!(out, "declare i32 @chic_rt_stderr_write_line_string(ptr)").ok();
            }
            "chic_rt_stderr_flush" => {
                writeln!(out, "declare i32 @chic_rt_stderr_flush()").ok();
            }
            "chic_rt_trace_enter" => {
                writeln!(
                    out,
                    "declare void @chic_rt_trace_enter(i64, ptr, i64, i64, i64, i64)"
                )
                .ok();
            }
            "chic_rt_trace_exit" => {
                writeln!(out, "declare void @chic_rt_trace_exit(i64)").ok();
            }
            "chic_rt_rc_clone" => {
                writeln!(out, "declare i32 @chic_rt_rc_clone(ptr, ptr)").ok();
            }
            "chic_rt_rc_drop" => {
                writeln!(out, "declare void @chic_rt_rc_drop(ptr)").ok();
            }
            "chic_rt_arc_new" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_arc_new(ptr, ptr, i64, i64, i64, i64)"
                )
                .ok();
            }
            "chic_rt_arc_clone" => {
                writeln!(out, "declare i32 @chic_rt_arc_clone(ptr, ptr)").ok();
            }
            "chic_rt_arc_drop" => {
                writeln!(out, "declare void @chic_rt_arc_drop(ptr)").ok();
            }
            "chic_rt_arc_get" => {
                writeln!(out, "declare ptr @chic_rt_arc_get(ptr)").ok();
            }
            "chic_rt_arc_get_mut" => {
                writeln!(out, "declare ptr @chic_rt_arc_get_mut(ptr)").ok();
            }
            "chic_rt_arc_downgrade" => {
                writeln!(out, "declare i32 @chic_rt_arc_downgrade(ptr, ptr)").ok();
            }
            "chic_rt_weak_clone" => {
                writeln!(out, "declare i32 @chic_rt_weak_clone(ptr, ptr)").ok();
            }
            "chic_rt_weak_drop" => {
                writeln!(out, "declare void @chic_rt_weak_drop(ptr)").ok();
            }
            "chic_rt_weak_upgrade" => {
                writeln!(out, "declare i32 @chic_rt_weak_upgrade(ptr, ptr)").ok();
            }
            "chic_rt_rc_new" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_rc_new(ptr, ptr, i64, i64, i64, i64)"
                )
                .ok();
            }
            "chic_rt_rc_get" => {
                writeln!(out, "declare ptr @chic_rt_rc_get(ptr)").ok();
            }
            "chic_rt_rc_get_mut" => {
                writeln!(out, "declare ptr @chic_rt_rc_get_mut(ptr)").ok();
            }
            "chic_rt_rc_downgrade" => {
                writeln!(out, "declare i32 @chic_rt_rc_downgrade(ptr, ptr)").ok();
            }
            "chic_rt_rc_strong_count" => {
                writeln!(out, "declare i64 @chic_rt_rc_strong_count(ptr)").ok();
            }
            "chic_rt_rc_weak_count" => {
                writeln!(out, "declare i64 @chic_rt_rc_weak_count(ptr)").ok();
            }
            "chic_rt_weak_rc_clone" => {
                writeln!(out, "declare i32 @chic_rt_weak_rc_clone(ptr, ptr)").ok();
            }
            "chic_rt_weak_rc_drop" => {
                writeln!(out, "declare void @chic_rt_weak_rc_drop(ptr)").ok();
            }
            "chic_rt_weak_rc_upgrade" => {
                writeln!(out, "declare i32 @chic_rt_weak_rc_upgrade(ptr, ptr)").ok();
            }
            "chic_rt_mutex_create" => {
                writeln!(out, "declare i64 @chic_rt_mutex_create()").ok();
            }
            "chic_rt_mutex_destroy" => {
                writeln!(out, "declare void @chic_rt_mutex_destroy(i64)").ok();
            }
            "chic_rt_mutex_lock" => {
                writeln!(out, "declare void @chic_rt_mutex_lock(i64)").ok();
            }
            "chic_rt_mutex_try_lock" => {
                writeln!(out, "declare i8 @chic_rt_mutex_try_lock(i64)").ok();
            }
            "chic_rt_mutex_unlock" => {
                writeln!(out, "declare void @chic_rt_mutex_unlock(i64)").ok();
            }
            "chic_rt_lock_create" => {
                writeln!(out, "declare i64 @chic_rt_lock_create()").ok();
            }
            "chic_rt_lock_destroy" => {
                writeln!(out, "declare void @chic_rt_lock_destroy(i64)").ok();
            }
            "chic_rt_lock_enter" => {
                writeln!(out, "declare void @chic_rt_lock_enter(i64)").ok();
            }
            "chic_rt_lock_try_enter" => {
                writeln!(out, "declare i8 @chic_rt_lock_try_enter(i64)").ok();
            }
            "chic_rt_lock_exit" => {
                writeln!(out, "declare void @chic_rt_lock_exit(i64)").ok();
            }
            "chic_rt_lock_is_held" => {
                writeln!(out, "declare i8 @chic_rt_lock_is_held(i64)").ok();
            }
            "chic_rt_lock_is_held_by_current_thread" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_lock_is_held_by_current_thread(i64)"
                )
                .ok();
            }
            "chic_rt_rwlock_create" => {
                writeln!(out, "declare i64 @chic_rt_rwlock_create()").ok();
            }
            "chic_rt_rwlock_destroy" => {
                writeln!(out, "declare void @chic_rt_rwlock_destroy(i64)").ok();
            }
            "chic_rt_rwlock_read_lock" => {
                writeln!(out, "declare void @chic_rt_rwlock_read_lock(i64)").ok();
            }
            "chic_rt_rwlock_try_read_lock" => {
                writeln!(out, "declare i8 @chic_rt_rwlock_try_read_lock(i64)").ok();
            }
            "chic_rt_rwlock_read_unlock" => {
                writeln!(out, "declare void @chic_rt_rwlock_read_unlock(i64)").ok();
            }
            "chic_rt_rwlock_write_lock" => {
                writeln!(out, "declare void @chic_rt_rwlock_write_lock(i64)").ok();
            }
            "chic_rt_rwlock_try_write_lock" => {
                writeln!(out, "declare i8 @chic_rt_rwlock_try_write_lock(i64)").ok();
            }
            "chic_rt_rwlock_write_unlock" => {
                writeln!(out, "declare void @chic_rt_rwlock_write_unlock(i64)").ok();
            }
            "chic_rt_condvar_create" => {
                writeln!(out, "declare i64 @chic_rt_condvar_create()").ok();
            }
            "chic_rt_condvar_destroy" => {
                writeln!(out, "declare void @chic_rt_condvar_destroy(i64)").ok();
            }
            "chic_rt_condvar_notify_one" => {
                writeln!(out, "declare void @chic_rt_condvar_notify_one(i64)").ok();
            }
            "chic_rt_condvar_notify_all" => {
                writeln!(out, "declare void @chic_rt_condvar_notify_all(i64)").ok();
            }
            "chic_rt_condvar_wait" => {
                writeln!(out, "declare void @chic_rt_condvar_wait(i64, i64)").ok();
            }
            "chic_rt_once_create" => {
                writeln!(out, "declare i64 @chic_rt_once_create()").ok();
            }
            "chic_rt_once_destroy" => {
                writeln!(out, "declare void @chic_rt_once_destroy(i64)").ok();
            }
            "chic_rt_once_try_begin" => {
                writeln!(out, "declare i8 @chic_rt_once_try_begin(i64)").ok();
            }
            "chic_rt_once_complete" => {
                writeln!(out, "declare void @chic_rt_once_complete(i64)").ok();
            }
            "chic_rt_once_wait" => {
                writeln!(out, "declare void @chic_rt_once_wait(i64)").ok();
            }
            "chic_rt_once_is_completed" => {
                writeln!(out, "declare i8 @chic_rt_once_is_completed(i64)").ok();
            }
            "chic_rt_atomic_bool_load" => {
                writeln!(out, "declare i8 @chic_rt_atomic_bool_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_bool_store" => {
                writeln!(out, "declare void @chic_rt_atomic_bool_store(ptr, i8, i8)").ok();
            }
            "chic_rt_atomic_bool_compare_exchange" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_atomic_bool_compare_exchange(ptr, i8, i8, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_usize_load" => {
                writeln!(out, "declare i64 @chic_rt_atomic_usize_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_usize_store" => {
                writeln!(
                    out,
                    "declare void @chic_rt_atomic_usize_store(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_usize_fetch_add" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_usize_fetch_add(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_usize_fetch_sub" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_usize_fetch_sub(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i32_load" => {
                writeln!(out, "declare i32 @chic_rt_atomic_i32_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_i32_store" => {
                writeln!(out, "declare void @chic_rt_atomic_i32_store(ptr, i32, i8)").ok();
            }
            "chic_rt_atomic_i32_compare_exchange" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_atomic_i32_compare_exchange(ptr, i32, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i32_fetch_add" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_atomic_i32_fetch_add(ptr, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i32_fetch_sub" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_atomic_i32_fetch_sub(ptr, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u32_load" => {
                writeln!(out, "declare i32 @chic_rt_atomic_u32_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_u32_store" => {
                writeln!(out, "declare void @chic_rt_atomic_u32_store(ptr, i32, i8)").ok();
            }
            "chic_rt_atomic_u32_compare_exchange" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_atomic_u32_compare_exchange(ptr, i32, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u32_fetch_add" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_atomic_u32_fetch_add(ptr, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u32_fetch_sub" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_atomic_u32_fetch_sub(ptr, i32, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i64_load" => {
                writeln!(out, "declare i64 @chic_rt_atomic_i64_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_i64_store" => {
                writeln!(out, "declare void @chic_rt_atomic_i64_store(ptr, i64, i8)").ok();
            }
            "chic_rt_atomic_i64_compare_exchange" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_atomic_i64_compare_exchange(ptr, i64, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i64_fetch_add" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_i64_fetch_add(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_i64_fetch_sub" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_i64_fetch_sub(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u64_load" => {
                writeln!(out, "declare i64 @chic_rt_atomic_u64_load(ptr, i8)").ok();
            }
            "chic_rt_atomic_u64_store" => {
                writeln!(out, "declare void @chic_rt_atomic_u64_store(ptr, i64, i8)").ok();
            }
            "chic_rt_atomic_u64_compare_exchange" => {
                writeln!(
                    out,
                    "declare i8 @chic_rt_atomic_u64_compare_exchange(ptr, i64, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u64_fetch_add" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_u64_fetch_add(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_atomic_u64_fetch_sub" => {
                writeln!(
                    out,
                    "declare i64 @chic_rt_atomic_u64_fetch_sub(ptr, i64, i8)"
                )
                .ok();
            }
            "chic_rt_object_new" => {
                writeln!(out, "declare ptr @chic_rt_object_new(i64)").ok();
            }
            "chic_rt_string_append_slice" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_slice(ptr, {LLVM_STR_TYPE}, i32, i32)"
                )
                .ok();
            }
            "chic_rt_string_append_bool" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_bool(ptr, i8, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_char" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_char(ptr, i16, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_signed" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_signed(ptr, i128, i32, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_unsigned" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_unsigned(ptr, i128, i32, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_f16" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_f16(ptr, i16, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_f32" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_f32(ptr, float, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_f128" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_f128(ptr, i128, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "chic_rt_string_append_f64" => {
                writeln!(
                    out,
                    "declare i32 @chic_rt_string_append_f64(ptr, double, i32, i32, {LLVM_STR_TYPE})"
                )
                .ok();
            }
            "posix_memalign" => {
                writeln!(out, "declare i32 @posix_memalign(ptr, i64, i64)").ok();
            }
            "malloc" => {
                writeln!(out, "declare ptr @malloc(i64)").ok();
            }
            "calloc" => {
                writeln!(out, "declare ptr @calloc(i64, i64)").ok();
            }
            "realloc" => {
                writeln!(out, "declare ptr @realloc(ptr, i64)").ok();
            }
            "free" => {
                writeln!(out, "declare void @free(ptr)").ok();
            }
            "memcpy" => {
                writeln!(out, "declare ptr @memcpy(ptr, ptr, i64)").ok();
            }
            "memmove" => {
                writeln!(out, "declare ptr @memmove(ptr, ptr, i64)").ok();
            }
            "memset" => {
                writeln!(out, "declare ptr @memset(ptr, i8, i64)").ok();
            }
            other => {
                writeln!(out, "declare void @{other}()").ok();
            }
        }
    }
    writeln!(out).ok();
}
