use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;
use std::io;

use crate::cli::CliError;
use crate::frontend::parser::ParseError;

/// Unified error type for the temporary compiler implementation.
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Cli(CliError),
    Parse(ParseError),
    Codegen {
        message: String,
        backtrace: Option<Backtrace>,
    },
    Internal {
        message: String,
        backtrace: Option<Backtrace>,
    },
}

/// Convenience result alias used across the compiler.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Construct a new code generation error.
    pub fn codegen(message: impl Into<String>) -> Self {
        Self::Codegen {
            message: message.into(),
            backtrace: capture_backtrace(),
        }
    }

    #[allow(non_snake_case)]
    pub fn Codegen(message: String) -> Self {
        Self::codegen(message)
    }

    /// Construct a new internal compiler error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            backtrace: capture_backtrace(),
        }
    }

    #[allow(non_snake_case)]
    pub fn Internal(message: String) -> Self {
        Self::internal(message)
    }

    /// Return the captured backtrace, if any.
    pub fn backtrace(&self) -> Option<&Backtrace> {
        match self {
            Error::Codegen { backtrace, .. } | Error::Internal { backtrace, .. } => {
                backtrace.as_ref()
            }
            _ => None,
        }
    }
}

fn capture_backtrace() -> Option<Backtrace> {
    if cfg!(debug_assertions) {
        Some(Backtrace::force_capture())
    } else {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {err}"),
            Error::Cli(err) => write!(f, "{err}"),
            Error::Parse(err) => write!(f, "parse error: {err}"),
            Error::Codegen { message, .. } => write!(f, "codegen error: {message}"),
            Error::Internal { message, .. } => write!(f, "internal error: {message}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Cli(err) => Some(err),
            Error::Parse(err) => Some(err),
            Error::Codegen { .. } | Error::Internal { .. } => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<CliError> for Error {
    fn from(error: CliError) -> Self {
        Error::Cli(error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Error::Parse(error)
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error::internal(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Diagnostic;
    use crate::frontend::parser::ParseError;

    #[test]
    fn display_formats_variants() {
        let io_error = Error::from(io::Error::new(io::ErrorKind::Other, "disk error"));
        assert_eq!(io_error.to_string(), "I/O error: disk error");

        let cli_error = Error::from(CliError::new("bad args"));
        assert_eq!(cli_error.to_string(), "bad args");

        let parse_error = Error::from(ParseError::new(
            "unexpected token",
            vec![Diagnostic::error("bad token", None)],
        ));
        assert_eq!(parse_error.to_string(), "parse error: unexpected token");

        let codegen_error = Error::codegen("lowering failed");
        assert_eq!(codegen_error.to_string(), "codegen error: lowering failed");

        let internal_error = Error::internal("panic");
        assert_eq!(internal_error.to_string(), "internal error: panic");
    }

    #[test]
    fn source_exposes_wrapped_errors() {
        let io_error = Error::from(io::Error::new(io::ErrorKind::Other, "boom"));
        let source = io_error.source().unwrap();
        assert!(source.downcast_ref::<io::Error>().is_some());

        let cli_error = Error::from(CliError::new("oops"));
        let source = cli_error.source().unwrap();
        assert!(source.downcast_ref::<CliError>().is_some());

        let parse_error = Error::from(ParseError::new(
            "parse fail",
            vec![Diagnostic::error("bad token", None)],
        ));
        let source = parse_error.source().unwrap();
        assert!(source.downcast_ref::<ParseError>().is_some());

        let codegen_error = Error::codegen("cgen");
        assert!(codegen_error.source().is_none());

        let internal_error = Error::internal("internal");
        assert!(internal_error.source().is_none());
    }

    #[test]
    fn debug_builds_capture_backtrace() {
        if cfg!(debug_assertions) {
            let err = Error::internal("capture");
            assert!(err.backtrace().is_some());
        }
    }
}
