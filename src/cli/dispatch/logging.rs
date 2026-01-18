use std::path::PathBuf;
use std::time::Duration;

use crate::cli::Command;
use crate::codegen::Backend;
use crate::logging::{LogFormat, LogLevel, LogOptions};
use crate::typeck::TraitSolverMetrics;
use crate::{ChicKind, Target};

pub(super) fn resolve_effective_level(options: &LogOptions, trace_requested: bool) -> LogLevel {
    let base = options.level;
    if trace_requested && base < LogLevel::Trace {
        LogLevel::Trace
    } else {
        base
    }
}

pub(super) fn command_requests_trace(command: &Command) -> bool {
    matches!(
        command,
        Command::Check { trace_pipeline, .. }
            | Command::Lint { trace_pipeline, .. }
            | Command::Build { trace_pipeline, .. }
            | Command::Run { trace_pipeline, .. }
            | Command::Test { trace_pipeline, .. }
            | Command::MirDump { trace_pipeline, .. }
            if *trace_pipeline
    )
}

pub(super) fn init_logging(options: &LogOptions, enforced_level: LogLevel) {
    use std::io::IsTerminal;
    use std::sync::OnceLock;
    use tracing_subscriber::{EnvFilter, fmt};

    static INITIALISED: OnceLock<()> = OnceLock::new();

    let _ = INITIALISED.get_or_init(|| {
        let use_ansi = std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal();
        let level = enforced_level.as_tracing_level();
        let make_filter = || {
            let directive = enforced_level.to_string();
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directive))
        };

        match options.format {
            LogFormat::Json => {
                let subscriber = fmt::fmt()
                    .with_env_filter(make_filter())
                    .with_max_level(level)
                    .with_ansi(use_ansi)
                    .with_writer(std::io::stderr)
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .json()
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);
            }
            _ => {
                let subscriber = fmt::fmt()
                    .with_env_filter(make_filter())
                    .with_max_level(level)
                    .with_ansi(use_ansi)
                    .with_writer(std::io::stderr)
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .compact()
                    .finish();
                let _ = tracing::subscriber::set_global_default(subscriber);
            }
        }
    });
}

pub(super) fn log_run_start(command: &Command, options: &LogOptions, trace_requested: bool) {
    if let Some(metadata) = command_metadata(command) {
        let inputs = metadata.inputs_summary();
        tracing::info!(
            target: "pipeline",
            stage = "cli.run.header",
            command = metadata.command(),
            log_level = %options.level,
            log_format = %options.format,
            trace_pipeline = trace_requested,
            target = metadata.target(),
            backend = metadata.backend(),
            kind = metadata.kind(),
            input_count = metadata.input_count(),
            inputs = %inputs
        );
        tracing::info!(
            target: "pipeline",
            stage = "cli.run.start",
            command = metadata.command(),
            status = "start",
            target = metadata.target(),
            backend = metadata.backend(),
            kind = metadata.kind(),
            input_count = metadata.input_count(),
            inputs = %inputs
        );
    } else {
        tracing::info!(
            target: "pipeline",
            stage = "cli.run.start",
            command = command_name(command),
            log_level = %options.level,
            log_format = %options.format,
            trace_pipeline = trace_requested,
        );
    }
}

pub(super) fn log_run_complete(
    command: &Command,
    elapsed: Duration,
    result: &crate::error::Result<()>,
) {
    let elapsed_ms = elapsed.as_millis() as u64;
    match result {
        Ok(_) => {
            if let Some(metadata) = command_metadata(command) {
                let inputs = metadata.inputs_summary();
                tracing::info!(
                    target: "pipeline",
                    stage = "cli.run.footer",
                    command = metadata.command(),
                    status = "ok",
                    target = metadata.target(),
                    backend = metadata.backend(),
                    kind = metadata.kind(),
                    input_count = metadata.input_count(),
                    inputs = %inputs,
                    elapsed_ms
                );
            } else {
                tracing::info!(
                    target: "pipeline",
                    stage = "cli.run.footer",
                    command = command_name(command),
                    status = "ok",
                    elapsed_ms
                );
            }
        }
        Err(err) => {
            if let Some(metadata) = command_metadata(command) {
                let inputs = metadata.inputs_summary();
                tracing::error!(
                    target: "pipeline",
                    stage = "cli.run.footer",
                    command = metadata.command(),
                    status = "error",
                    target = metadata.target(),
                    backend = metadata.backend(),
                    kind = metadata.kind(),
                    input_count = metadata.input_count(),
                    inputs = %inputs,
                    elapsed_ms,
                    error = %err
                );
            } else {
                tracing::error!(
                    target: "pipeline",
                    stage = "cli.run.footer",
                    command = command_name(command),
                    status = "error",
                    elapsed_ms,
                    error = %err
                );
            }
        }
    }
}

pub(super) fn format_input_list(inputs: &[PathBuf]) -> String {
    match inputs.len() {
        0 => "<none>".into(),
        1 => inputs[0].display().to_string(),
        _ => inputs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

pub(super) fn print_trait_solver_metrics(command: &str, metrics: &TraitSolverMetrics) {
    println!(
        "[{command}] trait solver: traits={}, impls={}, overlaps={}, cycles={}, elapsed={}µs",
        metrics.traits_checked,
        metrics.impls_checked,
        metrics.overlaps_detected,
        metrics.cycles_detected,
        metrics.elapsed.as_micros()
    );
}

fn command_name(command: &Command) -> &'static str {
    match command {
        Command::Check { .. } => "check",
        Command::Lint { .. } => "lint",
        Command::Build { .. } => "build",
        Command::Clean { .. } => "clean",
        Command::Run { .. } => "run",
        Command::Test { .. } => "test",
        Command::Init { .. } => "init",
        Command::Doc { .. } => "doc",
        Command::Format { .. } => "format",
        Command::MirDump { .. } => "mir-dump",
        Command::ShowSpec => "spec",
        Command::Header { .. } => "header",
        Command::Help { .. } => "help",
        Command::Version => "version",
        Command::Cc1 { .. } => "cc1",
        Command::ExternBind { .. } => "extern-bind",
        Command::PerfReport { .. } => "perf-report",
        Command::Seed { .. } => "seed",
    }
}

fn stringify_inputs(inputs: &[PathBuf]) -> Vec<String> {
    inputs
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

struct CommandLogMetadata {
    command: &'static str,
    inputs: Vec<String>,
    target: Option<String>,
    backend: Option<String>,
    kind: Option<String>,
}

impl CommandLogMetadata {
    fn command(&self) -> &'static str {
        self.command
    }

    fn input_count(&self) -> usize {
        self.inputs.len()
    }

    fn inputs_summary(&self) -> String {
        if self.inputs.is_empty() {
            "<none>".into()
        } else {
            self.inputs.join(", ")
        }
    }

    fn target(&self) -> &str {
        self.target.as_deref().unwrap_or("n/a")
    }

    fn backend(&self) -> &str {
        self.backend.as_deref().unwrap_or("n/a")
    }

    fn kind(&self) -> &str {
        self.kind.as_deref().unwrap_or("n/a")
    }
}

fn command_metadata(command: &Command) -> Option<CommandLogMetadata> {
    match command {
        Command::Check {
            inputs,
            target,
            kind,
            ..
        } => Some(CommandLogMetadata {
            command: "check",
            inputs: stringify_inputs(inputs),
            target: Some(target.triple().to_string()),
            backend: None,
            kind: Some(kind.as_str().to_string()),
        }),
        Command::Lint {
            inputs,
            target,
            kind,
            ..
        } => Some(CommandLogMetadata {
            command: "lint",
            inputs: stringify_inputs(inputs),
            target: Some(target.triple().to_string()),
            backend: None,
            kind: Some(kind.as_str().to_string()),
        }),
        Command::Doc { manifest, .. } => Some(CommandLogMetadata {
            command: "doc",
            inputs: stringify_inputs(&manifest.clone().into_iter().collect::<Vec<_>>()),
            target: None,
            backend: None,
            kind: None,
        }),
        Command::Init { output, .. } => Some(CommandLogMetadata {
            command: "init",
            inputs: vec![
                output
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .display()
                    .to_string(),
            ],
            target: None,
            backend: None,
            kind: None,
        }),
        Command::Build {
            inputs,
            target,
            kind,
            backend,
            ..
        } => Some(CommandLogMetadata {
            command: "build",
            inputs: stringify_inputs(inputs),
            target: Some(target.triple().to_string()),
            backend: Some(backend.as_str().to_string()),
            kind: Some(kind.as_str().to_string()),
        }),
        Command::Clean { input, .. } => Some(CommandLogMetadata {
            command: "clean",
            inputs: vec![
                input
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .display()
                    .to_string(),
            ],
            target: None,
            backend: None,
            kind: None,
        }),
        Command::Run {
            inputs,
            target,
            kind,
            backend,
            ..
        } => Some(CommandLogMetadata {
            command: "run",
            inputs: stringify_inputs(inputs),
            target: Some(target.triple().to_string()),
            backend: Some(backend.as_str().to_string()),
            kind: Some(kind.as_str().to_string()),
        }),
        Command::Test {
            inputs,
            target,
            kind,
            backend,
            ..
        } => Some(CommandLogMetadata {
            command: "test",
            inputs: stringify_inputs(inputs),
            target: Some(target.triple().to_string()),
            backend: Some(backend.as_str().to_string()),
            kind: Some(kind.as_str().to_string()),
        }),
        Command::Format { inputs, .. } => Some(CommandLogMetadata {
            command: "format",
            inputs: stringify_inputs(inputs),
            target: None,
            backend: None,
            kind: None,
        }),
        Command::MirDump { input, .. } => Some(CommandLogMetadata {
            command: "mir-dump",
            inputs: vec![input.display().to_string()],
            target: None,
            backend: None,
            kind: None,
        }),
        Command::Header { input, .. } => Some(CommandLogMetadata {
            command: "header",
            inputs: vec![input.display().to_string()],
            target: Some(Target::host().triple().to_string()),
            backend: None,
            kind: Some(ChicKind::StaticLibrary.as_str().to_string()),
        }),
        Command::Cc1 { input, target, .. } => Some(CommandLogMetadata {
            command: "cc1",
            inputs: vec![input.display().to_string()],
            target: Some(target.triple().to_string()),
            backend: Some(Backend::Cc1.as_str().to_string()),
            kind: None,
        }),
        Command::ExternBind { header, .. } => Some(CommandLogMetadata {
            command: "extern-bind",
            inputs: vec![header.display().to_string()],
            target: Some(Target::host().triple().to_string()),
            backend: None,
            kind: Some(ChicKind::StaticLibrary.as_str().to_string()),
        }),
        Command::PerfReport { perf_path, .. } => Some(CommandLogMetadata {
            command: "perf-report",
            inputs: vec![perf_path.display().to_string()],
            target: None,
            backend: None,
            kind: None,
        }),
        Command::Seed { run_path, .. } => Some(CommandLogMetadata {
            command: "seed",
            inputs: vec![run_path.display().to_string()],
            target: None,
            backend: None,
            kind: None,
        }),
        Command::ShowSpec | Command::Help { .. } | Command::Version => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Command;
    use crate::defines::DefineFlag;
    use crate::{ChicKind, Target};
    use std::path::PathBuf;

    #[test]
    fn effective_level_promotes_trace_when_requested() {
        let base = LogOptions {
            format: LogFormat::Text,
            level: LogLevel::Info,
        };
        assert_eq!(
            resolve_effective_level(&base, true),
            LogLevel::Trace,
            "trace flag should elevate the level"
        );
        assert_eq!(
            resolve_effective_level(&base, false),
            LogLevel::Info,
            "no elevation when trace is false"
        );
    }

    #[test]
    fn command_requests_trace_detects_flag_on_build() {
        let cmd = Command::Check {
            inputs: vec!["main.cl".into()],
            target: Target::host(),
            kind: ChicKind::Executable,
            const_eval_fuel: None,
            trace_pipeline: true,
            trait_solver_metrics: false,
            defines: vec![DefineFlag::new("DEBUG", None)],
        };
        assert!(
            command_requests_trace(&cmd),
            "trace flag on build command should be recognised"
        );
    }

    #[test]
    fn format_input_list_renders_expected_summaries() {
        assert_eq!(format_input_list(&[]), "<none>");
        assert_eq!(format_input_list(&[PathBuf::from("one")]), "one");
        assert_eq!(
            format_input_list(&[PathBuf::from("one"), PathBuf::from("two")]),
            "one, two"
        );
    }
}
