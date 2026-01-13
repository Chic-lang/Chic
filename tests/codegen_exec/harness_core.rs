use std::cell::RefCell;
use std::fmt;
use std::path::PathBuf;

use chic::codegen::{Backend, CpuIsaConfig};
use chic::driver::{BuildFfiOptions, BuildRequest, FrontendReport};
use chic::logging::LogLevel;
use chic::manifest::MissingDocsRule;
use chic::{ChicKind, Target};
use tempfile::{Builder, NamedTempFile};

use crate::{clang_available, codegen_exec_enabled, perf_enabled};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Happy,
    Error,
    Perf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarnessBackend {
    Wasm,
    Llvm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkipReason {
    CodegenDisabled,
    ClangMissing,
    PerfDisabled,
}

impl SkipReason {
    pub fn describe(&self) -> &'static str {
        match self {
            SkipReason::CodegenDisabled => {
                "skipping codegen exec because CHIC_ENABLE_CODEGEN_EXEC is not set"
            }
            SkipReason::ClangMissing => "skipping LLVM codegen exec because clang is not available",
            SkipReason::PerfDisabled => {
                "skipping perf-tagged codegen exec because CHIC_ENABLE_CODEGEN_PERF is not set"
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum HarnessError {
    Skip(SkipReason),
    Build(String),
}

impl HarnessError {
    pub fn build(err: impl fmt::Display) -> Self {
        HarnessError::Build(err.to_string())
    }

    pub fn into_test_result(
        self,
        harness: &ExecHarness<impl BuildRunner>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            HarnessError::Skip(reason) => harness.skip(reason),
            HarnessError::Build(message) => Err(message.into()),
        }
    }
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::Skip(reason) => write!(f, "skipped: {}", reason.describe()),
            HarnessError::Build(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for HarnessError {}

pub struct RunnerBuildResult {
    pub report: Option<FrontendReport>,
}

pub trait BuildRunner {
    fn build(&self, request: BuildRequest) -> Result<RunnerBuildResult, HarnessError>;
}

pub struct RealRunner;

impl BuildRunner for RealRunner {
    fn build(&self, request: BuildRequest) -> Result<RunnerBuildResult, HarnessError> {
        let driver = chic::driver::CompilerDriver::new();
        let report = driver.build(request).map_err(HarnessError::build)?;
        Ok(RunnerBuildResult {
            report: Some(report),
        })
    }
}

pub struct ExecHarness<R: BuildRunner = RealRunner> {
    pub(super) backend: HarnessBackend,
    pub(super) category: Category,
    pub(super) require_codegen_env: bool,
    pub(super) runner: R,
    pub(super) exec_flag_override: Option<bool>,
    pub(super) perf_flag_override: Option<bool>,
    pub(super) cpu_isa: CpuIsaConfig,
}

impl ExecHarness<RealRunner> {
    pub fn new(backend: HarnessBackend, category: Category) -> Self {
        ExecHarness {
            backend,
            category,
            require_codegen_env: true,
            runner: RealRunner,
            exec_flag_override: None,
            perf_flag_override: None,
            cpu_isa: CpuIsaConfig::default(),
        }
    }

    pub fn wasm(category: Category) -> Self {
        ExecHarness::new(HarnessBackend::Wasm, category)
    }

    pub fn llvm(category: Category) -> Self {
        ExecHarness::new(HarnessBackend::Llvm, category)
    }
}

impl<R: BuildRunner> ExecHarness<R> {
    pub fn with_runner<T: BuildRunner>(self, runner: T) -> ExecHarness<T> {
        ExecHarness {
            backend: self.backend,
            category: self.category,
            require_codegen_env: self.require_codegen_env,
            runner,
            exec_flag_override: self.exec_flag_override,
            perf_flag_override: self.perf_flag_override,
            cpu_isa: self.cpu_isa,
        }
    }

    pub fn allow_without_codegen_flag(mut self) -> Self {
        self.require_codegen_env = false;
        self
    }

    pub fn with_exec_flag_override(mut self, enabled: bool) -> Self {
        self.exec_flag_override = Some(enabled);
        self
    }

    pub fn with_perf_flag_override(mut self, enabled: bool) -> Self {
        self.perf_flag_override = Some(enabled);
        self
    }

    pub fn guard(&self) -> Result<(), HarnessError> {
        let perf_enabled = self.perf_flag_override.unwrap_or_else(perf_enabled);
        if self.category == Category::Perf && !perf_enabled {
            return Err(HarnessError::Skip(SkipReason::PerfDisabled));
        }
        let exec_enabled = self.exec_flag_override.unwrap_or_else(codegen_exec_enabled);
        if self.require_codegen_env && !exec_enabled {
            return Err(HarnessError::Skip(SkipReason::CodegenDisabled));
        }
        if self.backend == HarnessBackend::Llvm && !clang_available() {
            return Err(HarnessError::Skip(SkipReason::ClangMissing));
        }
        Ok(())
    }

    pub fn build_executable(
        &self,
        program: &str,
        output_suffix: Option<&str>,
    ) -> Result<BuildArtifact, HarnessError> {
        self.build_executable_with_inputs(program, output_suffix, &[])
    }

    pub fn build_executable_with_inputs(
        &self,
        program: &str,
        output_suffix: Option<&str>,
        extra_inputs: &[PathBuf],
    ) -> Result<BuildArtifact, HarnessError> {
        self.guard()?;
        let source = NamedTempFile::new().map_err(|err| HarnessError::build(err))?;
        std::fs::write(source.path(), program).map_err(HarnessError::build)?;
        let output = match output_suffix.or_else(|| self.backend.default_suffix()) {
            Some(ext) => Builder::new()
                .suffix(&format!(".{ext}"))
                .tempfile()
                .map_err(HarnessError::build)?,
            None => NamedTempFile::new().map_err(HarnessError::build)?,
        };

        let mut inputs = Vec::with_capacity(1 + extra_inputs.len());
        inputs.push(source.path().to_path_buf());
        inputs.extend_from_slice(extra_inputs);

        let request = BuildRequest {
            inputs,
            manifest: None,
            workspace: None,
            target: Target::host(),
            kind: ChicKind::Executable,
            backend: self.backend.into(),
            runtime_backend: chic::runtime::backend::RuntimeBackend::Chic,
            output: Some(output.path().to_path_buf()),
            emit_wat_text: false,
            emit_object: false,
            coverage: false,
            cpu_isa: self.cpu_isa.clone(),
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
        };

        let report = self.runner.build(request)?.report;
        Ok(BuildArtifact {
            _source: source,
            output,
            report,
        })
    }

    pub fn skip(&self, reason: SkipReason) -> Result<(), Box<dyn std::error::Error>> {
        eprintln!(
            "[{:?}:{:?}] {}",
            self.category,
            self.backend,
            reason.describe()
        );
        Ok(())
    }
}

impl HarnessBackend {
    fn default_suffix(self) -> Option<&'static str> {
        match self {
            HarnessBackend::Wasm => Some("wasm"),
            HarnessBackend::Llvm => None,
        }
    }
}

impl From<HarnessBackend> for Backend {
    fn from(value: HarnessBackend) -> Self {
        match value {
            HarnessBackend::Wasm => Backend::Wasm,
            HarnessBackend::Llvm => Backend::Llvm,
        }
    }
}

pub struct BuildArtifact {
    // Keep the temporary source alive so downstream tooling can inspect it.
    _source: NamedTempFile,
    pub output: NamedTempFile,
    // Only consumed by diagnostics-heavy codegen_exec tests; other harness consumers only need the
    // emitted binary.
    #[allow(dead_code)]
    pub report: Option<FrontendReport>,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRunner {
        last_build: RefCell<Vec<BuildRequest>>,
        next_result: RefCell<Option<Result<RunnerBuildResult, HarnessError>>>,
    }

    impl FakeRunner {
        fn succeeding() -> Self {
            let result = RunnerBuildResult { report: None };
            FakeRunner {
                last_build: RefCell::new(Vec::new()),
                next_result: RefCell::new(Some(Ok(result))),
            }
        }

        fn failing(message: &str) -> Self {
            FakeRunner {
                last_build: RefCell::new(Vec::new()),
                next_result: RefCell::new(Some(Err(HarnessError::build(message)))),
            }
        }
    }

    impl Default for FakeRunner {
        fn default() -> Self {
            FakeRunner::succeeding()
        }
    }

    impl BuildRunner for FakeRunner {
        fn build(&self, request: BuildRequest) -> Result<RunnerBuildResult, HarnessError> {
            self.last_build.borrow_mut().push(request);
            self.next_result
                .borrow_mut()
                .take()
                .unwrap_or_else(|| Ok(RunnerBuildResult { report: None }))
        }
    }

    #[test]
    fn guard_skips_when_env_disabled() {
        let harness = ExecHarness::wasm(Category::Happy)
            .allow_without_codegen_flag()
            .with_runner(FakeRunner::succeeding());
        assert!(harness.guard().is_ok());

        let gated = ExecHarness::wasm(Category::Happy)
            .with_exec_flag_override(false)
            .with_runner(FakeRunner::succeeding());
        let err = gated.guard().unwrap_err();
        assert!(matches!(
            err,
            HarnessError::Skip(SkipReason::CodegenDisabled)
        ));
    }

    #[test]
    fn build_records_backend_and_suffix() {
        let runner = FakeRunner::succeeding();
        let harness = ExecHarness::wasm(Category::Happy)
            .with_exec_flag_override(true)
            .with_runner(runner);
        let artifact = harness
            .build_executable("fn chic_main() {}", Some("wat"))
            .unwrap();
        assert!(
            artifact.output.path().extension().is_some(),
            "output should preserve requested suffix"
        );
        let requests = harness.runner.last_build.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].backend, Backend::Wasm);
    }

    #[test]
    fn build_surfaces_runner_failures() {
        let harness = ExecHarness::llvm(Category::Error)
            .with_exec_flag_override(true)
            .with_runner(FakeRunner::failing("boom"));
        let result = harness.build_executable("fn chic_main() {}", None);
        let err = match result {
            Ok(_) => panic!("expected harness to fail runner build"),
            Err(err) => err,
        };
        assert!(matches!(err, HarnessError::Build(message) if message.contains("boom")));
    }

    #[test]
    fn perf_harness_requires_opt_in_flag() {
        let harness = ExecHarness::wasm(Category::Perf)
            .with_perf_flag_override(false)
            .with_runner(FakeRunner::succeeding());
        let err = harness.guard().unwrap_err();
        assert!(matches!(err, HarnessError::Skip(SkipReason::PerfDisabled)));
    }
}
