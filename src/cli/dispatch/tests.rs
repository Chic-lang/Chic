use super::DispatchDriver;
use super::commands::{
    FormatCommandOptions, dispatch_command, run_build, run_check, run_format, run_header,
    run_mir_dump, run_run, run_tests,
};
use super::ffi::resolve_cli_ffi_options;
use super::logging;
use super::reporting::{
    print_report_diagnostics_to, relay_run_output, relay_run_output_to, report_error_to,
};
use crate::cli::dispatch::commands::{
    FormatCheckOutcome, FormattedFile, build_file_organization_diagnostics,
    build_ordering_diagnostics,
};
use crate::cli::{Cli, CliFfiOptions, Command, FfiDefaultPattern};
use crate::codegen::{Backend, CpuIsaConfig};
use crate::defines::DefineFlag;
use crate::diagnostics::{ColorMode, Diagnostic, ErrorFormat, FileCache, FormatOptions, Span};
use crate::driver::types::{TelemetrySetting, Verbosity};
use crate::driver::{
    BuildFfiOptions, BuildRequest, FormatResult, FrontendReport, MirDumpResult,
    MirVerificationIssue, ModuleReport, RunResult, TestCaseResult, TestOptions, TestRun,
    TestStatus,
};
use crate::error::{Error, Result};
use crate::format::{FormatConfig, FormatEnforcement, MemberSort, TypeMetadata, TypeSort};
use crate::frontend::ast::arena::AstArena;
use crate::frontend::parser::{ParseError, ParseResult};
use crate::logging::{LogFormat, LogLevel, LogOptions};
use crate::manifest::{MissingDocsRule, PROJECT_MANIFEST_BASENAME};
use crate::mir::{LoweringDiagnostic, MirModule, VerifyError};
use crate::monomorphize::MonomorphizationSummary;
use crate::perf::PerfMetadata;
use crate::runtime::backend::RuntimeBackend;
use crate::spec::Spec;
use crate::typeck::{TraitSolverMetrics, TypeConstraint};
use crate::{ChicKind, Target};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::cell::RefCell;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::Mutex;
use tempfile::tempdir;

fn sample_report() -> FrontendReport {
    let arena = AstArena::new();
    let module_id = arena.module_builder(Some("Sample".into())).finish_in();
    let source = "namespace Sample;".to_string();
    let path = PathBuf::from("sample.cl");
    let mut files = FileCache::default();
    let file_id = files.add_file(path.clone(), source);
    let diagnostics = vec![Diagnostic::warning(
        "parse diagnostic",
        Some(Span::in_file(file_id, 1, 3)),
    )];
    let module = arena.module_owned(module_id);
    let parse = ParseResult {
        arena,
        module_id,
        file_id,
        diagnostics,
        module,
        recovery_telemetry: None,
    };
    let mir = MirModule::default();

    FrontendReport {
        modules: vec![ModuleReport {
            input: path,
            parse,
            mir: mir.clone(),
            generated: None,
            object_path: None,
            metadata_path: None,
            assembly_path: None,
        }],
        files,
        target: Target::parse("x86_64-unknown-none").expect("target"),
        kind: ChicKind::Executable,
        runtime: None,
        artifact: None,
        library_pack: None,
        header: None,
        generated: Vec::new(),
        mir_module: mir,
        perf_metadata: PerfMetadata::default(),
        mir_lowering_diagnostics: Vec::new(),
        mir_verification: Vec::new(),
        reachability_diagnostics: vec![Diagnostic::warning("reachability diagnostic", None)],
        borrow_diagnostics: vec![Diagnostic::warning("borrow diagnostic", None)],
        fallible_diagnostics: vec![Diagnostic::warning("fallible diagnostic", None)],
        type_constraints: Vec::<TypeConstraint>::new(),
        type_diagnostics: vec![Diagnostic::warning("type diagnostic", None)],
        lint_diagnostics: Vec::new(),
        format_diagnostics: Vec::new(),
        doc_diagnostics: vec![Diagnostic::warning("doc diagnostic", None)],
        monomorphization: MonomorphizationSummary::default(),
        drop_glue: Vec::new(),
        clone_glue: Vec::new(),
        hash_glue: Vec::new(),
        eq_glue: Vec::new(),
        type_metadata: Vec::new(),
        trait_solver_metrics: TraitSolverMetrics::default(),
    }
}

fn test_format_options() -> FormatOptions {
    FormatOptions {
        format: ErrorFormat::Human,
        color: ColorMode::Never,
        is_terminal: false,
    }
}

static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn with_env_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = ENV_LOCK.lock().expect("env lock poisoned");
    f()
}

#[derive(Clone, Copy)]
enum ReportKind {
    Clean,
    WithDiagnostics,
}

#[derive(Default, Clone, Copy)]
struct Calls {
    check: usize,
    build: usize,
    run: usize,
    run_tests: usize,
    format: usize,
    mir_dump: usize,
}

struct StubDriver {
    calls: RefCell<Calls>,
    spec: Spec,
    load_stdlib: bool,
    check_kind: ReportKind,
    build_kind: ReportKind,
    run_status: ExitStatus,
    test_statuses: Vec<TestStatus>,
    mir_kind: ReportKind,
    format_changed: bool,
}

impl StubDriver {
    fn calls(&self) -> Calls {
        *self.calls.borrow()
    }

    fn with_check_kind(mut self, kind: ReportKind) -> Self {
        self.check_kind = kind;
        self
    }

    fn with_build_kind(mut self, kind: ReportKind) -> Self {
        self.build_kind = kind;
        self
    }

    fn with_mir_kind(mut self, kind: ReportKind) -> Self {
        self.mir_kind = kind;
        self
    }

    fn with_run_status(mut self, status: ExitStatus) -> Self {
        self.run_status = status;
        self
    }

    fn with_test_statuses(mut self, statuses: Vec<TestStatus>) -> Self {
        self.test_statuses = statuses;
        self
    }

    fn with_format_changed(mut self, changed: bool) -> Self {
        self.format_changed = changed;
        self
    }
}

impl Default for StubDriver {
    fn default() -> Self {
        Self {
            calls: RefCell::new(Calls::default()),
            spec: Spec::current(),
            load_stdlib: true,
            check_kind: ReportKind::Clean,
            build_kind: ReportKind::Clean,
            run_status: success_status(),
            test_statuses: vec![TestStatus::Passed],
            mir_kind: ReportKind::Clean,
            format_changed: false,
        }
    }
}

impl DispatchDriver for StubDriver {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn should_load_stdlib(&self, _inputs: &[PathBuf]) -> bool {
        self.load_stdlib
    }

    fn check(
        &self,
        _inputs: &[PathBuf],
        _target: &Target,
        _kind: ChicKind,
        _load_stdlib: bool,
        _trace_pipeline: bool,
        _trait_solver_metrics: bool,
        _defines: &[DefineFlag],
        _log_level: LogLevel,
    ) -> Result<FrontendReport> {
        self.calls.borrow_mut().check += 1;
        Ok(report_for(self.check_kind))
    }

    fn build(&self, _request: BuildRequest) -> Result<FrontendReport> {
        self.calls.borrow_mut().build += 1;
        Ok(report_for(self.build_kind))
    }

    fn run(&self, _request: BuildRequest) -> Result<RunResult> {
        self.calls.borrow_mut().run += 1;
        Ok(RunResult {
            report: clean_report(),
            status: self.run_status,
            stdout: b"stdout".to_vec(),
            stderr: b"stderr".to_vec(),
            wasm_trace: None,
        })
    }

    fn run_tests(&self, _request: BuildRequest, _test_options: TestOptions) -> Result<TestRun> {
        self.calls.borrow_mut().run_tests += 1;
        Ok(TestRun {
            report: clean_report(),
            cases: cases_from(&self.test_statuses),
            discovered: self.test_statuses.len(),
            filtered_out: 0,
            chic_coverage: None,
        })
    }

    fn format(&self, input: &Path, _config: &crate::format::FormatConfig) -> Result<FormatResult> {
        self.calls.borrow_mut().format += 1;
        Ok(FormatResult {
            input: input.to_path_buf(),
            changed: self.format_changed,
            original: "original".into(),
            formatted: "formatted".into(),
            metadata: crate::format::FormatMetadata::default(),
        })
    }

    fn mir_dump(
        &self,
        _input: &Path,
        _trace_pipeline: bool,
        _trait_solver_metrics: bool,
        _log_level: LogLevel,
    ) -> Result<MirDumpResult> {
        self.calls.borrow_mut().mir_dump += 1;
        Ok(MirDumpResult {
            report: report_for(self.mir_kind),
            rendered: "mir-dump".into(),
        })
    }
}

fn clean_report() -> FrontendReport {
    let mut report = sample_report();
    report.modules[0].parse.diagnostics.clear();
    report.mir_lowering_diagnostics.clear();
    report.mir_verification.clear();
    report.reachability_diagnostics.clear();
    report.borrow_diagnostics.clear();
    report.fallible_diagnostics.clear();
    report.type_diagnostics.clear();
    report.lint_diagnostics.clear();
    report.format_diagnostics.clear();
    report.doc_diagnostics.clear();
    report
}

fn report_for(kind: ReportKind) -> FrontendReport {
    match kind {
        ReportKind::Clean => clean_report(),
        ReportKind::WithDiagnostics => sample_report(),
    }
}

fn success_status() -> ExitStatus {
    #[cfg(unix)]
    {
        ExitStatusExt::from_raw(0)
    }
    #[cfg(windows)]
    {
        ExitStatusExt::from_raw(0)
    }
}

fn failure_status() -> ExitStatus {
    #[cfg(unix)]
    {
        ExitStatusExt::from_raw(1)
    }
    #[cfg(windows)]
    {
        ExitStatusExt::from_raw(1)
    }
}

fn cases_from(statuses: &[TestStatus]) -> Vec<TestCaseResult> {
    statuses
        .iter()
        .enumerate()
        .map(|(index, status)| TestCaseResult {
            id: format!("t-{index}"),
            name: format!("case{index}"),
            qualified_name: format!("case{index}"),
            namespace: None,
            categories: Vec::new(),
            is_async: false,
            status: *status,
            message: None,
            wasm_trace: None,
            duration: None,
        })
        .collect()
}

fn dispatch_request(inputs: Vec<PathBuf>) -> BuildRequest {
    BuildRequest {
        inputs,
        manifest: None,
        workspace: None,
        target: Target::host(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: RuntimeBackend::Chic,
        output: None,
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::baseline(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: false,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: BuildFfiOptions::default(),
        configuration: "Debug".to_string(),
        framework: None,
        artifacts_path: None,
        obj_dir: None,
        bin_dir: None,
        no_dependencies: false,
        no_restore: false,
        no_incremental: false,
        rebuild: false,
        incremental_validate: false,
        clean_only: false,
        disable_build_servers: false,
        source_root: None,
        properties: Vec::new(),
        verbosity: Verbosity::Normal,
        telemetry: TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    }
}

#[test]
fn logging_resolves_effective_level_when_trace_requested() {
    let options = LogOptions {
        format: LogFormat::Text,
        level: LogLevel::Info,
    };
    let level = logging::resolve_effective_level(&options, true);
    assert_eq!(level, LogLevel::Trace);
    let unchanged = logging::resolve_effective_level(&options, false);
    assert_eq!(unchanged, LogLevel::Info);
}

#[test]
fn command_requests_trace_follows_command_flags() {
    let traced = Command::Run {
        inputs: vec![PathBuf::from("main.cl")],
        manifest: None,
        workspace: None,
        target: Target::host(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: RuntimeBackend::Chic,
        cpu_isa: CpuIsaConfig::baseline(),
        const_eval_fuel: None,
        trace_pipeline: true,
        trait_solver_metrics: false,
        load_stdlib: Some(true),
        defines: Vec::new(),
        ffi: CliFfiOptions::default(),
        profile: None,
        configuration: "Debug".into(),
        artifacts_path: None,
        no_dependencies: false,
        no_restore: false,
        no_incremental: false,
        disable_build_servers: false,
        source_root: None,
        properties: Vec::new(),
        verbosity: Verbosity::Normal,
        telemetry: TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        framework: None,
        doc_enforcement: MissingDocsRule::default(),
    };
    assert!(logging::command_requests_trace(&traced));
    let format = Command::Format {
        inputs: vec![PathBuf::from("input.cl")],
        config: None,
        check: false,
        diff: false,
        write: true,
        stdin: false,
        stdout: false,
    };
    assert!(!logging::command_requests_trace(&format));
}

#[test]
fn canonical_target_key_covers_platforms() {
    let windows = Target::parse("x86_64-pc-windows-msvc").unwrap();
    assert_eq!(super::ffi::canonical_target_key(&windows), "windows");
    let macos = Target::parse("x86_64-apple-darwin").unwrap();
    assert_eq!(super::ffi::canonical_target_key(&macos), "macos");
    let linux = Target::parse("x86_64-unknown-linux-gnu").unwrap();
    assert_eq!(super::ffi::canonical_target_key(&linux), "linux");
    let wasi = Target::parse("x86_64-wasi").unwrap();
    assert_eq!(super::ffi::canonical_target_key(&wasi), "wasi");
    let other = Target::parse("x86_64-none").unwrap();
    assert_eq!(super::ffi::canonical_target_key(&other), "other");
}

#[test]
fn logging_helpers_cover_formatting_and_json_setup() {
    let inputs = vec![PathBuf::from("one.cl"), PathBuf::from("two.cl")];
    assert_eq!(logging::format_input_list(&[]), "<none>");
    assert_eq!(logging::format_input_list(&inputs[..1]), "one.cl");
    let summary = logging::format_input_list(&inputs);
    assert!(summary.contains("one.cl") && summary.contains("two.cl") && summary.contains(","));

    let opts = LogOptions {
        format: LogFormat::Json,
        level: LogLevel::Debug,
    };
    logging::init_logging(&opts, LogLevel::Trace);
    logging::init_logging(&opts, LogLevel::Trace);
    logging::print_trait_solver_metrics("json-init", &TraitSolverMetrics::default());
}

#[test]
fn logging_covers_metadata_variants() {
    let opts = LogOptions {
        format: LogFormat::Text,
        level: LogLevel::Info,
    };
    let ok: Result<()> = Ok(());
    let commands: Vec<Command> = vec![
        Command::Build {
            inputs: vec![PathBuf::from("input.cl")],
            manifest: None,
            workspace: None,
            output: None,
            artifacts_path: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            emit_wat: false,
            emit_obj: false,
            cpu_isa: CpuIsaConfig::baseline(),
            emit_header: false,
            emit_lib: false,
            cc1_args: Vec::new(),
            cc1_keep_temps: false,
            load_stdlib: None,
            const_eval_fuel: None,
            trace_pipeline: false,
            trait_solver_metrics: false,
            defines: Vec::new(),
            ffi: CliFfiOptions::default(),
            configuration: "Debug".to_string(),
            framework: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: crate::driver::types::Verbosity::Normal,
            telemetry: crate::driver::types::TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            doc_markdown: false,
            manifest_path: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        Command::Run {
            inputs: vec![PathBuf::from("run.cl")],
            manifest: None,
            workspace: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            cpu_isa: CpuIsaConfig::baseline(),
            const_eval_fuel: None,
            trace_pipeline: false,
            trait_solver_metrics: false,
            load_stdlib: Some(true),
            defines: Vec::new(),
            ffi: CliFfiOptions::default(),
            profile: None,
            configuration: "Debug".into(),
            artifacts_path: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: Verbosity::Normal,
            telemetry: TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            framework: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        Command::Test {
            inputs: vec![PathBuf::from("suite.cl")],
            manifest: None,
            workspace: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            cpu_isa: CpuIsaConfig::baseline(),
            const_eval_fuel: None,
            trace_pipeline: false,
            trait_solver_metrics: false,
            load_stdlib: Some(true),
            defines: Vec::new(),
            ffi: CliFfiOptions::default(),
            profile: None,
            test_options: crate::driver::types::TestOptions::default(),
            coverage: false,
            coverage_min: None,
            workspace_mode: false,
            coverage_only: false,
            configuration: "Debug".into(),
            artifacts_path: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: Verbosity::Normal,
            telemetry: TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            framework: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        Command::Format {
            inputs: vec![PathBuf::from("module.cl")],
            config: None,
            check: false,
            diff: false,
            write: true,
            stdin: false,
            stdout: false,
        },
        Command::MirDump {
            input: PathBuf::from("module.cl"),
            const_eval_fuel: None,
            trace_pipeline: false,
            trait_solver_metrics: false,
        },
        Command::Header {
            input: PathBuf::from("header.cl"),
            output: None,
            include_guard: Some("HDR".into()),
        },
        Command::Cc1 {
            input: PathBuf::from("cc1.cl"),
            output: None,
            target: Target::host(),
            extra_args: Vec::new(),
        },
        Command::ExternBind {
            header: PathBuf::from("sample.h"),
            output: PathBuf::from("bindings.cl"),
            namespace: "Tests.Interop".into(),
            library: "tests".into(),
            binding: "test".into(),
            convention: "C".into(),
            optional: false,
        },
    ];
    for cmd in commands {
        logging::log_run_start(&cmd, &opts, false);
        logging::log_run_complete(&cmd, std::time::Duration::from_millis(1), &ok);
    }
}

#[test]
fn print_report_diagnostics_writes_all_sections() {
    let mut report = sample_report();
    report.mir_lowering_diagnostics.push(LoweringDiagnostic {
        message: "lowering diagnostic".into(),
        span: None,
    });
    report.mir_verification.push(MirVerificationIssue {
        function: "Sample::Function".into(),
        errors: vec![VerifyError::EmptyBody],
    });
    let mut buffer = Vec::new();
    print_report_diagnostics_to(&report, test_format_options(), &mut buffer)
        .expect("write diagnostics");
    let output = String::from_utf8(buffer).expect("utf8");
    assert!(output.contains("parse diagnostic"));
    assert!(output.contains("type diagnostic"));
    assert!(output.contains("lowering diagnostic"));
    assert!(output.contains("verification failure"));
    assert!(output.contains("borrow diagnostic"));
    assert!(output.contains("fallible diagnostic"));
    assert!(output.contains("doc diagnostic"));
}

#[test]
fn print_report_supports_json_format() {
    let report = sample_report();
    let mut buffer = Vec::new();
    let options = FormatOptions {
        format: ErrorFormat::Json,
        color: ColorMode::Never,
        is_terminal: false,
    };
    print_report_diagnostics_to(&report, options, &mut buffer).expect("write diagnostics");
    let rendered = String::from_utf8(buffer).expect("utf8");
    let lines: Vec<_> = rendered.lines().collect();
    assert!(
        !lines.is_empty(),
        "json diagnostics should emit at least one line"
    );
    for line in lines {
        serde_json::from_str::<Value>(line).expect("line should parse as JSON diagnostic");
    }
}

#[test]
fn report_error_to_formats_parse_errors() {
    let parse_error = ParseError::new(
        "parse failure",
        vec![Diagnostic::error("expected token", Some(Span::new(2, 5)))],
    );
    let mut buffer = Vec::new();
    report_error_to(&Error::Parse(parse_error), &mut buffer).expect("write error");
    let output = String::from_utf8(buffer).expect("utf8");
    assert!(output.contains("error: parse failure"));
    assert!(output.contains("expected token"));
}

#[test]
fn report_error_to_handles_non_parse_errors() {
    let mut buffer = Vec::new();
    report_error_to(&Error::Internal("boom".into()), &mut buffer).expect("write error");
    let output = String::from_utf8(buffer).expect("utf8");
    assert!(output.contains("internal error: boom"));
    if cfg!(debug_assertions) {
        assert!(
            output.contains("stack trace:"),
            "expected stack trace in debug builds: {output}"
        );
    }
}

#[test]
fn report_error_wrapper_writes_to_stderr() {
    if std::env::var("CHIC_ENABLE_ERROR_WRAPPER_TEST").is_err() {
        eprintln!(
            "skipping report_error_wrapper_writes_to_stderr (set CHIC_ENABLE_ERROR_WRAPPER_TEST=1 to enable)"
        );
        return;
    }
    super::report_error(&Error::Internal("wrapper".into()));
}

#[test]
fn relay_run_output_replays_streams() {
    let run = RunResult {
        report: sample_report(),
        status: success_status(),
        stdout: b"stdout-data".to_vec(),
        stderr: b"stderr-data".to_vec(),
        wasm_trace: None,
    };
    relay_run_output(&run).expect("relay");
}

#[cfg(unix)]
#[test]
fn relay_run_output_to_writes_streams() {
    let run = RunResult {
        report: sample_report(),
        status: ExitStatusExt::from_raw(0),
        stdout: b"stdout".to_vec(),
        stderr: b"stderr".to_vec(),
        wasm_trace: None,
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    relay_run_output_to(&run, &mut out, &mut err).expect("relay");
    assert_eq!(out, b"stdout");
    assert_eq!(err, b"stderr");
}

#[test]
fn run_check_respects_diagnostics_flag() {
    let driver = StubDriver::default().with_check_kind(ReportKind::WithDiagnostics);
    let result = with_env_lock(|| {
        unsafe {
            env::set_var("CHIC_DIAGNOSTICS_FATAL", "1");
        }
        let outcome = run_check(
            &driver,
            vec![PathBuf::from("main.cl")],
            &Target::host(),
            ChicKind::Executable,
            None,
            false,
            false,
            Vec::new(),
            LogLevel::Info,
            test_format_options(),
        );
        unsafe {
            env::remove_var("CHIC_DIAGNOSTICS_FATAL");
        }
        outcome
    });
    assert!(
        result.is_err(),
        "diagnostics must be fatal when env opt-in set"
    );
    assert_eq!(driver.calls().check, 1);
}

#[test]
fn run_check_succeeds_without_diagnostics() {
    let driver = StubDriver::default();
    run_check(
        &driver,
        vec![PathBuf::from("main.cl")],
        &Target::host(),
        ChicKind::Executable,
        None,
        false,
        false,
        Vec::new(),
        LogLevel::Info,
        test_format_options(),
    )
    .expect("clean check runs");
    assert_eq!(driver.calls().check, 1);
}

#[test]
fn run_build_handles_clean_and_diagnostic_reports() {
    with_env_lock(|| {
        unsafe {
            env::remove_var("CHIC_DIAGNOSTICS_FATAL");
        }
        let driver = StubDriver::default().with_build_kind(ReportKind::WithDiagnostics);
        let request = dispatch_request(vec![PathBuf::from("input.cl")]);
        run_build(&driver, request.clone(), None, test_format_options()).expect("build completes");
        let clean_driver = StubDriver::default().with_build_kind(ReportKind::Clean);
        run_build(&clean_driver, request, None, test_format_options())
            .expect("clean build completes");
        assert_eq!(driver.calls().build, 1);
        assert_eq!(clean_driver.calls().build, 1);
    });
}

#[test]
fn run_run_reports_status_and_metrics() {
    let driver = StubDriver::default().with_run_status(success_status());
    let request = dispatch_request(vec![PathBuf::from("main.cl")]);
    run_run(&driver, request.clone(), None, None, test_format_options()).expect("run succeeds");
    assert_eq!(driver.calls().run, 1);

    let failing = StubDriver::default().with_run_status(failure_status());
    let failing_request = dispatch_request(vec![PathBuf::from("main.cl")]);
    let err = run_run(&failing, failing_request, None, None, test_format_options());
    assert!(err.is_err(), "non-zero status must fail");
    assert_eq!(failing.calls().run, 1);
}

#[test]
fn run_tests_surfaces_failure_and_success() {
    let failing = StubDriver::default().with_test_statuses(vec![TestStatus::Failed]);
    let request = dispatch_request(vec![PathBuf::from("suite.cl")]);
    let err = run_tests(
        &failing,
        request.clone(),
        None,
        None,
        crate::driver::types::TestOptions::default(),
        false,
        None,
        test_format_options(),
    );
    assert!(err.is_err(), "failing tests should produce error");
    assert_eq!(failing.calls().run_tests, 1);

    let passing = StubDriver::default().with_test_statuses(vec![TestStatus::Passed]);
    let passing_request = dispatch_request(vec![PathBuf::from("suite.cl")]);
    run_tests(
        &passing,
        passing_request,
        None,
        None,
        crate::driver::types::TestOptions::default(),
        false,
        None,
        test_format_options(),
    )
    .expect("passing tests should succeed");
    assert_eq!(passing.calls().run_tests, 1);
}

#[test]
fn run_mir_dump_reports_diagnostics() {
    with_env_lock(|| {
        unsafe {
            env::remove_var("CHIC_DIAGNOSTICS_FATAL");
        }
        let driver = StubDriver::default().with_mir_kind(ReportKind::WithDiagnostics);
        run_mir_dump(
            &driver,
            Path::new("module.cl"),
            None,
            false,
            false,
            LogLevel::Debug,
            test_format_options(),
        )
        .expect("mir-dump should succeed");
        assert_eq!(driver.calls().mir_dump, 1);
    });
}

#[test]
fn run_header_writes_output_for_clean_reports() {
    let driver = StubDriver::default();
    let temp = tempdir().expect("tempdir");
    let output = temp.path().join("out.h");
    run_header(
        &driver,
        Path::new("module.cl"),
        Some(output.as_path()),
        Some("TEST_GUARD"),
        LogLevel::Info,
        test_format_options(),
    )
    .expect("header must succeed");
    assert!(output.exists(), "header file must be written");
}

#[test]
fn run_header_prints_when_no_output_path() {
    let driver = StubDriver::default();
    run_header(
        &driver,
        Path::new("module.cl"),
        None,
        Some("PRINT_ONLY"),
        LogLevel::Info,
        test_format_options(),
    )
    .expect("header must print");
}

#[test]
fn run_header_rejects_diagnostics() {
    let driver = StubDriver::default().with_check_kind(ReportKind::WithDiagnostics);
    let temp = tempdir().expect("tempdir");
    let output = temp.path().join("out.h");
    let result = run_header(
        &driver,
        Path::new("module.cl"),
        Some(output.as_path()),
        None,
        LogLevel::Info,
        test_format_options(),
    );
    assert!(
        result.is_err(),
        "header generation should fail when diagnostics are present"
    );
}

#[test]
fn run_function_initialises_logging_and_dispatches() {
    let driver = StubDriver::default();
    let cli = Cli {
        command: Command::Version,
        log_options: LogOptions::DEFAULT,
        error_format: None,
    };
    super::run(&driver, cli).expect("version dispatch should succeed");
}

#[test]
fn resolve_cli_ffi_options_respects_globs_and_patterns() {
    let temp = tempdir().expect("tempdir");
    let package = temp.path().join("libsample.a");
    fs::write(&package, "ffi package").expect("write package");
    let search_root = temp.path().join("search");
    fs::create_dir_all(&search_root).expect("create search dir");
    let cli = CliFfiOptions {
        search_paths: vec![search_root],
        default_patterns: vec![FfiDefaultPattern {
            target: "any".into(),
            pattern: "*.a".into(),
        }],
        package_globs: vec![format!("{}/*.a", temp.path().display())],
    };
    let opts = resolve_cli_ffi_options(&cli, &Target::host()).expect("resolve ffi options");
    assert_eq!(opts.packages.len(), 1);
    assert!(
        opts.search_paths.iter().all(|path| path.is_absolute()),
        "search paths must be canonicalised"
    );
    assert_eq!(opts.default_pattern.as_deref(), Some("*.a"));
}

#[test]
fn resolve_cli_ffi_options_errors_on_missing_glob() {
    let cli = CliFfiOptions {
        search_paths: vec![],
        default_patterns: vec![],
        package_globs: vec!["/definitely/missing/*.a".into()],
    };
    let result = resolve_cli_ffi_options(&cli, &Target::host());
    assert!(result.is_err(), "missing globs must trigger an error");
}

#[test]
fn resolve_cli_ffi_options_prefers_platform_defaults() {
    let cli = CliFfiOptions {
        search_paths: vec![],
        default_patterns: vec![
            FfiDefaultPattern {
                target: "linux".into(),
                pattern: "linux/*.a".into(),
            },
            FfiDefaultPattern {
                target: "any".into(),
                pattern: "any/*.a".into(),
            },
        ],
        package_globs: vec![],
    };
    let opts =
        resolve_cli_ffi_options(&cli, &Target::parse("x86_64-unknown-linux-gnu").unwrap()).unwrap();
    assert_eq!(opts.default_pattern.as_deref(), Some("linux/*.a"));
}

#[test]
fn resolve_cli_ffi_options_rejects_directory_matches() {
    let temp = tempdir().expect("tempdir");
    let dir = temp.path().join("pkgdir");
    fs::create_dir_all(&dir).expect("dir");
    let cli = CliFfiOptions {
        search_paths: vec![],
        default_patterns: vec![],
        package_globs: vec![format!("{}/*", temp.path().display())],
    };
    let result = resolve_cli_ffi_options(&cli, &Target::host());
    assert!(result.is_err(), "directory matches must fail");
}

#[test]
fn run_format_reports_change_status() {
    let driver = StubDriver::default().with_format_changed(true);
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("formatme.cl");
    fs::write(&path, "class Sample {}").expect("write sample");
    run_format(
        &driver,
        FormatCommandOptions {
            inputs: vec![path.clone()],
            config_path: None,
            check: false,
            diff: false,
            write: true,
            stdin: false,
            stdout: false,
        },
    )
    .expect("format must succeed");
    assert_eq!(driver.calls().format, 1);
}

#[test]
fn run_format_reports_when_file_is_clean() {
    let driver = StubDriver::default();
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("formatme.cl");
    fs::write(&path, "class Sample {}").expect("write sample");
    run_format(
        &driver,
        FormatCommandOptions {
            inputs: vec![path.clone()],
            config_path: None,
            check: false,
            diff: false,
            write: true,
            stdin: false,
            stdout: false,
        },
    )
    .expect("format should succeed");
    assert_eq!(driver.calls().format, 1);
}

#[test]
fn file_org_diagnostic_is_emitted() {
    let mut files = FileCache::default();
    let outcome = FormatCheckOutcome {
        enforcement: FormatEnforcement::Warn,
        config: FormatConfig::default(),
        files: vec![FormattedFile {
            path: PathBuf::from("multi.cl"),
            original: "class A {} class B {}".into(),
            changed: true,
            metadata: crate::format::FormatMetadata {
                namespace: None,
                top_level_types: vec!["A".into(), "B".into()],
                types: Vec::new(),
            },
        }],
    };
    let diagnostics = build_file_organization_diagnostics(&outcome, &outcome.config, &mut files);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code.as_ref().unwrap().code, "FMT0100");
}

#[test]
fn ordering_diagnostics_are_emitted_for_members() {
    let mut files = FileCache::default();
    let mut config = FormatConfig::default();
    config.ordering.members = vec![MemberSort::Constructors, MemberSort::Fields];
    let outcome = FormatCheckOutcome {
        enforcement: FormatEnforcement::Warn,
        config: config.clone(),
        files: vec![FormattedFile {
            path: PathBuf::from("Foo.cl"),
            original: "class Foo {}".into(),
            changed: false,
            metadata: crate::format::FormatMetadata {
                namespace: None,
                top_level_types: vec!["Foo".into()],
                types: vec![TypeMetadata {
                    name: "Foo".into(),
                    kind: TypeSort::Class,
                    members: vec![MemberSort::Fields, MemberSort::Constructors],
                }],
            },
        }],
    };
    let diagnostics = build_ordering_diagnostics(&outcome, &config, &mut files);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code.as_ref().unwrap().code, "FMT0201");
}

#[test]
fn format_enforcement_errors_on_check_when_configured() {
    let driver = StubDriver::default().with_format_changed(true);
    let temp = tempdir().expect("tempdir");
    fs::write(
        temp.path().join(PROJECT_MANIFEST_BASENAME),
        "format:\n  enforce: error\n",
    )
    .expect("write manifest");
    let path = temp.path().join("needs_format.cl");
    fs::write(&path, "class Sample {}").expect("write sample");
    let result = run_check(
        &driver,
        vec![path.clone()],
        &Target::host(),
        ChicKind::Executable,
        None,
        false,
        false,
        Vec::new(),
        LogLevel::Info,
        test_format_options(),
    );
    assert!(
        result.is_err(),
        "format enforcement should fail the command"
    );
    assert_eq!(driver.calls().format, 1);
    assert_eq!(driver.calls().check, 0);
}

#[test]
fn format_enforcement_warns_but_builds() {
    with_env_lock(|| {
        unsafe {
            env::remove_var("CHIC_DIAGNOSTICS_FATAL");
        }
        let driver = StubDriver::default().with_format_changed(true);
        let temp = tempdir().expect("tempdir");
        fs::write(
            temp.path().join(PROJECT_MANIFEST_BASENAME),
            "format:\n  enforce: warn\n",
        )
        .expect("write manifest");
        let path = temp.path().join("needs_format.cl");
        fs::write(&path, "class Sample {}").expect("write sample");
        let request = dispatch_request(vec![path]);
        run_build(&driver, request, None, test_format_options())
            .expect("build should succeed with formatting warnings");
        assert_eq!(driver.calls().format, 1);
        assert_eq!(driver.calls().build, 1);
    });
}

#[test]
fn run_covers_metadata_logging_paths() {
    let driver = StubDriver::default();
    let cli = Cli {
        command: Command::Check {
            inputs: vec![PathBuf::from("main.cl")],
            target: Target::host(),
            kind: ChicKind::Executable,
            const_eval_fuel: Some(5),
            trace_pipeline: true,
            trait_solver_metrics: true,
            defines: Vec::new(),
        },
        log_options: LogOptions {
            format: LogFormat::Json,
            level: LogLevel::Debug,
        },
        error_format: None,
    };
    super::run(&driver, cli).expect("check dispatch should succeed");
    assert_eq!(driver.calls().check, 1);
}

#[test]
fn log_run_complete_reports_error_paths() {
    let result: Result<()> = Err(Error::Internal("boom".into()));
    logging::log_run_complete(
        &Command::Version,
        std::time::Duration::from_millis(5),
        &result,
    );
    logging::log_run_complete(
        &Command::Build {
            inputs: vec![PathBuf::from("main.cl")],
            manifest: None,
            workspace: None,
            output: None,
            artifacts_path: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            emit_wat: false,
            emit_obj: false,
            cpu_isa: CpuIsaConfig::baseline(),
            emit_header: false,
            emit_lib: false,
            cc1_args: Vec::new(),
            cc1_keep_temps: false,
            load_stdlib: None,
            const_eval_fuel: None,
            trace_pipeline: false,
            trait_solver_metrics: false,
            defines: Vec::new(),
            ffi: CliFfiOptions::default(),
            configuration: "Debug".to_string(),
            framework: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: crate::driver::types::Verbosity::Normal,
            telemetry: crate::driver::types::TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            doc_markdown: false,
            manifest_path: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        std::time::Duration::from_millis(5),
        &result,
    );
}

#[test]
fn dispatch_command_handles_show_spec_and_help() {
    let driver = StubDriver::default();
    dispatch_command(
        &driver,
        Command::ShowSpec,
        LogLevel::Info,
        test_format_options(),
    )
    .expect("spec renders");
    dispatch_command(
        &driver,
        Command::Help {
            topic: Some("build".into()),
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("help renders");
}

#[test]
fn dispatch_command_short_circuits_feature_gated_commands() {
    let driver = StubDriver::default();
    let (cc1_err, extern_err) = with_env_lock(|| {
        unsafe {
            env::set_var("CHIC_DISABLE_CLI_FEATURES", "1");
        }
        let cc1_err = dispatch_command(
            &driver,
            Command::Cc1 {
                input: PathBuf::from("main.cl"),
                output: None,
                target: Target::host(),
                extra_args: Vec::new(),
            },
            LogLevel::Info,
            test_format_options(),
        );
        let extern_err = dispatch_command(
            &driver,
            Command::ExternBind {
                header: PathBuf::from("sample.h"),
                output: PathBuf::from("bindings.cl"),
                namespace: "Tests.Interop".into(),
                library: "tests".into(),
                binding: "test".into(),
                convention: "C".into(),
                optional: false,
            },
            LogLevel::Info,
            test_format_options(),
        );
        unsafe {
            env::remove_var("CHIC_DISABLE_CLI_FEATURES");
        }
        (cc1_err, extern_err)
    });
    assert!(cc1_err.is_err(), "cc1 must be gated when feature disabled");
    assert!(
        extern_err.is_err(),
        "extern-bind must be gated when feature disabled"
    );
}

#[test]
fn dispatch_command_routes_mirdump_and_format() {
    let driver = StubDriver::default().with_mir_kind(ReportKind::Clean);
    let temp = tempdir().expect("tempdir");
    let format_path = temp.path().join("module.cl");
    fs::write(&format_path, "namespace Sample;").expect("write format input");
    dispatch_command(
        &driver,
        Command::MirDump {
            input: PathBuf::from("module.cl"),
            const_eval_fuel: None,
            trace_pipeline: true,
            trait_solver_metrics: true,
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("mir-dump dispatch should succeed");
    dispatch_command(
        &driver,
        Command::Format {
            inputs: vec![format_path.clone()],
            config: None,
            check: false,
            diff: false,
            write: true,
            stdin: false,
            stdout: false,
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("format dispatch should succeed");
    assert_eq!(driver.calls().mir_dump, 1);
}

#[test]
fn dispatch_command_routes_build_run_and_test_commands() {
    let driver = StubDriver::default();
    let cli_ffi = CliFfiOptions::default();
    dispatch_command(
        &driver,
        Command::Build {
            inputs: vec![PathBuf::from("input.cl")],
            manifest: None,
            workspace: None,
            output: None,
            artifacts_path: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            emit_wat: false,
            emit_obj: false,
            cpu_isa: CpuIsaConfig::baseline(),
            emit_header: false,
            emit_lib: false,
            cc1_args: Vec::new(),
            cc1_keep_temps: false,
            load_stdlib: None,
            const_eval_fuel: Some(3),
            trace_pipeline: true,
            trait_solver_metrics: true,
            defines: Vec::new(),
            ffi: cli_ffi.clone(),
            configuration: "Debug".to_string(),
            framework: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: crate::driver::types::Verbosity::Normal,
            telemetry: crate::driver::types::TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            doc_markdown: false,
            manifest_path: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("build dispatch");
    dispatch_command(
        &driver,
        Command::Run {
            inputs: vec![PathBuf::from("input.cl")],
            manifest: None,
            workspace: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            cpu_isa: CpuIsaConfig::baseline(),
            const_eval_fuel: None,
            trace_pipeline: true,
            trait_solver_metrics: true,
            load_stdlib: Some(true),
            defines: Vec::new(),
            ffi: cli_ffi.clone(),
            profile: None,
            configuration: "Debug".into(),
            artifacts_path: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: Verbosity::Normal,
            telemetry: TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            framework: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("run dispatch");
    dispatch_command(
        &driver,
        Command::Test {
            inputs: vec![PathBuf::from("input.cl")],
            manifest: None,
            workspace: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            runtime_backend: RuntimeBackend::Chic,
            cpu_isa: CpuIsaConfig::baseline(),
            const_eval_fuel: None,
            trace_pipeline: true,
            trait_solver_metrics: true,
            load_stdlib: Some(true),
            defines: Vec::new(),
            ffi: cli_ffi,
            profile: None,
            test_options: crate::driver::types::TestOptions::default(),
            coverage: false,
            coverage_min: None,
            workspace_mode: false,
            coverage_only: false,
            configuration: "Debug".into(),
            artifacts_path: None,
            no_dependencies: false,
            no_restore: false,
            no_incremental: false,
            disable_build_servers: false,
            source_root: None,
            properties: Vec::new(),
            verbosity: Verbosity::Normal,
            telemetry: TelemetrySetting::Auto,
            version_suffix: None,
            nologo: false,
            force: false,
            interactive: false,
            self_contained: None,
            framework: None,
            doc_enforcement: MissingDocsRule::default(),
        },
        LogLevel::Info,
        test_format_options(),
    )
    .expect("test dispatch");
    let calls = driver.calls();
    assert_eq!(calls.build, 1);
    assert_eq!(calls.run, 1);
    assert_eq!(calls.run_tests, 1);
}

#[test]
fn run_cc1_respects_output_only_fake_mode() {
    let temp = tempdir().expect("tempdir");
    let nested = temp.path().join("cc1").join("out.s");
    with_env_lock(|| {
        unsafe {
            env::set_var("CHIC_FAKE_CC1_OUTPUT_ONLY", "1");
        }
        super::commands::run_cc1(
            Path::new("main.cl"),
            Some(nested.as_path()),
            &Target::host(),
            &[],
        )
        .expect("cc1 should short-circuit");
        unsafe {
            env::remove_var("CHIC_FAKE_CC1_OUTPUT_ONLY");
        }
    });
    assert!(
        nested.parent().unwrap().exists(),
        "cc1 output directory should be created"
    );
}

#[test]
fn cc1_and_extern_bind_support_fake_modes() {
    let driver = StubDriver::default();
    with_env_lock(|| {
        unsafe {
            env::set_var("CHIC_FAKE_CC1", "1");
        }
        dispatch_command(
            &driver,
            Command::Cc1 {
                input: PathBuf::from("main.cl"),
                output: None,
                target: Target::host(),
                extra_args: Vec::new(),
            },
            LogLevel::Info,
            test_format_options(),
        )
        .expect("cc1 dispatch");
        unsafe {
            env::remove_var("CHIC_FAKE_CC1");
            env::set_var("CHIC_FAKE_EXTERN_BIND", "1");
        }
        dispatch_command(
            &driver,
            Command::ExternBind {
                header: PathBuf::from("sample.h"),
                output: PathBuf::from("bindings.cl"),
                namespace: "Tests.Interop".into(),
                library: "tests".into(),
                binding: "test".into(),
                convention: "C".into(),
                optional: false,
            },
            LogLevel::Info,
            test_format_options(),
        )
        .expect("extern-bind dispatch");
        unsafe {
            env::remove_var("CHIC_FAKE_EXTERN_BIND");
        }
    });
}
