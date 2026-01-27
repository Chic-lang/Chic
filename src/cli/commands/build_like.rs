use std::path::Path;
use std::{collections::HashSet, env, path::PathBuf, time::Duration};

use crate::chic_kind::ChicKind;
use crate::codegen::{Backend, CpuIsaConfig};
use crate::defines::DefineFlag;
use crate::diagnostics::ErrorFormat;
use crate::driver::types::{
    BuildPropertyOverride, TelemetrySetting, TestSelection, Verbosity, WatchdogConfig,
};
use crate::logging::{LogFormat, LogLevel, LogOptions, LogSettings};
use crate::manifest::{Manifest, MissingDocsRule, WorkspaceConfig};
use crate::runtime::backend::RuntimeBackend;
use crate::target::{Target, TargetArch, TargetOs, TargetRuntime};

use super::super::{Cli, CliError, CliFfiOptions, Command, FfiDefaultPattern};
use super::common::{
    is_help_flag, parse_chic_kind, parse_const_eval_fuel, parse_define_flag, parse_error_format,
    parse_target, partition_inputs_and_flags,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum CommandKind {
    Build,
    Run,
    Test,
    Coverage,
    Profile,
}

impl CommandKind {
    pub(super) fn verb(self) -> &'static str {
        match self {
            CommandKind::Build => "build",
            CommandKind::Run => "run",
            CommandKind::Test => "test",
            CommandKind::Coverage => "coverage",
            CommandKind::Profile => "profile",
        }
    }

    pub(super) fn help_topic(self) -> String {
        self.verb().to_string()
    }
}

pub(super) fn parse_build_like(args: Vec<String>, kind: CommandKind) -> Result<Cli, CliError> {
    let topic = kind.help_topic();
    if args.first().is_some_and(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some(topic.clone()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let (inputs, rest) = if matches!(
        kind,
        CommandKind::Build | CommandKind::Test | CommandKind::Coverage
    ) {
        partition_inputs_and_flags_allowing_empty(args)?
    } else {
        partition_inputs_and_flags(args, &format!("{} requires <file> argument", kind.verb()))?
    };
    if rest.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help { topic: Some(topic) },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }
    let workspace_mode_requested = rest.iter().any(|arg| arg == "--workspace");
    let project = resolve_inputs(inputs, kind, workspace_mode_requested)?;
    let options = parse_build_options(rest.into_iter(), kind)?;
    let BuildOptions {
        output,
        artifacts_path,
        target,
        kind: crate_kind,
        backend,
        runtime_backend,
        emit_wat,
        emit_obj,
        cpu_isa,
        emit_header,
        emit_lib,
        cc1_args,
        cc1_keep_temps,
        load_stdlib,
        run_timeout,
        const_eval_fuel,
        trace_pipeline,
        trait_solver_metrics,
        defines,
        log_settings,
        ffi,
        profile,
        error_format,
        configuration,
        no_dependencies,
        no_restore,
        no_incremental,
        disable_build_servers,
        source_root,
        properties,
        verbosity,
        telemetry,
        version_suffix,
        nologo,
        force,
        interactive,
        self_contained,
        configuration_from_cli,
        test_selection,
        test_parallelism,
        test_fail_fast,
        watchdog,
        coverage,
        coverage_min,
        workspace_mode,
        coverage_only,
        framework,
        doc_markdown,
        mut manifest_path,
        doc_enforcement,
        kind_from_cli,
    } = options;
    let log_options = log_settings.merged_with_env();

    if manifest_path.is_none() {
        manifest_path = project
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.path().map(Path::to_path_buf));
    }

    let mut configuration = configuration;
    if !configuration_from_cli {
        if let Some(value) = env::var("CHIC_CONFIGURATION")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            configuration = value;
        } else if let Some(value) = project
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.build.configuration.clone())
        {
            configuration = value;
        } else if let Some(value) = project
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.build().configuration.clone())
        {
            configuration = value;
        }
    }
    let mut defines = defines;
    append_configuration_defines(&mut defines, &configuration);
    let mut effective_kind = crate_kind;
    if !kind_from_cli {
        if let Some(kind) = project
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.build().kind)
        {
            effective_kind = kind;
        } else if let Some(kind) = project.workspace.as_ref().and_then(|ws| ws.build.kind) {
            effective_kind = kind;
        }
    }
    let mut load_stdlib = load_stdlib;
    if load_stdlib.is_none()
        && project
            .manifest
            .as_ref()
            .is_some_and(|manifest| manifest.is_no_std_runtime() || manifest.is_runtime_provider())
    {
        load_stdlib = Some(false);
    }

    Ok(Cli {
        command: match kind {
            CommandKind::Build => Command::Build {
                inputs: project.inputs.clone(),
                manifest: project.manifest.clone(),
                workspace: project.workspace.clone(),
                output,
                artifacts_path,
                target,
                kind: effective_kind,
                backend,
                runtime_backend,
                emit_wat,
                emit_obj,
                cpu_isa,
                emit_header,
                emit_lib,
                cc1_args,
                cc1_keep_temps,
                load_stdlib,
                const_eval_fuel,
                trace_pipeline,
                trait_solver_metrics,
                defines,
                ffi,
                configuration,
                framework,
                no_dependencies,
                no_restore,
                no_incremental,
                disable_build_servers,
                source_root,
                properties,
                verbosity,
                telemetry,
                version_suffix,
                nologo,
                force,
                interactive,
                self_contained,
                doc_markdown,
                manifest_path,
                doc_enforcement,
            },
            CommandKind::Run => Command::Run {
                inputs: project.inputs.clone(),
                manifest: project.manifest.clone(),
                workspace: project.workspace.clone(),
                target,
                kind: effective_kind,
                backend,
                runtime_backend,
                cpu_isa,
                run_timeout,
                const_eval_fuel,
                trace_pipeline,
                trait_solver_metrics,
                load_stdlib,
                defines,
                ffi,
                profile,
                configuration,
                artifacts_path,
                no_dependencies,
                no_restore,
                no_incremental,
                disable_build_servers,
                source_root,
                properties,
                verbosity,
                telemetry,
                version_suffix,
                nologo,
                force,
                interactive,
                self_contained,
                framework,
                doc_enforcement,
            },
            CommandKind::Test => Command::Test {
                inputs: project.inputs.clone(),
                manifest: project.manifest.clone(),
                workspace: project.workspace.clone(),
                target,
                kind: effective_kind,
                backend,
                runtime_backend,
                cpu_isa,
                const_eval_fuel,
                trace_pipeline,
                trait_solver_metrics,
                load_stdlib,
                defines,
                ffi,
                profile,
                test_options: crate::driver::types::TestOptions {
                    selection: test_selection,
                    parallelism: test_parallelism,
                    watchdog,
                    fail_fast: test_fail_fast,
                },
                coverage,
                coverage_min,
                workspace_mode,
                coverage_only,
                configuration,
                artifacts_path,
                no_dependencies,
                no_restore,
                no_incremental,
                disable_build_servers,
                source_root,
                properties,
                verbosity,
                telemetry,
                version_suffix,
                nologo,
                force,
                interactive,
                self_contained,
                framework,
                doc_enforcement,
            },
            CommandKind::Coverage => Command::Test {
                inputs: project.inputs.clone(),
                manifest: project.manifest.clone(),
                workspace: project.workspace.clone(),
                target,
                kind: effective_kind,
                backend,
                runtime_backend,
                cpu_isa,
                const_eval_fuel,
                trace_pipeline,
                trait_solver_metrics,
                load_stdlib,
                defines,
                ffi,
                profile,
                test_options: crate::driver::types::TestOptions {
                    selection: test_selection,
                    parallelism: test_parallelism,
                    watchdog,
                    fail_fast: test_fail_fast,
                },
                coverage,
                coverage_min,
                workspace_mode,
                coverage_only,
                configuration,
                artifacts_path,
                no_dependencies,
                no_restore,
                no_incremental,
                disable_build_servers,
                source_root,
                properties,
                verbosity,
                telemetry,
                version_suffix,
                nologo,
                force,
                interactive,
                self_contained,
                framework,
                doc_enforcement,
            },
            CommandKind::Profile => Command::Run {
                inputs: project.inputs.clone(),
                manifest: project.manifest.clone(),
                workspace: project.workspace.clone(),
                target,
                kind: crate_kind,
                backend,
                runtime_backend,
                cpu_isa,
                run_timeout,
                const_eval_fuel,
                trace_pipeline,
                trait_solver_metrics,
                load_stdlib,
                defines,
                ffi,
                profile,
                configuration,
                artifacts_path,
                no_dependencies,
                no_restore,
                no_incremental,
                disable_build_servers,
                source_root,
                properties,
                verbosity,
                telemetry,
                version_suffix,
                nologo,
                force,
                interactive,
                self_contained,
                framework,
                doc_enforcement,
            },
        },
        log_options,
        error_format,
    })
}

struct BuildOptions {
    output: Option<PathBuf>,
    artifacts_path: Option<PathBuf>,
    target: Target,
    kind: ChicKind,
    backend: Backend,
    runtime_backend: RuntimeBackend,
    emit_wat: bool,
    emit_obj: bool,
    cpu_isa: CpuIsaConfig,
    emit_header: bool,
    emit_lib: bool,
    cc1_args: Vec<String>,
    cc1_keep_temps: bool,
    load_stdlib: Option<bool>,
    run_timeout: Option<Duration>,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    defines: Vec<DefineFlag>,
    log_settings: LogSettings,
    ffi: CliFfiOptions,
    profile: Option<crate::cli::ProfileOptions>,
    error_format: Option<ErrorFormat>,
    configuration: String,
    configuration_from_cli: bool,
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
    test_selection: TestSelection,
    test_parallelism: Option<usize>,
    test_fail_fast: bool,
    watchdog: WatchdogConfig,
    coverage: bool,
    coverage_min: Option<u8>,
    workspace_mode: bool,
    coverage_only: bool,
    framework: Option<String>,
    doc_markdown: bool,
    manifest_path: Option<PathBuf>,
    doc_enforcement: MissingDocsRule,
    kind_from_cli: bool,
}

#[derive(Default)]
struct BuildOptionState {
    output: Option<PathBuf>,
    artifacts_path: Option<PathBuf>,
    target: Option<Target>,
    target_arch: Option<TargetArch>,
    target_os: Option<TargetOs>,
    target_runtime: Option<TargetRuntime>,
    use_current_runtime: bool,
    kind: Option<ChicKind>,
    kind_from_cli: bool,
    backend: Option<Backend>,
    runtime_backend: Option<RuntimeBackend>,
    emit_wat: bool,
    emit_obj: bool,
    cpu_isa: Option<CpuIsaConfig>,
    sve_bits: Option<u32>,
    emit_header: bool,
    emit_lib: bool,
    cc1_args: Vec<String>,
    cc1_keep_temps: bool,
    run_timeout: Option<Duration>,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    defines: Vec<DefineFlag>,
    log_settings: LogSettings,
    ffi_search_paths: Vec<PathBuf>,
    ffi_defaults: Vec<FfiDefaultPattern>,
    ffi_packages: Vec<String>,
    profile_enabled: bool,
    profile_output: Option<PathBuf>,
    profile_sample_ms: Option<u64>,
    profile_flamegraph: bool,
    error_format: Option<ErrorFormat>,
    configuration: Option<String>,
    no_dependencies: bool,
    no_restore: bool,
    no_incremental: bool,
    disable_build_servers: bool,
    source_root: Option<PathBuf>,
    properties: Vec<BuildPropertyOverride>,
    verbosity: Option<Verbosity>,
    telemetry: Option<TelemetrySetting>,
    version_suffix: Option<String>,
    nologo: bool,
    force: bool,
    interactive: bool,
    self_contained: Option<bool>,
    load_stdlib: Option<bool>,
    test_selection: TestSelection,
    test_parallelism: Option<usize>,
    test_fail_fast: bool,
    watchdog: WatchdogConfig,
    coverage: bool,
    coverage_min: Option<u8>,
    workspace_mode: bool,
    framework: Option<String>,
    doc_markdown: bool,
    manifest_path: Option<PathBuf>,
    doc_enforcement: MissingDocsRule,
}

impl BuildOptionState {
    fn consume_flag(&mut self, args: &[String], command: CommandKind) -> Result<usize, CliError> {
        let Some(flag) = args.first() else {
            return Err(CliError::with_usage("missing command option"));
        };

        match flag.as_str() {
            "-o" | "--output" => self.consume_output(args, command),
            "--artifacts-path" => self.consume_artifacts_path(args, command),
            "-t" | "--target" => self.consume_target(args),
            "-a" | "--arch" => self.consume_arch(args),
            "--os" => self.consume_os(args),
            "--ucr" | "--use-current-runtime" => {
                self.use_current_runtime = true;
                Ok(1)
            }
            "-c" | "--configuration" => self.consume_configuration(args),
            "--crate-type" | "--kind" => self.consume_kind(args),
            "--backend" => self.consume_backend(args),
            "--runtime" => self.consume_runtime(args),
            "--runtime-backend" => self.consume_runtime_backend(args),
            "--emit-wat" => self.consume_emit_wat(command),
            "--emit=obj" => self.consume_emit_obj(command),
            "--emit-object" => self.consume_emit_obj(command),
            "--cpu-isa" => self.consume_cpu_isa(args),
            "--sve-bits" => self.consume_sve_bits(args),
            "--emit-header" => self.consume_emit_header(command),
            "--emit-lib" => self.consume_emit_lib(command),
            "--cc1-arg" => self.consume_cc1_arg(args, command),
            "--cc1-keep-input" => self.consume_cc1_keep(command),
            "--consteval-fuel" => self.consume_const_eval_fuel(args),
            "--run-timeout" => self.consume_run_timeout(args, command),
            "-D" | "--define" => self.consume_define(args),
            "--ffi-search" => self.consume_ffi_search(args),
            "--ffi-default" => self.consume_ffi_default(args),
            "--ffi-package" => self.consume_ffi_package(args),
            "--no-dependencies" => {
                self.no_dependencies = true;
                Ok(1)
            }
            "--no-restore" => {
                self.no_restore = true;
                Ok(1)
            }
            "--no-incremental" => {
                self.no_incremental = true;
                Ok(1)
            }
            "--disable-build-servers" => {
                self.disable_build_servers = true;
                Ok(1)
            }
            "--source" => self.consume_source_root(args, command),
            "--sc" | "--self-contained" => {
                self.self_contained = Some(true);
                Ok(1)
            }
            "--no-self-contained" => {
                self.self_contained = Some(false);
                Ok(1)
            }
            "-v" | "--verbosity" => self.consume_verbosity(args),
            other if other.starts_with("--tl:") => self.consume_inline_telemetry(other),
            "--tl" => self.consume_telemetry(args),
            "--version-suffix" => self.consume_version_suffix(args, command),
            "--nologo" => {
                self.nologo = true;
                Ok(1)
            }
            "--force" => {
                self.force = true;
                Ok(1)
            }
            "--interactive" => {
                self.interactive = true;
                Ok(1)
            }
            other if other.starts_with("--property:") || other.starts_with("-p:") => {
                self.consume_inline_property(other)
            }
            "-p" | "--property" => self.consume_property(args),
            "--trace-pipeline" => {
                self.trace_pipeline = true;
                Ok(1)
            }
            "--trait-solver-metrics" => {
                self.trait_solver_metrics = true;
                Ok(1)
            }
            "--test" => self.consume_test_filter(args, command),
            "--test-group" => self.consume_test_group(args, command),
            "--all" | "--test-all" => self.consume_test_all(command),
            "--test-parallel" => self.consume_test_parallel(args, command),
            "--fail-fast" => {
                self.require_test_command(command, "--fail-fast")?;
                self.test_fail_fast = true;
                Ok(1)
            }
            "--watchdog" => self.consume_watchdog_limit(args, command),
            "--watchdog-timeout" => self.consume_watchdog_timeout(args, command),
            "--coverage" => {
                self.require_test_command(command, "--coverage")?;
                self.coverage = true;
                Ok(1)
            }
            "--min" | "--coverage-min" => self.consume_coverage_min(args, command),
            "--workspace" => {
                self.require_test_command(command, "--workspace")?;
                self.workspace_mode = true;
                Ok(1)
            }
            "--profile" => self.consume_profile_enabled(command),
            "--profile-out" => self.consume_profile_out(args, command),
            "--profile-sample-ms" => self.consume_profile_sample(args, command),
            "--profile-flamegraph" => self.consume_profile_flamegraph(command),
            "--log-format" => self.consume_log_format(args),
            "--log-level" => self.consume_log_level(args),
            "--error-format" => {
                let value = Self::next_value(args, "expected value after --error-format")?;
                self.error_format = Some(parse_error_format(value)?);
                Ok(2)
            }
            other => Err(CliError::with_usage(format!(
                "unsupported option '{other}' for command"
            ))),
        }
    }

    fn consume_output(&mut self, args: &[String], command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "output path is only supported for chic build",
            ));
        }
        let value = Self::next_value(args, "expected path after -o/--output")?;
        self.output = Some(PathBuf::from(value));
        Ok(2)
    }

    fn consume_artifacts_path(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--artifacts-path is only supported for chic build",
            ));
        }
        let value = Self::next_value(args, "expected path after --artifacts-path")?;
        self.artifacts_path = Some(PathBuf::from(value));
        Ok(2)
    }

    fn consume_target(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected triple after -t/--target")?;
        self.target = Some(parse_target(value)?);
        Ok(2)
    }

    fn consume_arch(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected architecture after -a/--arch")?;
        let parsed = TargetArch::parse(value).ok_or_else(|| {
            CliError::with_usage(format!(
                "unsupported architecture '{value}'; expected x86_64|amd64 or aarch64|arm64"
            ))
        })?;
        self.target_arch = Some(parsed);
        Ok(2)
    }

    fn consume_os(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected operating system after --os")?;
        let parsed = parse_target_os(value)?;
        self.target_os = Some(parsed);
        Ok(2)
    }

    fn consume_kind(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after --crate-type/--kind")?;
        self.kind = Some(parse_chic_kind(value)?);
        self.kind_from_cli = true;
        Ok(2)
    }

    fn consume_configuration(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after -c/--configuration")?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CliError::with_usage(
                "--configuration requires a non-empty value",
            ));
        }
        self.configuration = Some(trimmed.to_string());
        Ok(2)
    }

    fn consume_backend(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected backend after --backend")?;
        self.backend = Some(parse_backend(value)?);
        Ok(2)
    }

    fn consume_runtime_backend(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected runtime after --runtime-backend")?;
        self.runtime_backend = Some(parse_runtime_backend(value)?);
        Ok(2)
    }

    fn consume_runtime(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected runtime after --runtime")?;
        if let Ok(runtime_backend) = parse_runtime_backend(value) {
            self.runtime_backend = Some(runtime_backend);
        } else {
            let runtime = parse_runtime_flavor(value)?;
            self.target_runtime = Some(runtime);
        }
        Ok(2)
    }

    fn consume_cpu_isa(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected list after --cpu-isa")?;
        self.cpu_isa = Some(parse_cpu_isa(value)?);
        Ok(2)
    }

    fn consume_sve_bits(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected bit-length after --sve-bits")?;
        self.sve_bits = Some(parse_sve_bits(value)?);
        Ok(2)
    }

    fn consume_emit_wat(&mut self, command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--emit-wat is only supported for chic build",
            ));
        }
        self.emit_wat = true;
        Ok(1)
    }

    fn consume_emit_obj(&mut self, command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--emit=obj/--emit-object is only supported for chic build",
            ));
        }
        self.emit_obj = true;
        Ok(1)
    }

    fn consume_emit_header(&mut self, command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--emit-header is only supported for chic build",
            ));
        }
        self.emit_header = true;
        Ok(1)
    }

    fn consume_emit_lib(&mut self, command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--emit-lib is only supported for chic build",
            ));
        }
        self.emit_lib = true;
        Ok(1)
    }

    fn consume_profile_enabled(&mut self, command: CommandKind) -> Result<usize, CliError> {
        self.require_profile(command, "--profile")?;
        self.profile_enabled = true;
        Ok(1)
    }

    fn consume_profile_out(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_profile(command, "--profile-out")?;
        let value = Self::next_value(args, "expected path after --profile-out")?;
        self.profile_output = Some(PathBuf::from(value));
        self.profile_enabled = true;
        Ok(2)
    }

    fn consume_profile_sample(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_profile(command, "--profile-sample-ms")?;
        let value = Self::next_value(args, "expected integer after --profile-sample-ms")?;
        let parsed = value.parse::<u64>().map_err(|_| {
            CliError::with_usage(format!(
                "unable to parse sample interval `{value}` (expected milliseconds)"
            ))
        })?;
        self.profile_sample_ms = Some(parsed);
        self.profile_enabled = true;
        Ok(2)
    }

    fn consume_profile_flamegraph(&mut self, command: CommandKind) -> Result<usize, CliError> {
        self.require_profile(command, "--profile-flamegraph")?;
        self.profile_flamegraph = true;
        self.profile_enabled = true;
        Ok(1)
    }

    fn require_profile(&self, command: CommandKind, flag: &str) -> Result<(), CliError> {
        if matches!(
            command,
            CommandKind::Run | CommandKind::Test | CommandKind::Profile
        ) {
            return Ok(());
        }
        Err(CliError::with_usage(format!(
            "{flag} is only supported for chic run, chic test, or chic profile"
        )))
    }

    fn consume_cc1_arg(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--cc1-arg is only supported for chic build",
            ));
        }
        let value = Self::next_value(args, "expected value after --cc1-arg")?;
        self.cc1_args.push(value.to_string());
        Ok(2)
    }

    fn consume_cc1_keep(&mut self, command: CommandKind) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--cc1-keep-input is only supported for chic build",
            ));
        }
        self.cc1_keep_temps = true;
        Ok(1)
    }

    fn consume_const_eval_fuel(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after --consteval-fuel")?;
        let parsed = parse_const_eval_fuel(value)?;
        self.const_eval_fuel = Some(parsed);
        Ok(2)
    }

    fn consume_run_timeout(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Run | CommandKind::Profile) {
            return Err(CliError::with_usage(
                "--run-timeout is only supported for chic run (or chic profile)",
            ));
        }
        let value = Self::next_value(args, "expected timeout (ms) after --run-timeout")?;
        let parsed = value.parse::<u64>().map_err(|_| {
            CliError::with_usage(format!(
                "unable to parse run timeout `{value}` (expected non-negative milliseconds)"
            ))
        })?;
        self.run_timeout = if parsed == 0 {
            None
        } else {
            Some(Duration::from_millis(parsed))
        };
        Ok(2)
    }

    fn consume_define(&mut self, args: &[String]) -> Result<usize, CliError> {
        let raw = Self::next_value(args, "expected value after --define/-D")?;
        let flag = parse_define_flag(raw)?;
        self.defines.push(flag);
        Ok(2)
    }

    fn consume_ffi_search(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected path after --ffi-search")?;
        self.ffi_search_paths.push(PathBuf::from(value));
        Ok(2)
    }

    fn consume_ffi_default(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected <os>=<pattern> after --ffi-default")?;
        let mut parts = value.splitn(2, '=');
        let Some(target) = parts.next() else {
            return Err(CliError::with_usage(
                "--ffi-default requires <os>=<pattern>",
            ));
        };
        let Some(pattern) = parts.next() else {
            return Err(CliError::with_usage(
                "--ffi-default requires <os>=<pattern>",
            ));
        };
        if target.trim().is_empty() || pattern.trim().is_empty() {
            return Err(CliError::with_usage(
                "--ffi-default requires non-empty <os>=<pattern>",
            ));
        }
        self.ffi_defaults.push(FfiDefaultPattern {
            target: target.trim().to_ascii_lowercase(),
            pattern: pattern.to_string(),
        });
        Ok(2)
    }

    fn consume_ffi_package(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected glob after --ffi-package")?;
        self.ffi_packages.push(value.to_string());
        Ok(2)
    }

    fn consume_source_root(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--source is only supported for chic build",
            ));
        }
        let value = Self::next_value(args, "expected path after --source")?;
        self.source_root = Some(PathBuf::from(value));
        Ok(2)
    }

    fn consume_verbosity(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after -v/--verbosity")?;
        let parsed = parse_verbosity(value)?;
        self.log_settings.apply_level(parsed.to_log_level());
        self.verbosity = Some(parsed);
        Ok(2)
    }

    fn consume_inline_telemetry(&mut self, flag: &str) -> Result<usize, CliError> {
        let Some((_, value)) = flag.split_once(':') else {
            return Err(CliError::with_usage(
                "--tl:<value> requires auto, on, or off",
            ));
        };
        self.telemetry = Some(parse_telemetry(value)?);
        Ok(1)
    }

    fn consume_telemetry(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after --tl")?;
        self.telemetry = Some(parse_telemetry(value)?);
        Ok(2)
    }

    fn consume_version_suffix(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        if !matches!(command, CommandKind::Build) {
            return Err(CliError::with_usage(
                "--version-suffix is only supported for chic build",
            ));
        }
        let value = Self::next_value(args, "expected value after --version-suffix")?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CliError::with_usage(
                "--version-suffix requires a non-empty value",
            ));
        }
        self.version_suffix = Some(trimmed.to_string());
        Ok(2)
    }

    fn consume_inline_property(&mut self, flag: &str) -> Result<usize, CliError> {
        let Some((_, rest)) = flag.split_once(':') else {
            return Err(CliError::with_usage(
                "--property:<name>=<value> requires a property name",
            ));
        };
        let property = parse_property_override(rest)?;
        self.apply_property_override(property)?;
        Ok(1)
    }

    fn consume_property(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected <name>=<value> after -p/--property")?;
        let property = parse_property_override(value)?;
        self.apply_property_override(property)?;
        Ok(2)
    }

    fn require_test_command(&self, command: CommandKind, flag: &str) -> Result<(), CliError> {
        if !matches!(command, CommandKind::Test | CommandKind::Coverage) {
            return Err(CliError::with_usage(format!(
                "{flag} is only supported for chic test/coverage"
            )));
        }
        Ok(())
    }

    fn consume_coverage_min(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--min")?;
        let value = Self::next_value(args, "expected integer percent after --min")?;
        let parsed: u8 = value
            .parse()
            .map_err(|_| CliError::with_usage("--min expects an integer percent (0-100)"))?;
        if parsed > 100 {
            return Err(CliError::with_usage(
                "--min expects an integer percent (0-100)",
            ));
        }
        self.coverage_min = Some(parsed);
        Ok(2)
    }

    fn consume_test_filter(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--test")?;
        let value = Self::next_value(args, "expected testcase name or pattern after --test")?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CliError::with_usage(
                "--test requires a non-empty testcase name or pattern",
            ));
        }
        self.test_selection.tests.push(trimmed.to_string());
        Ok(2)
    }

    fn consume_test_group(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--test-group")?;
        let value = Self::next_value(
            args,
            "expected namespace/category pattern after --test-group",
        )?;
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(CliError::with_usage(
                "--test-group requires a non-empty namespace/category/tag pattern",
            ));
        }
        self.test_selection.groups.push(trimmed.to_string());
        Ok(2)
    }

    fn consume_test_all(&mut self, command: CommandKind) -> Result<usize, CliError> {
        self.require_test_command(command, "--all")?;
        self.test_selection.run_all = true;
        self.test_selection.tests.clear();
        self.test_selection.groups.clear();
        Ok(1)
    }

    fn consume_test_parallel(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--test-parallel")?;
        let value = Self::next_value(args, "expected worker count after --test-parallel")?;
        let parsed = value
            .trim()
            .parse::<usize>()
            .map_err(|_| CliError::with_usage("--test-parallel expects a positive integer"))?;
        if parsed == 0 {
            return Err(CliError::with_usage(
                "--test-parallel expects a positive integer",
            ));
        }
        self.test_parallelism = Some(parsed);
        Ok(2)
    }

    fn consume_watchdog_limit(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--watchdog")?;
        let value = Self::next_value(args, "expected step limit after --watchdog")?;
        let limit = value.trim().parse::<u64>().map_err(|_| {
            CliError::with_usage("--watchdog expects a non-negative integer (0 disables)")
        })?;
        self.watchdog.enable_in_release = true;
        self.watchdog.step_limit = if limit == 0 { None } else { Some(limit) };
        Ok(2)
    }

    fn consume_watchdog_timeout(
        &mut self,
        args: &[String],
        command: CommandKind,
    ) -> Result<usize, CliError> {
        self.require_test_command(command, "--watchdog-timeout")?;
        let value = Self::next_value(args, "expected timeout (ms) after --watchdog-timeout")?;
        let parsed = value.trim().parse::<u64>().map_err(|_| {
            CliError::with_usage(
                "--watchdog-timeout expects a non-negative integer in milliseconds (0 disables)",
            )
        })?;
        self.watchdog.enable_in_release = true;
        self.watchdog.timeout = if parsed == 0 {
            None
        } else {
            Some(Duration::from_millis(parsed))
        };
        Ok(2)
    }

    fn apply_property_override(&mut self, property: BuildPropertyOverride) -> Result<(), CliError> {
        let key = property.name.to_ascii_lowercase();
        match key.as_str() {
            "toolchain.cpu-isa" => {
                self.cpu_isa = Some(parse_cpu_isa(&property.value)?);
            }
            "load_stdlib" => {
                let value = parse_bool_property(&property.value)?;
                self.load_stdlib = Some(value);
            }
            "defines" => {
                for raw in property
                    .value
                    .split(|ch| ch == ';' || ch == ',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let flag = parse_define_flag(raw)?;
                    self.defines.push(flag);
                }
            }
            _ => {}
        }
        self.properties.push(property);
        Ok(())
    }

    fn consume_log_format(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after --log-format")?;
        let Some(format) = LogFormat::parse(value) else {
            return Err(CliError::with_usage(format!(
                "invalid log format '{value}'; supported values: auto, text, json"
            )));
        };
        self.log_settings.apply_format(format);
        Ok(2)
    }

    fn consume_log_level(&mut self, args: &[String]) -> Result<usize, CliError> {
        let value = Self::next_value(args, "expected value after --log-level")?;
        let Some(level) = LogLevel::parse(value) else {
            return Err(CliError::with_usage(format!(
                "invalid log level '{value}'; supported values: error, warn, info, debug, trace"
            )));
        };
        self.log_settings.apply_level(level);
        Ok(2)
    }

    fn next_value<'a>(args: &'a [String], message: &str) -> Result<&'a str, CliError> {
        args.get(1)
            .map(String::as_str)
            .ok_or_else(|| CliError::with_usage(message))
    }

    fn finish(self, command: CommandKind) -> Result<BuildOptions, CliError> {
        let host_target = Target::host();
        let had_explicit_target = self.target.is_some();
        let base_target = if self.use_current_runtime {
            host_target.clone()
        } else {
            self.target.unwrap_or_else(|| host_target.clone())
        };
        let arch = self.target_arch.unwrap_or(base_target.arch());
        let os = self.target_os.unwrap_or_else(|| base_target.os().clone());

        let target_runtime = self.target_runtime.clone();
        let mut runtime_flavor = if let Some(explicit_runtime) = target_runtime {
            explicit_runtime
        } else if had_explicit_target {
            base_target.runtime().clone()
        } else if let Some(backend) = self.backend {
            runtime_for_backend(backend)
        } else {
            base_target.runtime().clone()
        };

        if let Some(explicit_backend) = self.backend {
            if self.target_runtime.is_none() && !had_explicit_target {
                runtime_flavor = runtime_for_backend(explicit_backend);
            }
        }

        let backend = self
            .backend
            .unwrap_or_else(|| backend_for_runtime(runtime_flavor.clone()));
        if let Some(explicit_backend) = self.backend {
            let derived = backend_for_runtime(runtime_flavor.clone());
            let compatible = derived == explicit_backend
                || (matches!(explicit_backend, Backend::Cc1) && matches!(derived, Backend::Llvm));
            if !compatible {
                return Err(CliError::with_usage(
                    "--runtime value conflicts with --backend selection",
                ));
            }
        }
        let kind = self.kind.unwrap_or_default();
        let kind_from_cli = self.kind_from_cli;

        if self.emit_obj && backend == Backend::Cc1 {
            return Err(CliError::with_usage(
                "--emit=obj currently requires a non-cc1 backend",
            ));
        }
        if backend == Backend::Wasm && matches!(self.self_contained, Some(false)) {
            return Err(CliError::with_usage(
                "--no-self-contained is not supported for wasm targets",
            ));
        }
        let mut cpu_isa = self.cpu_isa.clone().unwrap_or_default();
        if let Some(bits) = self.sve_bits {
            cpu_isa.set_sve_bits(bits).map_err(CliError::with_usage)?;
        }

        if matches!(command, CommandKind::Run | CommandKind::Profile) && kind.is_library() {
            return Err(CliError::with_usage(
                "chic run only supports executable crate types",
            ));
        }
        if self.emit_wat && backend != Backend::Wasm {
            return Err(CliError::with_usage("--emit-wat requires --backend wasm"));
        }

        if (self.emit_header || self.emit_lib) && !kind.is_library() {
            return Err(CliError::with_usage(
                "--emit-header/--emit-lib require a library crate type (--crate-type lib or dylib)",
            ));
        }

        if self.emit_lib && backend != Backend::Llvm {
            return Err(CliError::with_usage(
                "--emit-lib currently requires the LLVM backend (--backend llvm)",
            ));
        }

        if self.emit_obj && self.emit_lib {
            return Err(CliError::with_usage(
                "--emit-lib cannot be combined with --emit=obj",
            ));
        }

        if !matches!(command, CommandKind::Build) && backend == Backend::Cc1 {
            return Err(CliError::with_usage(
                "--backend cc1 is only supported for chic build",
            ));
        }

        if (!self.cc1_args.is_empty() || self.cc1_keep_temps) && backend != Backend::Cc1 {
            return Err(CliError::with_usage(
                "--cc1-arg/--cc1-keep-input require --backend cc1",
            ));
        }

        let profile = if matches!(
            command,
            CommandKind::Run | CommandKind::Test | CommandKind::Profile
        ) {
            if self.profile_enabled
                || self.profile_output.is_some()
                || self.profile_sample_ms.is_some()
                || self.profile_flamegraph
            {
                let mut opts = crate::cli::ProfileOptions::default();
                if let Some(path) = self.profile_output {
                    opts.output = path;
                }
                if let Some(ms) = self.profile_sample_ms {
                    opts.sample_ms = Some(ms);
                }
                opts.flamegraph = self.profile_flamegraph;
                Some(opts)
            } else if matches!(command, CommandKind::Profile) {
                let mut opts = crate::cli::ProfileOptions::default();
                opts.flamegraph = true;
                Some(opts)
            } else {
                None
            }
        } else if self.profile_enabled
            || self.profile_output.is_some()
            || self.profile_sample_ms.is_some()
            || self.profile_flamegraph
        {
            return Err(CliError::with_usage(
                "profiling options are only supported for chic run, chic test, or chic profile",
            ));
        } else {
            None
        };

        let same_target = arch == base_target.arch()
            && os == base_target.os().clone()
            && runtime_flavor == base_target.runtime().clone();
        let target = if same_target {
            base_target
        } else {
            Target::from_components(arch, os, runtime_flavor.clone())
        };

        let runtime_backend = self.runtime_backend.unwrap_or(RuntimeBackend::Chic);

        let configuration_set = self.configuration.is_some();
        let configuration = self.configuration.unwrap_or_else(|| "Debug".to_string());
        let verbosity = self.verbosity.unwrap_or_default();
        let telemetry = self.telemetry.unwrap_or_default();
        let defines = self.defines;
        let mut load_stdlib = self.load_stdlib;
        if load_stdlib.is_none() && matches!(&runtime_flavor, TargetRuntime::NativeNoStd) {
            load_stdlib = Some(false);
        }
        let test_selection = if matches!(command, CommandKind::Test | CommandKind::Coverage) {
            resolve_test_selection(self.test_selection.clone())
        } else {
            TestSelection::default()
        };
        let test_parallelism = if matches!(command, CommandKind::Test | CommandKind::Coverage) {
            resolve_test_parallelism(self.test_parallelism)
        } else {
            None
        };
        let test_fail_fast = if matches!(command, CommandKind::Test | CommandKind::Coverage) {
            self.test_fail_fast || env_flag_truthy("CHIC_TEST_FAIL_FAST").unwrap_or(false)
        } else {
            false
        };
        let mut watchdog = if matches!(command, CommandKind::Test | CommandKind::Coverage) {
            self.watchdog
        } else {
            WatchdogConfig::default()
        };
        if env_flag_truthy("CHIC_TEST_WATCHDOG_ENABLE_RELEASE").unwrap_or(false) {
            watchdog.enable_in_release = true;
        }

        Ok(BuildOptions {
            output: self.output,
            artifacts_path: self.artifacts_path,
            target,
            kind,
            backend,
            runtime_backend,
            emit_wat: self.emit_wat,
            emit_obj: self.emit_obj,
            cpu_isa,
            emit_header: self.emit_header,
            emit_lib: self.emit_lib,
            cc1_args: self.cc1_args,
            cc1_keep_temps: self.cc1_keep_temps,
            load_stdlib,
            run_timeout: self.run_timeout,
            const_eval_fuel: self.const_eval_fuel,
            trace_pipeline: self.trace_pipeline,
            trait_solver_metrics: self.trait_solver_metrics,
            defines,
            log_settings: self.log_settings,
            ffi: CliFfiOptions {
                search_paths: self.ffi_search_paths,
                default_patterns: self.ffi_defaults,
                package_globs: self.ffi_packages,
            },
            profile,
            error_format: self.error_format,
            configuration,
            configuration_from_cli: configuration_set,
            no_dependencies: self.no_dependencies,
            no_restore: self.no_restore,
            no_incremental: self.no_incremental,
            disable_build_servers: self.disable_build_servers,
            source_root: self.source_root,
            properties: self.properties,
            verbosity,
            telemetry,
            version_suffix: self.version_suffix,
            nologo: self.nologo,
            force: self.force,
            interactive: self.interactive,
            self_contained: self.self_contained,
            test_selection,
            test_parallelism,
            test_fail_fast,
            watchdog,
            coverage: self.coverage || matches!(command, CommandKind::Coverage),
            coverage_min: self.coverage_min,
            workspace_mode: self.workspace_mode,
            coverage_only: matches!(command, CommandKind::Coverage),
            framework: self.framework,
            doc_markdown: self.doc_markdown,
            manifest_path: self.manifest_path,
            doc_enforcement: self.doc_enforcement,
            kind_from_cli,
        })
    }
}

fn parse_build_options<I, T>(args: I, command: CommandKind) -> Result<BuildOptions, CliError>
where
    I: Iterator<Item = T>,
    T: Into<String>,
{
    let iter = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut idx = 0;
    let mut state = BuildOptionState::default();

    while idx < iter.len() {
        let consumed = state.consume_flag(&iter[idx..], command)?;
        idx += consumed;
    }

    state.finish(command)
}

fn partition_inputs_and_flags_allowing_empty(
    args: Vec<String>,
) -> Result<(Vec<PathBuf>, Vec<String>), CliError> {
    if args.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut inputs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let value = &args[index];
        if value.starts_with('-') {
            break;
        }
        if value.trim().is_empty() {
            return Err(CliError::with_usage("input path must not be empty"));
        }
        inputs.push(PathBuf::from(value));
        index += 1;
    }
    let rest = args.into_iter().skip(index).collect();
    Ok((inputs, rest))
}

struct ProjectResolution {
    inputs: Vec<PathBuf>,
    manifest: Option<Manifest>,
    workspace: Option<WorkspaceConfig>,
}

fn resolve_inputs(
    inputs: Vec<PathBuf>,
    command: CommandKind,
    workspace_mode_requested: bool,
) -> Result<ProjectResolution, CliError> {
    let include_tests = matches!(command, CommandKind::Test | CommandKind::Coverage);
    let mut resolved_inputs = Vec::new();
    let mut manifest = None;
    let mut workspace = None;
    let mut raw_inputs = inputs;

    if raw_inputs.is_empty() && matches!(command, CommandKind::Build) {
        let cwd = env::current_dir()
            .map_err(|err| CliError::new(format!("failed to read current directory: {err}")))?;
        raw_inputs.push(cwd);
    }

    if workspace_mode_requested {
        let start = raw_inputs
            .first()
            .cloned()
            .or_else(|| env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        workspace =
            WorkspaceConfig::discover(&start).map_err(|err| CliError::new(err.to_string()))?;
        if workspace.is_none() {
            return Err(CliError::new(
                "No manifest.workspace.yaml found for --workspace invocation",
            ));
        }
        return Ok(ProjectResolution {
            inputs: Vec::new(),
            manifest: None,
            workspace,
        });
    }

    if raw_inputs.is_empty() {
        return Err(CliError::with_usage(&format!(
            "{} requires <file> argument",
            command.verb()
        )));
    }

    for input in raw_inputs {
        if should_try_manifest(&input) {
            match Manifest::discover(&input).map_err(|err| CliError::new(err.to_string()))? {
                Some(project_manifest) => {
                    let manifest_dir = project_manifest
                        .path()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| input.clone());
                    let mut files =
                        collect_project_files(&project_manifest, &manifest_dir, include_tests)?;
                    resolved_inputs.append(&mut files);
                    if manifest.is_none() {
                        manifest = Some(project_manifest);
                    }
                    if workspace.is_none() {
                        workspace = WorkspaceConfig::discover(&manifest_dir)
                            .map_err(|err| CliError::new(err.to_string()))?;
                    }
                    continue;
                }
                None => {
                    return Err(CliError::new("No manifest.yaml project file found"));
                }
            }
        }

        if input.extension().is_some_and(|ext| ext == "ch") {
            if !input.exists() {
                resolved_inputs.push(input);
                continue;
            }
            match Manifest::discover(&input).map_err(|err| CliError::new(err.to_string()))? {
                Some(project_manifest) => {
                    let manifest_dir = project_manifest
                        .path()
                        .and_then(Path::parent)
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| {
                            input
                                .parent()
                                .map(Path::to_path_buf)
                                .unwrap_or_else(|| input.clone())
                        });
                    let mut files =
                        collect_project_files(&project_manifest, &manifest_dir, include_tests)?;
                    resolved_inputs.append(&mut files);
                    if manifest.is_none() {
                        manifest = Some(project_manifest);
                    }
                    if workspace.is_none() {
                        workspace = WorkspaceConfig::discover(&manifest_dir)
                            .map_err(|err| CliError::new(err.to_string()))?;
                    }
                    continue;
                }
                None => {
                    resolved_inputs.push(input);
                    continue;
                }
            }
        }

        resolved_inputs.push(input);
    }

    if resolved_inputs.is_empty() {
        return Err(CliError::new("No manifest.yaml project file found"));
    }

    // Preserve the caller-provided ordering so explicitly ordered sources (for
    // example, bootstrapping files that define shared runtime structs) are
    // processed before their dependents. Deduplicate while keeping the first
    // occurrence of each path.
    let mut seen = HashSet::new();
    resolved_inputs.retain(|path| seen.insert(path.clone()));
    let resolved_inputs: Vec<_> = resolved_inputs
        .into_iter()
        .map(|path| {
            if let Ok(canon) = std::fs::canonicalize(&path) {
                return canon;
            }
            if path.is_absolute() {
                path
            } else {
                env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(path)
            }
        })
        .collect();

    Ok(ProjectResolution {
        inputs: resolved_inputs,
        manifest,
        workspace,
    })
}

fn should_try_manifest(path: &Path) -> bool {
    path.is_dir() || path.extension().is_some_and(|ext| ext == "yaml")
}

pub(crate) fn collect_project_files(
    manifest: &Manifest,
    manifest_dir: &Path,
    include_tests: bool,
) -> Result<Vec<PathBuf>, CliError> {
    let mut roots: Vec<PathBuf> = manifest
        .derived_source_roots()
        .into_iter()
        .map(|root| manifest_dir.join(root.path))
        .collect();
    if include_tests && manifest.tests().enabled && manifest.tests().include_tests_dir {
        roots.push(manifest_dir.join("tests"));
    }
    roots.retain(|root| root.exists());

    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            files.push(root);
        } else {
            collect_ch_files(&root, &mut files)?;
        }
    }

    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(CliError::new(format!(
            "no Chic source files found under {}",
            manifest_dir.display()
        )));
    }
    Ok(files)
}

pub(crate) fn collect_ch_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    let entries = std::fs::read_dir(root).map_err(|err| {
        CliError::new(format!(
            "failed to read source directory {}: {err}",
            root.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            CliError::new(format!(
                "failed to read source directory {}: {err}",
                root.display()
            ))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_ch_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "ch") {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_backend(spec: &str) -> Result<Backend, CliError> {
    match spec.to_ascii_lowercase().as_str() {
        "llvm" => Ok(Backend::Llvm),
        "wasm" | "wasmtime" => Ok(Backend::Wasm),
        "cc1" => Ok(Backend::Cc1),
        "" => Err(CliError::with_usage("backend must not be empty")),
        other => Err(CliError::with_usage(format!(
            "unsupported backend '{other}'; expected llvm, wasm, or cc1"
        ))),
    }
}

fn parse_runtime_backend(spec: &str) -> Result<RuntimeBackend, CliError> {
    match spec.to_ascii_lowercase().as_str() {
        "chic" | "chicx" | "native" => Ok(RuntimeBackend::Chic),
        "rust" | "rust-shim" | "shim" => Err(CliError::with_usage(
            "the Rust runtime shim has been removed; chic is the only supported runtime",
        )),
        "" => Err(CliError::with_usage("runtime must not be empty")),
        other => Err(CliError::with_usage(format!(
            "unsupported runtime '{other}'; chic is the only supported runtime"
        ))),
    }
}

fn parse_runtime_flavor(spec: &str) -> Result<TargetRuntime, CliError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_usage(
            "--runtime must not be empty (expected llvm, wasm, native-std, or native-no_std)",
        ));
    }
    Ok(TargetRuntime::parse(trimmed))
}

fn parse_cpu_isa(spec: &str) -> Result<CpuIsaConfig, CliError> {
    CpuIsaConfig::parse_list(spec).map_err(|err| {
        CliError::with_usage(format!(
            "invalid ISA list: {err} (expected comma-separated values such as baseline,avx2,avx512,amx,dotprod,fp16fml,bf16,i8mm or 'auto')"
        ))
    })
}

fn parse_sve_bits(spec: &str) -> Result<u32, CliError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_usage(
            "--sve-bits requires a positive integer value",
        ));
    }
    let value = trimmed.parse::<u32>().map_err(|_| {
        CliError::with_usage(format!(
            "invalid --sve-bits value '{trimmed}'; expected decimal bit-length"
        ))
    })?;
    if value < 128 || value % 128 != 0 {
        return Err(CliError::with_usage(
            "--sve-bits must be a multiple of 128 (minimum 128)",
        ));
    }
    Ok(value)
}

fn parse_target_os(spec: &str) -> Result<TargetOs, CliError> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(CliError::with_usage(
            "--os requires a non-empty operating system value",
        ));
    }
    Ok(TargetOs::parse(trimmed))
}

fn parse_verbosity(spec: &str) -> Result<Verbosity, CliError> {
    let value = spec.trim().to_ascii_lowercase();
    let verbosity = match value.as_str() {
        "quiet" => Verbosity::Quiet,
        "minimal" => Verbosity::Minimal,
        "normal" | "info" => Verbosity::Normal,
        "detailed" | "debug" => Verbosity::Detailed,
        "diagnostic" | "trace" => Verbosity::Diagnostic,
        _ => {
            return Err(CliError::with_usage(format!(
                "invalid verbosity '{spec}'; expected quiet, minimal, normal, detailed, or diagnostic"
            )));
        }
    };
    Ok(verbosity)
}

fn parse_telemetry(spec: &str) -> Result<TelemetrySetting, CliError> {
    let value = spec.trim().to_ascii_lowercase();
    let telemetry = match value.as_str() {
        "auto" => TelemetrySetting::Auto,
        "on" | "true" => TelemetrySetting::On,
        "off" | "false" => TelemetrySetting::Off,
        _ => {
            return Err(CliError::with_usage(
                "invalid telemetry setting; expected auto, on, or off",
            ));
        }
    };
    Ok(telemetry)
}

fn parse_property_override(spec: &str) -> Result<BuildPropertyOverride, CliError> {
    let trimmed = spec.trim();
    let Some((name, value)) = trimmed.split_once('=') else {
        return Err(CliError::with_usage(
            "properties require the form <name>=<value>",
        ));
    };
    if name.trim().is_empty() {
        return Err(CliError::with_usage(
            "property name must not be empty (use --property:<name>=<value>)",
        ));
    }
    Ok(BuildPropertyOverride {
        name: name.trim().to_string(),
        value: value.to_string(),
    })
}

fn parse_bool_property(spec: &str) -> Result<bool, CliError> {
    match spec.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        other => Err(CliError::with_usage(format!(
            "invalid boolean value '{other}' (expected true/false)"
        ))),
    }
}

fn resolve_test_selection(mut selection: TestSelection) -> TestSelection {
    if selection.tests.is_empty() {
        if let Ok(value) = env::var("CHIC_TEST") {
            selection.tests.extend(split_list(&value));
        }
    }
    if selection.groups.is_empty() {
        if let Ok(value) = env::var("CHIC_TEST_GROUP") {
            selection.groups.extend(split_list(&value));
        }
    }
    if env_flag_truthy("CHIC_TEST_ALL").unwrap_or(false) {
        selection.run_all = true;
        selection.tests.clear();
        selection.groups.clear();
    }
    selection
}

fn resolve_test_parallelism(cli_value: Option<usize>) -> Option<usize> {
    if let Some(value) = cli_value {
        return Some(value);
    }
    match env::var("CHIC_TEST_PARALLELISM") {
        Ok(value) => match value.trim().parse::<usize>() {
            Ok(0) => None,
            Ok(count) => Some(count),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(|ch| ch == ',' || ch == ';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn env_flag_truthy(name: &str) -> Option<bool> {
    env::var_os(name).map(|value| {
        let lower = value.to_string_lossy().trim().to_ascii_lowercase();
        !matches!(lower.as_str(), "0" | "false" | "off" | "no" | "disable")
    })
}

fn runtime_for_backend(backend: Backend) -> TargetRuntime {
    match backend {
        Backend::Wasm => TargetRuntime::Wasm,
        Backend::Llvm | Backend::Cc1 => TargetRuntime::Llvm,
    }
}

fn backend_for_runtime(runtime: TargetRuntime) -> Backend {
    match runtime {
        TargetRuntime::Wasm => Backend::Wasm,
        TargetRuntime::Llvm
        | TargetRuntime::NativeStd
        | TargetRuntime::NativeNoStd
        | TargetRuntime::Other(_) => Backend::Llvm,
    }
}

fn append_configuration_defines(defines: &mut Vec<DefineFlag>, configuration: &str) {
    let has_debug = has_define(defines, "DEBUG");
    let has_release = has_define(defines, "RELEASE");
    let has_profile = has_define(defines, "PROFILE");
    let lower = configuration.trim();
    let is_debug = lower.eq_ignore_ascii_case("debug");
    if !has_debug {
        defines.push(DefineFlag::new("DEBUG", Some(is_debug.to_string())));
    }
    if !has_release {
        defines.push(DefineFlag::new("RELEASE", Some((!is_debug).to_string())));
    }
    if !has_profile {
        defines.push(DefineFlag::new("PROFILE", Some(configuration.to_string())));
    }
}

fn has_define(defines: &[DefineFlag], name: &str) -> bool {
    defines
        .iter()
        .any(|flag| flag.name.eq_ignore_ascii_case(name))
}
