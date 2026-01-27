use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use super::diagnostics_are_fatal;
use crate::cc1;
use crate::cli::CliError;
use crate::cli::commands::build_like::{collect_ch_files, collect_project_files};
use crate::codegen::OptLevel;
use crate::diagnostics::{
    Diagnostic, DiagnosticCode, FileCache, FormatOptions, Severity, Span, Suggestion,
};
use crate::error::{Error, Result};
use crate::extern_bind::{self, ExternBindOptions};
use crate::format::FormatEnforcement;
use crate::format::{FormatConfig, RawFormatSection};
use crate::header::{HeaderOptions, generate_header};
use crate::logging::LogLevel;
use crate::manifest::{Manifest, WorkspaceConfig};
use crate::{ChicKind, Target};
use serde_yaml::Value;
use similar::TextDiff;

use super::super::DispatchDriver;
use super::super::logging::{format_input_list, print_trait_solver_metrics};
use super::super::reporting::print_report_diagnostics;
use super::apply_const_eval_config;

pub(in crate::cli::dispatch) fn run_clean(
    input: Option<&Path>,
    artifacts_path: Option<&Path>,
    configuration: &str,
    all: bool,
    dry_run: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read current directory: {err}"
        )))
    })?;
    let start = input.unwrap_or(cwd.as_path());
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(start)
    } else {
        start
    };

    let workspace_root = WorkspaceConfig::discover(start_dir)?
        .and_then(|workspace| workspace.path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| start_dir.to_path_buf());

    let artifacts_root = artifacts_path.unwrap_or(workspace_root.as_path());
    let obj_dir = artifacts_root.join("obj");
    let bin_dir = artifacts_root.join("bin");

    if all {
        if dry_run {
            println!("clean (dry-run): remove {}", obj_dir.display());
            println!("clean (dry-run): remove {}", bin_dir.display());
            return Ok(());
        }
        let _ = fs::remove_dir_all(&obj_dir);
        let _ = fs::remove_dir_all(&bin_dir);
        println!("clean: removed {}", obj_dir.display());
        println!("clean: removed {}", bin_dir.display());
        return Ok(());
    }

    let configuration = configuration.trim();
    if configuration.is_empty() {
        return Err(Error::Cli(CliError::new(
            "--configuration requires a non-empty value",
        )));
    }

    clean_configuration_dir(&obj_dir, configuration, dry_run)?;
    clean_configuration_dir(&bin_dir, configuration, dry_run)?;
    Ok(())
}

fn clean_configuration_dir(root: &Path, configuration: &str, dry_run: bool) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(Error::Cli(CliError::new(format!(
            "expected {} to be a directory",
            root.display()
        ))));
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let config_dir = path.join(configuration);
        if !config_dir.exists() {
            continue;
        }
        if dry_run {
            println!("clean (dry-run): remove {}", config_dir.display());
        } else {
            let _ = fs::remove_dir_all(&config_dir);
            println!("clean: removed {}", config_dir.display());
        }
    }

    Ok(())
}

pub(in crate::cli::dispatch) fn run_extern_bind(options: ExternBindOptions) -> Result<()> {
    if std::env::var_os("CHIC_FAKE_EXTERN_BIND").is_some() {
        println!(
            "extern bind {} -> {}",
            options.header.display(),
            options.output.display()
        );
        return Ok(());
    }
    extern_bind::generate_bindings(&options)
}

#[derive(Debug, Clone)]
pub struct FormatCommandOptions {
    pub inputs: Vec<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub check: bool,
    pub diff: bool,
    pub write: bool,
    pub stdin: bool,
    pub stdout: bool,
}

pub(in crate::cli::dispatch) fn run_format<D: DispatchDriver>(
    driver: &D,
    options: FormatCommandOptions,
) -> Result<()> {
    let cwd = std::env::current_dir().map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read current directory: {err}"
        )))
    })?;
    let discovered_manifest = Manifest::discover(&cwd)?;
    let manifest_dir = discovered_manifest.as_ref().and_then(|manifest| {
        manifest
            .path()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
    });
    let config =
        resolve_format_config(options.config_path.as_deref(), discovered_manifest.as_ref())?;

    if options.stdin {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|err| Error::Cli(CliError::new(format!("failed to read stdin: {err}"))))?;
        let formatted = crate::format::format_source(&buffer, &config)?.formatted;
        let changed = formatted != buffer;
        if options.diff && changed {
            let diff = render_diff("<stdin>", &buffer, &formatted);
            print!("{diff}");
        }
        if options.check && changed {
            return Err(Error::Cli(CliError::new(
                "formatting changes required; rerun without --check to apply",
            )));
        }
        if options.stdout || !options.write {
            print!("{formatted}");
        } else {
            print!("{formatted}");
        }
        return Ok(());
    }

    let files = resolve_format_inputs(
        &options.inputs,
        discovered_manifest.as_ref(),
        manifest_dir.as_deref(),
    )?;
    let mut changed_paths = Vec::new();
    for path in files {
        let result = driver.format(&path, &config)?;
        if result.changed {
            if options.diff {
                let diff = render_diff(
                    path.to_string_lossy().as_ref(),
                    &result.original,
                    &result.formatted,
                );
                print!("{diff}");
            }
            if options.write && !options.stdout {
                fs::write(&result.input, &result.formatted)?;
                println!("formatted {}", result.input.display());
            }
            if options.stdout {
                println!("// {}", result.input.display());
                print!("{}", result.formatted);
                if !result.formatted.ends_with('\n') {
                    println!();
                }
            }
            changed_paths.push(result.input.clone());
        } else if !options.check && !options.diff && !options.stdout {
            println!("{} is already formatted", result.input.display());
        }
    }

    if options.check && !changed_paths.is_empty() {
        return Err(Error::Cli(CliError::new(
            "formatting changes required; rerun with --write or --diff to view changes",
        )));
    }

    Ok(())
}

pub(in crate::cli::dispatch) fn run_mir_dump<D: DispatchDriver>(
    driver: &D,
    input: &Path,
    const_eval_fuel: Option<usize>,
    trace_pipeline: bool,
    trait_solver_metrics: bool,
    log_level: LogLevel,
    format_options: FormatOptions,
) -> Result<()> {
    apply_const_eval_config(const_eval_fuel);
    let dump = driver.mir_dump(input, trace_pipeline, trait_solver_metrics, log_level)?;
    let inputs: Vec<std::path::PathBuf> = dump
        .report
        .modules
        .iter()
        .map(|module| module.input.clone())
        .collect();
    println!("MIR dump for {}:", format_input_list(&inputs));
    let fatal = dump.report.has_errors() || diagnostics_are_fatal();
    if dump.report.has_diagnostics() {
        println!("diagnostics encountered during mir-dump:");
        print_report_diagnostics(&dump.report, format_options);
    }
    println!("{}", dump.rendered);
    if fatal {
        return Err(Error::Cli(crate::cli::CliError::new(
            "diagnostics reported during mir-dump",
        )));
    }
    if trait_solver_metrics {
        print_trait_solver_metrics("mir-dump", &dump.report.trait_solver_metrics);
    }
    Ok(())
}

pub(in crate::cli::dispatch) fn run_header<D: DispatchDriver>(
    driver: &D,
    input: &Path,
    output: Option<&Path>,
    include_guard: Option<&str>,
    log_level: LogLevel,
    format_options: FormatOptions,
) -> Result<()> {
    apply_const_eval_config(None);
    let report = driver.check(
        &[input.to_path_buf()],
        &Target::host(),
        ChicKind::StaticLibrary,
        false,
        false,
        false,
        &[],
        log_level,
    )?;
    if report.has_diagnostics() {
        println!("header generation encountered diagnostics:");
        print_report_diagnostics(&report, format_options);
        return Err(Error::Internal(
            "cannot generate header while diagnostics are present".into(),
        ));
    }

    if report.modules.len() != 1 {
        return Err(Error::Internal(
            "chic header currently supports a single input module".into(),
        ));
    }

    let module = &report.modules[0];
    let mut options = HeaderOptions::default();
    options.include_guard = include_guard.map(str::to_string);
    let module_ref = module.parse.module_ref();
    let header = generate_header(&module_ref, &options)
        .map_err(|err| Error::Internal(format!("header generation failed: {err}")))?;
    drop(module_ref);

    if let Some(path) = output {
        if let Some(parent) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, header)?;
        println!("wrote header {}", path.display());
    } else {
        println!("{header}");
    }

    Ok(())
}

pub(in crate::cli::dispatch) fn run_cc1(
    input: &Path,
    output: Option<&Path>,
    target: &Target,
    extra_args: &[String],
) -> Result<()> {
    let output_path = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| input.with_extension("s"));
    if std::env::var_os("CHIC_FAKE_CC1_OUTPUT_ONLY").is_some() {
        if let Some(parent) = output_path
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        println!("cc1 emitted assembly {}", output_path.display());
        return Ok(());
    }
    if std::env::var_os("CHIC_FAKE_CC1").is_some() {
        println!("cc1 emitted assembly {}", output_path.display());
        return Ok(());
    }
    if let Some(parent) = output_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    cc1::run_preprocessed_stage(input, &output_path, target, OptLevel::O2, extra_args)?;
    println!("cc1 emitted assembly {}", output_path.display());
    Ok(())
}

fn resolve_format_config(
    config_path: Option<&Path>,
    manifest: Option<&Manifest>,
) -> Result<FormatConfig> {
    if let Some(path) = config_path {
        return load_format_config(path);
    }
    if let Some(manifest) = manifest {
        return Ok(manifest.format().clone());
    }
    Ok(FormatConfig::default())
}

fn load_format_config(path: &Path) -> Result<FormatConfig> {
    let contents = fs::read_to_string(path).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read format config {}: {err}",
            path.display()
        )))
    })?;
    let yaml: Value = serde_yaml::from_str(&contents).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to parse {}: {err}",
            path.display()
        )))
    })?;
    let section = match &yaml {
        Value::Mapping(map) => map
            .get(&Value::String("format".into()))
            .cloned()
            .unwrap_or(yaml.clone()),
        _ => yaml.clone(),
    };
    let raw: RawFormatSection = serde_yaml::from_value(section).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to decode format settings in {}: {err}",
            path.display()
        )))
    })?;
    Ok(FormatConfig::from_raw(&raw))
}

fn resolve_format_inputs(
    inputs: &[PathBuf],
    manifest: Option<&Manifest>,
    manifest_dir: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    if inputs.is_empty() {
        if let (Some(manifest), Some(dir)) = (manifest, manifest_dir) {
            let mut files = collect_project_files(manifest, dir, true).map_err(Error::Cli)?;
            files.sort();
            files.dedup();
            return Ok(files);
        }
        return Err(Error::Cli(CliError::new(
            "no inputs provided and manifest.yaml not found",
        )));
    }

    let mut files = Vec::new();
    for input in inputs {
        if !input.exists() {
            return Err(Error::Cli(CliError::new(format!(
                "input {} does not exist",
                input.display()
            ))));
        }
        if input.is_dir() {
            collect_ch_files(input, &mut files).map_err(Error::Cli)?;
        } else if input.extension().is_some_and(|ext| ext == "ch") {
            files.push(input.clone());
        } else {
            return Err(Error::Cli(CliError::new(format!(
                "unsupported input {}; expected a .ch file or directory",
                input.display()
            ))));
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn render_diff(label: &str, before: &str, after: &str) -> String {
    TextDiff::from_lines(before, after)
        .unified_diff()
        .header(label, &format!("{label} (formatted)"))
        .to_string()
}

#[derive(Debug, Clone)]
pub struct FormattedFile {
    pub path: PathBuf,
    pub original: String,
    pub changed: bool,
    pub metadata: crate::format::FormatMetadata,
}

#[derive(Debug, Clone)]
pub struct FormatCheckOutcome {
    pub enforcement: FormatEnforcement,
    pub config: FormatConfig,
    pub files: Vec<FormattedFile>,
}

pub(in crate::cli::dispatch) fn check_formatting<D: DispatchDriver>(
    driver: &D,
    inputs: &[PathBuf],
    manifest: Option<&Manifest>,
) -> Result<Option<FormatCheckOutcome>> {
    let Some(config) = resolve_format_config_for_inputs(manifest, inputs)? else {
        return Ok(None);
    };
    if !config.enabled || matches!(config.enforce, FormatEnforcement::Off) {
        return Ok(None);
    }

    let mut files = Vec::new();
    for path in inputs {
        if path.is_dir() {
            continue;
        }
        if !path.extension().is_some_and(|ext| ext == "ch") {
            continue;
        }
        let result = driver.format(path, &config)?;
        files.push(FormattedFile {
            path: result.input.clone(),
            original: result.original,
            changed: result.changed,
            metadata: result.metadata,
        });
    }

    if files.is_empty() {
        return Ok(None);
    }

    Ok(Some(FormatCheckOutcome {
        enforcement: config.enforce,
        config,
        files,
    }))
}

fn resolve_format_config_for_inputs(
    manifest: Option<&Manifest>,
    inputs: &[PathBuf],
) -> Result<Option<FormatConfig>> {
    if let Some(manifest) = manifest {
        return Ok(Some(manifest.format().clone()));
    }
    let start = inputs
        .first()
        .cloned()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let discovered = Manifest::discover(&start)?;
    Ok(discovered.map(|manifest| manifest.format().clone()))
}

pub(in crate::cli::dispatch) fn build_format_diagnostics(
    outcome: &FormatCheckOutcome,
    files: &mut FileCache,
) -> Vec<Diagnostic> {
    let severity = match outcome.enforcement {
        FormatEnforcement::Error => Severity::Error,
        _ => Severity::Warning,
    };
    outcome
        .files
        .iter()
        .filter(|issue| issue.changed)
        .map(|issue| {
            let file_id = files
                .find_id_by_path(&issue.path)
                .unwrap_or_else(|| files.add_file(issue.path.clone(), issue.original.clone()));
            let span = Span::in_file(file_id, 0, issue.original.len());
            let mut diagnostic = match severity {
                Severity::Error => Diagnostic::error("file is not formatted", Some(span)),
                _ => Diagnostic::warning("file is not formatted", Some(span)),
            };
            diagnostic.code = Some(DiagnosticCode::new("FMT0001", Some("format".into())));
            diagnostic = diagnostic.with_primary_label("reformat with `chic format`");
            diagnostic.add_suggestion(Suggestion::new(
                format!("run `chic format {}`", issue.path.display()),
                Some(span),
                None,
            ));
            diagnostic
        })
        .collect()
}

pub(in crate::cli::dispatch) fn build_file_organization_diagnostics(
    outcome: &FormatCheckOutcome,
    config: &FormatConfig,
    files: &mut FileCache,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let severity = match config.enforce {
        FormatEnforcement::Error => Severity::Error,
        _ => Severity::Warning,
    };
    for issue in &outcome.files {
        if config.files.one_top_level_type_per_file && issue.metadata.top_level_types.len() > 1 {
            let file_id = files
                .find_id_by_path(&issue.path)
                .unwrap_or_else(|| files.add_file(issue.path.clone(), issue.original.clone()));
            let span = Span::in_file(file_id, 0, issue.original.len());
            let mut diagnostic = match severity {
                Severity::Error => Diagnostic::error(
                    "multiple top-level types found; prefer one per file",
                    Some(span),
                ),
                _ => Diagnostic::warning(
                    "multiple top-level types found; prefer one per file",
                    Some(span),
                ),
            };
            diagnostic.code = Some(DiagnosticCode::new("FMT0100", Some("format".into())));
            diagnostic.notes.push("stage: format".into());
            diagnostic.add_suggestion(Suggestion::new(
                "move each top-level type into its own .ch file",
                Some(span),
                None,
            ));
            diagnostics.push(diagnostic);
        }
        if config.files.require_filename_match {
            if let Some(expected) = expected_filename(&issue.metadata, config.files.naming) {
                let actual = issue
                    .path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                if !actual.eq_ignore_ascii_case(&expected) {
                    let file_id = files.find_id_by_path(&issue.path).unwrap_or_else(|| {
                        files.add_file(issue.path.clone(), issue.original.clone())
                    });
                    let span = Span::in_file(file_id, 0, issue.original.len());
                    let mut diagnostic = match severity {
                        Severity::Error => Diagnostic::error(
                            format!(
                                "filename does not match type `{}` (expected {expected})",
                                issue
                                    .metadata
                                    .top_level_types
                                    .first()
                                    .cloned()
                                    .unwrap_or_default()
                            ),
                            Some(span),
                        ),
                        _ => Diagnostic::warning(
                            format!(
                                "filename does not match type `{}` (expected {expected})",
                                issue
                                    .metadata
                                    .top_level_types
                                    .first()
                                    .cloned()
                                    .unwrap_or_default()
                            ),
                            Some(span),
                        ),
                    };
                    diagnostic.code = Some(DiagnosticCode::new("FMT0101", Some("format".into())));
                    diagnostic.notes.push("stage: format".into());
                    diagnostic.add_suggestion(Suggestion::new(
                        format!("rename file to {expected}.ch or adjust `format.files.naming`"),
                        Some(span),
                        None,
                    ));
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
    diagnostics
}

pub(in crate::cli::dispatch) fn build_ordering_diagnostics(
    outcome: &FormatCheckOutcome,
    config: &FormatConfig,
    files: &mut FileCache,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let severity = match config.enforce {
        FormatEnforcement::Error => Severity::Error,
        _ => Severity::Warning,
    };
    if !config.ordering.types.is_empty() {
        let order = &config.ordering.types;
        for issue in &outcome.files {
            let actual: Vec<crate::format::TypeSort> =
                issue.metadata.types.iter().map(|t| t.kind).collect();
            if actual.len() < 2 {
                continue;
            }
            let expected_indices = |kind: crate::format::TypeSort| {
                order
                    .iter()
                    .position(|entry| *entry == kind)
                    .unwrap_or(usize::MAX)
            };
            let mut sorted = actual.clone();
            sorted.sort_by_key(|kind| expected_indices(*kind));
            if sorted != actual {
                let file_id = files
                    .find_id_by_path(&issue.path)
                    .unwrap_or_else(|| files.add_file(issue.path.clone(), issue.original.clone()));
                let span = Span::in_file(file_id, 0, issue.original.len());
                let mut diagnostic = match severity {
                    Severity::Error => Diagnostic::error(
                        "type declarations do not follow configured ordering",
                        Some(span),
                    ),
                    _ => Diagnostic::warning(
                        "type declarations do not follow configured ordering",
                        Some(span),
                    ),
                };
                diagnostic.code = Some(DiagnosticCode::new("FMT0200", Some("format".into())));
                diagnostic.notes.push("stage: format".into());
                diagnostics.push(diagnostic);
            }
        }
    }
    if !config.ordering.members.is_empty() {
        let order = &config.ordering.members;
        for issue in &outcome.files {
            for ty in &issue.metadata.types {
                if ty.members.len() < 2 {
                    continue;
                }
                let expected_indices = |kind: crate::format::MemberSort| {
                    order
                        .iter()
                        .position(|entry| *entry == kind)
                        .unwrap_or(usize::MAX)
                };
                let mut sorted = ty.members.clone();
                sorted.sort_by_key(|kind| expected_indices(*kind));
                if sorted != ty.members {
                    let file_id = files.find_id_by_path(&issue.path).unwrap_or_else(|| {
                        files.add_file(issue.path.clone(), issue.original.clone())
                    });
                    let span = Span::in_file(file_id, 0, issue.original.len());
                    let mut diagnostic = match severity {
                        Severity::Error => Diagnostic::error(
                            format!("members of `{}` do not follow configured ordering", ty.name),
                            Some(span),
                        ),
                        _ => Diagnostic::warning(
                            format!("members of `{}` do not follow configured ordering", ty.name),
                            Some(span),
                        ),
                    };
                    diagnostic.code = Some(DiagnosticCode::new("FMT0201", Some("format".into())));
                    diagnostic.notes.push("stage: format".into());
                    diagnostics.push(diagnostic);
                }
            }
        }
    }
    diagnostics
}

fn expected_filename(
    metadata: &crate::format::FormatMetadata,
    naming: crate::format::FileNaming,
) -> Option<String> {
    if metadata.top_level_types.len() != 1 {
        return None;
    }
    let type_name = metadata.top_level_types.first()?;
    let expected = match naming {
        crate::format::FileNaming::TypeName => type_name.clone(),
        crate::format::FileNaming::NamespaceQualified => metadata
            .namespace
            .as_ref()
            .map(|ns| format!("{ns}.{type_name}"))
            .unwrap_or_else(|| type_name.clone()),
    };
    Some(expected)
}
