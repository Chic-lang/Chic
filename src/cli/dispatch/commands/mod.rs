use crate::cli::{CliError, Command, CommandFeature};
use crate::codegen::{Backend, CpuIsaConfig};
use crate::const_eval_config;
use crate::defines::DefineFlag;
use crate::diagnostics::FormatOptions;
use crate::driver::BuildRequest;
use crate::driver::types::{BuildPropertyOverride, TelemetrySetting, Verbosity};
use crate::driver::{BuildFfiOptions, TestOptions, TestStatus};
use crate::error::{Error, Result};
use crate::extern_bind::ExternBindOptions;
use crate::logging::LogLevel;
use crate::manifest::{Manifest, MissingDocsRule, PROJECT_MANIFEST_BASENAME, WorkspaceConfig};
use crate::target::Target;
use std::fs;
use std::path::{Path, PathBuf};

use super::ffi::resolve_cli_ffi_options;

mod check_build;
mod doc;
mod init;
mod perf_report;
mod run_test;
mod seed;
mod tools;

pub(super) use check_build::{run_build, run_check, run_lint};
pub(super) use doc::run_doc;
use init::run_init;
pub(super) use perf_report::run_perf_report;
pub(super) use run_test::{run_run, run_tests};
use seed::run_seed;
pub(super) use tools::run_clean;
#[cfg(test)]
pub(super) use tools::{
    FormatCheckOutcome, FormattedFile, build_file_organization_diagnostics,
    build_ordering_diagnostics,
};
pub(super) use tools::{
    FormatCommandOptions, run_cc1, run_extern_bind, run_format, run_header, run_mir_dump,
};

pub(super) fn dispatch_command<D: super::DispatchDriver>(
    driver: &D,
    command: Command,
    effective_level: LogLevel,
    format_options: FormatOptions,
) -> Result<()> {
    match command {
        Command::Check {
            inputs,
            target,
            kind,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
        } => run_check(
            driver,
            inputs,
            &target,
            kind,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            effective_level,
            format_options,
        ),
        Command::Lint {
            inputs,
            target,
            kind,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
        } => run_lint(
            driver,
            inputs,
            &target,
            kind,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            effective_level,
            format_options,
        ),
        Command::Build {
            inputs,
            manifest,
            workspace,
            output,
            artifacts_path,
            target,
            kind,
            backend,
            runtime_backend,
            emit_wat,
            emit_obj,
            cpu_isa,
            emit_header,
            emit_lib,
            cc1_args,
            cc1_keep_temps,
            load_stdlib: load_stdlib_override,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            ffi: cli_ffi,
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
        } => {
            let load_stdlib =
                load_stdlib_override.unwrap_or_else(|| driver.should_load_stdlib(&inputs));
            let ffi = resolve_cli_ffi_options(&cli_ffi, &target)?;
            let doc_defines = defines.clone();
            let doc_target = target.clone();
            run_build(
                driver,
                BuildRequest {
                    inputs,
                    manifest,
                    workspace,
                    target,
                    kind,
                    backend,
                    runtime_backend,
                    output,
                    run_timeout: None,
                    artifacts_path,
                    obj_dir: None,
                    bin_dir: None,
                    emit_wat_text: emit_wat,
                    emit_object: emit_obj,
                    coverage: false,
                    cpu_isa,
                    emit_header,
                    emit_library_pack: emit_lib,
                    cc1_args,
                    cc1_keep_temps,
                    load_stdlib,
                    trace_pipeline,
                    trait_solver_metrics,
                    defines,
                    log_level: effective_level,
                    ffi,
                    configuration,
                    framework,
                    no_dependencies,
                    no_restore,
                    no_incremental,
                    rebuild: false,
                    incremental_validate: false,
                    clean_only: false,
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
                    doc_enforcement,
                },
                const_eval_fuel,
                format_options,
            )?;
            if doc_markdown {
                if manifest_path.is_some() {
                    run_doc(
                        driver,
                        manifest_path,
                        None,
                        None,
                        None,
                        None,
                        Vec::new(),
                        None,
                        None,
                        None,
                        format_options,
                        Some(doc_target),
                        Some(kind),
                        doc_defines,
                        Some(effective_level),
                    )?;
                } else {
                    eprintln!(
                        "warning: --doc-markdown requires a manifest.yaml; skipping doc generation"
                    );
                }
            }
            Ok(())
        }
        Command::Run {
            inputs,
            manifest,
            workspace,
            target,
            kind,
            backend,
            runtime_backend,
            cpu_isa,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            ffi: cli_ffi,
            profile,
            run_timeout,
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
            load_stdlib,
            doc_enforcement,
        } => {
            let load_stdlib = load_stdlib.unwrap_or_else(|| driver.should_load_stdlib(&inputs));
            let ffi = resolve_cli_ffi_options(&cli_ffi, &target)?;
            run_run(
                driver,
                BuildRequest {
                    inputs,
                    manifest,
                    workspace,
                    target,
                    kind,
                    backend,
                    runtime_backend,
                    output: None,
                    run_timeout,
                    emit_wat_text: false,
                    emit_object: false,
                    coverage: false,
                    cpu_isa,
                    emit_header: false,
                    emit_library_pack: false,
                    cc1_args: Vec::new(),
                    cc1_keep_temps: false,
                    load_stdlib,
                    trace_pipeline,
                    trait_solver_metrics,
                    defines,
                    log_level: effective_level,
                    ffi,
                    configuration,
                    framework,
                    artifacts_path,
                    obj_dir: None,
                    bin_dir: None,
                    no_dependencies,
                    no_restore,
                    no_incremental,
                    rebuild: false,
                    incremental_validate: false,
                    clean_only: false,
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
                    doc_enforcement,
                },
                const_eval_fuel,
                profile,
                format_options,
            )
        }
        Command::Test {
            inputs,
            manifest,
            workspace,
            target,
            kind,
            backend,
            runtime_backend,
            cpu_isa,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            defines,
            ffi: cli_ffi,
            profile,
            test_options,
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
            load_stdlib,
            doc_enforcement,
        } => {
            let ffi = resolve_cli_ffi_options(&cli_ffi, &target)?;
            if workspace_mode {
                let Some(workspace) = workspace else {
                    return Err(Error::Cli(crate::cli::CliError::new(
                        "--workspace requires a manifest.workspace.yaml to be discoverable",
                    )));
                };
                return run_workspace_tests(
                    driver,
                    workspace,
                    target,
                    kind,
                    backend,
                    runtime_backend,
                    cpu_isa,
                    const_eval_fuel,
                    trace_pipeline,
                    trait_solver_metrics,
                    load_stdlib,
                    defines,
                    ffi,
                    test_options,
                    coverage,
                    coverage_min,
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
                    effective_level,
                    format_options,
                );
            }
            let load_stdlib = load_stdlib.unwrap_or_else(|| driver.should_load_stdlib(&inputs));
            run_tests(
                driver,
                BuildRequest {
                    inputs,
                    manifest,
                    workspace,
                    target,
                    kind,
                    backend,
                    runtime_backend,
                    output: None,
                    run_timeout: None,
                    emit_wat_text: false,
                    emit_object: false,
                    coverage,
                    cpu_isa,
                    emit_header: false,
                    emit_library_pack: false,
                    cc1_args: Vec::new(),
                    cc1_keep_temps: false,
                    load_stdlib,
                    trace_pipeline,
                    trait_solver_metrics,
                    defines,
                    log_level: effective_level,
                    ffi,
                    configuration,
                    framework,
                    artifacts_path,
                    obj_dir: None,
                    bin_dir: None,
                    no_dependencies,
                    no_restore,
                    no_incremental,
                    rebuild: false,
                    incremental_validate: false,
                    clean_only: false,
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
                    doc_enforcement,
                },
                const_eval_fuel,
                profile,
                test_options,
                coverage_only,
                coverage_min,
                format_options,
            )
        }
        Command::Init {
            template,
            output,
            name,
        } => run_init(&template, output, name),
        Command::Format {
            inputs,
            config,
            check,
            diff,
            write,
            stdin,
            stdout,
        } => run_format(
            driver,
            FormatCommandOptions {
                inputs,
                config_path: config,
                check,
                diff,
                write,
                stdin,
                stdout,
            },
        ),
        Command::MirDump {
            input,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
        } => run_mir_dump(
            driver,
            &input,
            const_eval_fuel,
            trace_pipeline,
            trait_solver_metrics,
            effective_level,
            format_options,
        ),
        Command::Header {
            input,
            output,
            include_guard,
        } => run_header(
            driver,
            &input,
            output.as_deref(),
            include_guard.as_deref(),
            effective_level,
            format_options,
        ),
        Command::Cc1 {
            input,
            output,
            target,
            extra_args,
        } => {
            ensure_feature_enabled(CommandFeature::Cc1, "cc1")?;
            run_cc1(&input, output.as_deref(), &target, &extra_args)
        }
        Command::ExternBind {
            header,
            output,
            namespace,
            library,
            binding,
            convention,
            optional,
        } => {
            ensure_feature_enabled(CommandFeature::ExternBind, "extern bind")?;
            run_extern_bind(ExternBindOptions {
                header,
                output,
                namespace,
                library,
                binding,
                convention,
                optional,
            })
        }
        Command::PerfReport {
            perf_path,
            baseline,
            profile,
            json,
            strict,
            tolerance,
        } => run_perf_report(
            &perf_path,
            baseline.as_deref(),
            profile.as_deref(),
            json,
            strict,
            tolerance,
        ),
        Command::Seed {
            run_path,
            profile,
            json,
        } => run_seed(&run_path, profile.as_deref(), json),
        Command::Doc {
            manifest,
            output,
            scope,
            template,
            front_matter,
            tag_handlers,
            link_resolver,
            layout,
            banner,
        } => run_doc(
            driver,
            manifest,
            output,
            scope,
            template,
            front_matter,
            tag_handlers,
            link_resolver,
            layout,
            banner,
            format_options,
            None,
            None,
            Vec::new(),
            None,
        ),
        Command::Clean {
            input,
            artifacts_path,
            configuration,
            all,
            dry_run,
        } => run_clean(
            input.as_deref(),
            artifacts_path.as_deref(),
            &configuration,
            all,
            dry_run,
        ),
        Command::ShowSpec => {
            let spec = driver.spec();
            println!("Specification: {}", spec.relative_path);
            println!("Summary: {}", spec.summary);
            println!("Embedded line count: {}", spec.line_count());
            Ok(())
        }
        Command::Help { ref topic } => {
            let help_text = match topic {
                Some(topic) => crate::cli::Cli::help_for(topic)?,
                None => crate::cli::Cli::usage(),
            };
            println!("{help_text}");
            Ok(())
        }
        Command::Version => {
            println!("{}", crate::version::formatted());
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_workspace_tests<D: super::DispatchDriver>(
    driver: &D,
    workspace: WorkspaceConfig,
    target: Target,
    kind: crate::chic_kind::ChicKind,
    backend: Backend,
    runtime_backend: crate::runtime::backend::RuntimeBackend,
    cpu_isa: CpuIsaConfig,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    load_stdlib_override: Option<bool>,
    defines: Vec<DefineFlag>,
    ffi: BuildFfiOptions,
    test_options: TestOptions,
    coverage: bool,
    coverage_min: Option<u8>,
    coverage_only: bool,
    configuration: String,
    artifacts_path: Option<PathBuf>,
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
    framework: Option<String>,
    doc_enforcement: MissingDocsRule,
    effective_level: LogLevel,
    _format_options: FormatOptions,
) -> Result<()> {
    apply_const_eval_config(const_eval_fuel);
    let coverage_requested = coverage || coverage_only || coverage_min.is_some();
    if coverage_requested {
        if let Some(settings) = workspace.coverage.as_ref() {
            if matches!(settings.backend, crate::manifest::CoverageBackend::Llvm) {
                return Err(Error::Cli(CliError::new(
                    "workspace Chic coverage does not support the LLVM backend yet; use coverage.backend: wasm or both",
                )));
            }
        }
    }

    let workspace_root = workspace
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let packages_root = workspace_root.join("packages");

    let entries = fs::read_dir(&packages_root).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read workspace packages directory {}: {err}",
            packages_root.display()
        )))
    })?;

    let mut manifest_paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| {
            Error::Cli(CliError::new(format!(
                "failed to read workspace packages directory {}: {err}",
                packages_root.display()
            )))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join(PROJECT_MANIFEST_BASENAME);
        if manifest_path.exists() {
            manifest_paths.push(manifest_path);
        }
    }
    manifest_paths.sort();
    if manifest_paths.is_empty() {
        return Err(Error::Cli(CliError::new(format!(
            "no packages found under {}",
            packages_root.display()
        ))));
    }

    let mut workspace_covered = 0usize;
    let mut workspace_total = 0usize;
    let mut workspace_packages = Vec::new();
    let mut saw_failed_tests = false;

    for manifest_path in manifest_paths {
        let Some(package_manifest) = Manifest::discover(&manifest_path)? else {
            return Err(Error::Cli(CliError::new(format!(
                "failed to load manifest {}",
                manifest_path.display()
            ))));
        };
        if package_manifest.is_no_std_runtime()
            && matches!(
                target.runtime(),
                crate::target::TargetRuntime::Llvm | crate::target::TargetRuntime::NativeStd
            )
            && matches!(kind, crate::chic_kind::ChicKind::Executable)
        {
            let package_name = package_manifest
                .package()
                .and_then(|pkg| pkg.name.as_deref())
                .unwrap_or("<unknown>");
            println!(
                "[workspace] {}: skipped (no_std runtime package cannot run on native target {})",
                package_name,
                target.triple()
            );
            continue;
        }
        let manifest_dir = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| packages_root.clone());

        let inputs = crate::cli::commands::build_like::collect_project_files(
            &package_manifest,
            &manifest_dir,
            true,
        )
        .map_err(Error::Cli)?;
        let mut load_stdlib =
            load_stdlib_override.unwrap_or_else(|| driver.should_load_stdlib(&inputs));
        if load_stdlib_override.is_none()
            && (package_manifest.is_no_std_runtime() || package_manifest.is_runtime_provider())
        {
            load_stdlib = false;
        }

        let format_violations = tools::check_formatting(driver, &inputs, Some(&package_manifest))?;
        if let Some(violations) = format_violations.as_ref() {
            if matches!(
                violations.enforcement,
                crate::format::FormatEnforcement::Error
            ) && violations.files.iter().any(|file| file.changed)
            {
                return Err(Error::Cli(CliError::new(
                    "formatting violations reported; run chic format or lower format.enforce",
                )));
            }
        }

        let (package_target, package_backend) = if package_manifest.is_no_std_runtime()
            && matches!(
                target.runtime(),
                crate::target::TargetRuntime::Llvm | crate::target::TargetRuntime::NativeStd
            ) {
            (
                Target::from_components(
                    target.arch(),
                    target.os().clone(),
                    crate::target::TargetRuntime::Wasm,
                ),
                Backend::Wasm,
            )
        } else {
            (target.clone(), backend)
        };

        let run = driver.run_tests(
            BuildRequest {
                inputs,
                manifest: Some(package_manifest.clone()),
                workspace: Some(workspace.clone()),
                target: package_target,
                kind,
                backend: package_backend,
                runtime_backend,
                output: None,
                run_timeout: None,
                emit_wat_text: false,
                emit_object: false,
                coverage: coverage_requested,
                cpu_isa: cpu_isa.clone(),
                emit_header: false,
                emit_library_pack: false,
                cc1_args: Vec::new(),
                cc1_keep_temps: false,
                load_stdlib,
                trace_pipeline,
                trait_solver_metrics,
                defines: defines.clone(),
                log_level: effective_level,
                ffi: ffi.clone(),
                configuration: configuration.clone(),
                framework: framework.clone(),
                artifacts_path: artifacts_path.clone(),
                obj_dir: None,
                bin_dir: None,
                no_dependencies,
                no_restore,
                no_incremental,
                rebuild: false,
                incremental_validate: false,
                clean_only: false,
                disable_build_servers,
                source_root: source_root.clone(),
                properties: properties.clone(),
                verbosity,
                telemetry,
                version_suffix: version_suffix.clone(),
                nologo,
                force,
                interactive,
                self_contained,
                doc_enforcement: doc_enforcement.clone(),
            },
            test_options.clone(),
        )?;

        let failures = run
            .cases
            .iter()
            .filter(|case| matches!(case.status, TestStatus::Failed))
            .count();
        if failures > 0 {
            saw_failed_tests = true;
        }

        if let Some(report) = run.chic_coverage.as_ref() {
            let package_name = package_manifest
                .package()
                .and_then(|pkg| pkg.name.as_deref())
                .or_else(|| manifest_dir.file_name().and_then(|name| name.to_str()))
                .unwrap_or("package");
            let package_output =
                crate::chic_coverage::package_report_path(&workspace_root, package_name);
            crate::chic_coverage::write_report_json(report, &package_output).map_err(|err| {
                Error::Cli(CliError::new(format!(
                    "failed to write Chic coverage report {}: {err}",
                    package_output.display()
                )))
            })?;
            if coverage_requested {
                if let Some(min) = coverage_min {
                    if report.percent + f64::EPSILON < f64::from(min) {
                        return Err(Error::Cli(CliError::new(format!(
                            "coverage {:.2}% is below the required {}%",
                            report.percent, min
                        ))));
                    }
                }
            }
            workspace_covered += report.covered;
            workspace_total += report.total;
            workspace_packages.push(crate::chic_coverage::ChicWorkspacePackageCoverage {
                name: package_name.to_string(),
                covered: report.covered,
                total: report.total,
                percent: report.percent,
            });
            println!(
                "[workspace] {}: coverage {:.2}% (covered {}, total {}), {} failed",
                package_name, report.percent, report.covered, report.total, failures
            );
            if coverage_requested {
                if let Some(settings) = package_manifest.coverage() {
                    if matches!(settings.scope, crate::manifest::CoverageScope::Package)
                        && matches!(
                            settings.enforce,
                            crate::manifest::CoverageEnforcement::Error
                        )
                        && report.percent + f64::EPSILON < f64::from(settings.min_percent)
                    {
                        return Err(Error::Cli(CliError::new(format!(
                            "Chic coverage {:.2}% is below the required {}%",
                            report.percent, settings.min_percent
                        ))));
                    }
                }
            }
        } else if coverage_requested {
            return Err(Error::Cli(CliError::new(
                "coverage was requested but no Chic coverage data was produced",
            )));
        } else if !coverage_only {
            println!(
                "[workspace] {}: {} test(s) ({} failed)",
                package_manifest
                    .package()
                    .and_then(|pkg| pkg.name.as_deref())
                    .unwrap_or("<unknown>"),
                run.cases.len(),
                failures
            );
        }
    }

    if workspace_total > 0 {
        let percent = (workspace_covered as f64) * 100.0 / (workspace_total as f64);
        let workspace_output = crate::chic_coverage::workspace_report_path(&workspace_root);
        let workspace_report = crate::chic_coverage::ChicWorkspaceCoverageReport {
            total: workspace_total,
            covered: workspace_covered,
            percent,
            packages: workspace_packages,
        };
        crate::chic_coverage::write_workspace_report_json(&workspace_report, &workspace_output)
            .map_err(|err| {
                Error::Cli(CliError::new(format!(
                    "failed to write Chic workspace coverage report {}: {err}",
                    workspace_output.display()
                )))
            })?;
        println!(
            "[workspace] coverage: {:.2}% (covered {}, total {})",
            percent, workspace_covered, workspace_total
        );
        if coverage_requested {
            if let Some(settings) = workspace.coverage.as_ref() {
                if matches!(settings.scope, crate::manifest::CoverageScope::Workspace)
                    && matches!(
                        settings.enforce,
                        crate::manifest::CoverageEnforcement::Error
                    )
                    && percent + f64::EPSILON < f64::from(settings.min_percent)
                {
                    return Err(Error::Cli(CliError::new(format!(
                        "workspace Chic coverage {:.2}% is below the required {}%",
                        percent, settings.min_percent
                    ))));
                }
            }
            if let Some(min) = coverage_min {
                if percent + f64::EPSILON < f64::from(min) {
                    return Err(Error::Cli(CliError::new(format!(
                        "workspace coverage {:.2}% is below the required {}%",
                        percent, min
                    ))));
                }
            }
        }
    } else if coverage_requested {
        return Err(Error::Cli(CliError::new(
            "workspace coverage requested but no coverage points were recorded",
        )));
    }

    if saw_failed_tests {
        return Err(Error::Cli(CliError::new("one or more tests failed")));
    }
    Ok(())
}

pub(super) fn apply_const_eval_config(cli_override: Option<usize>) {
    let config = const_eval_config::resolve(cli_override);
    const_eval_config::set_global(config);
}

pub(super) fn diagnostics_are_fatal() -> bool {
    std::env::var("CHIC_DIAGNOSTICS_FATAL")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn ensure_feature_enabled(feature: CommandFeature, command: &str) -> Result<()> {
    if super::super::build_time_feature_enabled(feature) {
        return Ok(());
    }
    Err(Error::Cli(CliError::new(format!(
        "command '{command}' requires feature {:?}",
        feature
    ))))
}
