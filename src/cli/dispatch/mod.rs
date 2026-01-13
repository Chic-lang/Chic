use std::path::PathBuf;
use std::time::Instant;

use crate::cli::Cli;
use crate::cli::commands::common::parse_error_format;
use crate::defines::DefineFlag;
use crate::diagnostics::{ColorMode, ErrorFormat, FormatOptions};
#[cfg(not(test))]
use crate::driver::CompilerDriver;
use crate::driver::{
    BuildRequest, FormatResult, FrontendReport, MirDumpResult, RunResult, TestOptions, TestRun,
};
use crate::error::{Error, Result};
use crate::format::FormatConfig;
use crate::logging::LogLevel;
use crate::spec::Spec;
use crate::{ChicKind, Target};
use std::io::IsTerminal;

mod commands;
mod ffi;
mod logging;
mod reporting;
#[cfg(test)]
mod tests;

pub trait DispatchDriver {
    fn spec(&self) -> &Spec;
    fn should_load_stdlib(&self, inputs: &[PathBuf]) -> bool;
    fn check(
        &self,
        inputs: &[PathBuf],
        target: &Target,
        kind: ChicKind,
        load_stdlib: bool,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: &[DefineFlag],
        log_level: LogLevel,
    ) -> Result<FrontendReport>;
    fn build(&self, request: BuildRequest) -> Result<FrontendReport>;
    fn run(&self, request: BuildRequest) -> Result<RunResult>;
    fn run_tests(&self, request: BuildRequest, test_options: TestOptions) -> Result<TestRun>;
    fn format(&self, input: &std::path::Path, config: &FormatConfig) -> Result<FormatResult>;
    fn mir_dump(
        &self,
        input: &std::path::Path,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        log_level: LogLevel,
    ) -> Result<MirDumpResult>;
}

#[cfg(not(test))]
impl DispatchDriver for CompilerDriver {
    fn spec(&self) -> &Spec {
        self.spec()
    }

    fn should_load_stdlib(&self, inputs: &[PathBuf]) -> bool {
        CompilerDriver::should_load_stdlib(inputs)
    }

    fn check(
        &self,
        inputs: &[PathBuf],
        target: &Target,
        kind: ChicKind,
        load_stdlib: bool,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: &[DefineFlag],
        log_level: LogLevel,
    ) -> Result<FrontendReport> {
        self.check(
            inputs,
            target,
            kind,
            load_stdlib,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            log_level,
        )
    }

    fn build(&self, request: BuildRequest) -> Result<FrontendReport> {
        self.build(request)
    }

    fn run(&self, request: BuildRequest) -> Result<RunResult> {
        CompilerDriver::run(self, request)
    }

    fn run_tests(&self, request: BuildRequest, test_options: TestOptions) -> Result<TestRun> {
        CompilerDriver::run_tests(self, request, test_options)
    }

    fn format(&self, input: &std::path::Path, config: &FormatConfig) -> Result<FormatResult> {
        self.format(input, config)
    }

    fn mir_dump(
        &self,
        input: &std::path::Path,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        log_level: LogLevel,
    ) -> Result<MirDumpResult> {
        self.mir_dump(input, trace_pipeline, trait_solver_metrics, log_level)
    }
}

/// Execute a parsed CLI command using the provided driver. Logging and diagnostics
/// reporting are configured here so the binary entrypoint can stay thin.
pub fn run<D: DispatchDriver>(driver: &D, cli: Cli) -> Result<()> {
    let log_options = cli.log_options.resolved();
    let command_for_logging = cli.command.clone();
    let trace_requested = logging::command_requests_trace(&command_for_logging);
    let effective_level = logging::resolve_effective_level(&log_options, trace_requested);
    logging::init_logging(&log_options, effective_level);
    let is_terminal = std::io::stderr().is_terminal();
    let env_error_format = std::env::var("CHIC_ERROR_FORMAT")
        .ok()
        .and_then(|value| parse_error_format(&value).ok());
    let default_format = env_error_format.unwrap_or_else(|| {
        if is_terminal {
            ErrorFormat::Human
        } else {
            ErrorFormat::Short
        }
    });
    let color_choice = if std::env::var_os("NO_COLOR").is_some() {
        ColorMode::Never
    } else {
        ColorMode::Auto
    };
    let format_options = FormatOptions {
        format: cli.error_format.unwrap_or(default_format),
        color: color_choice,
        is_terminal,
    };
    let start = Instant::now();
    logging::log_run_start(&command_for_logging, &log_options, trace_requested);
    let result = commands::dispatch_command(driver, cli.command, effective_level, format_options);
    logging::log_run_complete(&command_for_logging, start.elapsed(), &result);
    result
}

pub fn report_error(err: &Error) {
    reporting::report_error(err);
}
