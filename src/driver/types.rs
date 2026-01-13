use std::env;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;

use crate::chic_kind::ChicKind;
use crate::codegen::{Backend, CpuIsaConfig};
use crate::defines::DefineFlag;
use crate::logging::LogLevel;
use crate::manifest::{Manifest, MissingDocsRule, WorkspaceConfig};
use crate::mir::TestCaseMetadata;
use crate::runtime::WasmExecutionTrace;
use crate::target::Target;
use regex::Regex;

use super::FrontendReport;

pub struct TestRun {
    pub report: FrontendReport,
    pub cases: Vec<TestCaseResult>,
    pub discovered: usize,
    pub filtered_out: usize,
    pub chic_coverage: Option<crate::chic_coverage::ChicCoverageReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

pub struct TestCaseResult {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub namespace: Option<String>,
    pub categories: Vec<String>,
    pub is_async: bool,
    pub status: TestStatus,
    pub message: Option<String>,
    pub wasm_trace: Option<WasmExecutionTrace>,
    pub duration: Option<Duration>,
}

pub struct RunResult {
    pub report: FrontendReport,
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub wasm_trace: Option<WasmExecutionTrace>,
}

impl fmt::Debug for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunResult")
            .field("status", &self.status)
            .field("stdout_len", &self.stdout.len())
            .field("stderr_len", &self.stderr.len())
            .field("wasm_trace", &self.wasm_trace)
            .finish()
    }
}

/// Result of formatting source code.
pub struct FormatResult {
    pub input: PathBuf,
    pub changed: bool,
    pub original: String,
    pub formatted: String,
    pub metadata: crate::format::FormatMetadata,
}

#[derive(Clone)]
pub struct BuildRequest {
    pub inputs: Vec<PathBuf>,
    pub manifest: Option<Manifest>,
    pub workspace: Option<WorkspaceConfig>,
    pub target: Target,
    pub kind: ChicKind,
    pub backend: Backend,
    pub runtime_backend: crate::runtime::backend::RuntimeBackend,
    pub output: Option<PathBuf>,
    pub emit_wat_text: bool,
    pub emit_object: bool,
    pub coverage: bool,
    pub cpu_isa: CpuIsaConfig,
    pub emit_header: bool,
    pub emit_library_pack: bool,
    pub cc1_args: Vec<String>,
    pub cc1_keep_temps: bool,
    pub load_stdlib: bool,
    pub trace_pipeline: bool,
    pub trait_solver_metrics: bool,
    pub defines: Vec<DefineFlag>,
    pub log_level: LogLevel,
    pub ffi: BuildFfiOptions,
    pub configuration: String,
    pub framework: Option<String>,
    pub artifacts_path: Option<PathBuf>,
    pub obj_dir: Option<PathBuf>,
    pub bin_dir: Option<PathBuf>,
    pub no_dependencies: bool,
    pub no_restore: bool,
    pub no_incremental: bool,
    pub rebuild: bool,
    pub incremental_validate: bool,
    pub clean_only: bool,
    pub disable_build_servers: bool,
    pub source_root: Option<PathBuf>,
    pub properties: Vec<BuildPropertyOverride>,
    pub verbosity: Verbosity,
    pub telemetry: TelemetrySetting,
    pub version_suffix: Option<String>,
    pub nologo: bool,
    pub force: bool,
    pub interactive: bool,
    pub self_contained: Option<bool>,
    pub doc_enforcement: MissingDocsRule,
}

#[derive(Clone, Default)]
pub struct BuildFfiOptions {
    pub search_paths: Vec<PathBuf>,
    pub default_pattern: Option<String>,
    pub packages: Vec<PathBuf>,
}

pub fn resolve_trace_enabled(request_flag: bool, level: LogLevel) -> bool {
    if request_flag {
        return true;
    }

    if let Some(value) = env_flag_truthy("CHIC_TRACE_PIPELINE") {
        return value;
    }

    level >= LogLevel::Info
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

pub fn trait_solver_metrics_enabled(trace_enabled: bool, cli_requested: bool) -> bool {
    if cli_requested || trace_enabled {
        return true;
    }
    env_flag_truthy("CHIC_TRAIT_SOLVER_METRICS").unwrap_or(false)
}

/// CLI verbosity mapped to logging detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Minimal,
    Normal,
    Detailed,
    Diagnostic,
}

impl Verbosity {
    #[must_use]
    pub fn to_log_level(self) -> LogLevel {
        match self {
            Verbosity::Quiet => LogLevel::Error,
            Verbosity::Minimal => LogLevel::Warn,
            Verbosity::Normal => LogLevel::Info,
            Verbosity::Detailed => LogLevel::Debug,
            Verbosity::Diagnostic => LogLevel::Trace,
        }
    }
}

impl Default for Verbosity {
    fn default() -> Self {
        Verbosity::Normal
    }
}

/// Telemetry toggle for build performance collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetrySetting {
    Auto,
    On,
    Off,
}

impl Default for TelemetrySetting {
    fn default() -> Self {
        TelemetrySetting::Auto
    }
}

/// Arbitrary build property override from CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildPropertyOverride {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct TestSelection {
    pub tests: Vec<String>,
    pub groups: Vec<String>,
    pub run_all: bool,
}

impl TestSelection {
    #[must_use]
    pub fn is_unfiltered(&self) -> bool {
        self.run_all || (self.tests.is_empty() && self.groups.is_empty())
    }

    #[must_use]
    pub fn matches(&self, meta: &TestCaseMetadata) -> bool {
        if self.is_unfiltered() {
            return true;
        }
        let test_match = self.tests.is_empty()
            || self
                .tests
                .iter()
                .any(|pattern| matches_test_pattern(pattern, meta));
        let group_match = self.groups.is_empty()
            || self
                .groups
                .iter()
                .any(|pattern| matches_group_pattern(pattern, meta));
        test_match && group_match
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogConfig {
    pub step_limit: Option<u64>,
    pub timeout: Option<Duration>,
    pub enable_in_release: bool,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            step_limit: None,
            timeout: None,
            enable_in_release: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestOptions {
    pub selection: TestSelection,
    pub parallelism: Option<usize>,
    pub watchdog: WatchdogConfig,
    pub fail_fast: bool,
}

impl Default for TestOptions {
    fn default() -> Self {
        Self {
            selection: TestSelection::default(),
            parallelism: None,
            watchdog: WatchdogConfig::default(),
            fail_fast: false,
        }
    }
}

fn matches_test_pattern(pattern: &str, meta: &TestCaseMetadata) -> bool {
    let needle = pattern.to_ascii_lowercase();
    let candidates = [
        meta.id.as_str(),
        meta.qualified_name.as_str(),
        meta.name.as_str(),
    ];
    candidates
        .iter()
        .any(|candidate| wildcard_match(&needle, &candidate.to_ascii_lowercase()))
}

fn matches_group_pattern(pattern: &str, meta: &TestCaseMetadata) -> bool {
    let needle = pattern.to_ascii_lowercase();
    let mut candidates: Vec<String> = meta
        .categories
        .iter()
        .map(|category| category.to_ascii_lowercase())
        .collect();
    if let Some(namespace) = &meta.namespace {
        candidates.push(namespace.to_ascii_lowercase());
    }
    candidates
        .iter()
        .any(|candidate| wildcard_match(&needle, candidate))
}

fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let mut expr = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => expr.push_str(".*"),
            _ => expr.push_str(&regex::escape(&ch.to_string())),
        }
    }
    expr.push('$');
    Regex::new(&expr)
        .map(|re| re.is_match(candidate))
        .unwrap_or(false)
}
