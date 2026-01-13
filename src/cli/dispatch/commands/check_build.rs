use std::path::PathBuf;

use crate::defines::DefineFlag;
use crate::diagnostics::{FormatOptions, format_diagnostics};
use crate::driver::{BuildRequest, FrontendReport};
use crate::error::{Error, Result};
use crate::format::FormatEnforcement;
use crate::logging::LogLevel;
use crate::{ChicKind, Target};

use super::super::DispatchDriver;
use super::super::logging::{format_input_list, print_trait_solver_metrics};
use super::super::reporting::print_report_diagnostics;
use super::tools::{
    build_file_organization_diagnostics, build_format_diagnostics, build_ordering_diagnostics,
    check_formatting,
};
use super::{apply_const_eval_config, diagnostics_are_fatal};

pub(in crate::cli::dispatch) fn run_check<D: DispatchDriver>(
    driver: &D,
    inputs: Vec<PathBuf>,
    target: &Target,
    kind: ChicKind,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    defines: Vec<DefineFlag>,
    log_level: LogLevel,
    format_options: FormatOptions,
) -> Result<()> {
    let format_violations = check_formatting(driver, &inputs, None)?;
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
        if matches!(violations.enforcement, FormatEnforcement::Error) && !diagnostics.is_empty() {
            let rendered = format_diagnostics(&diagnostics, &files, format_options);
            println!("{rendered}");
            return Err(Error::Cli(crate::cli::CliError::new(
                "formatting violations reported; see diagnostics above",
            )));
        }
    }
    apply_const_eval_config(const_eval_fuel);
    let load_stdlib = driver.should_load_stdlib(&inputs);
    let mut report = driver.check(
        &inputs,
        target,
        kind,
        load_stdlib,
        trace_pipeline,
        trait_solver_metrics,
        &defines,
        log_level,
    )?;
    if let Some(violations) = format_violations {
        let mut diagnostics = build_format_diagnostics(&violations, &mut report.files);
        diagnostics.extend(build_file_organization_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        report.format_diagnostics.extend(diagnostics);
    }
    let has_errors = report.has_errors();
    let fatal = has_errors || diagnostics_are_fatal();
    if report.has_diagnostics() {
        println!("check completed with diagnostics:");
        print_report_diagnostics(&report, format_options);
        if fatal {
            return Err(Error::Cli(crate::cli::CliError::new(
                "diagnostics reported; see above",
            )));
        }
    } else {
        println!(
            "check passed for {} (target {}, crate type {})",
            format_input_list(&inputs),
            report.target.triple(),
            report.kind.as_str()
        );
    }
    if trait_solver_metrics {
        print_trait_solver_metrics("check", &report.trait_solver_metrics);
    }
    Ok(())
}

pub(in crate::cli::dispatch) fn run_lint<D: DispatchDriver>(
    driver: &D,
    inputs: Vec<PathBuf>,
    target: &Target,
    kind: ChicKind,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    defines: Vec<DefineFlag>,
    log_level: LogLevel,
    format_options: FormatOptions,
) -> Result<()> {
    let format_violations = check_formatting(driver, &inputs, None)?;
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
        if matches!(violations.enforcement, FormatEnforcement::Error) && !diagnostics.is_empty() {
            let rendered = format_diagnostics(&diagnostics, &files, format_options);
            println!("{rendered}");
            return Err(Error::Cli(crate::cli::CliError::new(
                "formatting violations reported; see diagnostics above",
            )));
        }
    }
    apply_const_eval_config(const_eval_fuel);
    let load_stdlib = driver.should_load_stdlib(&inputs);
    let mut report = driver.check(
        &inputs,
        target,
        kind,
        load_stdlib,
        trace_pipeline,
        trait_solver_metrics,
        &defines,
        log_level,
    )?;
    if let Some(violations) = format_violations {
        let mut diagnostics = build_format_diagnostics(&violations, &mut report.files);
        diagnostics.extend(build_file_organization_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        report.format_diagnostics.extend(diagnostics);
    }
    let has_errors = report.has_errors();
    let fatal = has_errors || diagnostics_are_fatal();
    if report.has_diagnostics() {
        println!("lint completed with diagnostics:");
        print_report_diagnostics(&report, format_options);
    } else {
        println!(
            "lint passed for {} (target {}, crate type {})",
            format_input_list(&inputs),
            report.target.triple(),
            report.kind.as_str()
        );
    }
    if fatal {
        return Err(Error::Cli(crate::cli::CliError::new(
            "lint errors reported; see above",
        )));
    }
    if trait_solver_metrics {
        print_trait_solver_metrics("lint", &report.trait_solver_metrics);
    }
    Ok(())
}

pub(in crate::cli::dispatch) fn run_build<D: DispatchDriver>(
    driver: &D,
    request: BuildRequest,
    const_eval_fuel: Option<usize>,
    format_options: FormatOptions,
) -> Result<()> {
    let format_inputs = request.inputs.clone();
    let format_violations = check_formatting(driver, &format_inputs, None)?;
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
        if matches!(violations.enforcement, FormatEnforcement::Error) && !diagnostics.is_empty() {
            let rendered = format_diagnostics(&diagnostics, &files, format_options);
            println!("{rendered}");
            return Err(Error::Cli(crate::cli::CliError::new(
                "formatting violations reported; see diagnostics above",
            )));
        }
    }
    apply_const_eval_config(const_eval_fuel);
    let trait_solver_metrics = request.trait_solver_metrics;
    let mut report = driver.build(request)?;
    if let Some(violations) = format_violations {
        let mut diagnostics = build_format_diagnostics(&violations, &mut report.files);
        diagnostics.extend(build_file_organization_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        diagnostics.extend(build_ordering_diagnostics(
            &violations,
            &violations.config,
            &mut report.files,
        ));
        report.format_diagnostics.extend(diagnostics);
    }
    let fatal = report.has_errors() || diagnostics_are_fatal();
    if report.has_diagnostics() {
        println!("build completed with diagnostics:");
        print_report_diagnostics(&report, format_options);
    } else {
        emit_build_success(&report);
    }
    if report.has_doc_errors() {
        return Err(Error::Cli(crate::cli::CliError::new(
            "documentation enforcement failed; see diagnostics above",
        )));
    }
    if fatal {
        return Err(Error::Cli(crate::cli::CliError::new(
            "diagnostics reported; see above",
        )));
    }
    if trait_solver_metrics {
        print_trait_solver_metrics("build", &report.trait_solver_metrics);
    }
    Ok(())
}

fn emit_build_success(report: &FrontendReport) {
    let inputs: Vec<PathBuf> = report
        .modules
        .iter()
        .map(|module| module.input.clone())
        .collect();
    let artifact_display = report
        .artifact
        .as_ref()
        .map_or_else(|| "<none>".into(), |p| p.display().to_string());
    let pack_display = report
        .library_pack
        .as_ref()
        .map(|p| p.display().to_string());
    let header_display = report.header.as_ref().map(|p| p.display().to_string());
    println!(
        "frontend succeeded for {} targeting {} as {} (artifact: {})",
        format_input_list(&inputs),
        report.target.triple(),
        report.kind.as_str(),
        artifact_display
    );
    if let Some(pack) = pack_display {
        println!("  reusable archive: {pack}");
    }
    if let Some(header) = header_display {
        println!("  header: {header}");
    }
    let assemblies: Vec<String> = report
        .modules
        .iter()
        .filter_map(|module| module.assembly_path.as_ref())
        .map(|path| path.display().to_string())
        .collect();
    if !assemblies.is_empty() {
        for asm in assemblies {
            println!("  assembly: {asm}");
        }
    }
}
