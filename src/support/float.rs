use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Float128Mode {
    Native,
    Emulated,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FloatSupport {
    pub has_f16: bool,
    pub f128: Float128Mode,
}

fn parse_float128_env() -> Option<Float128Mode> {
    let value = env::var("CHIC_FLOAT128").ok()?;
    let lowered = value.to_ascii_lowercase();
    match lowered.as_str() {
        "emulate" | "emulated" => Some(Float128Mode::Emulated),
        "disable" | "off" | "none" | "unsupported" => Some(Float128Mode::Unsupported),
        "native" | "on" => Some(Float128Mode::Native),
        _ => None,
    }
}

fn default_float128_mode() -> Float128Mode {
    if cfg!(target_arch = "wasm32") {
        Float128Mode::Emulated
    } else {
        Float128Mode::Native
    }
}

/// Detects float16 and float128 availability for the current host/target.
#[must_use]
pub fn detect_float_support() -> FloatSupport {
    let f128 = parse_float128_env().unwrap_or_else(default_float128_mode);
    FloatSupport {
        has_f16: !cfg!(target_arch = "wasm32"),
        f128,
    }
}

#[must_use]
pub fn float16_supported() -> bool {
    detect_float_support().has_f16
}

#[must_use]
pub fn float128_mode() -> Float128Mode {
    detect_float_support().f128
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The current toolchain marks env var mutation as unsafe; confine it here.
    fn reset_env(original: Option<String>) {
        unsafe {
            if let Some(value) = original {
                env::set_var("CHIC_FLOAT128", value);
            } else {
                env::remove_var("CHIC_FLOAT128");
            }
        }
    }

    #[test]
    fn env_overrides_float128_mode() {
        let original = env::var("CHIC_FLOAT128").ok();
        unsafe {
            env::set_var("CHIC_FLOAT128", "disable");
            assert_eq!(float128_mode(), Float128Mode::Unsupported);
            env::set_var("CHIC_FLOAT128", "emulate");
        }
        assert_eq!(float128_mode(), Float128Mode::Emulated);
        reset_env(original);
    }
}
