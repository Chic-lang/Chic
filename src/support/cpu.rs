use std::env;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Snapshot of CPU feature support relevant to runtime SIMD paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatures {
    pub sse2: bool,
    pub sse42: bool,
    pub avx2: bool,
    pub neon: bool,
}

impl CpuFeatures {
    /// Construct a new feature snapshot with the provided flags.
    #[must_use]
    pub const fn new(sse2: bool, sse42: bool, avx2: bool, neon: bool) -> Self {
        Self {
            sse2,
            sse42,
            avx2,
            neon,
        }
    }

    /// Convenience constructor representing a CPU with no SIMD support.
    #[must_use]
    pub const fn none() -> Self {
        Self::new(false, false, false, false)
    }

    /// Whether SSE2 support is available.
    #[must_use]
    pub const fn has_sse2(&self) -> bool {
        self.sse2
    }

    /// Whether SSE4.2 support is available.
    #[must_use]
    pub const fn has_sse42(&self) -> bool {
        self.sse42
    }

    /// Whether AVX2 support is available.
    #[must_use]
    pub const fn has_avx2(&self) -> bool {
        self.avx2
    }

    /// Whether NEON support is available.
    #[must_use]
    pub const fn has_neon(&self) -> bool {
        self.neon
    }

    /// Whether the CPU exposes any byte-wise SIMD path we can target.
    #[must_use]
    pub const fn has_byte_simd(&self) -> bool {
        self.avx2 || self.neon || self.sse2 || self.sse42
    }
}

static DETECTED: OnceLock<CpuFeatures> = OnceLock::new();
static OVERRIDE: AtomicU8 = AtomicU8::new(0);

const OVERRIDE_ACTIVE: u8 = 1 << 7;
const SSE2_BIT: u8 = 1 << 0;
const SSE42_BIT: u8 = 1 << 1;
const AVX2_BIT: u8 = 1 << 2;
const NEON_BIT: u8 = 1 << 3;

const FEATURE_BITS_MASK: u8 = SSE2_BIT | SSE42_BIT | AVX2_BIT | NEON_BIT;
const ENV_OVERRIDE_VAR: &str = "CHIC_CPU_OVERRIDE";

fn detected_features() -> &'static CpuFeatures {
    DETECTED.get_or_init(detect_features)
}

#[cfg(any(test, feature = "simd-test-hooks"))]
fn features_to_bits(features: CpuFeatures) -> u8 {
    let mut bits = 0;
    if features.has_sse2() {
        bits |= SSE2_BIT;
    }
    if features.has_sse42() {
        bits |= SSE42_BIT;
    }
    if features.has_avx2() {
        bits |= AVX2_BIT;
    }
    if features.has_neon() {
        bits |= NEON_BIT;
    }
    bits
}

#[cfg(any(test, feature = "simd-test-hooks"))]
fn encode_features(features: CpuFeatures) -> u8 {
    features_to_bits(features)
}

fn decode_features(bits: u8) -> CpuFeatures {
    CpuFeatures::new(
        bits & SSE2_BIT != 0,
        bits & SSE42_BIT != 0,
        bits & AVX2_BIT != 0,
        bits & NEON_BIT != 0,
    )
}

fn apply_env_override() {
    static ENV_APPLIED: OnceLock<()> = OnceLock::new();
    ENV_APPLIED.get_or_init(|| {
        if OVERRIDE.load(Ordering::Acquire) & OVERRIDE_ACTIVE != 0 {
            return;
        }
        if let Some(bits) = read_env_override() {
            OVERRIDE.store(bits | OVERRIDE_ACTIVE, Ordering::Release);
        }
    });
}

fn read_env_override() -> Option<u8> {
    let raw = env::var(ENV_OVERRIDE_VAR).ok()?;
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    if value.eq_ignore_ascii_case("auto") {
        return None;
    }
    match parse_env_override(value) {
        Ok(bits) => Some(bits),
        Err(err) => {
            eprintln!(
                "warning: ignoring {} value `{raw}` ({err})",
                ENV_OVERRIDE_VAR
            );
            None
        }
    }
}

fn parse_env_override(value: &str) -> Result<u8, String> {
    if value.eq_ignore_ascii_case("scalar") || value.eq_ignore_ascii_case("none") {
        return Ok(0);
    }
    if value.eq_ignore_ascii_case("simd") {
        return Ok(FEATURE_BITS_MASK);
    }

    let mut bits = 0;
    for token in value
        .split(|ch: char| ch == ',' || ch == '+' || ch == '|' || ch.is_whitespace())
        .filter(|segment| !segment.trim().is_empty())
    {
        let lowered = token.trim().to_ascii_lowercase();
        match lowered.as_str() {
            "sse2" => bits |= SSE2_BIT,
            "sse4.2" | "sse42" => bits |= SSE42_BIT,
            "avx2" => bits |= AVX2_BIT,
            "neon" => bits |= NEON_BIT,
            other => {
                return Err(format!("unrecognised SIMD feature `{other}`"));
            }
        }
    }

    if bits == 0 {
        Err("no recognised SIMD features provided".into())
    } else {
        Ok(bits)
    }
}

/// Return the cached CPU feature detection result, applying any testing override.
#[must_use]
pub fn features() -> CpuFeatures {
    apply_env_override();
    let override_bits = OVERRIDE.load(Ordering::Acquire);
    if override_bits & OVERRIDE_ACTIVE != 0 {
        return decode_features(override_bits & FEATURE_BITS_MASK);
    }
    *detected_features()
}

/// Whether SSE4.2 support is available on the current CPU (or testing override).
#[must_use]
pub fn has_sse42() -> bool {
    features().has_sse42()
}

/// Whether NEON support is available on the current CPU (or testing override).
#[must_use]
pub fn has_neon() -> bool {
    features().has_neon()
}

/// Whether SSE2 support is available on the current CPU (or testing override).
#[must_use]
pub fn has_sse2() -> bool {
    features().has_sse2()
}

/// Whether AVX2 support is available on the current CPU (or testing override).
#[must_use]
pub fn has_avx2() -> bool {
    features().has_avx2()
}

#[cfg(any(test, feature = "simd-test-hooks"))]
/// Apply a temporary feature override for tests/benchmarks. Restores the previous value on drop.
pub fn override_for_testing(features: CpuFeatures) -> FeatureOverrideGuard {
    let encoded = encode_features(features) | OVERRIDE_ACTIVE;
    let previous = OVERRIDE.swap(encoded, Ordering::AcqRel);
    FeatureOverrideGuard { previous }
}

#[cfg(any(test, feature = "simd-test-hooks"))]
/// Guard returned by `override_for_testing` to automatically restore the original state.
pub struct FeatureOverrideGuard {
    previous: u8,
}

#[cfg(any(test, feature = "simd-test-hooks"))]
impl Drop for FeatureOverrideGuard {
    fn drop(&mut self) {
        OVERRIDE.store(self.previous, Ordering::Release);
    }
}

#[cfg(all(
    feature = "runtime-simd",
    any(target_arch = "x86", target_arch = "x86_64")
))]
fn detect_features() -> CpuFeatures {
    CpuFeatures::new(
        std::arch::is_x86_feature_detected!("sse2"),
        std::arch::is_x86_feature_detected!("sse4.2"),
        std::arch::is_x86_feature_detected!("avx2"),
        false,
    )
}

#[cfg(all(feature = "runtime-simd", target_arch = "aarch64"))]
fn detect_features() -> CpuFeatures {
    CpuFeatures::new(
        false,
        false,
        false,
        std::arch::is_aarch64_feature_detected!("neon"),
    )
}

#[cfg(any(
    not(feature = "runtime-simd"),
    all(
        feature = "runtime-simd",
        not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))
    )
))]
fn detect_features() -> CpuFeatures {
    CpuFeatures::none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_is_cached() {
        let first = detected_features() as *const CpuFeatures;
        let second = detected_features() as *const CpuFeatures;
        assert_eq!(first, second);
    }

    #[test]
    fn byte_simd_flag_reflects_available_features() {
        let feats = features();
        assert_eq!(
            feats.has_byte_simd(),
            feats.has_avx2() || feats.has_neon() || feats.has_sse2() || feats.has_sse42()
        );
    }

    #[test]
    fn overrides_take_precedence() {
        let guard = override_for_testing(CpuFeatures::new(true, true, false, false));
        assert!(has_sse2());
        assert!(has_sse42());
        assert!(!has_neon());
        drop(guard);
    }

    #[test]
    fn override_restores_previous_value() {
        let original = features();
        {
            let _guard = override_for_testing(CpuFeatures::new(false, false, false, true));
            assert!(has_neon());
        }
        assert_eq!(features(), original);
    }

    #[test]
    fn parse_env_override_accepts_scalar() {
        assert_eq!(parse_env_override("scalar").unwrap(), 0);
        assert_eq!(parse_env_override("none").unwrap(), 0);
    }

    #[test]
    fn parse_env_override_accepts_simd_alias() {
        assert_eq!(
            parse_env_override("simd").unwrap(),
            SSE2_BIT | SSE42_BIT | AVX2_BIT | NEON_BIT
        );
    }

    #[test]
    fn parse_env_override_allows_feature_list() {
        let bits = parse_env_override("sse2 + avx2, neon").unwrap();
        assert!(bits & SSE2_BIT != 0);
        assert!(bits & AVX2_BIT != 0);
        assert!(bits & NEON_BIT != 0);
        assert!(bits & SSE42_BIT == 0);
    }

    #[test]
    fn parse_env_override_rejects_unknown_token() {
        assert!(parse_env_override("sse3").is_err());
    }
}
