//! Chic artifact kinds for the Chic bootstrap tool.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported build outputs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChicKind {
    #[default]
    Executable,
    StaticLibrary,
    DynamicLibrary,
}

impl ChicKind {
    /// Parse a user-supplied descriptor (case-insensitive).
    ///
    /// # Errors
    /// Returns [`ChicKindError::Empty`] when `spec` is blank, or
    /// [`ChicKindError::Unsupported`] when the descriptor is unknown.
    pub fn parse(spec: &str) -> Result<Self, ChicKindError> {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return Err(ChicKindError::Empty);
        }
        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "exe" | "bin" | "app" | "program" | "executable" => Ok(ChicKind::Executable),
            "lib" | "library" | "static" | "staticlib" | "rlib" => Ok(ChicKind::StaticLibrary),
            "dylib" | "shared" | "dynamic" | "dynamiclib" | "sharedlib" => {
                Ok(ChicKind::DynamicLibrary)
            }
            other => Err(ChicKindError::Unsupported(other.to_string())),
        }
    }

    /// Return a canonical string for diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ChicKind::Executable => "executable",
            ChicKind::StaticLibrary => "static-library",
            ChicKind::DynamicLibrary => "dynamic-library",
        }
    }

    /// Return the default artifact file extension for this kind.
    #[must_use]
    pub fn default_extension(self) -> &'static str {
        match self {
            ChicKind::Executable => "clbin",
            ChicKind::StaticLibrary => "a",
            ChicKind::DynamicLibrary => "dylib",
        }
    }

    /// Returns true if this represents any library form.
    #[must_use]
    pub fn is_library(self) -> bool {
        matches!(self, ChicKind::StaticLibrary | ChicKind::DynamicLibrary)
    }
}

/// Errors raised when parsing a Chic build kind.
#[derive(Debug, Clone)]
pub enum ChicKindError {
    Empty,
    Unsupported(String),
}

impl fmt::Display for ChicKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChicKindError::Empty => write!(f, "build kind must not be empty"),
            ChicKindError::Unsupported(kind) => write!(
                f,
                "unsupported build kind '{kind}'; expected one of exe, executable, bin, app, lib, library"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_alias(alias: &str) -> ChicKind {
        match ChicKind::parse(alias) {
            Ok(kind) => kind,
            Err(err) => panic!("alias `{alias}` should parse: {err}"),
        }
    }

    #[test]
    fn parses_executable_aliases() {
        for alias in ["exe", "bin", "app", "executable", "program"] {
            assert_eq!(parse_alias(alias), ChicKind::Executable);
        }
    }

    #[test]
    fn parses_library_aliases() {
        for alias in ["lib", "library", "static", "staticlib", "rlib"] {
            assert_eq!(parse_alias(alias), ChicKind::StaticLibrary);
        }
    }

    #[test]
    fn parses_dynamic_library_aliases() {
        for alias in ["dylib", "shared", "dynamic", "dynamiclib", "sharedlib"] {
            assert_eq!(parse_alias(alias), ChicKind::DynamicLibrary);
        }
    }

    #[test]
    fn rejects_unknown_kind() {
        match ChicKind::parse("plugin") {
            Ok(kind) => panic!("expected unsupported kind, parsed {kind:?}"),
            Err(ChicKindError::Unsupported(_)) => {}
            Err(err) => panic!("unexpected parse error: {err}"),
        }
    }
}
