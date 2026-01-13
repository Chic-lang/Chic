use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::codegen::CpuIsaTier;

use super::sanitise_sysctl_key;

pub(super) fn emit_cpu_dispatch_helpers(
    out: &mut String,
    isa_tiers: &[CpuIsaTier],
    externals: &mut BTreeSet<&'static str>,
) {
    if isa_tiers.len() <= 1 {
        return;
    }

    let mut feature_descriptors: Vec<(CpuIsaTier, &'static [&'static str])> = Vec::new();
    for tier in isa_tiers {
        if let Some(keys) = apple_sysctl_keys(*tier) {
            feature_descriptors.push((*tier, keys));
        }
    }

    if feature_descriptors.is_empty() {
        return;
    }

    externals.insert("sysctlbyname");

    writeln!(
        out,
        "@chic_cpu_active_tier = internal global i32 -1, align 4"
    )
    .ok();
    writeln!(out).ok();

    let mut sysctl_constants: BTreeMap<&'static str, (String, usize)> = BTreeMap::new();
    for (_, keys) in &feature_descriptors {
        for key in *keys {
            if !sysctl_constants.contains_key(key) {
                let symbol = format!(".str.arm_{}", sanitise_sysctl_key(key));
                let len = key.len() + 1;
                writeln!(
                    out,
                    "@{symbol} = private unnamed_addr constant [{len} x i8] c\"{key}\\00\", align 1"
                )
                .ok();
                sysctl_constants.insert(*key, (symbol, len));
            }
        }
    }
    writeln!(out).ok();

    writeln!(
        out,
        "define internal i1 @chic_arm_sysctl_flag(ptr %name) {{"
    )
    .ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  %value = alloca i32, align 4").ok();
    writeln!(out, "  %size = alloca i64, align 8").ok();
    writeln!(out, "  store i64 4, ptr %size").ok();
    writeln!(out, "  store i32 0, ptr %value").ok();
    writeln!(
        out,
        "  %call = call i32 @sysctlbyname(ptr %name, ptr %value, ptr %size, ptr null, i64 0)"
    )
    .ok();
    writeln!(out, "  %call_ok = icmp eq i32 %call, 0").ok();
    writeln!(out, "  %loaded = load i32, ptr %value").ok();
    writeln!(out, "  %has = icmp ne i32 %loaded, 0").ok();
    writeln!(out, "  %result = and i1 %call_ok, %has").ok();
    writeln!(out, "  ret i1 %result").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    let mut detection_functions = BTreeMap::new();

    for (tier, keys) in &feature_descriptors {
        if *tier == CpuIsaTier::Baseline {
            continue;
        }

        let func_name = format!("chic_arm_has_{}", tier.suffix());
        detection_functions.insert(*tier, func_name.clone());
        writeln!(out, "define internal i1 @{func_name}() {{").ok();
        writeln!(out, "entry:").ok();
        let mut local_counter = 0usize;
        let mut next_temp = |prefix: &str| {
            local_counter += 1;
            format!("%{}_{}", prefix, local_counter)
        };

        let mut current_flag: Option<String> = None;
        for key in *keys {
            let (symbol, len) = sysctl_constants
                .get(key)
                .expect("sysctl constant should exist");
            let gep_tmp = next_temp("sys");
            writeln!(
                out,
                "  {gep_tmp} = getelementptr inbounds [{len} x i8], ptr @{symbol}, i32 0, i32 0"
            )
            .ok();
            let call_tmp = next_temp("flag");
            writeln!(
                out,
                "  {call_tmp} = call i1 @chic_arm_sysctl_flag(ptr {gep_tmp})"
            )
            .ok();
            current_flag = Some(match current_flag {
                Some(existing) => {
                    let and_tmp = next_temp("and");
                    writeln!(out, "  {and_tmp} = and i1 {existing}, {call_tmp}").ok();
                    and_tmp
                }
                None => call_tmp,
            });
        }

        let final_flag = current_flag.unwrap_or_else(|| {
            let tmp = next_temp("true");
            writeln!(out, "  {tmp} = icmp eq i32 0, 0").ok();
            tmp
        });
        writeln!(out, "  ret i1 {final_flag}").ok();
        writeln!(out, "}}").ok();
        writeln!(out).ok();
    }

    writeln!(out, "define internal i32 @chic_cpu_select_tier() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  %cached = load i32, ptr @chic_cpu_active_tier").ok();
    writeln!(out, "  %is_cached = icmp sge i32 %cached, 0").ok();
    writeln!(out, "  br i1 %is_cached, label %exit, label %detect").ok();

    writeln!(out, "detect:").ok();
    let mut dispatch_counter = 0usize;
    let mut next_temp = |prefix: &str| {
        dispatch_counter += 1;
        format!("%{}_{}", prefix, dispatch_counter)
    };
    let mut current_value = CpuIsaTier::Baseline.index().to_string();

    for tier in isa_tiers.iter().rev() {
        match tier {
            CpuIsaTier::Baseline => {}
            CpuIsaTier::DotProd
            | CpuIsaTier::Fp16Fml
            | CpuIsaTier::Bf16
            | CpuIsaTier::I8mm
            | CpuIsaTier::Sve
            | CpuIsaTier::Sve2
            | CpuIsaTier::Crypto
            | CpuIsaTier::Pauth
            | CpuIsaTier::Bti
            | CpuIsaTier::Sme => {
                if let Some(func) = detection_functions.get(tier) {
                    let call_tmp = next_temp("has");
                    writeln!(out, "  {call_tmp} = call i1 @{func}()").ok();
                    let sel_tmp = next_temp("sel");
                    writeln!(
                        out,
                        "  {sel_tmp} = select i1 {call_tmp}, i32 {}, i32 {}",
                        tier.index(),
                        current_value
                    )
                    .ok();
                    current_value = sel_tmp;
                }
            }
            CpuIsaTier::Avx2 | CpuIsaTier::Avx512 | CpuIsaTier::Amx => {}
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

fn apple_sysctl_keys(tier: CpuIsaTier) -> Option<&'static [&'static str]> {
    const DOTPROD: [&str; 1] = ["hw.optional.arm.FEAT_DotProd"];
    const I8MM: [&str; 1] = ["hw.optional.arm.FEAT_I8MM"];
    const BF16: [&str; 1] = ["hw.optional.arm.FEAT_BF16"];
    const FHM: [&str; 1] = ["hw.optional.arm.FEAT_FHM"];
    const SVE: [&str; 1] = ["hw.optional.arm.FEAT_SVE"];
    const SVE2: [&str; 1] = ["hw.optional.arm.FEAT_SVE2"];
    const SME: [&str; 1] = ["hw.optional.arm.FEAT_SME"];
    const CRYPTO: [&str; 1] = ["hw.optional.arm.FEAT_AES"];
    const PAUTH: [&str; 1] = ["hw.optional.arm.FEAT_PAuth"];
    const BTI: [&str; 1] = ["hw.optional.arm.FEAT_BTI"];

    match tier {
        CpuIsaTier::DotProd => Some(&DOTPROD),
        CpuIsaTier::I8mm => Some(&I8MM),
        CpuIsaTier::Bf16 => Some(&BF16),
        CpuIsaTier::Fp16Fml => Some(&FHM),
        CpuIsaTier::Sve => Some(&SVE),
        CpuIsaTier::Sve2 => Some(&SVE2),
        CpuIsaTier::Sme => Some(&SME),
        CpuIsaTier::Crypto => Some(&CRYPTO),
        CpuIsaTier::Pauth => Some(&PAUTH),
        CpuIsaTier::Bti => Some(&BTI),
        _ => None,
    }
}
