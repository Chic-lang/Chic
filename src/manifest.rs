use crate::chic_kind::ChicKind;
use crate::code_style::{CodeStyleConfig, RawCodeStyleSection};
use crate::driver::types::Verbosity;
use crate::format::{FormatConfig, RawFormatSection};
use crate::package::version::{Version, VersionReq};
use crate::target::Target;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_MANIFEST_BASENAME: &str = "manifest.yaml";
pub const WORKSPACE_MANIFEST_BASENAME: &str = "manifest.workspace.yaml";

/// Parsed representation of `manifest.yaml`.
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    path: Option<PathBuf>,
    wasm: Option<WasmRuntimeSection>,
    runtime: Option<RuntimeSelection>,
    runtime_policy: Option<bool>,
    runtime_provides: Option<RuntimeProvides>,
    package: Option<PackageSection>,
    tests: TestsSettings,
    coverage: Option<CoverageSettings>,
    build: BuildSettings,
    docs: DocsSettings,
    format: FormatConfig,
    code_style: CodeStyleConfig,
    sources: Vec<SourceRoot>,
    dependencies: Vec<Dependency>,
    issues: Vec<ManifestIssue>,
}

/// Effective WASM runtime configuration after merging defaults and target overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WasmRuntimeSettings {
    pub memory_limit_pages: Option<u32>,
    pub env: HashMap<String, String>,
    pub feature_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Native,
    NoStd,
}

impl RuntimeKind {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "native" | "native-std" | "std" => Some(Self::Native),
            "no_std" | "nostd" | "native-no_std" | "native-no-std" => Some(Self::NoStd),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::NoStd => "no_std",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCompat {
    Strict,
    AllowMinor,
}

impl Default for RuntimeCompat {
    fn default() -> Self {
        Self::AllowMinor
    }
}

impl RuntimeCompat {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "strict" => Some(Self::Strict),
            "allow_minor" | "allow-minor" | "minor" => Some(Self::AllowMinor),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::AllowMinor => "allow_minor",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeSelection {
    pub kind: RuntimeKind,
    pub package: String,
    pub version: Option<VersionReq>,
    pub path: Option<PathBuf>,
    pub compat: RuntimeCompat,
    pub abi: Option<String>,
    pub require_native_runtime: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RuntimeProvides {
    pub kind: RuntimeKind,
    pub abi: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackageSection {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub version: Option<Version>,
    pub version_raw: Option<String>,
    pub friends: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRoot {
    pub path: PathBuf,
    pub namespace_prefix: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildSettings {
    pub configuration: Option<String>,
    pub framework: Option<String>,
    pub runtime: Option<String>,
    pub target: Option<String>,
    pub kind: Option<ChicKind>,
    pub verbosity: Option<Verbosity>,
    pub properties: Vec<ManifestProperty>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocsSettings {
    pub markdown: MarkdownDocsSettings,
    pub enforcement: DocsEnforcementSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownDocsSettings {
    pub enabled: Option<bool>,
    pub output: Option<PathBuf>,
    pub layout: Option<String>,
    pub template: Option<String>,
    pub front_matter_template: Option<PathBuf>,
    pub banner: Option<bool>,
    pub tag_handlers: Vec<String>,
    pub link_resolver: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocsEnforcementSettings {
    pub missing_docs: MissingDocsRule,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MissingDocsRule {
    pub severity: DocEnforcementSeverity,
    pub scope: DocEnforcementScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocEnforcementSeverity {
    Error,
    Warning,
    Ignore,
}

impl Default for DocEnforcementSeverity {
    fn default() -> Self {
        Self::Ignore
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocEnforcementScope {
    Public,
    PublicAndInternal,
    All,
}

impl Default for DocEnforcementScope {
    fn default() -> Self {
        Self::Public
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestProperty {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub requirement: Option<VersionReq>,
    pub source: DependencySource,
}

#[derive(Debug, Clone)]
pub enum DependencySource {
    Registry {
        registry: Option<String>,
    },
    Git {
        repo: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
        subdir: Option<PathBuf>,
    },
    Path(PathBuf),
}

#[derive(Debug, Clone)]
pub struct ManifestIssue {
    pub code: &'static str,
    pub message: String,
}

impl ManifestIssue {
    #[must_use]
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceConfig {
    pub path: PathBuf,
    pub build: BuildSettings,
    pub coverage: Option<CoverageSettings>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageEnforcement {
    Ignore,
    Warn,
    Error,
}

impl CoverageEnforcement {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ignore" | "off" | "none" => Some(Self::Ignore),
            "warn" | "warning" => Some(Self::Warn),
            "error" | "fail" => Some(Self::Error),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageBackend {
    Llvm,
    Wasm,
    Both,
}

impl CoverageBackend {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "llvm" => Some(Self::Llvm),
            "wasm" | "wasmtime" => Some(Self::Wasm),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Llvm => "llvm",
            Self::Wasm => "wasm",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageScope {
    Package,
    Workspace,
}

impl CoverageScope {
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "package" => Some(Self::Package),
            "workspace" => Some(Self::Workspace),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Workspace => "workspace",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoverageSettings {
    pub min_percent: u8,
    pub enforce: CoverageEnforcement,
    pub backend: CoverageBackend,
    pub scope: CoverageScope,
}

#[derive(Debug, Clone)]
pub struct TestsSettings {
    pub enabled: bool,
    pub include_tests_dir: bool,
    pub require_testcases: bool,
    pub enforce: CoverageEnforcement,
}

impl Default for TestsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            include_tests_dir: true,
            require_testcases: false,
            enforce: CoverageEnforcement::Ignore,
        }
    }
}

impl Manifest {
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    #[must_use]
    pub fn package(&self) -> Option<&PackageSection> {
        self.package.as_ref()
    }

    #[must_use]
    pub fn build(&self) -> &BuildSettings {
        &self.build
    }

    #[must_use]
    pub fn tests(&self) -> &TestsSettings {
        &self.tests
    }

    #[must_use]
    pub fn coverage(&self) -> Option<&CoverageSettings> {
        self.coverage.as_ref()
    }

    #[must_use]
    pub fn runtime(&self) -> Option<&RuntimeSelection> {
        self.runtime.as_ref()
    }

    #[must_use]
    pub fn require_native_runtime(&self, kind: ChicKind) -> bool {
        if let Some(runtime) = &self.runtime {
            if let Some(require) = runtime.require_native_runtime {
                return require;
            }
        }
        if let Some(policy) = self.runtime_policy {
            return policy;
        }
        !kind.is_library() || kind == ChicKind::DynamicLibrary
    }

    #[must_use]
    pub fn runtime_provides(&self) -> Option<&RuntimeProvides> {
        self.runtime_provides.as_ref()
    }

    #[must_use]
    pub fn is_runtime_provider(&self) -> bool {
        self.runtime_provides.is_some()
    }

    #[must_use]
    pub fn is_no_std_runtime(&self) -> bool {
        self.runtime()
            .is_some_and(|runtime| matches!(runtime.kind, RuntimeKind::NoStd))
            || self
                .runtime_provides()
                .is_some_and(|runtime| matches!(runtime.kind, RuntimeKind::NoStd))
    }

    #[must_use]
    pub fn docs(&self) -> &DocsSettings {
        &self.docs
    }

    #[must_use]
    pub fn source_roots(&self) -> &[SourceRoot] {
        &self.sources
    }

    #[must_use]
    pub fn format(&self) -> &FormatConfig {
        &self.format
    }

    #[must_use]
    pub fn code_style(&self) -> &CodeStyleConfig {
        &self.code_style
    }

    #[must_use]
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    #[must_use]
    pub fn issues(&self) -> &[ManifestIssue] {
        &self.issues
    }

    #[must_use]
    pub fn package_version(&self) -> Option<&Version> {
        self.package.as_ref().and_then(|pkg| pkg.version.as_ref())
    }

    #[must_use]
    pub fn derived_source_roots(&self) -> Vec<SourceRoot> {
        if self.sources.is_empty() {
            let namespace = self
                .package
                .as_ref()
                .and_then(|pkg| pkg.namespace.clone().or_else(|| pkg.name.clone()));
            return vec![SourceRoot {
                path: PathBuf::from("src"),
                namespace_prefix: namespace,
            }];
        }
        self.sources.clone()
    }

    /// Attempt to discover a manifest by walking upwards from the provided path.
    ///
    /// Returns `Ok(None)` when no project file was found.
    pub fn discover(start: &Path) -> crate::error::Result<Option<Self>> {
        let mut current = if start.is_file() {
            start.parent().map(Path::to_path_buf)
        } else {
            Some(start.to_path_buf())
        };

        while let Some(dir) = current {
            let manifest_path = dir.join(PROJECT_MANIFEST_BASENAME);
            if manifest_path.exists() {
                let canonical_path = manifest_path
                    .canonicalize()
                    .unwrap_or_else(|_| manifest_path.clone());
                validate_manifest_location(&canonical_path)?;
                let contents = fs::read_to_string(&canonical_path)?;
                let raw: RawManifest = serde_yaml::from_str(&contents).map_err(|err| {
                    crate::error::Error::internal(format!(
                        "failed to parse `{}`: {err}",
                        canonical_path.display()
                    ))
                })?;
                return Ok(Some(Self::from_raw(raw, Some(canonical_path))));
            }
            current = parent_directory(&dir);
        }

        Ok(None)
    }

    /// Resolve WASM runtime settings for the given target triple.
    #[must_use]
    pub fn wasm_settings_for_target(&self, target: &Target) -> Option<WasmRuntimeSettings> {
        let section = self.wasm.as_ref()?;
        let mut settings = WasmRuntimeSettings::from_raw(&section.defaults);
        if let Some(target_specific) = section.resolve_target(target) {
            settings.apply(target_specific);
        }
        Some(settings)
    }

    fn from_raw(raw: RawManifest, path: Option<PathBuf>) -> Self {
        let mut issues = Vec::new();
        let package = raw.package.clone().map(|pkg| {
            let (pkg, pkg_issues) = PackageSection::from_raw(pkg);
            issues.extend(pkg_issues);
            pkg
        });
        let manifest_dir = path.as_deref().and_then(Path::parent);
        let runtime_selector = raw
            .toolchain
            .runtime
            .as_ref()
            .or_else(|| raw.runtime.selector.as_option());
        let runtime_policy = raw
            .toolchain
            .runtime
            .as_ref()
            .and_then(|selector| selector.policy.as_ref())
            .and_then(|policy| policy.require_native_runtime)
            .or_else(|| {
                raw.runtime
                    .selector
                    .policy
                    .as_ref()
                    .and_then(|policy| policy.require_native_runtime)
            });
        let (runtime, mut runtime_issues) =
            RuntimeSelection::from_raw(runtime_selector, manifest_dir);
        issues.append(&mut runtime_issues);
        let (runtime_provides, mut provides_issues) =
            RuntimeProvides::from_raw(raw.runtime.provides.as_ref());
        issues.append(&mut provides_issues);
        let (tests, mut test_issues) = TestsSettings::from_raw(&raw.tests);
        issues.append(&mut test_issues);
        let (coverage, mut coverage_issues) = CoverageSettings::from_raw(raw.coverage.as_ref());
        issues.append(&mut coverage_issues);
        let build = BuildSettings::from_raw(&raw);
        let docs = DocsSettings::from_raw(&raw.docs);
        let format = FormatConfig::from_raw(&raw.format);
        let code_style = CodeStyleConfig::from_raw(&raw.code_style);
        let sources = raw
            .sources
            .into_iter()
            .filter_map(SourceRoot::from_raw)
            .collect();
        let mut dependencies = Vec::new();
        for (name, raw_dep) in raw.dependencies {
            match Dependency::from_raw(name, raw_dep, manifest_dir) {
                Ok(dep) => dependencies.push(dep),
                Err(issue) => issues.push(issue),
            }
        }
        Self {
            path,
            wasm: raw.runtime.wasm.map(WasmRuntimeSection::from_raw),
            runtime,
            runtime_policy,
            runtime_provides,
            package,
            tests,
            coverage,
            build,
            docs,
            format,
            code_style,
            sources,
            dependencies,
            issues,
        }
    }
}

impl CoverageSettings {
    fn from_raw(raw: Option<&RawCoverageSection>) -> (Option<Self>, Vec<ManifestIssue>) {
        let mut issues = Vec::new();
        let Some(raw) = raw else {
            return (None, issues);
        };
        let min_percent = raw.min_percent.unwrap_or(95);
        let enforce = raw
            .enforce
            .as_deref()
            .and_then(CoverageEnforcement::parse)
            .unwrap_or(CoverageEnforcement::Ignore);
        let backend = raw
            .backend
            .as_deref()
            .and_then(CoverageBackend::parse)
            .unwrap_or(CoverageBackend::Wasm);
        let scope = raw
            .scope
            .as_deref()
            .and_then(CoverageScope::parse)
            .unwrap_or(CoverageScope::Package);

        if min_percent > 100 {
            issues.push(ManifestIssue::new(
                "MN-COV-0001",
                "coverage.min_percent must be between 0 and 100",
            ));
        }

        (
            Some(Self {
                min_percent: min_percent.min(100),
                enforce,
                backend,
                scope,
            }),
            issues,
        )
    }
}

impl TestsSettings {
    fn from_raw(raw: &RawTestsSection) -> (Self, Vec<ManifestIssue>) {
        let issues = Vec::new();
        let enabled = raw.enabled.unwrap_or(true);
        let include_tests_dir = raw.include_tests_dir.unwrap_or(true);
        let require_testcases = raw.require_testcases.unwrap_or(false);
        let enforce = raw
            .enforce
            .as_deref()
            .and_then(CoverageEnforcement::parse)
            .unwrap_or(CoverageEnforcement::Ignore);

        (
            Self {
                enabled,
                include_tests_dir,
                require_testcases,
                enforce,
            },
            issues,
        )
    }
}

impl WasmRuntimeSettings {
    fn from_raw(raw: &RawWasmSettings) -> Self {
        let mut settings = Self::default();
        settings.apply(raw);
        settings
    }

    fn apply(&mut self, raw: &RawWasmSettings) {
        if let Some(limit) = raw.memory_limit_pages() {
            self.memory_limit_pages = Some(limit);
        }

        if !raw.env.is_empty() {
            for (key, value) in &raw.env {
                self.env.insert(key.clone(), value.clone());
            }
        }

        for flag in raw.all_feature_flags() {
            if !self
                .feature_flags
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(flag))
            {
                self.feature_flags.push(flag.to_string());
            }
        }
    }

    #[must_use]
    pub fn to_execution_options(&self) -> crate::runtime::wasm_executor::WasmExecutionOptions {
        crate::runtime::wasm_executor::WasmExecutionOptions {
            memory_limit_pages: self.memory_limit_pages,
            env: self.env.clone(),
            feature_flags: self.feature_flags.clone(),
            error_hook: None,
            coverage_hook: None,
            io_hooks: None,
            async_layout: None,
            async_result_len: None,
            async_result_align: None,
            await_entry_task: true,
            stdin: Vec::new(),
            stdin_is_terminal: false,
            stdout_is_terminal: false,
            stderr_is_terminal: false,
            capture_stdout: true,
            capture_stderr: true,
            rounding_mode: None,
            watchdog_step_limit: None,
            watchdog_timeout: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(default)]
    package: Option<RawPackage>,
    #[serde(default)]
    runtime: RawRuntimeSection,
    #[serde(default)]
    build: RawBuildSection,
    #[serde(default)]
    sources: Vec<RawSourceRoot>,
    #[serde(default)]
    dependencies: HashMap<String, RawDependency>,
    #[serde(default)]
    targets: RawTargetsSection,
    #[serde(default)]
    toolchain: RawToolchainSection,
    #[serde(default)]
    docs: RawDocsSection,
    #[serde(default)]
    format: RawFormatSection,
    #[serde(default, rename = "code_style")]
    code_style: RawCodeStyleSection,
    #[serde(default)]
    tests: RawTestsSection,
    #[serde(default)]
    coverage: Option<RawCoverageSection>,
}

#[derive(Debug, Deserialize, Default)]
struct RawRuntimeSection {
    #[serde(default)]
    wasm: Option<RawWasmRuntimeSection>,
    #[serde(flatten)]
    selector: RawRuntimeSelector,
    #[serde(default)]
    provides: Option<RawRuntimeProvides>,
}

#[derive(Debug, Deserialize, Default)]
struct RawWasmRuntimeSection {
    #[serde(default)]
    defaults: RawWasmSettings,
    #[serde(default)]
    targets: HashMap<String, RawWasmSettings>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawRuntimeSelector {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    compat: Option<String>,
    #[serde(default)]
    abi: Option<String>,
    #[serde(default)]
    policy: Option<RawRuntimePolicy>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawRuntimePolicy {
    #[serde(default, rename = "require_native_runtime")]
    require_native_runtime: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawRuntimeProvides {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    abi: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawPackage {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    friends: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
enum RawDependency {
    Version(String),
    Detailed(RawDependencyDetail),
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawDependencyDetail {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    git: Option<String>,
    #[serde(default)]
    rev: Option<String>,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    subdir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTargetsSection {
    #[serde(default)]
    default: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawToolchainSection {
    #[serde(default, rename = "target-triples")]
    target_triples: Vec<String>,
    #[serde(default)]
    runtime: Option<RawRuntimeSelector>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawDocsSection {
    #[serde(default)]
    markdown: RawMarkdownDocs,
    #[serde(default)]
    enforcement: RawDocsEnforcement,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawMarkdownDocs {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    layout: Option<String>,
    #[serde(default)]
    template: Option<String>,
    #[serde(default, rename = "front_matter_template")]
    front_matter_template: Option<String>,
    #[serde(default)]
    banner: Option<bool>,
    #[serde(default, rename = "tag_handlers")]
    tag_handlers: Vec<String>,
    #[serde(default, rename = "link_resolver")]
    link_resolver: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawDocsEnforcement {
    #[serde(default, rename = "missing_docs")]
    missing_docs: Option<RawMissingDocsRule>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawMissingDocsRule {
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawTestsSection {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default, rename = "include_tests_dir")]
    include_tests_dir: Option<bool>,
    #[serde(default)]
    require_testcases: Option<bool>,
    #[serde(default)]
    enforce: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawCoverageSection {
    #[serde(default)]
    min_percent: Option<u8>,
    #[serde(default)]
    enforce: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct RawWasmSettings {
    #[serde(default, rename = "memory-limit-pages")]
    memory_limit_pages: Option<u32>,
    #[serde(default)]
    memory: Option<RawMemory>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default, rename = "feature-flags")]
    feature_flags: Vec<String>,
    #[serde(default)]
    features: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct RawMemory {
    #[serde(default, rename = "max-pages")]
    max_pages: Option<u32>,
    #[serde(default, rename = "limit-pages")]
    limit_pages: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct RawBuildSection {
    #[serde(default)]
    configuration: Option<String>,
    #[serde(default)]
    framework: Option<String>,
    #[serde(default)]
    runtime: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    verbosity: Option<String>,
    #[serde(default)]
    properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Default)]
struct RawSourceRoot {
    #[serde(default)]
    path: Option<String>,
    #[serde(default, rename = "namespace_prefix")]
    namespace_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawWorkspace {
    #[serde(default)]
    build: RawBuildSection,
    #[serde(default)]
    defaults: RawBuildSection,
    #[serde(default)]
    coverage: Option<RawCoverageSection>,
}

fn merge_build_sections(base: &RawBuildSection, overlay: &RawBuildSection) -> RawBuildSection {
    let mut merged = base.clone();
    if overlay.configuration.is_some() {
        merged.configuration = overlay.configuration.clone();
    }
    if overlay.framework.is_some() {
        merged.framework = overlay.framework.clone();
    }
    if overlay.runtime.is_some() {
        merged.runtime = overlay.runtime.clone();
    }
    if overlay.target.is_some() {
        merged.target = overlay.target.clone();
    }
    if overlay.kind.is_some() {
        merged.kind = overlay.kind.clone();
    }
    if overlay.verbosity.is_some() {
        merged.verbosity = overlay.verbosity.clone();
    }
    merged.properties = merge_properties(base.properties.as_ref(), overlay.properties.as_ref());
    merged
}

fn merge_properties(
    base: Option<&HashMap<String, String>>,
    overlay: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    let mut combined: HashMap<String, String> =
        base.map_or_else(HashMap::new, std::clone::Clone::clone);
    if let Some(additional) = overlay {
        for (key, value) in additional {
            combined.insert(key.clone(), value.clone());
        }
    }
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

fn manifest_properties(source: Option<&HashMap<String, String>>) -> Vec<ManifestProperty> {
    let mut properties = Vec::new();
    if let Some(map) = source {
        for (name, value) in map {
            properties.push(ManifestProperty {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
    properties
}

fn parse_manifest_verbosity(value: &str) -> Option<Verbosity> {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "quiet" => Some(Verbosity::Quiet),
        "minimal" => Some(Verbosity::Minimal),
        "normal" | "info" => Some(Verbosity::Normal),
        "detailed" | "debug" => Some(Verbosity::Detailed),
        "diagnostic" | "trace" => Some(Verbosity::Diagnostic),
        _ => None,
    }
}

fn parse_doc_severity(value: &str) -> Option<DocEnforcementSeverity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "error" | "err" => Some(DocEnforcementSeverity::Error),
        "warn" | "warning" => Some(DocEnforcementSeverity::Warning),
        "ignore" | "off" => Some(DocEnforcementSeverity::Ignore),
        _ => None,
    }
}

fn parse_doc_scope(value: &str) -> Option<DocEnforcementScope> {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => Some(DocEnforcementScope::Public),
        "public+internal" | "public-internal" | "public_internal" => {
            Some(DocEnforcementScope::PublicAndInternal)
        }
        "all" | "private" | "everything" => Some(DocEnforcementScope::All),
        _ => None,
    }
}

fn parse_runtime_kind(value: &str) -> Option<RuntimeKind> {
    RuntimeKind::parse(value)
}

fn parse_runtime_compat(value: &str) -> Option<RuntimeCompat> {
    RuntimeCompat::parse(value)
}

impl RawRuntimeSelector {
    fn is_empty(&self) -> bool {
        self.kind
            .as_deref()
            .map(str::trim)
            .map_or(true, str::is_empty)
            && self
                .package
                .as_deref()
                .map(str::trim)
                .map_or(true, str::is_empty)
            && self
                .version
                .as_deref()
                .map(str::trim)
                .map_or(true, str::is_empty)
            && self
                .path
                .as_deref()
                .map(str::trim)
                .map_or(true, str::is_empty)
            && self
                .compat
                .as_deref()
                .map(str::trim)
                .map_or(true, str::is_empty)
            && self
                .abi
                .as_deref()
                .map(str::trim)
                .map_or(true, str::is_empty)
    }

    fn as_option(&self) -> Option<&Self> {
        if self.is_empty() { None } else { Some(self) }
    }
}

impl RuntimeSelection {
    fn from_raw(
        raw: Option<&RawRuntimeSelector>,
        manifest_dir: Option<&Path>,
    ) -> (Option<Self>, Vec<ManifestIssue>) {
        let mut issues = Vec::new();
        let Some(raw) = raw else {
            return (None, issues);
        };
        if raw.is_empty() {
            return (None, issues);
        }
        let kind_text = match raw.kind.as_deref() {
            Some(value) if !value.trim().is_empty() => value,
            _ => {
                issues.push(ManifestIssue::new(
                    "RT0001",
                    "toolchain.runtime.kind is required when selecting a runtime",
                ));
                return (None, issues);
            }
        };
        let kind = match parse_runtime_kind(kind_text) {
            Some(kind) => kind,
            None => {
                issues.push(ManifestIssue::new(
                    "RT0002",
                    format!("unsupported runtime kind `{kind_text}`; expected native or no_std"),
                ));
                return (None, issues);
            }
        };
        let package = match raw.package.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => {
                issues.push(ManifestIssue::new(
                    "RT0003",
                    "toolchain.runtime.package is required when selecting a runtime",
                ));
                return (None, issues);
            }
        };
        let version = match raw.version.as_deref().map(str::trim) {
            Some(text) if !text.is_empty() => match VersionReq::parse(text) {
                Ok(req) => Some(req),
                Err(err) => {
                    issues.push(ManifestIssue::new(
                        "RT0004",
                        format!("invalid runtime version requirement `{text}`: {err}"),
                    ));
                    None
                }
            },
            _ => None,
        };
        let compat = raw
            .compat
            .as_deref()
            .and_then(parse_runtime_compat)
            .unwrap_or_default();
        if raw
            .compat
            .as_deref()
            .is_some_and(|value| parse_runtime_compat(value).is_none())
        {
            if let Some(value) = raw.compat.as_deref() {
                issues.push(ManifestIssue::new(
                    "RT0005",
                    format!(
                        "unsupported runtime compatibility policy `{value}`; expected strict or allow_minor"
                    ),
                ));
            }
        }
        let abi = raw
            .abi
            .as_ref()
            .map(|abi| abi.trim().to_string())
            .filter(|abi| !abi.is_empty());
        let require_native_runtime = raw
            .policy
            .as_ref()
            .and_then(|policy| policy.require_native_runtime);
        let path = raw.path.as_ref().map(|path| {
            let candidate = PathBuf::from(path);
            if candidate.is_absolute() {
                candidate
            } else if let Some(dir) = manifest_dir {
                dir.join(path)
            } else {
                candidate
            }
        });
        (
            Some(Self {
                kind,
                package,
                version,
                path,
                compat,
                abi,
                require_native_runtime,
            }),
            issues,
        )
    }
}

impl RuntimeProvides {
    fn from_raw(raw: Option<&RawRuntimeProvides>) -> (Option<Self>, Vec<ManifestIssue>) {
        let mut issues = Vec::new();
        let Some(raw) = raw else {
            return (None, issues);
        };
        if raw.kind.is_none() && raw.abi.is_none() {
            return (None, issues);
        }
        let kind_text = match raw.kind.as_deref() {
            Some(kind) if !kind.trim().is_empty() => kind,
            _ => {
                issues.push(ManifestIssue::new(
                    "RT0100",
                    "runtime.provides.kind is required when declaring a runtime provider",
                ));
                return (None, issues);
            }
        };
        let kind = match parse_runtime_kind(kind_text) {
            Some(kind) => kind,
            None => {
                issues.push(ManifestIssue::new(
                    "RT0101",
                    format!(
                        "runtime.provides.kind `{kind_text}` is not supported; expected native or no_std"
                    ),
                ));
                return (None, issues);
            }
        };
        let abi = raw
            .abi
            .as_ref()
            .map(|abi| abi.trim().to_string())
            .filter(|abi| !abi.is_empty());
        (Some(Self { kind, abi }), issues)
    }
}

impl PackageSection {
    fn from_raw(raw: RawPackage) -> (Self, Vec<ManifestIssue>) {
        let mut issues = Vec::new();
        let (version, version_raw) = match raw.version {
            Some(text) => match Version::parse(text.trim()) {
                Ok(parsed) => (Some(parsed), Some(text)),
                Err(err) => {
                    issues.push(ManifestIssue::new(
                        "PKG0200",
                        format!("invalid package version `{text}`: {err}"),
                    ));
                    (None, Some(text))
                }
            },
            None => (None, None),
        };
        (
            Self {
                name: raw.name,
                namespace: raw.namespace,
                version,
                version_raw,
                friends: raw.friends,
            },
            issues,
        )
    }
}

impl Dependency {
    fn from_raw(
        name: String,
        raw: RawDependency,
        manifest_dir: Option<&Path>,
    ) -> Result<Self, ManifestIssue> {
        let dep_name = name.trim();
        if dep_name.is_empty() {
            return Err(ManifestIssue::new(
                "PKG0205",
                "dependency name must not be empty",
            ));
        }
        let name = dep_name.to_string();
        match raw {
            RawDependency::Version(text) => {
                let requirement = VersionReq::parse(text.trim()).map_err(|err| {
                    ManifestIssue::new(
                        "PKG0201",
                        format!("invalid version requirement for `{name}`: {err}"),
                    )
                })?;
                Ok(Self {
                    name,
                    requirement: Some(requirement),
                    source: DependencySource::Registry { registry: None },
                })
            }
            RawDependency::Detailed(detail) => {
                let requirement = match detail.version {
                    Some(spec) => Some(VersionReq::parse(spec.trim()).map_err(|err| {
                        ManifestIssue::new(
                            "PKG0201",
                            format!("invalid version requirement for `{name}`: {err}"),
                        )
                    })?),
                    None => None,
                };

                if let Some(path) = detail.path {
                    let resolved = if Path::new(&path).is_absolute() {
                        PathBuf::from(path)
                    } else if let Some(dir) = manifest_dir {
                        dir.join(path)
                    } else {
                        PathBuf::from(path)
                    };
                    return Ok(Self {
                        name,
                        requirement,
                        source: DependencySource::Path(resolved),
                    });
                }

                if let Some(repo) = detail.git {
                    return Ok(Self {
                        name,
                        requirement,
                        source: DependencySource::Git {
                            repo,
                            rev: detail.rev,
                            branch: detail.branch,
                            tag: detail.tag,
                            subdir: detail.subdir.map(PathBuf::from),
                        },
                    });
                }

                let registry = detail.source;
                if requirement.is_none() {
                    return Err(ManifestIssue::new(
                        "PKG0202",
                        format!(
                            "dependency `{name}` requires a version when no git/path source is provided"
                        ),
                    ));
                }
                Ok(Self {
                    name,
                    requirement,
                    source: DependencySource::Registry { registry },
                })
            }
        }
    }
}

impl SourceRoot {
    fn from_raw(raw: RawSourceRoot) -> Option<Self> {
        let path = raw.path?;
        if path.trim().is_empty() {
            return None;
        }
        Some(Self {
            path: PathBuf::from(path),
            namespace_prefix: raw.namespace_prefix,
        })
    }
}

impl BuildSettings {
    fn from_raw(raw: &RawManifest) -> Self {
        let merged_kind = raw
            .build
            .kind
            .as_deref()
            .or(raw.targets.default.as_deref())
            .and_then(|value| ChicKind::parse(value).ok());
        let target = raw
            .build
            .target
            .clone()
            .or_else(|| raw.toolchain.target_triples.first().cloned());
        let verbosity = raw
            .build
            .verbosity
            .as_deref()
            .and_then(parse_manifest_verbosity);
        let properties = manifest_properties(raw.build.properties.as_ref());
        Self {
            configuration: raw.build.configuration.clone(),
            framework: raw.build.framework.clone(),
            runtime: raw.build.runtime.clone(),
            target,
            kind: merged_kind,
            verbosity,
            properties,
        }
    }
}

impl DocsSettings {
    fn from_raw(raw: &RawDocsSection) -> Self {
        Self {
            markdown: MarkdownDocsSettings::from_raw(&raw.markdown),
            enforcement: DocsEnforcementSettings::from_raw(&raw.enforcement),
        }
    }
}

impl MarkdownDocsSettings {
    fn from_raw(raw: &RawMarkdownDocs) -> Self {
        Self {
            enabled: raw.enabled,
            output: raw.output.as_ref().map(PathBuf::from),
            layout: raw.layout.clone(),
            template: raw.template.clone(),
            front_matter_template: raw.front_matter_template.as_ref().map(PathBuf::from),
            banner: raw.banner,
            tag_handlers: raw.tag_handlers.clone(),
            link_resolver: raw.link_resolver.clone(),
        }
    }
}

impl DocsEnforcementSettings {
    fn from_raw(raw: &RawDocsEnforcement) -> Self {
        Self {
            missing_docs: raw
                .missing_docs
                .as_ref()
                .map(MissingDocsRule::from_raw)
                .unwrap_or_default(),
        }
    }
}

impl MissingDocsRule {
    fn from_raw(raw: &RawMissingDocsRule) -> Self {
        let severity = raw
            .severity
            .as_deref()
            .and_then(parse_doc_severity)
            .unwrap_or_default();
        let scope = raw
            .scope
            .as_deref()
            .and_then(parse_doc_scope)
            .unwrap_or_default();
        Self { severity, scope }
    }
}

impl WorkspaceConfig {
    pub fn discover(start: &Path) -> crate::error::Result<Option<Self>> {
        let mut current = if start.is_file() {
            start.parent().map(Path::to_path_buf)
        } else {
            Some(start.to_path_buf())
        };

        while let Some(dir) = current {
            let workspace_path = dir.join(WORKSPACE_MANIFEST_BASENAME);
            if workspace_path.exists() {
                let canonical_path = workspace_path
                    .canonicalize()
                    .unwrap_or_else(|_| workspace_path.clone());
                let contents = fs::read_to_string(&canonical_path)?;
                let raw: RawWorkspace = serde_yaml::from_str(&contents).map_err(|err| {
                    crate::error::Error::internal(format!(
                        "failed to parse `{}`: {err}",
                        canonical_path.display()
                    ))
                })?;
                return Ok(Some(Self::from_raw(raw, canonical_path)));
            }
            current = parent_directory(&dir);
        }

        Ok(None)
    }

    fn from_raw(raw: RawWorkspace, path: PathBuf) -> Self {
        let merged = merge_build_sections(&raw.defaults, &raw.build);
        let manifest = RawManifest {
            package: None,
            runtime: RawRuntimeSection::default(),
            build: merged,
            sources: Vec::new(),
            dependencies: HashMap::new(),
            targets: RawTargetsSection::default(),
            toolchain: RawToolchainSection::default(),
            docs: RawDocsSection::default(),
            format: RawFormatSection::default(),
            code_style: RawCodeStyleSection::default(),
            tests: RawTestsSection::default(),
            coverage: None,
        };
        let (coverage, _issues) = CoverageSettings::from_raw(raw.coverage.as_ref());
        Self {
            path,
            build: BuildSettings::from_raw(&manifest),
            coverage,
        }
    }
}

impl RawWasmSettings {
    fn memory_limit_pages(&self) -> Option<u32> {
        self.memory_limit_pages.or_else(|| {
            self.memory
                .as_ref()
                .and_then(|memory| memory.effective_limit())
        })
    }

    fn all_feature_flags(&self) -> impl Iterator<Item = &str> {
        self.feature_flags
            .iter()
            .map(String::as_str)
            .chain(self.features.iter().map(String::as_str))
    }
}

impl RawMemory {
    fn effective_limit(&self) -> Option<u32> {
        self.limit_pages.or(self.max_pages)
    }
}

#[derive(Debug, Clone)]
struct WasmRuntimeSection {
    defaults: RawWasmSettings,
    targets: HashMap<String, RawWasmSettings>,
}

impl WasmRuntimeSection {
    fn from_raw(raw: RawWasmRuntimeSection) -> Self {
        Self {
            defaults: raw.defaults,
            targets: raw
                .targets
                .into_iter()
                .map(|(key, value)| (key.to_ascii_lowercase(), value))
                .collect(),
        }
    }

    fn resolve_target(&self, target: &Target) -> Option<&RawWasmSettings> {
        let triple_key = target.triple().to_ascii_lowercase();
        self.targets
            .get(&triple_key)
            .or_else(|| self.targets.get(target.arch().as_str()))
            .or_else(|| self.targets.get("default"))
    }
}

fn parent_directory(path: &Path) -> Option<PathBuf> {
    path.parent()
        .filter(|parent| parent != &path)
        .map(Path::to_path_buf)
}

fn validate_manifest_location(path: &Path) -> crate::error::Result<()> {
    let components: Vec<_> = path.components().collect();
    let mut packages_index = None;
    for (idx, component) in components.iter().enumerate() {
        if component.as_os_str() == "packages" {
            packages_index = Some(idx);
            break;
        }
    }
    if let Some(idx) = packages_index {
        if let Some(package_component) = components.get(idx + 1) {
            let mut package_root = PathBuf::new();
            for component in &components[..=idx + 1] {
                package_root.push(component.as_os_str());
            }
            let expected = package_root.join(PROJECT_MANIFEST_BASENAME);
            if path != expected {
                let package_name = package_component.as_os_str().to_string_lossy();
                return Err(crate::error::Error::internal(format!(
                    "found manifest at `{}` inside package `{package_name}`; expected {} (nested manifests are not supported)",
                    path.display(),
                    expected.display()
                )));
            }
        }
    }
    if path
        .components()
        .any(|component| component.as_os_str() == "src")
    {
        return Err(crate::error::Error::internal(format!(
            "manifest `{}` is nested under `src`; move manifest.yaml to packages/<name>/manifest.yaml",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    fn write_manifest(dir: &Path, contents: &str) {
        fs::write(dir.join(PROJECT_MANIFEST_BASENAME), contents).expect("write manifest");
    }

    #[test]
    fn discover_returns_none_when_missing() {
        let dir = tempdir().expect("tempdir");
        let source = dir.path().join("main.cl");
        File::create(&source).expect("touch source");
        let manifest = Manifest::discover(&source).expect("discover");
        assert!(manifest.is_none());
    }

    #[test]
    fn parses_format_section() {
        let dir = tempdir().expect("tempdir");
        write_manifest(
            dir.path(),
            r#"
format:
  version: 1
  enabled: true
  enforce: error
  max_line_length: 80
  indent:
    size: 2
    use_tabs: true
  newline: crlf
  trailing_newline: false
  trim_trailing_whitespace: false
  braces:
    style: k&r
    require_for_single_line_if: true
    require_for_single_line_loops: true
  if:
    else_on_new_line: false
    space_before_parentheses: false
    wrap_conditions: always
  switch:
    case_indent: 0
    braces_style: allman
    blank_line_between_cases: true
    align_case_labels: false
  usings:
    sort: false
    group: none
    blank_line_between_groups: false
  ordering:
    types:
      - interface
      - class
    members:
      - constructors
      - fields
    access:
      - private
      - internal
  files:
    one_top_level_type_per_file: false
    require_filename_match: false
    action: apply
    naming: namespace+typename
"#,
        );
        let manifest = Manifest::discover(dir.path())
            .expect("discover")
            .expect("manifest");
        let format = manifest.format();
        assert_eq!(format.max_line_length, 80);
        assert_eq!(format.enforce, crate::format::FormatEnforcement::Error);
        assert_eq!(format.indent.size, 2);
        assert!(format.indent.use_tabs);
        assert_eq!(format.newline, crate::format::NewlineStyle::Crlf);
        assert!(!format.trailing_newline);
        assert!(!format.trim_trailing_whitespace);
        assert_eq!(format.braces.style, crate::format::BraceStyle::KAndR);
        assert!(format.braces.require_single_line_if);
        assert!(format.braces.require_single_line_loops);
        assert!(!format.r#if.else_on_new_line);
        assert!(!format.r#if.space_before_parentheses);
        assert_eq!(
            format.r#if.wrap_conditions,
            crate::format::WrapStyle::Always
        );
        assert_eq!(format.switch.case_indent, 0);
        assert_eq!(
            format.switch.braces_style,
            Some(crate::format::BraceStyle::Allman)
        );
        assert!(format.switch.blank_line_between_cases);
        assert!(!format.switch.align_case_labels);
        assert!(!format.usings.sort);
        assert_eq!(format.usings.group, crate::format::UsingGroup::None);
        assert!(!format.usings.blank_line_between_groups);
        assert_eq!(
            format.ordering.types,
            vec![
                crate::format::TypeSort::Interface,
                crate::format::TypeSort::Class
            ]
        );
        assert_eq!(
            format.ordering.members,
            vec![
                crate::format::MemberSort::Constructors,
                crate::format::MemberSort::Fields
            ]
        );
        assert_eq!(
            format.ordering.access,
            vec![
                crate::format::AccessSort::Private,
                crate::format::AccessSort::Internal
            ]
        );
        assert!(!format.files.one_top_level_type_per_file);
        assert!(!format.files.require_filename_match);
        assert_eq!(
            format.files.action,
            crate::format::FileOrganizationAction::Apply
        );
        assert_eq!(
            format.files.naming,
            crate::format::FileNaming::NamespaceQualified
        );
    }

    #[test]
    fn parses_tests_and_coverage_sections() {
        let dir = tempdir().expect("tempdir");
        write_manifest(
            dir.path(),
            r#"
tests:
  enabled: true
  include_tests_dir: true
  require_testcases: true
  enforce: error
coverage:
  min_percent: 95
  enforce: error
  backend: wasm
  scope: package
"#,
        );
        let manifest = Manifest::discover(dir.path())
            .expect("discover")
            .expect("manifest");
        assert!(manifest.tests().enabled);
        assert!(manifest.tests().include_tests_dir);
        assert!(manifest.tests().require_testcases);
        assert_eq!(manifest.tests().enforce, CoverageEnforcement::Error);
        let coverage = manifest.coverage().expect("coverage");
        assert_eq!(coverage.min_percent, 95);
        assert_eq!(coverage.enforce, CoverageEnforcement::Error);
        assert_eq!(coverage.backend, CoverageBackend::Wasm);
        assert_eq!(coverage.scope, CoverageScope::Package);
    }

    #[test]
    fn discover_loads_manifest_and_merges_defaults() {
        let dir = tempdir().expect("tempdir");
        write_manifest(
            dir.path(),
            r#"
runtime:
  wasm:
    defaults:
      memory-limit-pages: 4
      env:
        LOG: info
      feature-flags:
        - simd
"#,
        );
        let nested = dir.path().join("src");
        fs::create_dir_all(&nested).expect("create nested");
        let source = nested.join("main.cl");
        File::create(&source).expect("touch source");

        let manifest = Manifest::discover(&source)
            .expect("discover")
            .expect("manifest");
        let settings = manifest
            .wasm_settings_for_target(&Target::host())
            .expect("settings");
        assert_eq!(settings.memory_limit_pages, Some(4));
        assert_eq!(settings.env.get("LOG"), Some(&"info".to_string()));
        assert!(
            settings
                .feature_flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("simd"))
        );
    }

    #[test]
    fn target_overrides_defaults() {
        let dir = tempdir().expect("tempdir");
        write_manifest(
            dir.path(),
            r#"
runtime:
  wasm:
    defaults:
      memory-limit-pages: 2
      env:
        LEVEL: warn
      feature-flags:
        - simd
    targets:
      x86_64-unknown-none:
        memory:
          max-pages: 8
        env:
          LEVEL: trace
          EXTRA: on
        feature-flags:
          - threads
      aarch64:
        memory-limit-pages: 3
"#,
        );

        let manifest = Manifest::discover(dir.path())
            .expect("discover")
            .expect("manifest");

        let x86 = manifest
            .wasm_settings_for_target(&Target::parse("x86_64-unknown-none").unwrap())
            .expect("settings");
        assert_eq!(x86.memory_limit_pages, Some(8));
        assert_eq!(x86.env.get("LEVEL"), Some(&"trace".to_string()));
        assert_eq!(x86.env.get("EXTRA"), Some(&"on".to_string()));
        assert!(
            x86.feature_flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("threads"))
        );
        assert!(
            x86.feature_flags
                .iter()
                .any(|flag| flag.eq_ignore_ascii_case("simd"))
        );

        let arm = manifest
            .wasm_settings_for_target(&Target::parse("aarch64-unknown-none").unwrap())
            .expect("settings");
        assert_eq!(arm.memory_limit_pages, Some(3));
        assert_eq!(arm.env.get("LEVEL"), Some(&"warn".to_string()));
    }
}
