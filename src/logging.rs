use std::env;
use std::fmt;

/// Output format for compiler log events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Auto,
    Text,
    Json,
}

impl LogFormat {
    pub fn parse(spec: &str) -> Option<Self> {
        match spec.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "text" | "plain" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

impl fmt::Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            LogFormat::Auto => "auto",
            LogFormat::Text => "text",
            LogFormat::Json => "json",
        };
        f.write_str(text)
    }
}

/// Logging verbosity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn parse(spec: &str) -> Option<Self> {
        match spec.to_ascii_lowercase().as_str() {
            "error" | "err" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" | "verbose" => Some(Self::Trace),
            _ => None,
        }
    }

    pub fn as_tracing_level(self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        };
        f.write_str(text)
    }
}

/// User-specified or environment-provided log configuration.
#[derive(Debug, Clone, Copy)]
pub struct LogOptions {
    pub format: LogFormat,
    pub level: LogLevel,
}

impl LogOptions {
    pub const DEFAULT: Self = Self {
        format: LogFormat::Auto,
        level: LogLevel::Info,
    };

    #[must_use]
    pub fn with_overrides(base: Self, overrides: LogSettings) -> Self {
        Self {
            format: overrides.format.unwrap_or(base.format),
            level: overrides.level.unwrap_or(base.level),
        }
    }

    #[must_use]
    pub fn from_env() -> Self {
        let format =
            env::var_os("CHIC_LOG_FORMAT").map(|value| value.to_string_lossy().to_string());
        let level = env::var_os("CHIC_LOG_LEVEL").map(|value| value.to_string_lossy().to_string());
        apply_env_overrides(Self::DEFAULT, format.as_deref(), level.as_deref())
    }

    #[must_use]
    pub fn resolved(self) -> Self {
        let format = match self.format {
            LogFormat::Auto => LogFormat::Text,
            other => other,
        };
        Self { format, ..self }
    }
}

impl Default for LogOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Parsed CLI overrides for logging.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogSettings {
    pub format: Option<LogFormat>,
    pub level: Option<LogLevel>,
}

impl LogSettings {
    pub fn is_empty(&self) -> bool {
        self.format.is_none() && self.level.is_none()
    }

    #[must_use]
    pub fn merged_with_env(self) -> LogOptions {
        let env = LogOptions::from_env();
        LogOptions::with_overrides(env, self)
    }

    pub fn apply_format(&mut self, value: LogFormat) {
        self.format = Some(value);
    }

    pub fn apply_level(&mut self, value: LogLevel) {
        self.level = Some(value);
    }
}

fn apply_env_overrides(
    mut options: LogOptions,
    format: Option<&str>,
    level: Option<&str>,
) -> LogOptions {
    if let Some(spec) = format.and_then(LogFormat::parse) {
        options.format = spec;
    }
    if let Some(spec) = level.and_then(LogLevel::parse) {
        options.level = spec;
    }
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_format_and_level_parse_expected_values() {
        assert_eq!(LogFormat::parse("text"), Some(LogFormat::Text));
        assert_eq!(LogFormat::parse("JSON"), Some(LogFormat::Json));
        assert_eq!(LogFormat::parse("auto"), Some(LogFormat::Auto));
        assert_eq!(LogFormat::parse("invalid"), None);

        assert_eq!(LogLevel::parse("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::parse("trace"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::parse("verbose"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::parse("noop"), None);
    }

    #[test]
    fn log_options_from_env_reads_format_and_level() {
        let opts = apply_env_overrides(LogOptions::DEFAULT, Some("json"), Some("debug"));
        assert_eq!(opts.format, LogFormat::Json);
        assert_eq!(opts.level, LogLevel::Debug);
    }

    #[test]
    fn log_settings_merge_overrides_env_defaults() {
        let env_opts = apply_env_overrides(LogOptions::DEFAULT, Some("text"), Some("warn"));
        let merged = LogOptions::with_overrides(
            env_opts,
            LogSettings {
                format: Some(LogFormat::Json),
                level: None,
            },
        );

        assert_eq!(merged.format, LogFormat::Json, "cli format overrides env");
        assert_eq!(merged.level, LogLevel::Warn, "env level is preserved");
    }

    #[test]
    fn resolved_auto_defaults_to_text() {
        let resolved = LogOptions {
            format: LogFormat::Auto,
            level: LogLevel::Info,
        }
        .resolved();
        assert_eq!(resolved.format, LogFormat::Text);
    }
}
