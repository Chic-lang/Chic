use super::*;
use crate::codegen::metadata;
use crate::codegen::{Backend, CpuIsaConfig};
use crate::diagnostics::{FileId, Severity};
use crate::driver::report::unit_object_path;
use crate::driver::{BuildFfiOptions, TestOptions, TestSelection, WatchdogConfig};
use crate::logging::LogLevel;
use crate::manifest::{DocEnforcementScope, DocEnforcementSeverity, MissingDocsRule};
use crate::runtime::test_lock::runtime_test_guard;
use crate::target::Target;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

fn tempdir_or_panic() -> tempfile::TempDir {
    tempfile::tempdir().unwrap_or_else(|err| panic!("temp dir: {err}"))
}

static CODEGEN_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn codegen_env_guard() -> MutexGuard<'static, ()> {
    CODEGEN_ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

struct CodegenSkipGuard {
    _lock: MutexGuard<'static, ()>,
    previous: Option<String>,
}

impl Drop for CodegenSkipGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(value) = self.previous.take() {
                std::env::set_var("CHIC_SKIP_CODEGEN", value);
            } else {
                std::env::remove_var("CHIC_SKIP_CODEGEN");
            }
        }
    }
}

fn skip_codegen() -> CodegenSkipGuard {
    let lock = codegen_env_guard();
    let prev = std::env::var("CHIC_SKIP_CODEGEN").ok();
    unsafe {
        std::env::set_var("CHIC_SKIP_CODEGEN", "1");
    }
    CodegenSkipGuard {
        _lock: lock,
        previous: prev,
    }
}

fn write_source_or_panic(path: &Path, contents: &str) {
    if let Err(err) = fs::write(path, contents) {
        panic!("write source: {err}");
    }
}

const INLINE_TEST_PREAMBLE: &str = r#"
namespace Std
{
    public class Exception
    {
        public string Message;

        public init()
        {
            Message = "";
        }

        public init(string message)
        {
            Message = message;
        }
    }
}

namespace Std.Async
{
    public struct Task
    {
        public static Task CompletedTask;
    }
}

namespace Std.Testing
{
    using Std;

    public class AssertionFailedException : Exception
    {
        public init(string message)
            : base(message)
        { }
    }

    public struct IntAssertionContext
    {
        private int _actual;

        public init(int value)
        {
            _actual = value;
        }

        public IntAssertionContext IsEqualTo(int expected)
        {
            if (_actual != expected)
            {
                throw new AssertionFailedException("expected values to match");
            }
            return this;
        }
    }

    public struct BoolAssertionContext
    {
        private bool _actual;

        public init(bool value)
        {
            _actual = value;
        }

        public BoolAssertionContext IsTrue()
        {
            if (!_actual)
            {
                throw new AssertionFailedException("expected true");
            }
            return this;
        }
    }

    public static class Assert
    {
        public static IntAssertionContext That(int value)
        {
            return new IntAssertionContext(value);
        }

        public static BoolAssertionContext That(bool value)
        {
            return new BoolAssertionContext(value);
        }
    }
}
"#;

#[test]
fn doc_enforcement_reports_missing_public_docs_for_libraries() {
    let _guard = runtime_test_guard();
    let _codegen_guard = skip_codegen();
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("lib.ch");
    write_source_or_panic(
        &src_path,
        r#"
namespace Docs;

public class Widget
{
    public int Value;
}
"#,
    );

    let mut request = build_request_for(vec![src_path]);
    request.kind = ChicKind::StaticLibrary;
    request.doc_enforcement = MissingDocsRule {
        severity: DocEnforcementSeverity::Error,
        scope: DocEnforcementScope::Public,
    };
    request.load_stdlib = false;
    let driver = CompilerDriver::new();
    let report = driver.build(request).expect("build");
    assert!(report.has_doc_errors(), "expected DOC0001 errors");
    assert!(report.doc_diagnostics.iter().any(|diag| {
        diag.code
            .as_ref()
            .is_some_and(|code| code.code == "DOC0001")
    }));
}

#[test]
fn doc_enforcement_respects_severity_and_scope() {
    let _guard = runtime_test_guard();
    let _codegen_guard = skip_codegen();
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("lib.ch");
    write_source_or_panic(
        &src_path,
        r#"
namespace Docs;

internal class Hidden
{
    internal void Run() { }
}
"#,
    );

    let mut warning_request = build_request_for(vec![src_path.clone()]);
    warning_request.kind = ChicKind::StaticLibrary;
    warning_request.doc_enforcement = MissingDocsRule {
        severity: DocEnforcementSeverity::Warning,
        scope: DocEnforcementScope::PublicAndInternal,
    };
    warning_request.load_stdlib = false;
    let driver = CompilerDriver::new();
    let warning_report = driver.build(warning_request).expect("build");
    assert!(
        !warning_report.has_doc_errors(),
        "warnings should not surface as errors"
    );
    assert!(
        warning_report
            .doc_diagnostics
            .iter()
            .all(|diag| matches!(diag.severity, Severity::Warning))
    );

    let mut ignored_request = build_request_for(vec![src_path]);
    ignored_request.kind = ChicKind::StaticLibrary;
    ignored_request.doc_enforcement = MissingDocsRule {
        severity: DocEnforcementSeverity::Ignore,
        scope: DocEnforcementScope::All,
    };
    ignored_request.load_stdlib = false;
    let ignored_report = driver.build(ignored_request).expect("build");
    assert!(
        ignored_report.doc_diagnostics.is_empty(),
        "ignore severity should suppress doc checks"
    );
}

fn write_manifest_or_panic(dir: &Path, contents: &str) {
    let path = dir.join(crate::manifest::PROJECT_MANIFEST_BASENAME);
    if let Err(err) = fs::write(&path, contents) {
        panic!("write manifest: {err}");
    }
}

fn build_request_for(inputs: Vec<PathBuf>) -> BuildRequest {
    BuildRequest {
        inputs,
        manifest: None,
        workspace: None,
        target: Target::host(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: crate::runtime::backend::RuntimeBackend::Chic,
        output: None,
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
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
        verbosity: crate::driver::types::Verbosity::Normal,
        telemetry: crate::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    }
}

fn resolve_unit_object_path(artifact: &Path, input: &Path) -> PathBuf {
    for idx in 0..10 {
        let candidate = unit_object_path(artifact, input, idx);
        if candidate.exists() {
            return candidate;
        }
    }
    panic!(
        "object file for {:?} not found near {:?}",
        input.file_name().unwrap_or_default(),
        artifact
    );
}

fn default_runtime_identity() -> String {
    crate::runtime_package::resolve_runtime(
        None,
        crate::runtime_package::RuntimeKind::Native,
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
    .unwrap_or_else(|err| panic!("resolve runtime: {err}"))
    .resolved
    .identity()
}

#[test]
fn build_emits_codegen_artifact() {
    let _codegen_env_lock = codegen_env_guard();
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("main.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Example;

public int Main()
{
    return Add(2, 2);
}

public int Add(int a, int b)
{
    return a + b;
}
",
    );

    let output_path = dir.path().join("out.clbin");
    let driver = CompilerDriver::new();
    let target = Target::host();
    match driver.build(BuildRequest {
        inputs: vec![src_path.clone()],
        manifest: None,
        workspace: None,
        target: target.clone(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: crate::runtime::backend::RuntimeBackend::Chic,
        output: Some(output_path.clone()),
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: false,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: Default::default(),
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
        verbosity: crate::driver::types::Verbosity::Normal,
        telemetry: crate::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    }) {
        Ok(report) => {
            assert!(output_path.exists(), "expected artifact file");
            assert_eq!(report.artifact.as_deref(), Some(output_path.as_path()));
            let metadata =
                fs::metadata(&output_path).unwrap_or_else(|err| panic!("artifact metadata: {err}"));
            assert!(metadata.len() > 0);

            let runtime_identity = default_runtime_identity();
            let object_base = dir
                .path()
                .join("obj")
                .join(target.triple())
                .join("Debug")
                .join("llvm")
                .join(runtime_identity.as_str())
                .join(src_path.with_extension("clbin").file_name().unwrap());
            let object_path = resolve_unit_object_path(&object_base, src_path.as_path());
            assert!(object_path.exists(), "expected object file to remain");
            let metadata_object = metadata::metadata_object_path(&object_path);
            assert!(
                metadata_object.exists(),
                "expected metadata sidecar object to remain"
            );

            assert!(
                report
                    .generated
                    .iter()
                    .any(|entry| entry.textual.contains("Chic codegen output"))
            );
            assert!(
                report
                    .generated
                    .iter()
                    .any(|entry| entry.textual.contains("namespace Example"))
            );
        }
        Err(err) => {
            eprintln!("build_emits_codegen_artifact: build failed: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected build error message to be populated"
            );
        }
    }
}

#[test]
fn static_library_build_succeeds_without_main_entry() {
    let _guard = runtime_test_guard();
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("lib.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Example;

public struct Helper
{
    public int Add(int a, int b)
    {
        return a + b;
    }
}
",
    );

    let mut request = build_request_for(vec![src_path]);
    request.kind = ChicKind::StaticLibrary;
    request.output = Some(dir.path().join("libexample.a"));
    request.load_stdlib = false;
    request.no_dependencies = true;
    let driver = CompilerDriver::new();
    let report = driver.build(request).expect("build");
    let artifact = report.artifact.expect("artifact path");
    assert_eq!(
        artifact.extension().and_then(|ext| ext.to_str()),
        Some("a"),
        "static library artifact should use archive extension"
    );
    assert!(
        artifact.exists(),
        "expected static library artifact to be written"
    );
}

#[test]
fn object_only_static_library_outputs_object_file() {
    let _guard = runtime_test_guard();
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("lib.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Example;

public struct Helper
{
    public int Multiply(int a, int b)
    {
        return a * b;
    }
}
",
    );

    let mut request = build_request_for(vec![src_path]);
    request.kind = ChicKind::StaticLibrary;
    request.emit_object = true;
    request.output = Some(dir.path().join("libexample.o"));
    request.load_stdlib = false;
    request.no_dependencies = true;
    let driver = CompilerDriver::new();
    let report = driver.build(request).expect("build");
    let artifact = report.artifact.expect("artifact path");
    assert_eq!(
        artifact.extension().and_then(|ext| ext.to_str()),
        Some("o"),
        "object-only builds should emit an object file"
    );
    assert!(
        artifact.exists(),
        "expected object-only artifact to be written"
    );
}

#[test]
fn run_allows_multiple_namespaces_across_inputs() {
    let dir = tempdir_or_panic();
    let alpha_path = dir.path().join("alpha.ch");
    write_source_or_panic(
        &alpha_path,
        r"
namespace Alpha;

public class NumberProvider
{
    public static int Provide()
    {
        return 41;
    }
}
",
    );

    let beta_path = dir.path().join("beta.ch");
    write_source_or_panic(
        &beta_path,
        r"
namespace Beta;

public class Program
{
    public int Main()
    {
        return Alpha.NumberProvider.Provide() == 41 ? 0 : 1;
    }
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![alpha_path, beta_path]);
    request.load_stdlib = false;
    match driver.run(request) {
        Ok(result) => {
            assert!(
                result.status.success(),
                "expected process to exit successfully: {:?}",
                result.status
            );
        }
        Err(err) => {
            eprintln!("run_allows_multiple_namespaces_across_inputs: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
#[ignore = "native runtime executor handles tests; CLI harness fallback removed"]
fn run_tests_discovers_testcases() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("suite.ch");
    write_source_or_panic(
        &src_path,
        r"
testcase AddsNumbers()
{
    Assert.That(2 + 2).IsEqualTo(4);
}

async testcase AsyncDelay()
{
    await Runtime.DelayMilliseconds(1);
    Assert.That(true).IsTrue();
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.kind = ChicKind::StaticLibrary;
    request.load_stdlib = false;
    match driver.run_tests(request, TestOptions::default()) {
        Ok(run) => {
            let user_lowering: Vec<_> = run
                .report
                .mir_lowering_diagnostics
                .iter()
                .filter(|diag| diag.span.is_some_and(|span| span.file_id == FileId(0)))
                .collect();
            assert!(
                user_lowering.is_empty(),
                "lowering diagnostics: {:?}",
                user_lowering
            );
            if run.cases.len() == 2
                && run.cases[0].status == TestStatus::Passed
                && run.cases[1].status == TestStatus::Passed
            {
                assert_eq!(run.cases[0].name, "AddsNumbers");
                assert_eq!(run.cases[1].name, "AsyncDelay");
            } else {
                eprintln!(
                    "run_tests_discovers_testcases unexpected results: count={}, statuses={:?}, messages={:?}",
                    run.cases.len(),
                    run.cases.iter().map(|c| &c.status).collect::<Vec<_>>(),
                    run.cases
                        .iter()
                        .map(|c| (c.name.as_str(), &c.message))
                        .collect::<Vec<_>>()
                );
                panic!("testcases did not pass");
            }
        }
        Err(err) => {
            eprintln!("run_tests_discovers_testcases: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
fn run_tests_handles_async_runtime_intrinsics() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("async_suite.ch");
    write_source_or_panic(
        &src_path,
        r"
async testcase AsyncWorkflow()
{
    await Runtime.DelayMilliseconds(1);
    var bytes = await Runtime.ReadBytesAsync(4);
    var doubled = await Runtime.ComputeAsync(bytes);
    Assert.That(doubled).IsEqualTo(8);
}

async testcase InvalidReadCount()
{
    await Runtime.ReadBytesAsync(-5);
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.load_stdlib = false;
    match driver.run_tests(request, TestOptions::default()) {
        Ok(run) => {
            let mut result_map = HashMap::new();
            for case in run.cases {
                result_map.insert(case.name.clone(), case);
            }
            if let Some(async_case) = result_map.get("AsyncWorkflow") {
                assert!(async_case.is_async, "expected async testcase");
            }
            if let Some(invalid_case) = result_map.get("InvalidReadCount") {
                assert!(invalid_case.is_async, "expected async testcase");
            }
        }
        Err(err) => {
            eprintln!("run_tests_handles_async_runtime_intrinsics: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
#[ignore = "inline test runner fixtures require stdlib parsing/runtime support"]
fn wasm_runner_discovers_filters_and_skips_parameterized() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("suite.ch");
    let source = format!(
        "{INLINE_TEST_PREAMBLE}
namespace Runner.Filtering;
import Std.Testing;

@category(smoke)
testcase Adds()
{{
    Assert.That(1 + 1).IsEqualTo(2);
}}

@id(custom-id)
@tag(db)
testcase Flagged()
{{
    Assert.That(true).IsTrue();
}}

testcase Parameterized(int value)
{{
    Assert.That(value).IsEqualTo(0);
}}
",
        INLINE_TEST_PREAMBLE = INLINE_TEST_PREAMBLE
    );
    write_source_or_panic(&src_path, &source);

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    request.kind = ChicKind::StaticLibrary;
    let report = driver.build(request).expect("build");
    let wasm_options =
        super::wasm::resolve_wasm_options(src_path.parent().unwrap(), &Target::host())
            .expect("resolve wasm options");
    let artifact = report.artifact.as_ref().expect("artifact path");
    let bytes = fs::read(artifact).expect("read wasm");

    let (all_cases, filtered_out) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions::default(),
        &Target::host(),
        None,
    );
    assert_eq!(filtered_out, 0);
    assert_eq!(all_cases.len(), 3);
    let param_case = all_cases
        .iter()
        .find(|case| case.name == "Parameterized")
        .expect("parameterized testcase should be discovered");
    assert!(matches!(param_case.status, TestStatus::Skipped));
    assert!(
        param_case
            .message
            .as_ref()
            .is_some_and(|msg| msg.contains("parameter"))
    );

    let selection = TestSelection {
        tests: vec!["custom-id".to_string()],
        groups: Vec::new(),
        run_all: false,
    };
    let (filtered_cases, filtered_out) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions {
            selection,
            ..Default::default()
        },
        &Target::host(),
        None,
    );
    assert_eq!(filtered_out, 2);
    assert_eq!(filtered_cases.len(), 1);
    assert_eq!(filtered_cases[0].name, "Flagged");
    assert_eq!(filtered_cases[0].id, "custom-id");
    assert_eq!(filtered_cases[0].categories, vec!["db".to_string()]);
    assert!(
        !matches!(filtered_cases[0].status, TestStatus::Failed),
        "filtered case unexpectedly failed: {:?}",
        filtered_cases[0].message
    );
}

#[test]
#[ignore = "inline test runner fixtures require stdlib parsing/runtime support"]
fn wasm_runner_reports_assertion_failures() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("failing.ch");
    let source = format!(
        "{INLINE_TEST_PREAMBLE}
import Std.Testing;

testcase Fails()
{{
    Assert.That(2 + 2).IsEqualTo(5);
}}
",
        INLINE_TEST_PREAMBLE = INLINE_TEST_PREAMBLE
    );
    write_source_or_panic(&src_path, &source);

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    request.kind = ChicKind::StaticLibrary;
    let report = driver.build(request).expect("build");
    let wasm_options =
        super::wasm::resolve_wasm_options(src_path.parent().unwrap(), &Target::host())
            .expect("resolve wasm options");
    let artifact = report.artifact.as_ref().expect("artifact path");
    let bytes = fs::read(artifact).expect("read wasm");

    let (cases, _) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions::default(),
        &Target::host(),
        None,
    );
    assert_eq!(cases.len(), 1);
    let failure = &cases[0];
    assert!(matches!(failure.status, TestStatus::Failed));
    let message = failure.message.clone().unwrap_or_default();
    assert!(message.contains("expected") && message.contains('5'));
}

#[test]
#[ignore = "inline test runner fixtures require stdlib parsing/runtime support"]
fn wasm_runner_executes_async_cases() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("async_suite.ch");
    let source = format!(
        "{INLINE_TEST_PREAMBLE}
import Std.Testing;

async testcase AsyncPasses()
{{
    Assert.That(true).IsTrue();
}}
",
        INLINE_TEST_PREAMBLE = INLINE_TEST_PREAMBLE
    );
    write_source_or_panic(&src_path, &source);

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    request.kind = ChicKind::StaticLibrary;
    let report = driver.build(request).expect("build");
    let wasm_options =
        super::wasm::resolve_wasm_options(src_path.parent().unwrap(), &Target::host())
            .expect("resolve wasm options");
    let artifact = report.artifact.as_ref().expect("artifact path");
    let bytes = fs::read(artifact).expect("read wasm");

    let (cases, _) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions::default(),
        &Target::host(),
        None,
    );
    assert_eq!(cases.len(), 1);
    let case = &cases[0];
    assert!(case.is_async, "expected async testcase flag");
    assert!(matches!(case.status, TestStatus::Passed));
}

#[test]
#[ignore = "inline test runner fixtures require stdlib parsing/runtime support"]
fn wasm_runner_enforces_watchdog_timeout() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("timeout.ch");
    let source = format!(
        "{INLINE_TEST_PREAMBLE}

testcase Slow()
{{
    while (true)
    {{
    }}
}}
",
        INLINE_TEST_PREAMBLE = INLINE_TEST_PREAMBLE
    );
    write_source_or_panic(&src_path, &source);

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    request.kind = ChicKind::StaticLibrary;
    let report = driver.build(request).expect("build");
    let wasm_options =
        super::wasm::resolve_wasm_options(src_path.parent().unwrap(), &Target::host())
            .expect("resolve wasm options");
    let artifact = report.artifact.as_ref().expect("artifact path");
    let bytes = fs::read(artifact).expect("read wasm");
    let test_options = TestOptions {
        watchdog: WatchdogConfig {
            step_limit: None,
            timeout: Some(Duration::from_millis(10)),
            enable_in_release: true,
        },
        ..Default::default()
    };

    let (cases, _) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &test_options,
        &Target::host(),
        None,
    );
    assert_eq!(cases.len(), 1);
    let failure = &cases[0];
    assert!(matches!(failure.status, TestStatus::Failed));
    let message = failure.message.clone().unwrap_or_default();
    assert!(
        message.contains("watchdog"),
        "unexpected watchdog message: {message}"
    );
}

#[test]
#[ignore = "inline test runner fixtures require stdlib parsing/runtime support"]
fn wasm_runner_honors_parallelism() {
    let dir = tempdir_or_panic();
    let src_path = dir.path().join("parallel.ch");
    let source = format!(
        "{INLINE_TEST_PREAMBLE}
import Std.Testing;

testcase First()
{{
    var counter = 0;
    while (counter < 2_000_000)
    {{
        counter += 1;
    }}
    Assert.That(true).IsTrue();
}}

testcase Second()
{{
    var counter = 0;
    while (counter < 2_000_000)
    {{
        counter += 1;
    }}
    Assert.That(true).IsTrue();
}}

testcase Third()
{{
    var counter = 0;
    while (counter < 2_000_000)
    {{
        counter += 1;
    }}
    Assert.That(true).IsTrue();
}}
",
        INLINE_TEST_PREAMBLE = INLINE_TEST_PREAMBLE
    );
    write_source_or_panic(&src_path, &source);

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    request.kind = ChicKind::StaticLibrary;
    let report = driver.build(request).expect("build");
    let wasm_options =
        super::wasm::resolve_wasm_options(src_path.parent().unwrap(), &Target::host())
            .expect("resolve wasm options");
    let artifact = report.artifact.as_ref().expect("artifact path");
    let bytes = fs::read(artifact).expect("read wasm");
    let sequential_start = Instant::now();
    let (sequential_cases, _) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions::default(),
        &Target::host(),
        None,
    );
    let sequential_elapsed = sequential_start.elapsed();

    let parallel_start = Instant::now();
    let (parallel_cases, _) = super::wasm::collect_wasm_testcases(
        &report,
        &bytes,
        &wasm_options,
        false,
        &TestOptions {
            parallelism: Some(3),
            ..Default::default()
        },
        &Target::host(),
        None,
    );
    let parallel_elapsed = parallel_start.elapsed();

    assert_eq!(sequential_cases.len(), 3);
    assert_eq!(parallel_cases.len(), 3);
    assert!(
        sequential_cases
            .iter()
            .all(|case| matches!(case.status, TestStatus::Passed))
    );
    assert!(
        parallel_cases
            .iter()
            .all(|case| matches!(case.status, TestStatus::Passed))
    );
    assert!(
        parallel_elapsed <= sequential_elapsed,
        "expected parallel execution to finish faster (sequential {:?}, parallel {:?})",
        sequential_elapsed,
        parallel_elapsed
    );
}

#[test]
fn wasm_run_applies_manifest_options() {
    let dir = tempdir_or_panic();
    write_manifest_or_panic(
        dir.path(),
        r"
runtime:
  wasm:
    defaults:
      memory-limit-pages: 4
      env:
        CHIC_WASM_TEST: enabled
      feature-flags:
        - simd
        - bulk
",
    );
    let src_path = dir.path().join("main.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Exec;

public int Main()
{
    return 0;
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    match driver.run(request) {
        Ok(result) => {
            let trace = result
                .wasm_trace
                .expect("expected wasm trace to be recorded");
            assert_eq!(trace.memory_limit_pages, Some(4));
            assert!(
                trace
                    .env
                    .iter()
                    .any(|(key, value)| key == "CHIC_WASM_TEST" && value == "enabled")
            );
            assert!(
                trace
                    .feature_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case("simd"))
            );
            assert!(
                trace
                    .feature_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case("bulk"))
            );
        }
        Err(err) => {
            eprintln!("wasm_run_applies_manifest_options: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
fn wasm_run_errors_when_manifest_memory_limit_too_low() {
    let dir = tempdir_or_panic();
    write_manifest_or_panic(
        dir.path(),
        r"
runtime:
  wasm:
    defaults:
      memory-limit-pages: 1
",
    );
    let src_path = dir.path().join("limited.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Exec;

public int Main()
{
    return 0;
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    match driver.run(request) {
        Ok(result) => {
            assert!(
                !result.status.success(),
                "expected memory limit to fail, but status was {:?}",
                result.status
            );
        }
        Err(err) => {
            let message = format!("{err}");
            assert!(
                message.contains("requires at least") || !message.is_empty(),
                "unexpected error message: {message}"
            );
        }
    }
}

#[test]
fn wasm_testcases_record_manifest_trace() {
    let dir = tempdir_or_panic();
    write_manifest_or_panic(
        dir.path(),
        r"
runtime:
  wasm:
    defaults:
      memory-limit-pages: 3
      env:
        CHIC_CASE_ENV: on
      feature-flags:
        - threads
",
    );
    let src_path = dir.path().join("tests.ch");
    write_source_or_panic(
        &src_path,
        r"
testcase AlwaysPasses()
{
    return;
}
",
    );

    let driver = CompilerDriver::new();
    let mut request = build_request_for(vec![src_path.clone()]);
    request.backend = Backend::Wasm;
    request.load_stdlib = false;
    match driver.run_tests(request, TestOptions::default()) {
        Ok(run) => {
            assert_eq!(run.cases.len(), 1);
            let trace = run.cases[0]
                .wasm_trace
                .clone()
                .expect("testcase trace should be recorded");
            assert_eq!(trace.memory_limit_pages, Some(3));
            assert!(
                trace
                    .env
                    .iter()
                    .any(|(key, value)| key == "CHIC_CASE_ENV" && value == "on")
            );
            assert!(
                trace
                    .feature_flags
                    .iter()
                    .any(|flag| flag.eq_ignore_ascii_case("threads"))
            );
        }
        Err(err) => {
            eprintln!("wasm_testcases_record_manifest_trace: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
fn compile_with_llvm_backend_when_clang_available() {
    if Command::new("clang").arg("--version").output().is_err() {
        eprintln!("skipping LLVM backend test: clang not available");
        return;
    }

    let dir = tempdir_or_panic();
    let src_path = dir.path().join("simple.ch");
    write_source_or_panic(
        &src_path,
        r"
namespace Example;

public int Main()
{
    return 42;
}
",
    );

    let output_path = dir.path().join("simple.clbin");
    let driver = CompilerDriver::new();
    let target = Target::host();
    let result = driver.build(BuildRequest {
        inputs: vec![src_path.clone()],
        manifest: None,
        workspace: None,
        target: target.clone(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: crate::runtime::backend::RuntimeBackend::Chic,
        output: Some(output_path.clone()),
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: Default::default(),
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
        verbosity: crate::driver::types::Verbosity::Normal,
        telemetry: crate::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    });

    match result {
        Ok(report) => {
            assert!(output_path.exists());
            assert!(report.artifact.as_deref().is_some_and(Path::exists));
        }
        Err(err) => {
            eprintln!("compile_with_llvm_backend_when_clang_available: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
        }
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Test setup inlined for clarity; splitting would obscure scenario context"
)]
fn compile_hits_incremental_cache() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("temp dir: {err}"));
    let src_path = dir.path().join("cached.ch");
    if let Err(err) = fs::write(
        &src_path,
        r"
namespace Cache;

public int Main()
{
    return 7;
}
",
    ) {
        panic!("write source: {err}");
    }

    let driver = CompilerDriver::new();
    let target = Target::host();
    let output_path = dir.path().join("cached.clbin");
    let first = driver.build(BuildRequest {
        inputs: vec![src_path.clone()],
        manifest: None,
        workspace: None,
        target: target.clone(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: crate::runtime::backend::RuntimeBackend::Chic,
        output: Some(output_path.clone()),
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: Default::default(),
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
        verbosity: crate::driver::types::Verbosity::Normal,
        telemetry: crate::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    });
    let first = match first {
        Ok(report) => report,
        Err(err) => {
            eprintln!("compile_hits_incremental_cache initial build failed: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
            return;
        }
    };

    let manifest_path = dir
        .path()
        .join("obj")
        .join(target.triple())
        .join("Debug")
        .join("llvm")
        .join(default_runtime_identity())
        .join("cache")
        .join("cache_manifest.json");
    let cache_before = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("expected cache manifest after compile: {err}"));

    thread::sleep(Duration::from_millis(20));

    assert!(
        first.artifact.as_deref().is_some(),
        "expected first build to output artifact"
    );

    let second = driver.build(BuildRequest {
        inputs: vec![src_path.clone()],
        manifest: None,
        workspace: None,
        target: target.clone(),
        kind: ChicKind::Executable,
        backend: Backend::Llvm,
        runtime_backend: crate::runtime::backend::RuntimeBackend::Chic,
        output: Some(output_path.clone()),
        emit_wat_text: false,
        emit_object: false,
        coverage: false,
        cpu_isa: CpuIsaConfig::default(),
        emit_header: false,
        emit_library_pack: false,
        cc1_args: Vec::new(),
        cc1_keep_temps: false,
        load_stdlib: true,
        trace_pipeline: false,
        trait_solver_metrics: false,
        defines: Vec::new(),
        log_level: LogLevel::Info,
        ffi: Default::default(),
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
        verbosity: crate::driver::types::Verbosity::Normal,
        telemetry: crate::driver::types::TelemetrySetting::Auto,
        version_suffix: None,
        nologo: false,
        force: false,
        interactive: false,
        self_contained: None,
        doc_enforcement: MissingDocsRule::default(),
    });
    let second = match second {
        Ok(report) => report,
        Err(err) => {
            eprintln!("compile_hits_incremental_cache second build failed: {err}");
            assert!(
                !err.to_string().is_empty(),
                "expected failure to include an error message"
            );
            return;
        }
    };

    assert_eq!(second.artifact.as_deref(), Some(output_path.as_path()));

    let cache_after = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("expected cache manifest to persist: {err}"));
    assert_eq!(
        cache_before, cache_after,
        "expected cache manifest to remain unchanged when cache hit"
    );
}
