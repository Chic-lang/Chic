use std::fmt;
use std::str::FromStr;

use crate::target::TargetArch;

/// CPU ISA tiers supported by the LLVM backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum CpuIsaTier {
    Baseline,
    Avx2,
    Avx512,
    Amx,
    DotProd,
    Fp16Fml,
    Bf16,
    I8mm,
    Sve,
    Sve2,
    Crypto,
    Pauth,
    Bti,
    Sme,
}

impl CpuIsaTier {
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            CpuIsaTier::Baseline => "baseline",
            CpuIsaTier::Avx2 => "avx2",
            CpuIsaTier::Avx512 => "avx512",
            CpuIsaTier::Amx => "amx",
            CpuIsaTier::DotProd => "dotprod",
            CpuIsaTier::Fp16Fml => "fp16fml",
            CpuIsaTier::Bf16 => "bf16",
            CpuIsaTier::I8mm => "i8mm",
            CpuIsaTier::Sve => "sve",
            CpuIsaTier::Sve2 => "sve2",
            CpuIsaTier::Crypto => "crypto",
            CpuIsaTier::Pauth => "pauth",
            CpuIsaTier::Bti => "bti",
            CpuIsaTier::Sme => "sme",
        }
    }

    #[must_use]
    pub fn index(self) -> i32 {
        match self {
            CpuIsaTier::Baseline => 0,
            CpuIsaTier::Avx2 => 1,
            CpuIsaTier::Avx512 => 2,
            CpuIsaTier::Amx => 3,
            CpuIsaTier::DotProd => 4,
            CpuIsaTier::Fp16Fml => 5,
            CpuIsaTier::Bf16 => 6,
            CpuIsaTier::I8mm => 7,
            CpuIsaTier::Sve => 8,
            CpuIsaTier::Sve2 => 9,
            CpuIsaTier::Crypto => 10,
            CpuIsaTier::Pauth => 11,
            CpuIsaTier::Bti => 12,
            CpuIsaTier::Sme => 13,
        }
    }

    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            CpuIsaTier::Baseline => "baseline (SSE4.2 or NEON)",
            CpuIsaTier::Avx2 => "AVX2",
            CpuIsaTier::Avx512 => "AVX-512F",
            CpuIsaTier::Amx => "AMX (Tile/VNNI)",
            CpuIsaTier::DotProd => "Armv8.2-A dot-product (UDOT/SDOT)",
            CpuIsaTier::Fp16Fml => "Arm FP16 FMA (FP16FML)",
            CpuIsaTier::Bf16 => "Arm BF16 arithmetic",
            CpuIsaTier::I8mm => "Arm int8 matrix multiply (I8MM)",
            CpuIsaTier::Sve => "Arm Scalable Vector Extension (SVE)",
            CpuIsaTier::Sve2 => "Arm Scalable Vector Extension 2 (SVE2)",
            CpuIsaTier::Crypto => "Arm cryptography extensions (AES/SHA/PMULL/CRC)",
            CpuIsaTier::Pauth => "Arm pointer authentication",
            CpuIsaTier::Bti => "Arm Branch Target Identification",
            CpuIsaTier::Sme => "Arm Scalable Matrix Extension (SME)",
        }
    }

    fn is_x86_tier(self) -> bool {
        matches!(
            self,
            CpuIsaTier::Baseline | CpuIsaTier::Avx2 | CpuIsaTier::Avx512 | CpuIsaTier::Amx
        )
    }

    fn is_aarch64_tier(self) -> bool {
        matches!(
            self,
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
                | CpuIsaTier::Sme
        )
    }
}

impl fmt::Display for CpuIsaTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.suffix())
    }
}

impl FromStr for CpuIsaTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "baseline" | "sse4.2" | "sse42" | "neon" => Ok(CpuIsaTier::Baseline),
            "avx2" => Ok(CpuIsaTier::Avx2),
            "avx512" | "avx-512" | "avx512f" => Ok(CpuIsaTier::Avx512),
            "amx" => Ok(CpuIsaTier::Amx),
            "dotprod" | "dot-product" | "udot" | "sdot" => Ok(CpuIsaTier::DotProd),
            "fp16fml" | "fp16" | "half-fma" => Ok(CpuIsaTier::Fp16Fml),
            "bf16" | "bfloat16" => Ok(CpuIsaTier::Bf16),
            "i8mm" | "int8mm" => Ok(CpuIsaTier::I8mm),
            "sve" => Ok(CpuIsaTier::Sve),
            "sve2" => Ok(CpuIsaTier::Sve2),
            "crypto" | "aes" => Ok(CpuIsaTier::Crypto),
            "pauth" | "pointer-auth" => Ok(CpuIsaTier::Pauth),
            "bti" => Ok(CpuIsaTier::Bti),
            "sme" => Ok(CpuIsaTier::Sme),
            other => Err(format!("unrecognised ISA tier '{other}'")),
        }
    }
}

/// User-configurable CPU ISA selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuIsaConfig {
    tiers: Vec<CpuIsaTier>,
    sve_bits: Option<u32>,
}

impl CpuIsaConfig {
    #[must_use]
    pub fn auto() -> Self {
        Self {
            tiers: vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::Avx2,
                CpuIsaTier::Avx512,
                CpuIsaTier::Amx,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
                CpuIsaTier::Bf16,
                CpuIsaTier::I8mm,
                CpuIsaTier::Sve,
                CpuIsaTier::Sve2,
                CpuIsaTier::Crypto,
                CpuIsaTier::Pauth,
                CpuIsaTier::Bti,
                CpuIsaTier::Sme,
            ],
            sve_bits: None,
        }
    }

    #[must_use]
    pub fn baseline() -> Self {
        Self {
            tiers: vec![CpuIsaTier::Baseline],
            sve_bits: None,
        }
    }

    #[must_use]
    pub fn from_tiers(mut tiers: Vec<CpuIsaTier>) -> Self {
        if !tiers.contains(&CpuIsaTier::Baseline) {
            tiers.insert(0, CpuIsaTier::Baseline);
        }
        tiers.sort();
        tiers.dedup();
        Self {
            tiers,
            sve_bits: None,
        }
    }

    /// Parse a comma-separated ISA tier list or profile name.
    ///
    /// # Errors
    ///
    /// Returns an error when no tiers are provided or when an unknown tier/profile is encountered.
    pub fn parse_list(spec: &str) -> Result<Self, String> {
        if spec.trim().eq_ignore_ascii_case("auto") {
            return Ok(Self::auto());
        }
        let mut tiers = Vec::new();
        for token in spec.split(',') {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(tier) = trimmed.parse::<CpuIsaTier>() {
                tiers.push(tier);
                continue;
            }
            let normalised = trimmed.to_ascii_lowercase();
            if let Some(profile) = profile_tiers(&normalised) {
                tiers.extend_from_slice(profile);
            } else {
                return Err(format!("unrecognised ISA tier '{trimmed}'"));
            }
        }
        if tiers.is_empty() {
            return Err("CPU ISA list must contain at least one entry".into());
        }
        Ok(Self::from_tiers(tiers))
    }

    #[must_use]
    pub fn tiers(&self) -> &[CpuIsaTier] {
        &self.tiers
    }

    #[must_use]
    pub fn effective_tiers(&self, arch: TargetArch) -> Vec<CpuIsaTier> {
        match arch {
            TargetArch::X86_64 => self
                .tiers
                .iter()
                .filter(|tier| tier.is_x86_tier())
                .copied()
                .collect(),
            TargetArch::Aarch64 => self
                .tiers
                .iter()
                .filter(|tier| tier.is_aarch64_tier())
                .copied()
                .collect(),
        }
    }

    ///
    /// # Errors
    ///
    /// Returns an error if `bits` is less than 128 or not a multiple of 128.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn set_sve_bits(&mut self, bits: u32) -> Result<(), String> {
        if bits < 128 || bits % 128 != 0 {
            return Err(format!(
                "SVE vector length must be a multiple of 128 bits (received {bits})"
            ));
        }
        self.sve_bits = Some(bits);
        Ok(())
    }

    #[must_use]
    pub fn sve_vector_bits(&self) -> Option<u32> {
        self.sve_bits
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn fingerprint_bytes(&self) -> Vec<u8> {
        let mut bytes = self
            .tiers
            .iter()
            .map(|tier| tier.index() as u8)
            .collect::<Vec<_>>();
        if let Some(bits) = self.sve_bits {
            bytes.extend_from_slice(&bits.to_le_bytes());
        }
        bytes
    }
}

impl Default for CpuIsaConfig {
    fn default() -> Self {
        Self::auto()
    }
}

#[allow(clippy::too_many_lines)]
fn profile_tiers(token: &str) -> Option<&'static [CpuIsaTier]> {
    const APPLE_M1: &[CpuIsaTier] = &[
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Crypto,
        CpuIsaTier::Pauth,
        CpuIsaTier::Bti,
    ];
    const APPLE_M2: &[CpuIsaTier] = &[
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
        CpuIsaTier::Crypto,
        CpuIsaTier::Pauth,
        CpuIsaTier::Bti,
    ];
    const APPLE_M4: &[CpuIsaTier] = &[
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
        CpuIsaTier::Crypto,
        CpuIsaTier::Pauth,
        CpuIsaTier::Bti,
        CpuIsaTier::Sme,
    ];
    const AMPERE_ALTRA: &[CpuIsaTier] = &[CpuIsaTier::DotProd, CpuIsaTier::Fp16Fml];
    const AMPERE_ONE: &[CpuIsaTier] = &[
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
    ];
    const GRACE: &[CpuIsaTier] = &[
        CpuIsaTier::DotProd,
        CpuIsaTier::Fp16Fml,
        CpuIsaTier::Bf16,
        CpuIsaTier::I8mm,
        CpuIsaTier::Sve,
        CpuIsaTier::Sve2,
    ];

    match token {
        "apple-m1" | "apple_m1" => Some(APPLE_M1),
        "apple-m2" | "apple_m2" | "apple-m3" | "apple_m3" => Some(APPLE_M2),
        "apple-m4" | "apple_m4" => Some(APPLE_M4),
        "ampere-altra" | "ampere_altra" | "neoverse-n1" | "neoverse_n1" => Some(AMPERE_ALTRA),
        "ampere-one" | "ampere_one" | "ampere1" | "ampere1a" | "ampere1b" => Some(AMPERE_ONE),
        "nvidia-grace" | "nvidia_grace" | "grace" | "neoverse-v2" | "neoverse_v2" => Some(GRACE),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_apple_m2_profile() {
        let config = CpuIsaConfig::parse_list("apple-m2").expect("parse apple profile");
        assert!(config.tiers().contains(&CpuIsaTier::DotProd));
        assert!(config.tiers().contains(&CpuIsaTier::Bf16));
        assert!(config.tiers().contains(&CpuIsaTier::I8mm));
        assert!(!config.tiers().contains(&CpuIsaTier::Sme));

        let tiers = config.effective_tiers(TargetArch::Aarch64);
        assert_eq!(
            tiers,
            vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
                CpuIsaTier::Bf16,
                CpuIsaTier::I8mm,
                CpuIsaTier::Crypto,
                CpuIsaTier::Pauth,
                CpuIsaTier::Bti,
            ]
        );
    }

    #[test]
    fn parses_apple_m4_profile() {
        let config = CpuIsaConfig::parse_list("apple-m4").expect("parse apple profile");
        assert!(config.tiers().contains(&CpuIsaTier::Sme));
        let tiers = config.effective_tiers(TargetArch::Aarch64);
        assert_eq!(
            tiers,
            vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
                CpuIsaTier::Bf16,
                CpuIsaTier::I8mm,
                CpuIsaTier::Crypto,
                CpuIsaTier::Pauth,
                CpuIsaTier::Bti,
                CpuIsaTier::Sme,
            ]
        );
    }

    #[test]
    fn parses_ampere_profiles() {
        let altra = CpuIsaConfig::parse_list("ampere-altra").expect("parse altra profile");
        let altra_tiers = altra.effective_tiers(TargetArch::Aarch64);
        assert_eq!(
            altra_tiers,
            vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
            ]
        );

        let ampere_one = CpuIsaConfig::parse_list("ampere1").expect("parse ampere1 profile");
        let one_tiers = ampere_one.effective_tiers(TargetArch::Aarch64);
        assert_eq!(
            one_tiers,
            vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
                CpuIsaTier::Bf16,
                CpuIsaTier::I8mm,
            ]
        );
    }

    #[test]
    fn parses_grace_profile() {
        let grace = CpuIsaConfig::parse_list("nvidia-grace").expect("parse grace profile");
        let tiers = grace.effective_tiers(TargetArch::Aarch64);
        assert_eq!(
            tiers,
            vec![
                CpuIsaTier::Baseline,
                CpuIsaTier::DotProd,
                CpuIsaTier::Fp16Fml,
                CpuIsaTier::Bf16,
                CpuIsaTier::I8mm,
                CpuIsaTier::Sve,
                CpuIsaTier::Sve2,
            ]
        );
    }
}
