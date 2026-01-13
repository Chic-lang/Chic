//! Target selection for the Chic bootstrap compiler.

use std::env;
use std::fmt;

/// Supported architecture families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Aarch64,
}

impl TargetArch {
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "x86_64" | "amd64" => Some(TargetArch::X86_64),
            "aarch64" | "arm64" => Some(TargetArch::Aarch64),
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn canonical_triple(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64-unknown-none",
            TargetArch::Aarch64 => "aarch64-unknown-none",
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::Aarch64 => "aarch64",
        }
    }
}

/// Supported operating systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetOs {
    Macos,
    Linux,
    Windows,
    None,
    Other(String),
}

impl TargetOs {
    pub fn parse(token: &str) -> Self {
        match token {
            "darwin" | "macos" | "ios" => TargetOs::Macos,
            "linux" => TargetOs::Linux,
            "windows" | "pc" | "win" => TargetOs::Windows,
            "none" | "unknown" => TargetOs::None,
            other => TargetOs::Other(other.to_string()),
        }
    }

    fn triple_component(&self) -> &str {
        match self {
            TargetOs::Macos => "macos",
            TargetOs::Linux => "linux",
            TargetOs::Windows => "windows",
            TargetOs::None => "none",
            TargetOs::Other(value) => value.as_str(),
        }
    }
}

/// Target runtime profile/back-end flavour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetRuntime {
    Llvm,
    Wasm,
    NativeStd,
    NativeNoStd,
    Other(String),
}

impl TargetRuntime {
    pub fn parse(token: &str) -> Self {
        match token {
            "llvm" | "native" => TargetRuntime::Llvm,
            "wasm" | "wasmtime" => TargetRuntime::Wasm,
            "native-std" | "std" => TargetRuntime::NativeStd,
            "native-no_std" | "no-std" | "nostd" | "native-no-std" => TargetRuntime::NativeNoStd,
            other => TargetRuntime::Other(other.to_string()),
        }
    }

    fn env_component(&self) -> Option<&str> {
        match self {
            TargetRuntime::Llvm => None,
            TargetRuntime::Wasm => Some("wasm"),
            TargetRuntime::NativeStd => Some("std"),
            TargetRuntime::NativeNoStd => Some("nostd"),
            TargetRuntime::Other(value) => Some(value.as_str()),
        }
    }
}

/// Target triple description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    arch: TargetArch,
    os: TargetOs,
    runtime: TargetRuntime,
    triple: String,
}

impl Target {
    /// Construct the target representing the current host.
    #[must_use]
    pub fn host() -> Self {
        let arch = TargetArch::parse(env::consts::ARCH).unwrap_or(TargetArch::X86_64);
        let os = TargetOs::parse(env::consts::OS);
        Self::from_components(arch, os, TargetRuntime::Llvm)
    }

    /// Parse a target triple or shorthand from CLI input.
    ///
    /// # Errors
    ///
    /// Returns [`TargetError::Empty`] when the input is blank or comprised of
    /// whitespace, and [`TargetError::UnsupportedArch`] when the architecture
    /// prefix is not recognised.
    pub fn parse(spec: &str) -> Result<Self, TargetError> {
        let trimmed = spec.trim().to_ascii_lowercase();
        if trimmed.is_empty() {
            return Err(TargetError::Empty);
        }

        let mut parts = trimmed.splitn(2, '-');
        let arch_token = parts.next().unwrap_or_default();
        let arch = TargetArch::parse(arch_token)
            .ok_or_else(|| TargetError::UnsupportedArch(arch_token.to_string()))?;

        if let Some(rest) = parts.next() {
            let mut rest_iter = rest.split('-');
            let vendor = rest_iter.next().unwrap_or_default();
            let os_token = rest_iter.next().unwrap_or("none");
            let mut env_token = rest_iter.next();
            let os = TargetOs::parse(os_token);
            if env_token.is_none() && matches!(os, TargetOs::Linux) {
                env_token = Some("gnu");
            }
            let runtime = env_token
                .map(TargetRuntime::parse)
                .unwrap_or(TargetRuntime::Llvm);
            let triple = if let Some(env) = env_token {
                format!("{arch_token}-{vendor}-{os_token}-{env}")
            } else {
                format!("{arch_token}-{vendor}-{os_token}")
            };
            Ok(Self {
                arch,
                os,
                runtime,
                triple,
            })
        } else {
            Ok(Self::from_components(
                arch,
                TargetOs::None,
                TargetRuntime::Llvm,
            ))
        }
    }

    /// Construct a target from explicit components.
    #[must_use]
    pub fn from_components(arch: TargetArch, os: TargetOs, runtime: TargetRuntime) -> Self {
        let vendor = match os {
            TargetOs::Macos => "apple",
            TargetOs::Linux => "unknown",
            TargetOs::Windows => "pc",
            TargetOs::None | TargetOs::Other(_) => "unknown",
        };
        let os_component = os.triple_component();
        let triple = if let Some(env) = runtime.env_component() {
            format!("{}-{}-{}-{}", arch.as_str(), vendor, os_component, env)
        } else {
            format!("{}-{}-{}", arch.as_str(), vendor, os_component)
        };
        Self {
            arch,
            os,
            runtime,
            triple,
        }
    }

    /// Return the architecture family.
    #[must_use]
    pub fn arch(&self) -> TargetArch {
        self.arch
    }

    /// Return the target operating system.
    #[must_use]
    pub fn os(&self) -> &TargetOs {
        &self.os
    }

    /// Return the target runtime flavour.
    #[must_use]
    pub fn runtime(&self) -> &TargetRuntime {
        &self.runtime
    }

    /// Return the canonical triple for this target.
    #[must_use]
    pub fn triple(&self) -> &str {
        &self.triple
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::host()
    }
}

/// Errors encountered while parsing a target specification.
#[derive(Debug, Clone)]
pub enum TargetError {
    Empty,
    UnsupportedArch(String),
}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetError::Empty => write!(f, "target string must not be empty"),
            TargetError::UnsupportedArch(arch) => {
                write!(
                    f,
                    "unsupported architecture '{arch}'; expected one of x86_64, amd64, aarch64, arm64"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_triple() {
        let parsed = Target::parse("x86_64-apple-darwin");

        let target = match parsed {
            Ok(value) => value,
            Err(err) => panic!("expected Ok target, found Err: {err}"),
        };
        assert_eq!(target.arch(), TargetArch::X86_64);
        assert_eq!(target.triple(), "x86_64-apple-darwin");
    }

    #[test]
    fn parses_shorthand_arm64() {
        let parsed = Target::parse("arm64");

        let target = match parsed {
            Ok(value) => value,
            Err(err) => panic!("expected Ok target, found Err: {err}"),
        };
        assert_eq!(target.arch(), TargetArch::Aarch64);
        assert_eq!(target.triple(), "aarch64-unknown-none");
    }

    #[test]
    fn rejects_unknown_arch() {
        let parsed = Target::parse("mips");

        let err = match parsed {
            Ok(value) => panic!("expected Err, found Ok target: {value:?}"),
            Err(err) => err,
        };
        assert!(matches!(err, TargetError::UnsupportedArch(_)));
    }
}
