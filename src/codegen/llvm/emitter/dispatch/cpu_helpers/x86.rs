use std::collections::BTreeSet;
use std::fmt::Write;

use crate::codegen::CpuIsaTier;

pub(super) fn emit_cpu_dispatch_helpers(
    out: &mut String,
    isa_tiers: &[CpuIsaTier],
    externals: &mut BTreeSet<&'static str>,
) {
    if isa_tiers.len() <= 1 {
        return;
    }

    externals.insert("__cpu_indicator_init");
    let mut needs_model = false;
    let mut needs_features2 = false;
    for tier in isa_tiers {
        match tier {
            CpuIsaTier::Avx2 | CpuIsaTier::Avx512 => needs_model = true,
            CpuIsaTier::Amx => needs_features2 = true,
            CpuIsaTier::Baseline
            | CpuIsaTier::DotProd
            | CpuIsaTier::Fp16Fml
            | CpuIsaTier::Bf16
            | CpuIsaTier::I8mm
            | CpuIsaTier::Sve
            | CpuIsaTier::Sve2
            | CpuIsaTier::Crypto
            | CpuIsaTier::Pauth
            | CpuIsaTier::Bti
            | CpuIsaTier::Sme => {}
        }
    }
    if needs_model {
        externals.insert("__cpu_model");
    }
    if needs_features2 {
        externals.insert("__cpu_features2");
    }

    writeln!(
        out,
        "@chic_cpu_active_tier = internal global i32 -1, align 4"
    )
    .ok();
    writeln!(out).ok();

    writeln!(out, "define internal i32 @chic_cpu_select_tier() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  %cached = load i32, ptr @chic_cpu_active_tier").ok();
    writeln!(out, "  %is_cached = icmp sge i32 %cached, 0").ok();
    writeln!(out, "  br i1 %is_cached, label %exit, label %detect").ok();

    writeln!(out, "detect:").ok();
    writeln!(out, "  call void @__cpu_indicator_init()").ok();

    let mut temp_counter = 0usize;
    let mut next_temp = |prefix: &str| -> String {
        temp_counter += 1;
        format!("%{}{}", prefix, temp_counter)
    };

    let mut current_value = CpuIsaTier::Baseline.index().to_string();

    let model_value = if needs_model {
        let tmp = next_temp("model");
        writeln!(
    out,
    "  {tmp} = load i32, ptr getelementptr inbounds ({{ i32, i32, i32, [1 x i32] }}, ptr @__cpu_model, i32 0, i32 3, i32 0)"
    )
    .ok();
        Some(tmp)
    } else {
        None
    };

    let features_value = if needs_features2 {
        let tmp = next_temp("features");
        writeln!(
    out,
    "  {tmp} = load i32, ptr getelementptr inbounds ([3 x i32], ptr @__cpu_features2, i32 0, i32 1)"
    )
    .ok();
        Some(tmp)
    } else {
        None
    };

    for tier in isa_tiers.iter().rev() {
        match tier {
            CpuIsaTier::Baseline => {}
            CpuIsaTier::Avx2 => {
                if let Some(model) = &model_value {
                    let mask = 1024;
                    let and_tmp = next_temp("feat");
                    writeln!(out, "  {and_tmp} = and i32 {model}, {mask}").ok();
                    let cmp_tmp = next_temp("cmp");
                    writeln!(out, "  {cmp_tmp} = icmp eq i32 {and_tmp}, {mask}").ok();
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
            }
            CpuIsaTier::Avx512 => {
                if let Some(model) = &model_value {
                    let mask = 32768;
                    let and_tmp = next_temp("feat");
                    writeln!(out, "  {and_tmp} = and i32 {model}, {mask}").ok();
                    let cmp_tmp = next_temp("cmp");
                    writeln!(out, "  {cmp_tmp} = icmp eq i32 {and_tmp}, {mask}").ok();
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
            }
            CpuIsaTier::Amx => {
                if let Some(features) = &features_value {
                    let mask = 2_097_152;
                    let and_tmp = next_temp("feat");
                    writeln!(out, "  {and_tmp} = and i32 {features}, {mask}").ok();
                    let cmp_tmp = next_temp("cmp");
                    writeln!(out, "  {cmp_tmp} = icmp eq i32 {and_tmp}, {mask}").ok();
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
            }
            CpuIsaTier::DotProd
            | CpuIsaTier::Fp16Fml
            | CpuIsaTier::Bf16
            | CpuIsaTier::I8mm
            | CpuIsaTier::Sve
            | CpuIsaTier::Sve2
            | CpuIsaTier::Crypto
            | CpuIsaTier::Pauth
            | CpuIsaTier::Bti
            | CpuIsaTier::Sme => {}
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
