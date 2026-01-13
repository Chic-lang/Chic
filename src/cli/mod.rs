//! CLI front-end: command parsing, registry, and dispatch helpers used by the `chic` binary.

mod commands;
mod help;
pub mod templates;

use std::env;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

use crate::chic_kind::ChicKind;
use crate::codegen::{Backend, CpuIsaConfig};
use crate::defines::DefineFlag;
use crate::diagnostics::ErrorFormat;
use crate::driver::types::{BuildPropertyOverride, TelemetrySetting, Verbosity};
use crate::logging::LogOptions;
use crate::manifest::{Manifest, MissingDocsRule, WorkspaceConfig};
use crate::target::Target;
use commands::common::is_help_flag;

pub mod dispatch;

pub(crate) type CommandParser = fn(Vec<String>) -> Result<Cli, CliError>;

/// Feature toggles for optional commands. Commands that require non-default tooling live
/// behind a feature flag so downstream builds can opt out cleanly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandFeature {
    /// Use the cc1-compatible preprocessing/assembly command.
    Cc1,
    /// Generate extern bindings via the FFI binder.
    ExternBind,
}

#[derive(Clone, Copy)]
pub(crate) struct CommandDescriptor {
    name: &'static str,
    aliases: &'static [&'static str],
    parser: CommandParser,
    feature: Option<CommandFeature>,
}

impl CommandDescriptor {
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }

    pub(crate) fn parse(&self, args: Vec<String>) -> Result<Cli, CliError> {
        (self.parser)(args)
    }

    fn matches(&self, name: &str) -> bool {
        self.name() == name || self.aliases().iter().any(|alias| *alias == name)
    }

    fn is_enabled(&self, feature_check: fn(CommandFeature) -> bool) -> bool {
        match self.feature {
            Some(feature) => feature_check(feature),
            None => true,
        }
    }
}

pub(crate) struct CommandRegistry {
    entries: &'static [CommandDescriptor],
    feature_check: fn(CommandFeature) -> bool,
}

impl CommandRegistry {
    pub(crate) fn new(
        entries: &'static [CommandDescriptor],
        feature_check: fn(CommandFeature) -> bool,
    ) -> Self {
        Self {
            entries,
            feature_check,
        }
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<&'static CommandDescriptor> {
        self.iter().find(|descriptor| {
            descriptor.matches(name) && descriptor.is_enabled(self.feature_check)
        })
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &'static CommandDescriptor> {
        self.entries
            .iter()
            .filter(move |descriptor| descriptor.is_enabled(self.feature_check))
    }
}

pub(crate) fn registry() -> CommandRegistry {
    CommandRegistry::new(commands::descriptors(), build_time_feature_enabled)
}

fn build_time_feature_enabled(feature: CommandFeature) -> bool {
    if std::env::var_os("CHIC_DISABLE_CLI_FEATURES").is_some() {
        return false;
    }
    match feature {
        CommandFeature::Cc1 => cfg!(feature = "cli-cc1"),
        CommandFeature::ExternBind => cfg!(feature = "cli-extern-bind"),
    }
}

#[derive(Debug, Clone)]
pub struct CliFfiOptions {
    pub search_paths: Vec<PathBuf>,
    pub default_patterns: Vec<FfiDefaultPattern>,
    pub package_globs: Vec<String>,
}

impl Default for CliFfiOptions {
    fn default() -> Self {
        Self {
            search_paths: Vec::new(),
            default_patterns: Vec::new(),
            package_globs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileOptions {
    pub output: PathBuf,
    pub sample_ms: Option<u64>,
    pub flamegraph: bool,
}

impl Default for ProfileOptions {
    fn default() -> Self {
        Self {
            output: PathBuf::from("profiling/latest/perf.json"),
            sample_ms: Some(1),
            flamegraph: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FfiDefaultPattern {
    pub target: String,
    pub pattern: String,
}

/// Top-level commands supported by the `chic` CLI.
#[derive(Debug, Clone)]
pub enum Command {
    Check {
        inputs: Vec<PathBuf>,
        target: Target,
        kind: ChicKind,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: Vec<DefineFlag>,
    },
    Lint {
        inputs: Vec<PathBuf>,
        target: Target,
        kind: ChicKind,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: Vec<DefineFlag>,
    },
    Build {
        inputs: Vec<PathBuf>,
        manifest: Option<Manifest>,
        workspace: Option<WorkspaceConfig>,
        output: Option<PathBuf>,
        target: Target,
        kind: ChicKind,
        backend: Backend,
        emit_wat: bool,
        emit_obj: bool,
        cpu_isa: CpuIsaConfig,
        emit_header: bool,
        emit_lib: bool,
        runtime_backend: crate::runtime::backend::RuntimeBackend,
        cc1_args: Vec<String>,
        cc1_keep_temps: bool,
        load_stdlib: Option<bool>,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: Vec<DefineFlag>,
        ffi: CliFfiOptions,
        configuration: String,
        framework: Option<String>,
        artifacts_path: Option<PathBuf>,
        no_dependencies: bool,
        no_restore: bool,
        no_incremental: bool,
        disable_build_servers: bool,
        source_root: Option<PathBuf>,
        properties: Vec<BuildPropertyOverride>,
        verbosity: Verbosity,
        telemetry: TelemetrySetting,
        version_suffix: Option<String>,
        nologo: bool,
        force: bool,
        interactive: bool,
        self_contained: Option<bool>,
        doc_markdown: bool,
        manifest_path: Option<std::path::PathBuf>,
        doc_enforcement: MissingDocsRule,
    },
    Run {
        inputs: Vec<PathBuf>,
        manifest: Option<Manifest>,
        workspace: Option<WorkspaceConfig>,
        target: Target,
        kind: ChicKind,
        backend: Backend,
        cpu_isa: CpuIsaConfig,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        runtime_backend: crate::runtime::backend::RuntimeBackend,
        load_stdlib: Option<bool>,
        defines: Vec<DefineFlag>,
        ffi: CliFfiOptions,
        profile: Option<ProfileOptions>,
        configuration: String,
        artifacts_path: Option<PathBuf>,
        no_dependencies: bool,
        no_restore: bool,
        no_incremental: bool,
        disable_build_servers: bool,
        source_root: Option<PathBuf>,
        properties: Vec<BuildPropertyOverride>,
        verbosity: Verbosity,
        telemetry: TelemetrySetting,
        version_suffix: Option<String>,
        nologo: bool,
        force: bool,
        interactive: bool,
        self_contained: Option<bool>,
        framework: Option<String>,
        doc_enforcement: MissingDocsRule,
    },
    Test {
        inputs: Vec<PathBuf>,
        manifest: Option<Manifest>,
        workspace: Option<WorkspaceConfig>,
        target: Target,
        kind: ChicKind,
        backend: Backend,
        cpu_isa: CpuIsaConfig,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        runtime_backend: crate::runtime::backend::RuntimeBackend,
        load_stdlib: Option<bool>,
        defines: Vec<DefineFlag>,
        ffi: CliFfiOptions,
        profile: Option<ProfileOptions>,
        test_options: crate::driver::types::TestOptions,
        coverage: bool,
        coverage_min: Option<u8>,
        workspace_mode: bool,
        coverage_only: bool,
        configuration: String,
        artifacts_path: Option<PathBuf>,
        no_dependencies: bool,
        no_restore: bool,
        no_incremental: bool,
        disable_build_servers: bool,
        source_root: Option<PathBuf>,
        properties: Vec<BuildPropertyOverride>,
        verbosity: Verbosity,
        telemetry: TelemetrySetting,
        version_suffix: Option<String>,
        nologo: bool,
        force: bool,
        interactive: bool,
        self_contained: Option<bool>,
        framework: Option<String>,
        doc_enforcement: MissingDocsRule,
    },
    Init {
        template: String,
        output: Option<PathBuf>,
        name: Option<String>,
    },
    Format {
        inputs: Vec<PathBuf>,
        config: Option<PathBuf>,
        check: bool,
        diff: bool,
        write: bool,
        stdin: bool,
        stdout: bool,
    },
    MirDump {
        input: PathBuf,
        const_eval_fuel: Option<usize>,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
    },
    ShowSpec,
    Header {
        input: PathBuf,
        output: Option<PathBuf>,
        include_guard: Option<String>,
    },
    Help {
        topic: Option<String>,
    },
    Version,
    Cc1 {
        input: PathBuf,
        output: Option<PathBuf>,
        target: Target,
        extra_args: Vec<String>,
    },
    ExternBind {
        header: PathBuf,
        output: PathBuf,
        namespace: String,
        library: String,
        binding: String,
        convention: String,
        optional: bool,
    },
    PerfReport {
        perf_path: PathBuf,
        baseline: Option<PathBuf>,
        profile: Option<String>,
        json: bool,
        strict: bool,
        tolerance: f64,
    },
    Seed {
        run_path: PathBuf,
        profile: Option<String>,
        json: bool,
    },
    Doc {
        manifest: Option<PathBuf>,
        output: Option<PathBuf>,
        scope: Option<String>,
        template: Option<PathBuf>,
        front_matter: Option<PathBuf>,
        tag_handlers: Vec<String>,
        link_resolver: Option<String>,
        layout: Option<String>,
        banner: Option<bool>,
    },
    Clean {
        input: Option<PathBuf>,
        artifacts_path: Option<PathBuf>,
        configuration: String,
        all: bool,
        dry_run: bool,
    },
}

/// Parsed CLI invocation.
#[derive(Debug, Clone)]
pub struct Cli {
    pub command: Command,
    pub log_options: LogOptions,
    pub error_format: Option<ErrorFormat>,
}

/// Error emitted while parsing command-line arguments.
#[derive(Debug, Clone)]
pub struct CliError {
    message: String,
}

impl CliError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn with_usage(message: impl Into<String>) -> Self {
        let mut owned = message.into();
        owned.push_str("\n\n");
        let usage = Cli::usage();
        owned.push_str(&usage);
        Self::new(owned)
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl StdError for CliError {}

impl Cli {
    /// Parse arguments from the environment.
    ///
    /// # Errors
    /// Returns a [`CliError`] when the arguments cannot be interpreted as a supported command.
    pub fn parse() -> Result<Self, CliError> {
        Self::parse_from(env::args().skip(1))
    }

    /// Parse arguments from an iterator (useful for testing).
    ///
    /// # Errors
    /// Returns a [`CliError`] when the provided iterator does not describe a valid invocation.
    #[allow(clippy::too_many_lines)]
    pub fn parse_from<I, T>(args: I) -> Result<Self, CliError>
    where
        I: Iterator<Item = T>,
        T: Into<String>,
    {
        let mut iter = args.into_iter().map(Into::into).peekable();
        let mut global_prefix = Vec::new();
        while let Some(flag) = iter.peek().cloned() {
            if !flag.starts_with('-') || flag == "--" {
                break;
            }
            if is_help_flag(&flag) || matches!(flag.as_str(), "--version" | "-V") {
                break;
            }
            if let Some(consumed) = consume_global_option(&mut iter)? {
                global_prefix.extend(consumed);
                continue;
            }
            return Err(CliError::with_usage(format!(
                "unsupported global option '{flag}'"
            )));
        }

        let Some(raw_command) = iter.next() else {
            return Err(CliError::with_usage("missing command"));
        };

        match raw_command.as_str() {
            "--help" | "-h" => {
                let topic = iter.next();
                let topic = topic.and_then(|value| {
                    if is_help_flag(&value) {
                        None
                    } else {
                        Some(value.to_ascii_lowercase())
                    }
                });
                return Ok(Cli {
                    command: Command::Help { topic },
                    log_options: LogOptions::from_env(),
                    error_format: None,
                });
            }
            "help" => {
                let remaining: Vec<String> = iter.collect();
                let topic = remaining
                    .first()
                    .filter(|value| !is_help_flag(value))
                    .cloned()
                    .map(|value| value.to_ascii_lowercase());
                return Ok(Cli {
                    command: Command::Help { topic },
                    log_options: LogOptions::from_env(),
                    error_format: None,
                });
            }
            "--version" | "-V" => {
                if let Some(arg) = iter.next() {
                    let flag: String = arg.into();
                    if is_help_flag(&flag) {
                        return Ok(Cli {
                            command: Command::Help {
                                topic: Some("version".into()),
                            },
                            log_options: LogOptions::from_env(),
                            error_format: None,
                        });
                    }
                    return Err(CliError::with_usage(format!(
                        "unsupported option '{flag}' for command"
                    )));
                }
                return Ok(Cli {
                    command: Command::Version,
                    log_options: LogOptions::from_env(),
                    error_format: None,
                });
            }
            "version" => {
                let remaining: Vec<String> = iter.collect();
                if remaining.iter().any(|value| is_help_flag(value)) {
                    return Ok(Cli {
                        command: Command::Help {
                            topic: Some("version".into()),
                        },
                        log_options: LogOptions::from_env(),
                        error_format: None,
                    });
                }
                if !remaining.is_empty() {
                    return Err(CliError::with_usage(
                        "chic version does not accept additional arguments",
                    ));
                }
                return Ok(Cli {
                    command: Command::Version,
                    log_options: LogOptions::from_env(),
                    error_format: None,
                });
            }
            _ => {}
        }

        let mut remaining: Vec<String> = iter.collect();
        if !global_prefix.is_empty() {
            let mut merged = global_prefix;
            merged.append(&mut remaining);
            remaining = merged;
        }
        if let Some(descriptor) = registry().resolve(&raw_command) {
            return descriptor.parse(remaining);
        }

        Err(CliError::with_usage(format!(
            "unknown command '{raw_command}'"
        )))
    }

    /// Return formatted general help text.
    #[must_use]
    pub fn usage() -> String {
        help::render_general_help()
    }

    /// Return help text for a specific command.
    ///
    /// # Errors
    /// Returns a [`CliError`] when the requested topic is unknown.
    pub fn help_for(topic: &str) -> Result<String, CliError> {
        help::render_command_help(topic)
            .ok_or_else(|| CliError::with_usage(help::format_unknown_topic(topic)))
    }
}

fn consume_global_option<I>(
    iter: &mut std::iter::Peekable<I>,
) -> Result<Option<Vec<String>>, CliError>
where
    I: Iterator<Item = String>,
{
    let Some(flag) = iter.peek().cloned() else {
        return Ok(None);
    };
    match flag.as_str() {
        "-c" | "--configuration" | "-f" | "--framework" | "-r" | "--runtime" | "-v"
        | "--verbosity" => {
            iter.next();
            let value = iter
                .next()
                .ok_or_else(|| CliError::with_usage(format!("expected value after {flag}")))?;
            Ok(Some(vec![flag, value]))
        }
        "-p" | "--property" => {
            iter.next();
            let value = iter.next().ok_or_else(|| {
                CliError::with_usage("expected <name>=<value> after -p/--property")
            })?;
            Ok(Some(vec![flag, value]))
        }
        _ if flag.starts_with("--property:") || flag.starts_with("-p:") => {
            iter.next();
            Ok(Some(vec![flag]))
        }
        _ => Ok(None),
    }
}

// Command-specific parsers and helpers live in `src/cli/commands`.

#[cfg(test)]
mod tests;
