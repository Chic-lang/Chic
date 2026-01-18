use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;

use crate::Target;
use crate::cli::{CliError, ProfileOptions};
use crate::diagnostics::FormatOptions;
use crate::driver::{BuildRequest, TestOptions, TestStatus};
use crate::error::{Error, Result};
use crate::format::FormatEnforcement;
use crate::manifest::{Manifest, WorkspaceConfig};
use inferno::flamegraph;

use super::super::DispatchDriver;
use super::super::logging::{format_input_list, print_trait_solver_metrics};
use super::super::reporting::{print_report_diagnostics, relay_run_output};
use super::apply_const_eval_config;
use super::diagnostics_are_fatal;
use super::tools::{
    build_file_organization_diagnostics, build_format_diagnostics, build_ordering_diagnostics,
    check_formatting,
};

pub(in crate::cli::dispatch) fn run_run<D: DispatchDriver>(
    driver: &D,
    request: BuildRequest,
    const_eval_fuel: Option<usize>,
    profile: Option<ProfileOptions>,
    format_options: FormatOptions,
) -> Result<()> {
    let enforce_formatting = request.manifest.is_some() || request.workspace.is_some();
    let format_violations = if enforce_formatting {
        check_formatting(driver, &request.inputs, None)?
    } else {
        None
    };
    if let Some(violations) = format_violations.as_ref() {
        let mut files = crate::diagnostics::FileCache::default();
        let mut diagnostics = build_format_diagnostics(violations, &mut files);
        diagnostics.extend(build_file_organization_diagnostics(
            violations,
            &violations.config,
            &mut files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            violations,
            &violations.config,
            &mut files,
        ));
        if enforce_formatting
            && matches!(violations.enforcement, FormatEnforcement::Error)
            && !diagnostics.is_empty()
        {
            let rendered =
                crate::diagnostics::format_diagnostics(&diagnostics, &files, format_options);
            println!("{rendered}");
            return Err(Error::Cli(crate::cli::CliError::new(
                "formatting violations reported; see diagnostics above",
            )));
        }
    }
    apply_const_eval_config(const_eval_fuel);
    if request.kind.is_library() {
        return Err(Error::Cli(crate::cli::CliError::new(
            "chic run requires an executable crate type",
        )));
    }

    let _profile_guard = profile
        .as_ref()
        .map(|options| ProfileEnvGuard::new(options, &request.target));

    let trait_solver_metrics = request.trait_solver_metrics;
    let mut result = driver.run(request)?;
    if let Some(violations) = format_violations {
        let mut diagnostics = build_format_diagnostics(&violations, &mut result.report.files);
        diagnostics.extend(build_file_organization_diagnostics(
            &violations,
            &violations.config,
            &mut result.report.files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            &violations,
            &violations.config,
            &mut result.report.files,
        ));
        result.report.format_diagnostics.extend(diagnostics);
    }
    let fatal = result.report.has_errors() || diagnostics_are_fatal();
    if result.report.has_diagnostics() {
        print_report_diagnostics(&result.report, format_options);
        if fatal {
            return Err(Error::Cli(crate::cli::CliError::new(
                "diagnostics reported; see above",
            )));
        }
    }
    relay_run_output(&result)?;
    if trait_solver_metrics {
        print_trait_solver_metrics("run", &result.report.trait_solver_metrics);
    }

    if let Some(options) = &profile {
        write_profile_outputs(&options.output)?;
        if options.flamegraph {
            render_flamegraph(&options.output)?;
        }
        println!(
            "profiling artefacts written to {} (perf.json, perf.summary.json, perf.folded{})",
            options.output.display(),
            if options.flamegraph { ", perf.svg" } else { "" }
        );
    }

    if let Some(code) = result.status.code() {
        if !result.status.success() {
            // Propagate the program’s exit status so CLI callers observe the
            // real process result instead of a generic internal error.
            std::process::exit(code);
        }
    } else if !result.status.success() {
        return Err(Error::Internal("program terminated by signal".to_string()));
    }
    Ok(())
}

pub(in crate::cli::dispatch) fn run_tests<D: DispatchDriver>(
    driver: &D,
    request: BuildRequest,
    const_eval_fuel: Option<usize>,
    profile: Option<ProfileOptions>,
    test_options: TestOptions,
    coverage_only: bool,
    coverage_min: Option<u8>,
    format_options: FormatOptions,
) -> Result<()> {
    let enforce_formatting = request.manifest.is_some() || request.workspace.is_some();
    let coverage_requested = request.coverage || coverage_only || coverage_min.is_some();
    let coverage_output_path =
        package_coverage_output_path(request.manifest.as_ref(), request.workspace.as_ref());
    let format_violations = if enforce_formatting {
        check_formatting(driver, &request.inputs, None)?
    } else {
        None
    };
    if let Some(violations) = format_violations.as_ref() {
        let mut files = crate::diagnostics::FileCache::default();
        let mut diagnostics = build_format_diagnostics(violations, &mut files);
        diagnostics.extend(build_file_organization_diagnostics(
            violations,
            &violations.config,
            &mut files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            violations,
            &violations.config,
            &mut files,
        ));
        if enforce_formatting
            && matches!(violations.enforcement, FormatEnforcement::Error)
            && !diagnostics.is_empty()
        {
            let rendered =
                crate::diagnostics::format_diagnostics(&diagnostics, &files, format_options);
            println!("{rendered}");
            return Err(Error::Cli(crate::cli::CliError::new(
                "formatting violations reported; see diagnostics above",
            )));
        }
    }
    apply_const_eval_config(const_eval_fuel);
    let _profile_guard = profile
        .as_ref()
        .map(|options| ProfileEnvGuard::new(options, &request.target));
    let trait_solver_metrics = request.trait_solver_metrics;
    let manifest_coverage = request
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.coverage())
        .cloned();
    let mut run = driver.run_tests(request, test_options)?;
    if let Some(violations) = format_violations {
        let mut diagnostics = build_format_diagnostics(&violations, &mut run.report.files);
        diagnostics.extend(build_file_organization_diagnostics(
            &violations,
            &violations.config,
            &mut run.report.files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            &violations,
            &violations.config,
            &mut run.report.files,
        ));
        run.report.format_diagnostics.extend(diagnostics);
    }
    let fatal = run.report.has_errors() || diagnostics_are_fatal();
    if run.report.has_diagnostics() {
        println!("test discovery completed with diagnostics:");
        print_report_diagnostics(&run.report, format_options);
        if fatal {
            return Err(Error::Cli(crate::cli::CliError::new(
                "diagnostics reported; see above",
            )));
        }
    } else {
        println!(
            "discovered {} test(s) (selected {}, filtered out {}) in {}",
            run.discovered,
            run.cases.len(),
            run.filtered_out,
            format_input_list(
                &run.report
                    .modules
                    .iter()
                    .map(|module| module.input.clone())
                    .collect::<Vec<_>>()
            )
        );
    }

    if run.cases.is_empty() {
        if run.discovered == 0 {
            println!("no testcases found");
        } else {
            println!(
                "no testcases matched the selection (filtered out {})",
                run.filtered_out
            );
        }
        return Ok(());
    }

    let discovered = run.discovered;
    let filtered_out = run.filtered_out;
    let total_duration: Option<std::time::Duration> = run
        .cases
        .iter()
        .filter_map(|case| case.duration)
        .reduce(|a, b| a + b);
    let passed = run
        .cases
        .iter()
        .filter(|case| matches!(case.status, TestStatus::Passed))
        .count();
    let failed = run
        .cases
        .iter()
        .filter(|case| matches!(case.status, TestStatus::Failed))
        .count();
    let skipped = run
        .cases
        .iter()
        .filter(|case| matches!(case.status, TestStatus::Skipped))
        .count();

    println!(
        "discovered {} testcase(s) (filtered out {})",
        discovered, filtered_out
    );

    if !coverage_only {
        for case in &run.cases {
            let status = match case.status {
                TestStatus::Passed => "PASS",
                TestStatus::Failed => "FAIL",
                TestStatus::Skipped => "SKIP",
            };
            let async_flag = if case.is_async { " (async)" } else { "" };
            let duration = case
                .duration
                .map(|elapsed| format!("{:.2}ms", elapsed.as_secs_f64() * 1000.0));
            let mut details = Vec::new();
            details.push(format!("id={}", case.id));
            if let Some(ns) = &case.namespace {
                details.push(format!("ns={ns}"));
            }
            if !case.categories.is_empty() {
                details.push(format!("groups={}", case.categories.join(",")));
            }
            if let Some(message) = &case.message {
                details.push(message.clone());
            }
            if let Some(time) = duration {
                details.push(format!("duration={time}"));
            }
            println!(
                "[{status}] {}{async_flag} -- {}",
                case.name,
                details.join("; ")
            );
        }
    }

    if let Some(coverage) = run.chic_coverage.as_ref() {
        if let Some(path) = coverage_output_path.as_ref() {
            crate::chic_coverage::write_report_json(coverage, path).map_err(|err| {
                Error::Cli(CliError::new(format!(
                    "failed to write Chic coverage report {}: {err}",
                    path.display()
                )))
            })?;
        }
        println!(
            "chic coverage: {:.2}% (covered {}, total {})",
            coverage.percent, coverage.covered, coverage.total
        );
        if coverage_requested {
            if let Some(min) = coverage_min {
                if coverage.percent + f64::EPSILON < f64::from(min) {
                    return Err(Error::Cli(CliError::new(format!(
                        "coverage {:.2}% is below the required {}%",
                        coverage.percent, min
                    ))));
                }
            }
            if let Some(settings) = manifest_coverage.as_ref() {
                if matches!(settings.scope, crate::manifest::CoverageScope::Package)
                    && matches!(
                        settings.enforce,
                        crate::manifest::CoverageEnforcement::Error
                    )
                    && coverage.percent + f64::EPSILON < f64::from(settings.min_percent)
                {
                    return Err(Error::Cli(CliError::new(format!(
                        "Chic coverage {:.2}% is below the required {}%",
                        coverage.percent, settings.min_percent
                    ))));
                }
            }
        }
    } else if coverage_only {
        return Err(Error::Cli(CliError::new(
            "coverage output requested but no Chic coverage data was produced",
        )));
    } else if coverage_requested {
        return Err(Error::Cli(CliError::new(
            "coverage was requested but no Chic coverage data was produced",
        )));
    }

    if trait_solver_metrics {
        print_trait_solver_metrics("test", &run.report.trait_solver_metrics);
    }

    if let Some(options) = &profile {
        write_profile_outputs(&options.output)?;
        if options.flamegraph {
            render_flamegraph(&options.output)?;
        }
        println!(
            "profiling artefacts written to {} (perf.json, perf.summary.json, perf.folded{})",
            options.output.display(),
            if options.flamegraph { ", perf.svg" } else { "" }
        );
    }

    println!(
        "summary: {passed} passed, {failed} failed, {skipped} skipped{}",
        total_duration
            .map(|elapsed| format!(" in {:.2}ms", elapsed.as_secs_f64() * 1000.0))
            .unwrap_or_default()
    );

    if failed > 0 {
        Err(Error::Cli(CliError::new("one or more tests failed")))
    } else {
        Ok(())
    }
}

fn render_flamegraph(base: &Path) -> Result<()> {
    let folded = folded_output_path(base);
    if !folded.exists() {
        return Err(Error::Cli(CliError::new(format!(
            "profiling output missing folded stacks at {}",
            folded.display()
        ))));
    }
    if std::fs::metadata(&folded)
        .map(|meta| meta.len() == 0)
        .unwrap_or(false)
    {
        println!(
            "profiling skipped flamegraph generation: no samples found at {}",
            folded.display()
        );
        return Ok(());
    }
    let svg = svg_output_path(base);
    if let Some(parent) = svg.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|err| {
            Error::Cli(CliError::new(format!(
                "failed to create flamegraph directory {}: {err}",
                parent.display()
            )))
        })?;
    }
    let reader = File::open(&folded).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read folded stacks {}: {err}",
            folded.display()
        )))
    })?;
    let mut reader = BufReader::new(reader);
    let output = File::create(&svg).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to create flamegraph {}: {err}",
            svg.display()
        )))
    })?;
    let mut writer = BufWriter::new(output);
    let mut opts = flamegraph::Options::default();
    flamegraph::from_reader(&mut opts, &mut reader, &mut writer).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to render flamegraph {}: {err}",
            svg.display()
        )))
    })?;
    println!("profiling flamegraph written to {}", svg.display());
    Ok(())
}

fn write_profile_outputs(base: &Path) -> Result<()> {
    let body = std::fs::read_to_string(base).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read profiling output {}: {err}",
            base.display()
        )))
    })?;
    let snapshot: crate::perf::PerfSnapshot = serde_json::from_str(&body).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to decode profiling output {}: {err}",
            base.display()
        )))
    })?;

    let summary_path = base.with_extension("summary.json");
    if let Some(parent) = summary_path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|err| {
            Error::Cli(CliError::new(format!(
                "failed to create profiling output directory {}: {err}",
                parent.display()
            )))
        })?;
    }
    let summary = serde_json::to_string_pretty(&snapshot.summary).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to encode profiling summary {}: {err}",
            summary_path.display()
        )))
    })?;
    std::fs::write(&summary_path, summary).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to write profiling summary {}: {err}",
            summary_path.display()
        )))
    })?;

    let profile = std::env::var("CHIC_TRACE_PROFILE").ok();
    let run = snapshot
        .run_by_profile(profile.as_deref())
        .ok_or_else(|| Error::Cli(CliError::new("profiling output did not contain any runs")))?;

    let folded_path = folded_output_path(base);
    if let Some(parent) = folded_path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|err| {
            Error::Cli(CliError::new(format!(
                "failed to create profiling output directory {}: {err}",
                parent.display()
            )))
        })?;
    }

    let mut stacks: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_cpu_us = 0.0f64;
    for metric in &run.metrics {
        let frame = sanitize_folded_frame(&metric.label);
        if metric.cpu_us <= 0.0 {
            continue;
        }
        let weight = metric.cpu_us.ceil() as u64;
        if weight == 0 {
            continue;
        }
        total_cpu_us += metric.cpu_us;
        *stacks.entry(frame).or_insert(0) += weight;
    }

    if let Some(summary) = snapshot.summary.as_ref() {
        let wall_us = (summary.wall_time_ns as f64 / 1000.0).ceil();
        let idle_us = wall_us - total_cpu_us;
        if idle_us > 0.0 {
            let weight = idle_us.ceil() as u64;
            if weight > 0 {
                *stacks.entry("[idle]".to_string()).or_insert(0) += weight;
            }
        }
    }

    let mut folded = String::new();
    for (stack, count) in stacks {
        folded.push_str(&stack);
        folded.push(' ');
        folded.push_str(&count.to_string());
        folded.push('\n');
    }
    std::fs::write(&folded_path, folded).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to write folded stacks {}: {err}",
            folded_path.display()
        )))
    })?;

    Ok(())
}

fn sanitize_folded_frame(frame: &str) -> String {
    let trimmed = frame.trim();
    if trimmed.is_empty() {
        return "[unknown]".to_string();
    }
    trimmed
        .chars()
        .map(|ch| if ch.is_whitespace() { '_' } else { ch })
        .collect()
}

fn folded_output_path(base: &Path) -> PathBuf {
    base.with_extension("folded")
}

fn svg_output_path(base: &Path) -> PathBuf {
    base.with_extension("svg")
}

fn package_coverage_output_path(
    manifest: Option<&Manifest>,
    workspace: Option<&WorkspaceConfig>,
) -> Option<PathBuf> {
    let root = workspace
        .and_then(|config| config.path.parent().map(PathBuf::from))
        .or_else(|| {
            manifest
                .and_then(|manifest| manifest.path())
                .and_then(|path| path.parent())
                .map(PathBuf::from)
        })
        .or_else(|| std::env::current_dir().ok())?;
    let package_name = manifest
        .and_then(|manifest| manifest.package())
        .and_then(|package| package.name.as_deref())
        .or_else(|| {
            manifest
                .and_then(|manifest| manifest.path())
                .and_then(|path| path.parent())
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
        })
        .unwrap_or("package");
    Some(crate::chic_coverage::package_report_path(
        &root,
        package_name,
    ))
}

struct ProfileEnvGuard {
    saved: Vec<(String, Option<OsString>)>,
}

impl ProfileEnvGuard {
    fn new(options: &ProfileOptions, target: &Target) -> Self {
        let mut saved = Vec::new();
        saved.push(set_env("CHIC_PROFILE_AUTO_TRACE", "1"));
        saved.push(set_env(
            "CHIC_TRACE_OUTPUT",
            options.output.display().to_string(),
        ));
        saved.push(set_env("CHIC_TRACE_TARGET", target.triple().to_string()));
        saved.push(set_env(
            "CHIC_TRACE_PROFILE",
            std::env::var("PROFILE").unwrap_or_else(|_| "debug".into()),
        ));
        if let Some(ms) = options.sample_ms {
            saved.push(set_env("CHIC_TRACE_SAMPLE_MS", ms.to_string()));
        }
        Self { saved }
    }
}

fn set_env(key: &str, value: impl Into<String>) -> (String, Option<OsString>) {
    let previous = std::env::var_os(key);
    unsafe { std::env::set_var(key, value.into()) };
    (key.to_string(), previous)
}

impl Drop for ProfileEnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.saved.drain(..).rev() {
            match previous {
                Some(value) => unsafe { std::env::set_var(&key, value) },
                None => unsafe { std::env::remove_var(&key) },
            }
        }
    }
}
