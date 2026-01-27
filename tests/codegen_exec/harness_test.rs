use std::path::PathBuf;

use chic::codegen::{Backend, CpuIsaConfig};
use chic::driver::{BuildFfiOptions, BuildRequest, TestRun, TestStatus};
use chic::logging::LogLevel;
use chic::manifest::MissingDocsRule;
use chic::{ChicKind, Target};
use tempfile::NamedTempFile;

use super::harness_core::{BuildRunner, ExecHarness, HarnessError, RealRunner};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCaseResult {
    pub name: String,
    pub status: TestStatus,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestRunResult {
    pub cases: Vec<TestCaseResult>,
}

impl From<TestRun> for TestRunResult {
    fn from(run: TestRun) -> Self {
        let cases = run
            .cases
            .into_iter()
            .map(|case| TestCaseResult {
                name: case.name,
                status: case.status,
                message: case.message,
            })
            .collect();
        TestRunResult { cases }
    }
}

pub struct TestRunRequest {
    pub inputs: Vec<PathBuf>,
    pub kind: ChicKind,
    pub backend: Backend,
}

pub trait TestRunner: BuildRunner {
    fn run_tests(&self, request: TestRunRequest) -> Result<TestRunResult, HarnessError>;
}

impl TestRunner for RealRunner {
    fn run_tests(&self, request: TestRunRequest) -> Result<TestRunResult, HarnessError> {
        let driver = chic::driver::CompilerDriver::new();
        let run = driver
            .run_tests(
                BuildRequest {
                    inputs: request.inputs.clone(),
                    manifest: None,
                    workspace: None,
                    target: Target::host(),
                    kind: request.kind,
                    backend: request.backend,
                    runtime_backend: chic::runtime::backend::RuntimeBackend::Chic,
                    output: None,
                    run_timeout: None,
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
                    verbosity: chic::driver::types::Verbosity::Normal,
                    telemetry: chic::driver::types::TelemetrySetting::Auto,
                    version_suffix: None,
                    nologo: false,
                    force: false,
                    interactive: false,
                    self_contained: None,
                    doc_enforcement: MissingDocsRule::default(),
                },
                chic::driver::TestOptions::default(),
            )
            .map_err(HarnessError::build)?;
        Ok(TestRunResult::from(run))
    }
}

impl<R: TestRunner> ExecHarness<R> {
    pub fn with_cpu_isa(mut self, cpu_isa: CpuIsaConfig) -> Self {
        self.cpu_isa = cpu_isa;
        self
    }

    pub fn run_tests(&self, program: &str) -> Result<TestRunResult, HarnessError> {
        self.run_tests_with_inputs(program, &[])
    }

    pub fn run_tests_with_inputs(
        &self,
        program: &str,
        extra_inputs: &[PathBuf],
    ) -> Result<TestRunResult, HarnessError> {
        self.guard()?;
        let source = NamedTempFile::new().map_err(HarnessError::build)?;
        std::fs::write(source.path(), program).map_err(HarnessError::build)?;
        let mut inputs = Vec::with_capacity(1 + extra_inputs.len());
        inputs.push(source.path().to_path_buf());
        inputs.extend_from_slice(extra_inputs);
        let request = TestRunRequest {
            inputs,
            kind: ChicKind::StaticLibrary,
            backend: self.backend.into(),
        };
        self.runner.run_tests(request)
    }
}
