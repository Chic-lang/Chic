use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::chic_kind::ChicKind;
use crate::codegen::Backend;
use crate::error::Result;
use crate::frontend::cfg::apply_cfg;
use crate::frontend::conditional::{ConditionalDefines, preprocess};
use crate::frontend::diagnostics::{Diagnostic, FileCache, FileId};
use crate::frontend::macro_expander::{MacroRegistry, expand_module as expand_macros};
use crate::frontend::parser::parse_module_in_file;
use crate::manifest::Manifest;
use crate::package::resolver::ResolvedPackage;

use super::logging;
use super::{FrontendModuleState, log_stdlib_parse_error};

fn filter_std_bootstrap_files(files: Vec<PathBuf>) -> Vec<PathBuf> {
    files
}

pub(super) fn load_standard_library(
    library_label: &str,
    _backend: Backend,
    _kind: ChicKind,
    macro_registry: &MacroRegistry,
    files: &mut FileCache,
    loaded_modules: &mut HashSet<PathBuf>,
    trace_enabled: bool,
    stdlib_files: &[PathBuf],
    defines: &ConditionalDefines,
    metadata: &logging::PipelineLogMetadata,
) -> Result<Vec<FrontendModuleState>> {
    let mut modules = Vec::new();
    let mut manifest_cache: HashMap<PathBuf, Manifest> = HashMap::new();
    let stage_prefix = format!("frontend.{library_label}");
    let filtered_files = filter_std_bootstrap_files(stdlib_files.to_vec());
    for relative in filtered_files {
        let requires_codegen = true;
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
        let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !loaded_modules.insert(canonical) {
            continue;
        }
        let read_start = Instant::now();
        let mut source = fs::read_to_string(&path)?;
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            &format!("{stage_prefix}.read_source"),
            &path,
            read_start,
        );

        let parse_start = Instant::now();
        let mut preprocess_result = preprocess(&source, defines);
        if let Some(rewritten) = preprocess_result.rewritten {
            source = rewritten;
        }
        let file_id = files.add_file(path.clone(), source.clone());
        stamp_file_id(&mut preprocess_result.diagnostics, file_id);
        let mut parse = match parse_module_in_file(&source, file_id) {
            Ok(parsed) => parsed,
            Err(err) => {
                log_stdlib_parse_error(&path, &source, &err);
                return Err(err.with_file(path.clone(), source).into());
            }
        };
        parse.diagnostics.extend(preprocess_result.diagnostics);
        let mut cfg_diags = {
            let mut module = parse.module_mut();
            apply_cfg(&mut module, defines)
        };
        parse.diagnostics.append(&mut cfg_diags);
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            &format!("{stage_prefix}.parse"),
            &path,
            parse_start,
        );

        let macro_start = Instant::now();
        let expansion = {
            let mut module = parse.module_mut();
            expand_macros(&mut module, macro_registry)
        };
        parse.diagnostics.extend(expansion.diagnostics);
        let mut cfg_diags = {
            let mut module = parse.module_mut();
            apply_cfg(&mut module, defines)
        };
        parse.diagnostics.append(&mut cfg_diags);
        parse.module = parse.module_owned();
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            &format!("{stage_prefix}.expand_macros"),
            &path,
            macro_start,
        );

        let manifest = Manifest::discover(&path)?.and_then(|manifest| {
            let Some(manifest_path) = manifest.path().map(PathBuf::from) else {
                return Some(manifest);
            };
            if let Some(existing) = manifest_cache.get(&manifest_path) {
                return Some(existing.clone());
            }
            manifest_cache.insert(manifest_path, manifest.clone());
            Some(manifest)
        });

        modules.push(FrontendModuleState {
            input: path,
            source,
            parse,
            manifest,
            is_stdlib: true,
            requires_codegen,
        });
    }
    Ok(modules)
}

pub(super) fn stamp_file_id(diagnostics: &mut [Diagnostic], file_id: FileId) {
    for diagnostic in diagnostics {
        if let Some(label) = diagnostic.primary_label.as_mut() {
            if label.span.file_id == FileId::UNKNOWN {
                label.span = label.span.with_file(file_id);
            }
        }
        for label in diagnostic.secondary_labels.iter_mut() {
            if label.span.file_id == FileId::UNKNOWN {
                label.span = label.span.with_file(file_id);
            }
        }
    }
}

pub(super) fn append_workspace_source(workspace: &mut String, path: &PathBuf, source: &str) {
    if !workspace.is_empty() {
        workspace.push_str("\n\n");
    }
    workspace.push_str(&format!("// __module: {}\n", path.display()));
    workspace.push_str(source);
    if !source.ends_with('\n') {
        workspace.push('\n');
    }
}

fn collect_package_source_files(manifest: &Manifest, root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for source_root in manifest.derived_source_roots() {
        let base = root.join(&source_root.path);
        collect_cl_files(&base, &mut files)?;
    }
    Ok(filter_std_bootstrap_files(files))
}

fn collect_cl_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_cl_files(&path, files)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("cl"))
            .unwrap_or(false)
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(super) fn parse_dependency_modules(
    package: &ResolvedPackage,
    files: &mut FileCache,
    defines: &ConditionalDefines,
    macro_registry: &MacroRegistry,
    trace_enabled: bool,
    metadata: &logging::PipelineLogMetadata,
) -> Result<Vec<FrontendModuleState>> {
    let mut modules = Vec::new();
    let sources = collect_package_source_files(&package.manifest, &package.root)?;
    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
        eprintln!(
            "[chic-debug] parsing package {} from {} ({} sources)",
            package.name,
            package.root.display(),
            sources.len()
        );
    }
    for path in sources {
        let read_start = Instant::now();
        let mut source = fs::read_to_string(&path)?;
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            "frontend.package.read_source",
            &path,
            read_start,
        );

        let parse_start = Instant::now();
        let mut preprocess_result = preprocess(&source, defines);
        if let Some(rewritten) = preprocess_result.rewritten {
            source = rewritten;
        }
        let file_id = files.add_file(path.clone(), source.clone());
        stamp_file_id(&mut preprocess_result.diagnostics, file_id);
        let mut parse = match parse_module_in_file(&source, file_id) {
            Ok(parsed) => parsed,
            Err(err) => {
                log_stdlib_parse_error(&path, &source, &err);
                return Err(err.with_file(path.clone(), source).into());
            }
        };
        parse.diagnostics.extend(preprocess_result.diagnostics);
        let mut cfg_diags = {
            let mut module = parse.module_mut();
            apply_cfg(&mut module, defines)
        };
        parse.diagnostics.append(&mut cfg_diags);
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            "frontend.package.parse",
            &path,
            parse_start,
        );

        let macro_start = Instant::now();
        let expansion = {
            let mut module = parse.module_mut();
            expand_macros(&mut module, macro_registry)
        };
        parse.diagnostics.extend(expansion.diagnostics);
        let mut cfg_diags = {
            let mut module = parse.module_mut();
            apply_cfg(&mut module, defines)
        };
        parse.diagnostics.append(&mut cfg_diags);
        parse.module = parse.module_owned();
        logging::log_stage_with_path(
            trace_enabled,
            metadata,
            "frontend.package.expand_macros",
            &path,
            macro_start,
        );

        modules.push(FrontendModuleState {
            input: path,
            source,
            parse,
            manifest: Some(package.manifest.clone()),
            is_stdlib: false,
            requires_codegen: true,
        });
    }
    Ok(modules)
}
