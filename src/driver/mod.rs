use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use self::defines::resolve_conditional_defines;
use crate::chic_kind::ChicKind;
use crate::codegen::Backend;
use crate::defines::DefineFlag;
use crate::error::Result;
use crate::logging::LogLevel;
use crate::mir::{FunctionKind, TestCaseMetadata, format_module};
use crate::runtime::{WasmProgram, execute_wasm_with_options, hooks};
use crate::runtime_package::{ResolvedRuntime, RuntimeKind, resolve_runtime};
use crate::spec::Spec;
use crate::target::Target;
mod build;
mod defines;
pub mod graph_registry;
mod incremental;
mod pipeline;
pub mod profile_loader;
mod report;
pub mod types;
mod wasm;
use self::pipeline::CompilerPipelineBuilder;
use self::report::ModuleArtifact;
pub use self::report::{
    FrontendReport, GeneratedModuleIr, MirDumpResult, MirVerificationIssue, ModuleReport,
};
pub use self::types::{
    BuildFfiOptions, BuildRequest, FormatResult, RunResult, TestCaseResult, TestOptions, TestRun,
    TestSelection, TestStatus, WatchdogConfig, resolve_trace_enabled, trait_solver_metrics_enabled,
};

pub(super) fn summarize_inputs(inputs: &[PathBuf]) -> String {
    if inputs.is_empty() {
        "<none>".into()
    } else if inputs.len() == 1 {
        inputs[0].display().to_string()
    } else {
        inputs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn collect_library_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_library_files(&path, files)?;
        } else if path.extension().map(|ext| ext == "cl").unwrap_or(false) {
            let relative = path
                .strip_prefix(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
                .unwrap_or(&path)
                .to_path_buf();
            files.push(relative);
        }
    }
    Ok(())
}

fn stdlib_filters() -> (Vec<String>, Vec<String>) {
    let allowlist: Vec<String> = std::env::var("CHIC_STDLIB_ALLOWLIST")
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| entry.to_string())
                .collect()
        })
        .unwrap_or_default();
    let blocklist: Vec<String> = std::env::var("CHIC_STDLIB_BLOCKLIST")
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| entry.to_string())
                .collect()
        })
        .unwrap_or_default();
    (allowlist, blocklist)
}

fn apply_stdlib_filters(files: &mut Vec<PathBuf>) {
    let (allowlist, blocklist) = stdlib_filters();
    let debug = std::env::var("CHIC_DEBUG_STDLIB_FILTERS").is_ok();
    let original_len = files.len();
    if !allowlist.is_empty() {
        if debug {
            eprintln!(
                "[chic-debug stdlib-filter] applying allowlist ({} entries) to {} files",
                allowlist.len(),
                original_len
            );
        }
        files.retain(|path| {
            let path_str = path.to_string_lossy();
            allowlist.iter().any(|needle| path_str.contains(needle))
        });
    }
    if !blocklist.is_empty() {
        if debug {
            eprintln!(
                "[chic-debug stdlib-filter] applying blocklist ({} entries) after allowlist ({} -> {})",
                blocklist.len(),
                original_len,
                files.len()
            );
        }
        files.retain(|path| {
            let path_str = path.to_string_lossy();
            !blocklist.iter().any(|needle| path_str.contains(needle))
        });
    }
    if debug {
        eprintln!(
            "[chic-debug stdlib-filter] result: {} -> {} entries",
            original_len,
            files.len()
        );
    }
}

pub(super) fn collect_core_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let core_dir = manifest_dir.join("packages").join("std.core").join("src");
    collect_library_files(&core_dir, &mut files)?;
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_stdlib_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stdlib_roots = [
        manifest_dir
            .join("packages")
            .join("std.compiler.object")
            .join("src"),
        manifest_dir.join("packages").join("std").join("src"),
        manifest_dir.join("packages").join("std.io").join("src"),
        manifest_dir.join("packages").join("std.async").join("src"),
        manifest_dir
            .join("packages")
            .join("std.testing")
            .join("src"),
        manifest_dir
            .join("packages")
            .join("std.runtime")
            .join("src"),
        manifest_dir
            .join("packages")
            .join("std.platform")
            .join("src"),
        manifest_dir.join("packages").join("std.net").join("src"),
        manifest_dir
            .join("packages")
            .join("std.security")
            .join("src"),
        manifest_dir.join("packages").join("std.text").join("src"),
        manifest_dir.join("packages").join("std.data").join("src"),
        manifest_dir
            .join("packages")
            .join("std.compression")
            .join("src"),
    ];
    for root in stdlib_roots {
        if root.exists() {
            collect_library_files(&root, &mut files)?;
        }
    }
    apply_stdlib_filters(&mut files);
    files.retain(|path| {
        if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
            !file_name.eq_ignore_ascii_case("bootstrap_native_main.cl")
        } else {
            true
        }
    });
    files.sort();
    files.dedup();
    Ok(files)
}

pub(super) fn collect_foundation_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let foundation_dir = manifest_dir
        .join("packages")
        .join("std.foundation")
        .join("src");
    collect_library_files(&foundation_dir, &mut files)?;
    apply_stdlib_filters(&mut files);
    files.sort();
    files.dedup();
    Ok(files)
}

pub(super) fn collect_alloc_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let alloc_dir = manifest_dir.join("packages").join("std.alloc").join("src");
    collect_library_files(&alloc_dir, &mut files)?;
    files.sort();
    files.dedup();
    Ok(files)
}

pub(super) fn collect_runtime_package_files(runtime: &ResolvedRuntime) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for root in runtime.manifest.derived_source_roots() {
        let path = runtime.root.join(&root.path);
        collect_library_files(&path, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn stdlib_files_for(_kind: ChicKind, _backend: Backend) -> Result<Vec<PathBuf>> {
    let files = collect_stdlib_files()?;
    Ok(files)
}

const DRIVER_STACK_DEFAULT: usize = 32 * 1024 * 1024;

fn driver_stack_size() -> Option<usize> {
    match std::env::var("CHIC_DRIVER_STACK_SIZE") {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed == "0" {
                return None;
            }
            trimmed.parse().ok().filter(|value: &usize| *value > 0)
        }
        Err(_) => Some(DRIVER_STACK_DEFAULT),
    }
}

fn run_with_stack<T, F>(stack_bytes: Option<usize>, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    if let Some(bytes) = stack_bytes {
        let handle = thread::Builder::new()
            .name("chic-driver-worker".into())
            .stack_size(bytes)
            .spawn(f)
            .map_err(|err| {
                crate::error::Error::internal(format!("failed to spawn worker thread: {err}"))
            })?;
        match handle.join() {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    } else {
        f()
    }
}

/// Coordinates high-level compiler actions.
#[derive(Clone)]
pub struct CompilerDriver {
    spec: Spec,
}

impl CompilerDriver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: Spec::current(),
        }
    }

    pub fn should_load_stdlib(inputs: &[PathBuf]) -> bool {
        let _ = inputs;
        let force_stdlib = std::env::var("CHIC_FORCE_STDLIB")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let skip_stdlib = std::env::var("CHIC_SKIP_STDLIB")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if force_stdlib {
            return true;
        }
        if skip_stdlib {
            return false;
        }
        true
    }

    #[must_use]
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// Run parsing, lowering, type checking, and verification without emitting code.
    ///
    /// # Errors
    ///
    /// Returns any I/O failures when reading the source file, parse/lower/type-check diagnostics,
    /// or backend verification errors encountered during MIR analysis.
    pub fn check(
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
        let driver = self.clone();
        let inputs = inputs.to_vec();
        let target = target.clone();
        let defines = defines.to_vec();
        run_with_stack(driver_stack_size(), move || {
            driver.check_inner(
                inputs,
                target,
                kind,
                load_stdlib,
                trace_pipeline,
                trait_solver_metrics,
                defines,
                log_level,
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn check_inner(
        &self,
        inputs: Vec<PathBuf>,
        target: Target,
        kind: ChicKind,
        load_stdlib: bool,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        defines: Vec<DefineFlag>,
        log_level: LogLevel,
    ) -> Result<FrontendReport> {
        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let solver_metrics_enabled =
            trait_solver_metrics_enabled(trace_enabled, trait_solver_metrics);
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let runtime_kind = match target.runtime() {
            crate::target::TargetRuntime::NativeNoStd => RuntimeKind::NoStd,
            _ => RuntimeKind::Native,
        };
        let runtime_resolution = resolve_runtime(None, runtime_kind, &project_root)?;
        let check_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target_str = target.triple().to_string();
        let backend_name = Backend::Llvm.as_str();
        let kind_name = kind.as_str();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.check.start",
                command = "check",
                status = "start",
                target = %target_str,
                backend = backend_name,
                kind = kind_name,
                input_count = inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }
        let corelib_files = collect_core_files()?;
        let foundationlib_files = collect_foundation_files()?;
        let alloclib_files = collect_alloc_files()?;
        let nostd_runtime_files = if runtime_resolution.resolved.kind == RuntimeKind::NoStd {
            collect_runtime_package_files(&runtime_resolution.resolved)?
        } else {
            Vec::new()
        };
        let stdlib_files = stdlib_files_for(kind, Backend::Llvm)?;
        let conditional_defines = resolve_conditional_defines(&target, &defines);
        crate::frontend::conditional::set_active_defines(conditional_defines.clone());
        let lint_config = crate::lint::discover(&inputs)?;
        let pipeline = CompilerPipelineBuilder::new("check", &inputs, &target, conditional_defines)
            .backend(Backend::Llvm)
            .kind(kind)
            .load_stdlib(load_stdlib)
            .corelib_files(&corelib_files)
            .foundationlib_files(&foundationlib_files)
            .alloclib_files(&alloclib_files)
            .stdlib_files(&stdlib_files)
            .nostd_runtime_files(&nostd_runtime_files)
            .runtime(Some(runtime_resolution.resolved))
            .trace_enabled(trace_enabled)
            .trait_solver_metrics(solver_metrics_enabled)
            .lint_config(lint_config)
            .build();
        let frontend = pipeline.execute()?;
        let module_count = frontend.modules.len();
        let lowering_count = frontend.mir_lowering_diagnostics.len();
        let borrow_count = frontend.borrow_diagnostics.len();
        let type_count = frontend.type_diagnostics.len();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.check.complete",
                command = "check",
                status = "ok",
                target = %target_str,
                backend = backend_name,
                kind = kind_name,
                input_count = inputs.len(),
                inputs = %inputs_summary,
                module_count,
                lowering_diagnostics = lowering_count,
                borrow_diagnostics = borrow_count,
                type_diagnostics = type_count,
                elapsed_ms = check_start.elapsed().as_millis() as u64
            );
        }
        let module_artifacts = vec![ModuleArtifact::default(); module_count];
        Ok(frontend.into_report(None, None, None, Vec::new(), module_artifacts))
    }

    /// Build the provided source using the requested backend and emit the final artifact.
    ///
    /// # Errors
    ///
    /// Propagates I/O failures, frontend diagnostics, MIR verification/borrow checking errors,
    /// and backend code generation issues.
    pub fn build(&self, request: BuildRequest) -> Result<FrontendReport> {
        run_with_stack(driver_stack_size(), move || build::execute(request))
    }

    /// Execute the compiled artifact within the current environment.
    ///
    /// # Errors
    ///
    /// Returns I/O failures, frontend/codegen diagnostics, or runtime execution errors.
    pub fn run(&self, request: BuildRequest) -> Result<RunResult> {
        let driver = self.clone();
        run_with_stack(driver_stack_size(), move || driver.run_inner(request))
    }

    fn run_inner(&self, mut request: BuildRequest) -> Result<RunResult> {
        if request.kind.is_library() {
            return Err(crate::error::Error::Cli(crate::cli::CliError::new(
                "chic run requires an executable crate type",
            )));
        }
        let backend = request.backend;
        let target = request.target.clone();
        let run_timeout = request.run_timeout;
        let trace_pipeline = request.trace_pipeline;
        let log_level = request.log_level;
        let load_stdlib = request.load_stdlib;
        let inputs = request.inputs.clone();
        use tempfile::Builder;

        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let run_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target_str = target.triple().to_string();
        let backend_name = backend.as_str().to_string();
        let kind_name = request.kind.as_str().to_string();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run.start",
                command = "run",
                status = "start",
                target = %target_str,
                backend = backend_name.as_str(),
                kind = kind_name.as_str(),
                input_count = request.inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }

        let tempdir = Builder::new()
            .prefix("chic-run-")
            .tempdir()
            .map_err(crate::error::Error::Io)?;
        let tempdir_path = tempdir.path().to_path_buf();
        if std::env::var_os("CHIC_KEEP_TEMPS").is_some() {
            // Keep the temporary directory for inspection by preventing TempDir drop cleanup.
            std::mem::forget(tempdir);
        }

        let ext = match backend {
            Backend::Wasm => "wasm",
            Backend::Llvm => ChicKind::Executable.default_extension(),
            Backend::Cc1 => "s",
        };
        let artifact_path = request
            .output
            .clone()
            .unwrap_or_else(|| tempdir_path.join(format!("chic-run.{ext}")));
        request.output = Some(artifact_path.clone());
        request.emit_wat_text = false;
        request.emit_object = false;
        request.emit_header = false;
        request.emit_library_pack = false;
        request.kind = ChicKind::Executable;
        if request.configuration.is_empty() {
            request.configuration = "Debug".to_string();
        }
        request.no_incremental = true;

        let report = self.build(request)?;

        if backend == Backend::Wasm {
            if wasm::find_entry_function(&report.mir_module).is_none() {
                if report.mir_module.attributes.is_no_main() {
                    return Err(crate::error::Error::internal(
                        "`#![no_main]` crate did not supply a `Main` entry point; provide a custom exported start function to run under WASM",
                    ));
                }
                return Err(crate::error::Error::internal(
                    "unable to locate entry point `Main` in module for wasm execution",
                ));
            }
            let artifact_path = report.artifact.as_ref().ok_or_else(|| {
                crate::error::Error::internal("wasm build did not produce an artifact path")
            })?;
            let bytes = std::fs::read(artifact_path)?;
            let wasm_options = wasm::resolve_wasm_options(
                inputs
                    .first()
                    .map(PathBuf::as_path)
                    .unwrap_or_else(|| Path::new(".")),
                &target,
            )?;
            let outcome = if let Some(timeout) = run_timeout {
                let (tx, rx) = std::sync::mpsc::channel();
                let bytes_clone = bytes.clone();
                let options_for_thread = wasm_options.clone();
                let _ = thread::Builder::new()
                    .name("wasm-run-watchdog".into())
                    .spawn(move || {
                        let result = execute_wasm_with_options(
                            &bytes_clone,
                            "chic_main",
                            &options_for_thread,
                        );
                        let _ = tx.send(result);
                    });
                match rx.recv_timeout(timeout) {
                    Ok(result) => result.map_err(|err| {
                        crate::error::Error::internal(format!(
                            "wasm execution failed: {}",
                            err.message
                        ))
                    })?,
                    Err(_) => {
                        let mut stderr =
                            format!("watchdog timeout after {}ms\n", timeout.as_millis())
                                .into_bytes();
                        if trace_enabled {
                            tracing::info!(
                                target: "pipeline",
                                stage = "driver.run.complete",
                                command = "run",
                                status = "error",
                                target = %target_str,
                                backend = backend_name.as_str(),
                                kind = kind_name.as_str(),
                                input_count = inputs.len(),
                                inputs = %inputs_summary,
                                exit_code = 124,
                                load_stdlib,
                                elapsed_ms = run_start.elapsed().as_millis() as u64
                            );
                        }
                        return Ok(RunResult {
                            report,
                            status: exit_status_from_code(124),
                            stdout: Vec::new(),
                            stderr: std::mem::take(&mut stderr),
                            wasm_trace: Some(crate::runtime::WasmExecutionTrace::from_options(
                                &wasm_options,
                            )),
                        });
                    }
                }
            } else {
                execute_wasm_with_options(&bytes, "chic_main", &wasm_options).map_err(|err| {
                    crate::error::Error::internal(format!("wasm execution failed: {}", err.message))
                })?
            };
            let status = exit_status_from_code(outcome.exit_code);
            let mut stderr = Vec::new();
            if let Some(termination) = outcome.termination {
                let message = match termination.kind {
                    hooks::RuntimeTerminationKind::Panic => format!(
                        "wasm panic: {}",
                        hooks::panic_message(termination.exit_code())
                    ),
                    hooks::RuntimeTerminationKind::Abort => format!(
                        "wasm abort: {}",
                        hooks::abort_message(termination.exit_code())
                    ),
                };
                stderr.extend_from_slice(message.as_bytes());
                if !message.ends_with('\n') {
                    stderr.push(b'\n');
                }
            }
            if trace_enabled {
                tracing::info!(
                    target: "pipeline",
                    stage = "driver.run.complete",
                    command = "run",
                    status = "ok",
                    target = %target_str,
                    backend = backend_name.as_str(),
                    kind = kind_name.as_str(),
                    input_count = inputs.len(),
                    inputs = %inputs_summary,
                    exit_code = status.code(),
                    load_stdlib,
                    elapsed_ms = run_start.elapsed().as_millis() as u64
                );
            }
            return Ok(RunResult {
                report,
                status,
                stdout: Vec::new(),
                stderr,
                wasm_trace: Some(outcome.trace),
            });
        }

        let mut cmd = Command::new(&artifact_path);
        configure_native_child_process(&mut cmd, &target);
        let output = if let Some(timeout) = run_timeout {
            let timed = wait_with_output_timeout(&mut cmd, timeout)?;
            if timed.timed_out {
                let mut output = timed.output;
                output.stderr.extend_from_slice(
                    format!("watchdog timeout after {}ms\n", timeout.as_millis()).as_bytes(),
                );
                output.status = exit_status_from_code(124);
                output
            } else {
                timed.output
            }
        } else {
            cmd.output()?
        };
        let result = RunResult {
            report,
            status: output.status,
            stdout: output.stdout,
            stderr: output.stderr,
            wasm_trace: None,
        };
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run.complete",
                command = "run",
                status = "ok",
                target = %target_str,
                backend = backend_name.as_str(),
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                exit_code = result.status.code(),
                load_stdlib,
                elapsed_ms = run_start.elapsed().as_millis() as u64
            );
        }
        Ok(result)
    }

    /// Format Chic source by normalising trailing whitespace.
    ///
    /// # Errors
    ///
    /// Returns I/O failures or parse errors encountered while validating the source.
    pub fn format(
        &self,
        path: &Path,
        config: &crate::format::FormatConfig,
    ) -> Result<FormatResult> {
        let source = fs::read_to_string(path)?;
        let outcome = crate::format::format_source(&source, config)?;
        let formatted = outcome.formatted;
        let changed = formatted != source;

        Ok(FormatResult {
            input: path.to_path_buf(),
            changed,
            original: source,
            formatted,
            metadata: outcome.metadata,
        })
    }
}

impl Default for CompilerDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerDriver {
    /// Execute Chic testcases for the provided source.
    ///
    /// # Errors
    ///
    /// Returns frontend/codegen failures, runtime execution errors, or filesystem issues.
    pub fn run_tests(
        &self,
        request: BuildRequest,
        test_options: crate::driver::types::TestOptions,
    ) -> Result<TestRun> {
        let driver = self.clone();
        run_with_stack(driver_stack_size(), move || {
            driver.run_tests_inner(request, test_options)
        })
    }

    fn run_tests_inner(
        &self,
        request: BuildRequest,
        test_options: crate::driver::types::TestOptions,
    ) -> Result<TestRun> {
        let backend = request.backend;
        let target = request.target.clone();
        let kind = request.kind;
        let trace_pipeline = request.trace_pipeline;
        let log_level = request.log_level;
        let load_stdlib = request.load_stdlib;
        let inputs = request.inputs.clone();
        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let tests_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target_str = target.triple().to_string();
        let backend_name = backend.as_str().to_string();
        let kind_name = kind.as_str().to_string();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run_tests.start",
                command = "test",
                status = "start",
                target = %target_str,
                backend = backend_name.as_str(),
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }

        let result = match backend {
            Backend::Wasm => self.run_tests_wasm(request, test_options.clone()),
            Backend::Llvm => self.run_tests_native(request, test_options.clone()),
            Backend::Cc1 => Err(crate::error::Error::internal(
                "chic test does not support the cc1 backend",
            )),
        };

        if trace_enabled {
            match &result {
                Ok(run) => {
                    tracing::info!(
                        target: "pipeline",
                        stage = "driver.run_tests.complete",
                        command = "test",
                        status = "ok",
                        target = %target_str,
                        backend = backend_name.as_str(),
                        kind = kind_name.as_str(),
                        input_count = inputs.len(),
                        inputs = %inputs_summary,
                        testcase_count = run.cases.len(),
                        load_stdlib,
                        elapsed_ms = tests_start.elapsed().as_millis() as u64
                    );
                }
                Err(err) => {
                    tracing::info!(
                        target: "pipeline",
                        stage = "driver.run_tests.error",
                        command = "test",
                        status = "error",
                        target = %target_str,
                        backend = backend_name.as_str(),
                        kind = kind_name.as_str(),
                        input_count = inputs.len(),
                        inputs = %inputs_summary,
                        load_stdlib,
                        elapsed_ms = tests_start.elapsed().as_millis() as u64,
                        error = %err
                    );
                }
            }
        }

        result
    }

    fn run_tests_wasm(
        &self,
        mut request: BuildRequest,
        test_options: crate::driver::types::TestOptions,
    ) -> Result<TestRun> {
        let trace_pipeline = request.trace_pipeline;
        let log_level = request.log_level;
        let load_stdlib = request.load_stdlib;
        let inputs = request.inputs.clone();
        let target = request.target.clone();
        let kind = request.kind;
        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let wasm_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target_str = target.triple().to_string();
        let kind_name = kind.as_str().to_string();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run_tests_wasm.start",
                command = "test",
                status = "start",
                target = %target_str,
                backend = "wasm",
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }

        // WASM testcase execution must not require a `Main` entrypoint; testcases are invoked
        // directly by the runner. Keep the requested kind (typically `static-library`).
        let tests_settings = request
            .manifest
            .as_ref()
            .map(|manifest| manifest.tests().clone());
        let coverage_settings = request
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.coverage().cloned());
        let coverage_required = request.coverage;
        if coverage_required {
            if let Some(settings) = coverage_settings.as_ref() {
                if matches!(settings.backend, crate::manifest::CoverageBackend::Llvm) {
                    return Err(crate::error::Error::Cli(crate::cli::CliError::new(
                        "Chic coverage for the LLVM backend is not supported yet; use coverage.backend: wasm or both",
                    )));
                }
            }
        }
        request.coverage = coverage_required;

        let manifest_path = request
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.path())
            .map(PathBuf::from);
        let testcase_roots: Option<Vec<PathBuf>> = request.manifest.as_ref().and_then(|manifest| {
            let manifest_path = manifest.path()?;
            let manifest_dir = manifest_path.parent()?;
            let mut roots = Vec::new();
            roots.push(manifest_dir.join("src"));
            if manifest.tests().include_tests_dir {
                roots.push(manifest_dir.join("tests"));
            }
            Some(roots)
        });
        let mut wasm_options = wasm::resolve_wasm_options(
            inputs
                .first()
                .map(PathBuf::as_path)
                .unwrap_or_else(|| Path::new(".")),
            &target,
        )?;
        let chic_coverage_hits = if coverage_required {
            use crate::chic_coverage::ChicCoveragePointId;
            use std::collections::BTreeSet;
            use std::sync::{Arc, Mutex};

            let hits: Arc<Mutex<BTreeSet<ChicCoveragePointId>>> =
                Arc::new(Mutex::new(BTreeSet::new()));
            let hook_hits = hits.clone();
            wasm_options.coverage_hook = Some(Arc::new(move |id: u64| {
                if let Ok(mut guard) = hook_hits.lock() {
                    guard.insert(ChicCoveragePointId(id));
                }
            }));
            Some(hits)
        } else {
            None
        };
        // Preserve original request.kind (typically static-library).
        request.backend = Backend::Wasm;
        request.output = None;
        request.emit_wat_text = false;
        request.emit_object = false;
        request.emit_header = false;
        request.emit_library_pack = false;
        if request.configuration.is_empty() {
            request.configuration = "Debug".to_string();
        }
        let report = self.build(request)?;
        if std::env::var_os("CHIC_DEBUG_TESTCASE_RET_BUILT").is_some() {
            for name in [
                "Std::Async::cancel_token_respects_deadline",
                "Std::Random::rng_sequence_is_deterministic",
            ] {
                for (idx, func) in report
                    .mir_module
                    .functions
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.name == name)
                {
                    eprintln!(
                        "[testcase-ret-built] idx={} name={} kind={:?} ret={:?}",
                        idx, name, func.kind, func.signature.ret
                    );
                }
            }
        }
        let artifact_path = report.artifact.as_ref().ok_or_else(|| {
            crate::error::Error::internal("wasm build did not produce an artifact path")
        })?;
        let bytes = fs::read(artifact_path)?;
        WasmProgram::from_bytes(&bytes).map_err(|err| {
            crate::error::Error::internal(format!(
                "failed to parse wasm module for test execution: {err}"
            ))
        })?;

        let discovered = crate::mir::collect_test_metadata(&report.mir_module).len();
        if let Some(settings) = &tests_settings {
            if settings.enabled
                && settings.require_testcases
                && discovered == 0
                && matches!(
                    settings.enforce,
                    crate::manifest::CoverageEnforcement::Error
                )
            {
                return Err(crate::error::Error::Cli(crate::cli::CliError::new(
                    "package has zero Chic testcases; add `testcase` blocks or disable tests.require_testcases",
                )));
            }
        }
        let (cases, filtered_out) = wasm::collect_wasm_testcases(
            &report,
            &bytes,
            &wasm_options,
            trace_enabled,
            &test_options,
            &target,
            testcase_roots.as_deref(),
        );
        let chic_coverage = chic_coverage_hits
            .as_ref()
            .and_then(|hits| hits.lock().ok().map(|guard| guard.clone()))
            .and_then(|hits| {
                let src_root = crate::chic_coverage::package_src_root(manifest_path.as_deref())?;
                let points = crate::chic_coverage::collect_statement_points(&report.mir_module);
                if std::env::var("CHIC_COVERAGE_DEBUG").is_ok() {
                    let uncovered = crate::chic_coverage::uncovered_functions(
                        &report.mir_module,
                        &points,
                        &hits,
                        &report.files,
                        &src_root,
                    );
                    if !uncovered.is_empty() {
                        eprintln!("chic coverage uncovered functions:");
                        for func in &uncovered {
                            eprintln!("{} | {} | {}", func.name, func.path, func.statements);
                        }
                    }
                }
                Some(crate::chic_coverage::report_for_root(
                    &report.mir_module,
                    &points,
                    &hits,
                    &report.files,
                    &src_root,
                ))
            });
        // Coverage gate enforcement happens in CLI dispatch so test output
        // can still be reported before failing the command.

        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run_tests_wasm.complete",
                command = "test",
                status = "ok",
                target = %target_str,
                backend = "wasm",
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                testcase_count = cases.len(),
                load_stdlib,
                elapsed_ms = wasm_start.elapsed().as_millis() as u64
            );
        }

        Ok(TestRun {
            report,
            cases,
            discovered,
            filtered_out,
            chic_coverage,
        })
    }

    fn run_tests_native(
        &self,
        mut request: BuildRequest,
        test_options: crate::driver::types::TestOptions,
    ) -> Result<TestRun> {
        let trace_pipeline = request.trace_pipeline;
        let log_level = request.log_level;
        let load_stdlib = request.load_stdlib;
        let inputs = request.inputs.clone();
        let target = request.target.clone();
        let kind = request.kind;
        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let native_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target_str = target.triple().to_string();
        let kind_name = kind.as_str().to_string();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run_tests_native.start",
                command = "test",
                status = "start",
                target = %target_str,
                backend = "llvm",
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }

        let tests_settings = request
            .manifest
            .as_ref()
            .map(|manifest| manifest.tests().clone());
        let testcase_roots: Option<Vec<PathBuf>> = request.manifest.as_ref().and_then(|manifest| {
            let manifest_path = manifest.path()?;
            let manifest_dir = manifest_path.parent()?;
            let mut roots = Vec::new();
            roots.push(manifest_dir.join("src"));
            if manifest.tests().include_tests_dir {
                roots.push(manifest_dir.join("tests"));
            }
            Some(roots)
        });
        let coverage_settings = request
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.coverage().cloned());
        let coverage_required = request.coverage;
        if coverage_required {
            if let Some(settings) = coverage_settings.as_ref() {
                if matches!(settings.backend, crate::manifest::CoverageBackend::Llvm) {
                    return Err(crate::error::Error::Cli(crate::cli::CliError::new(
                        "Chic coverage for the LLVM backend is not supported yet; use coverage.backend: wasm or both",
                    )));
                }
            }
        }
        let mut wasm_request = if coverage_required {
            Some(request.clone())
        } else {
            None
        };

        request.coverage = false;
        request.kind = ChicKind::Executable;
        request.backend = Backend::Llvm;
        request.output = None;
        request.emit_wat_text = false;
        request.emit_object = false;
        request.emit_header = false;
        request.emit_library_pack = false;
        if request.configuration.is_empty() {
            request.configuration = "Debug".to_string();
        }
        let report = self.build(request)?;

        let all_entries = collect_native_testcases(&report);
        let discovered = all_entries.len();
        if let Some(settings) = &tests_settings {
            if settings.enabled
                && settings.require_testcases
                && discovered == 0
                && matches!(
                    settings.enforce,
                    crate::manifest::CoverageEnforcement::Error
                )
            {
                return Err(crate::error::Error::Cli(crate::cli::CliError::new(
                    "package has zero Chic testcases; add `testcase` blocks or disable tests.require_testcases",
                )));
            }
        }

        let selected: Vec<_> = all_entries
            .into_iter()
            .filter(|entry| {
                let in_scope = if let Some(roots) = testcase_roots.as_deref() {
                    let span = entry.meta.span;
                    if let Some(path) = span.and_then(|span| report.files.path(span.file_id)) {
                        let cwd = std::env::current_dir().ok();
                        let to_abs = |p: &Path| -> PathBuf {
                            if p.is_absolute() {
                                p.to_path_buf()
                            } else if let Some(cwd) = &cwd {
                                cwd.join(p)
                            } else {
                                p.to_path_buf()
                            }
                        };
                        let abs_path = to_abs(path);
                        roots.iter().any(|root| abs_path.starts_with(to_abs(root)))
                    } else {
                        true
                    }
                } else {
                    true
                };

                in_scope
                    && test_options.selection.matches(&entry.meta)
                    && !should_skip_for_executor(&entry.meta, false)
            })
            .collect();
        let filtered_out = discovered.saturating_sub(selected.len());

        let mut runnable_indices = Vec::new();
        for entry in &selected {
            if entry.meta.parameters.is_empty()
                && report
                    .mir_module
                    .functions
                    .get(entry.meta.function_index)
                    .is_some()
            {
                runnable_indices.push(entry.index);
            }
        }

        let mut native_results: HashMap<usize, NativeTestcaseReport> = HashMap::new();
        let mut native_status: Option<ExitStatus> = None;
        let mut native_timed_out = false;
        if !runnable_indices.is_empty() {
            let artifact_path = report.artifact.as_ref().ok_or_else(|| {
                crate::error::Error::internal("native test build did not produce an artifact path")
            })?;
            if std::env::var_os("CHIC_DEBUG_NATIVE_TEST_RUNNER").is_some() {
                let max_dump = 256usize;
                eprintln!(
                    "[native-test-runner] selected_testcases={} (dumping up to {max_dump})",
                    runnable_indices.len()
                );
                for entry in selected.iter().take(max_dump) {
                    eprintln!(
                        "[native-test-runner] testcase index={} name={}",
                        entry.index, entry.meta.qualified_name
                    );
                }
                if selected.len() > max_dump {
                    eprintln!(
                        "[native-test-runner] ... {} more testcases not shown",
                        selected.len() - max_dump
                    );
                }
            }
            let selection_list = runnable_indices
                .iter()
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let mut cmd = Command::new(artifact_path);
            configure_native_child_process(&mut cmd, &target);
            cmd.arg("--run-tests");
            cmd.arg(format!("--chic-test-indexes={selection_list}"));
            cmd.env("CHIC_TEST_INDEXES", &selection_list);
            if test_options.fail_fast {
                cmd.arg("--chic-test-fail-fast");
                cmd.env("CHIC_TEST_FAIL_FAST", "1");
            }
            let timeout =
                resolve_native_test_runner_timeout(&test_options.watchdog, &runnable_indices);
            let output = if let Some(timeout) = timeout {
                let timed = wait_with_output_timeout(&mut cmd, timeout)?;
                native_timed_out = timed.timed_out;
                timed.output
            } else {
                cmd.output()?
            };
            if std::env::var_os("CHIC_DEBUG_NATIVE_TEST_RUNNER").is_some() {
                eprintln!(
                    "[native-test-runner] artifact={} status={} stdout={}B stderr={}B",
                    artifact_path.display(),
                    output.status,
                    output.stdout.len(),
                    output.stderr.len()
                );
                eprintln!("[native-test-runner] selection={selection_list}");
                if !output.stderr.is_empty() {
                    eprintln!(
                        "[native-test-runner] stderr:\n{}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                if !output.status.success() && !output.stdout.is_empty() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let preview = stdout.get(..stdout.len().min(65_536)).unwrap_or(&stdout);
                    eprintln!("[native-test-runner] stdout:\n{preview}");
                }
            }
            native_status = Some(output.status);
            native_results = parse_native_test_output(&output.stdout);
        }

        let mut cases: Vec<TestCaseResult> = Vec::new();
        let mut saw_failure = false;
        let missing_message =
            missing_native_result_message(native_status, native_timed_out, test_options.watchdog);
        for entry in selected {
            let meta = entry.meta;
            if !meta.parameters.is_empty() {
                cases.push(skip_parameterized(&meta));
                continue;
            }
            if report
                .mir_module
                .functions
                .get(meta.function_index)
                .is_none()
            {
                saw_failure = true;
                cases.push(missing_function_result(&meta));
                continue;
            }
            if let Some(result) = native_results.remove(&entry.index) {
                let status = result.status;
                if matches!(status, TestStatus::Failed) {
                    saw_failure = true;
                }
                cases.push(TestCaseResult {
                    id: meta.id,
                    name: meta.name,
                    qualified_name: meta.qualified_name,
                    namespace: meta.namespace,
                    categories: meta.categories,
                    is_async: meta.is_async,
                    status,
                    message: result.message,
                    wasm_trace: None,
                    duration: None,
                });
                continue;
            }
            if test_options.fail_fast && saw_failure {
                cases.push(skip_after_fail_fast(&meta));
                continue;
            }
            saw_failure = true;
            cases.push(missing_native_result(&meta, missing_message.as_deref()));
        }

        let mut chic_coverage = None;
        if let Some(mut wasm_request) = wasm_request.take() {
            wasm_request.coverage = coverage_required;
            let wasm_run = self.run_tests_wasm(wasm_request, test_options)?;
            if let Some(coverage) = wasm_run.chic_coverage {
                chic_coverage = Some(coverage);
            }
            merge_wasm_results(&mut cases, &wasm_run.cases);
        }

        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.run_tests_native.complete",
                command = "test",
                status = "ok",
                target = %target_str,
                backend = "llvm",
                kind = kind_name.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                testcase_count = cases.len(),
                load_stdlib,
                elapsed_ms = native_start.elapsed().as_millis() as u64
            );
        }

        Ok(TestRun {
            report,
            cases,
            discovered,
            filtered_out,
            chic_coverage,
        })
    }

    /// Render MIR into a human-readable textual representation.
    ///
    /// # Errors
    ///
    /// Returns frontend or verification failures encountered during lowering.
    pub fn mir_dump(
        &self,
        path: &Path,
        trace_pipeline: bool,
        trait_solver_metrics: bool,
        log_level: LogLevel,
    ) -> Result<MirDumpResult> {
        let inputs = vec![path.to_path_buf()];
        let load_stdlib = Self::should_load_stdlib(&inputs);
        let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
        let mir_start = Instant::now();
        let inputs_summary = summarize_inputs(&inputs);
        let target = Target::host();
        let target_str = target.triple().to_string();
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.mir_dump.start",
                command = "mir-dump",
                status = "start",
                target = %target_str,
                backend = Backend::Llvm.as_str(),
                kind = ChicKind::Executable.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                load_stdlib
            );
        }
        let report = self.check(
            &inputs,
            &target,
            ChicKind::Executable,
            load_stdlib,
            trace_pipeline,
            trait_solver_metrics,
            &[],
            log_level,
        )?;
        let rendered = format_module(&report.mir_module, &report.type_constraints);
        if trace_enabled {
            tracing::info!(
                target: "pipeline",
                stage = "driver.mir_dump.complete",
                command = "mir-dump",
                status = "ok",
                target = %target_str,
                backend = Backend::Llvm.as_str(),
                kind = ChicKind::Executable.as_str(),
                input_count = inputs.len(),
                inputs = %inputs_summary,
                has_diagnostics = report.has_diagnostics(),
                load_stdlib,
                elapsed_ms = mir_start.elapsed().as_millis() as u64
            );
        }
        Ok(MirDumpResult { report, rendered })
    }
}

// namespace traversal helpers remain for future harness extensions

#[derive(Debug, Clone)]
struct NativeTestcaseEntry {
    index: usize,
    meta: TestCaseMetadata,
}

#[derive(Debug, Clone)]
struct NativeTestcaseReport {
    status: TestStatus,
    message: Option<String>,
}

fn collect_native_testcases(report: &FrontendReport) -> Vec<NativeTestcaseEntry> {
    let mut meta_map = HashMap::new();
    for meta in crate::mir::collect_test_metadata(&report.mir_module) {
        meta_map.insert(meta.qualified_name.clone(), meta);
    }
    let mut entries = Vec::new();
    for (func_index, function) in report.mir_module.functions.iter().enumerate() {
        if !matches!(function.kind, FunctionKind::Testcase) {
            continue;
        }
        let mut meta = meta_map.remove(&function.name).unwrap_or_else(|| {
            let (namespace, name) = TestCaseMetadata::split_namespace(&function.name);
            TestCaseMetadata {
                function_index: func_index,
                id: TestCaseMetadata::stable_id(&function.name, None),
                qualified_name: function.name.clone(),
                name,
                namespace,
                categories: Vec::new(),
                parameters: Vec::new(),
                is_async: function.is_async,
                span: function.span,
            }
        });
        meta.function_index = func_index;
        meta.is_async = function.is_async;
        let index = entries.len();
        entries.push(NativeTestcaseEntry { index, meta });
    }
    entries
}

fn parse_native_test_output(stdout: &[u8]) -> HashMap<usize, NativeTestcaseReport> {
    let mut results = HashMap::new();
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines() {
        if !line.starts_with("CHIC_TESTCASE\t") {
            continue;
        }
        let mut parts = line.splitn(5, '\t');
        let prefix = parts.next();
        if prefix != Some("CHIC_TESTCASE") {
            continue;
        }
        let Some(index) = parts.next().and_then(|raw| raw.parse::<usize>().ok()) else {
            continue;
        };
        let status = match parts.next() {
            Some("PASS") => TestStatus::Passed,
            Some("FAIL") => TestStatus::Failed,
            Some("SKIP") => TestStatus::Skipped,
            _ => continue,
        };
        let _ = parts.next();
        let message = parts
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());
        results.insert(index, NativeTestcaseReport { status, message });
    }
    results
}

fn missing_native_result_message(
    status: Option<ExitStatus>,
    timed_out: bool,
    watchdog: crate::driver::types::WatchdogConfig,
) -> Option<String> {
    if timed_out {
        if let Some(timeout) = watchdog.timeout {
            return Some(format!(
                "missing testcase result from native runner (timed out after {}ms)",
                timeout.as_millis()
            ));
        }
        return Some("missing testcase result from native runner (timed out)".into());
    }

    let mut message = String::from("missing testcase result from native runner");
    if let Some(status) = status {
        if let Some(code) = status.code() {
            if code != 0 {
                message.push_str(&format!(" (exit status {code})"));
            }
        } else if !status.success() {
            message.push_str(" (terminated by signal)");
        }
    }
    Some(message)
}

#[derive(Debug)]
struct TimedProcessOutput {
    output: std::process::Output,
    timed_out: bool,
}

fn resolve_native_test_runner_timeout(
    watchdog: &crate::driver::types::WatchdogConfig,
    runnable_indices: &[usize],
) -> Option<Duration> {
    let timeout = watchdog.timeout?;
    if runnable_indices.is_empty() {
        return None;
    }
    let multiplier = runnable_indices.len().max(1) as u32;
    timeout.checked_mul(multiplier)
}

fn wait_with_output_timeout(
    cmd: &mut Command,
    timeout: Duration,
) -> std::io::Result<TimedProcessOutput> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let child_pid = child.id() as i32;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "missing child stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "missing child stderr"))?;

    let (stdout_tx, stdout_rx) = std::sync::mpsc::channel();
    let (stderr_tx, stderr_rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = std::io::BufReader::new(stdout).read_to_end(&mut buffer);
        let _ = stdout_tx.send(buffer);
    });
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = std::io::BufReader::new(stderr).read_to_end(&mut buffer);
        let _ = stderr_tx.send(buffer);
    });

    let status = wait_for_child_timeout(&mut child, timeout)?;
    let (status, timed_out) = match status {
        Some(status) => (status, false),
        None => {
            #[cfg(unix)]
            unsafe {
                let _ = libc::kill(-child_pid, libc::SIGKILL);
            }
            let _ = child.kill();
            (child.wait()?, true)
        }
    };

    let read_timeout_total = if timed_out {
        Duration::from_millis(250)
    } else {
        Duration::from_secs(2)
    };
    let drain_deadline = Instant::now() + read_timeout_total;
    let recv_with_deadline = |rx: &std::sync::mpsc::Receiver<Vec<u8>>| {
        let remaining = drain_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(std::sync::mpsc::RecvTimeoutError::Timeout);
        }
        rx.recv_timeout(remaining)
    };
    let stdout_result = recv_with_deadline(&stdout_rx);
    let stderr_result = recv_with_deadline(&stderr_rx);
    let mut stdout = stdout_result.clone().unwrap_or_default();
    let mut stderr = stderr_result.clone().unwrap_or_default();

    let drain_timed_out = matches!(
        stdout_result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ) || matches!(
        stderr_result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    );
    if drain_timed_out {
        #[cfg(unix)]
        unsafe {
            let _ = libc::kill(-child_pid, libc::SIGKILL);
        }
        if matches!(
            stdout_result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ) {
            stdout = stdout_rx
                .recv_timeout(Duration::from_millis(250))
                .unwrap_or_default();
        }
        if matches!(
            stderr_result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ) {
            stderr = stderr_rx
                .recv_timeout(Duration::from_millis(250))
                .unwrap_or_default();
        }
    }

    Ok(TimedProcessOutput {
        output: std::process::Output {
            status,
            stdout,
            stderr,
        },
        timed_out,
    })
}

fn wait_for_child_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> std::io::Result<Option<ExitStatus>> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if start.elapsed() >= timeout {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn missing_native_result(meta: &TestCaseMetadata, message: Option<&str>) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Failed,
        message: Some(
            message
                .unwrap_or("missing testcase result from native runner")
                .to_string(),
        ),
        wasm_trace: None,
        duration: None,
    }
}

fn merge_wasm_results(cases: &mut [TestCaseResult], wasm_cases: &[TestCaseResult]) {
    let mut wasm_map = HashMap::new();
    for case in wasm_cases {
        wasm_map.insert(case.id.clone(), case);
    }
    for case in cases {
        let Some(wasm_case) = wasm_map.get(&case.id) else {
            continue;
        };
        if !matches!(wasm_case.status, TestStatus::Failed) {
            continue;
        }
        let wasm_message = wasm_case.message.as_deref().unwrap_or("testcase failed");
        let message = match case.message.take() {
            Some(existing) => format!("{existing}; wasm: {wasm_message}"),
            None => format!("wasm: {wasm_message}"),
        };
        case.status = TestStatus::Failed;
        case.message = Some(message);
    }
}

fn configure_native_child_process(cmd: &mut Command, target: &Target) {
    // macOS 26's libmalloc nano zone can terminate with `Unlock of an os_unfair_lock not owned`
    // for our generated binaries; disable nano for deterministic native execution.
    if std::env::consts::OS == "macos" && target.triple().contains("apple") {
        cmd.env("MallocNanoZone", "0");
    }
}

fn should_skip_for_executor(meta: &TestCaseMetadata, is_wasm_executor: bool) -> bool {
    if is_wasm_executor {
        return meta
            .categories
            .iter()
            .any(|category| category.eq_ignore_ascii_case("native"));
    }
    meta.categories
        .iter()
        .any(|category| category.eq_ignore_ascii_case("wasm"))
}

fn skip_after_fail_fast(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Skipped,
        message: Some("skipped due to --fail-fast".into()),
        wasm_trace: None,
        duration: None,
    }
}

fn missing_function_result(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Failed,
        message: Some(format!(
            "test metadata refers to missing function index {}",
            meta.function_index
        )),
        wasm_trace: None,
        duration: None,
    }
}

fn skip_parameterized(meta: &TestCaseMetadata) -> TestCaseResult {
    TestCaseResult {
        id: meta.id.clone(),
        name: meta.name.clone(),
        qualified_name: meta.qualified_name.clone(),
        namespace: meta.namespace.clone(),
        categories: meta.categories.clone(),
        is_async: meta.is_async,
        status: TestStatus::Skipped,
        message: Some("parameterized testcases are not supported yet".into()),
        wasm_trace: None,
        duration: None,
    }
}

#[cfg(unix)]
fn exit_status_from_code(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw((code & 0xff) << 8)
}

#[cfg(not(unix))]
fn exit_status_from_code(_code: i32) -> ExitStatus {
    panic!("wasm execution currently supported only on unix platforms");
}

#[cfg(test)]
mod tests;
