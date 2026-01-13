mod apple;
mod linux;
mod x86;

use std::collections::BTreeSet;

use crate::codegen::CpuIsaTier;
use crate::target::TargetArch;

pub(crate) use linux::emit_external_declarations;

pub(crate) fn emit_cpu_dispatch_helpers(
    out: &mut String,
    isa_tiers: &[CpuIsaTier],
    externals: &mut BTreeSet<&'static str>,
    arch: TargetArch,
    is_apple_target: bool,
    sve_bits: Option<u32>,
) {
    match arch {
        TargetArch::X86_64 => x86::emit_cpu_dispatch_helpers(out, isa_tiers, externals),
        TargetArch::Aarch64 => {
            if is_apple_target {
                apple::emit_cpu_dispatch_helpers(out, isa_tiers, externals);
            } else {
                linux::emit_cpu_dispatch_helpers(out, isa_tiers, externals, sve_bits);
            }
        }
    }
}

pub(super) fn sanitise_sysctl_key(key: &str) -> String {
    key.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}
