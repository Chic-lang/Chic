use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::diagnostics_are_fatal;
use crate::chic_kind::ChicKind;
use crate::cli::CliError;
use crate::cli::dispatch::DispatchDriver;
use crate::cli::dispatch::logging::format_input_list;
use crate::cli::dispatch::reporting::print_report_diagnostics;
use crate::defines::DefineFlag;
use crate::diagnostics::FormatOptions;
use crate::doc::{DocGenerationOptions, DocOutputLayout, DocTemplate, generate_markdown};
use crate::frontend::metadata::collect_reflection_tables;
use crate::logging::LogLevel;
use crate::manifest::Manifest;
use crate::target::Target;

pub(in crate::cli::dispatch) fn run_doc<D: DispatchDriver>(
    driver: &D,
    manifest_hint: Option<PathBuf>,
    output: Option<PathBuf>,
    scope: Option<String>,
    template: Option<PathBuf>,
    front_matter: Option<PathBuf>,
    tag_handlers: Vec<String>,
    link_resolver: Option<String>,
    layout: Option<String>,
    banner: Option<bool>,
    format_options: FormatOptions,
    target_override: Option<Target>,
    kind_override: Option<ChicKind>,
    defines: Vec<DefineFlag>,
    log_level_override: Option<LogLevel>,
) -> crate::error::Result<()> {
    let (manifest, manifest_path) = resolve_manifest(manifest_hint)?;
    let manifest_path = manifest_path.unwrap_or_else(|| PathBuf::from("manifest.yaml"));
    let manifest_dir = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let inputs = collect_sources(&manifest, &manifest_dir)?;
    let kind = kind_override
        .or(manifest.build().kind)
        .unwrap_or(ChicKind::Executable);
    let target = target_override
        .or_else(|| {
            manifest
                .build()
                .target
                .as_deref()
                .and_then(|raw| Target::parse(raw).ok())
        })
        .unwrap_or_else(Target::host);
    let verbosity = manifest.build().verbosity.unwrap_or_default();
    let log_level = log_level_override.unwrap_or_else(|| verbosity.to_log_level());
    let load_stdlib = driver.should_load_stdlib(&inputs);
    let defines = if defines.is_empty() {
        Vec::new()
    } else {
        defines
    };

    let report = driver.check(
        &inputs,
        &target,
        kind,
        load_stdlib,
        false,
        false,
        &defines,
        log_level,
    )?;

    let fatal = report.has_errors() || diagnostics_are_fatal();
    if report.has_diagnostics() {
        println!("doc pass completed with diagnostics:");
        print_report_diagnostics(&report, format_options);
        if fatal {
            return Err(crate::error::Error::Cli(CliError::new(
                "diagnostics reported; see above",
            )));
        }
    } else {
        println!(
            "checked {} (target {}, crate type {}) for docs",
            format_input_list(&inputs),
            report.target.triple(),
            report.kind.as_str()
        );
    }

    let mut tables = crate::frontend::metadata::ReflectionTables::default();
    let mut seen = HashSet::new();
    for module in &report.modules {
        let module_tables = collect_reflection_tables(&module.parse.module);
        for ty in module_tables.types {
            if seen.insert(ty.name.clone()) {
                tables.types.push(ty);
            }
        }
    }

    let output_override = output.clone();
    let options = build_options(
        &manifest_dir,
        &manifest,
        output,
        template,
        front_matter,
        tag_handlers,
        link_resolver,
        layout,
        banner,
    )?;

    if options.markdown_enabled == Some(false) && output_override.is_none() {
        println!("docs.markdown.enabled is false; skipping generation");
        return Ok(());
    }

    let generation_options = DocGenerationOptions {
        output_root: options.output_root.clone(),
        layout: options.layout,
        template: options.template,
        front_matter_template: options.front_matter_template.clone(),
        banner: options.banner.clone(),
        heading_level: 1,
        tag_handlers: options.tag_handlers.clone(),
        link_resolver: options.link_resolver.clone(),
    };

    let result = generate_markdown(&tables, &generation_options)?;
    let mut had_errors = false;
    for diag in result.diagnostics {
        match diag.severity {
            crate::diagnostics::Severity::Error => {
                eprintln!("error: {}", diag.message);
                had_errors = true;
            }
            crate::diagnostics::Severity::Warning => {
                eprintln!("warning: {}", diag.message);
            }
            _ => {}
        }
    }
    if had_errors {
        return Err(crate::error::Error::Cli(CliError::new(
            "documentation generation reported errors",
        )));
    }

    println!(
        "documentation written to {} (scope: {})",
        generation_options.output_root.display(),
        scope.unwrap_or_else(|| "package".to_string())
    );
    Ok(())
}

struct DocCliOptions {
    output_root: PathBuf,
    layout: DocOutputLayout,
    template: DocTemplate,
    front_matter_template: Option<String>,
    banner: Option<String>,
    tag_handlers: Vec<String>,
    link_resolver: Option<String>,
    markdown_enabled: Option<bool>,
}

fn build_options(
    manifest_dir: &Path,
    manifest: &Manifest,
    output: Option<PathBuf>,
    template: Option<PathBuf>,
    front_matter: Option<PathBuf>,
    tag_handlers: Vec<String>,
    link_resolver: Option<String>,
    layout: Option<String>,
    banner: Option<bool>,
) -> crate::error::Result<DocCliOptions> {
    let md = &manifest.docs().markdown;
    let mut output_root = output
        .or_else(|| md.output.clone())
        .unwrap_or_else(|| PathBuf::from("docs/api"));
    if output_root.is_relative() {
        output_root = manifest_dir.join(output_root);
    }

    let layout = layout
        .or_else(|| md.layout.clone())
        .as_deref()
        .and_then(parse_layout)
        .unwrap_or(DocOutputLayout::PerType);

    let mut template_path = template.or_else(|| md.template.as_ref().map(PathBuf::from));
    if let Some(path) = &template_path {
        if path.is_relative() {
            template_path = Some(manifest_dir.join(path));
        }
    }
    let template = if let Some(path) = template_path {
        DocTemplate::from_path(&path)?
    } else {
        DocTemplate::none()
    };

    let mut front_path = front_matter.or_else(|| md.front_matter_template.clone());
    if let Some(path) = &front_path {
        if path.is_relative() {
            front_path = Some(manifest_dir.join(path));
        }
    }
    let front_matter_template = if let Some(path) = front_path {
        Some(fs::read_to_string(&path).map_err(crate::error::Error::Io)?)
    } else {
        None
    };

    let banner_enabled = banner.or(md.banner).unwrap_or(true);
    let banner_text = if banner_enabled {
        Some(default_banner(manifest_dir))
    } else {
        None
    };

    let mut handlers = md.tag_handlers.clone();
    for handler in tag_handlers {
        if !handlers.contains(&handler) {
            handlers.push(handler);
        }
    }

    Ok(DocCliOptions {
        output_root,
        layout,
        template,
        front_matter_template,
        banner: banner_text,
        tag_handlers: handlers,
        link_resolver: link_resolver.or_else(|| md.link_resolver.clone()),
        markdown_enabled: md.enabled,
    })
}

fn parse_layout(raw: &str) -> Option<DocOutputLayout> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "single" | "single-file" | "singlefile" => Some(DocOutputLayout::SingleFile),
        "per-type" | "pertype" | "multi" | "multi-file" => Some(DocOutputLayout::PerType),
        _ => None,
    }
}

fn resolve_manifest(hint: Option<PathBuf>) -> crate::error::Result<(Manifest, Option<PathBuf>)> {
    if let Some(path) = hint {
        let start = if path.is_dir() {
            path.clone()
        } else {
            path.clone()
        };
        if let Some(manifest) = Manifest::discover(&start)? {
            let path = manifest.path().map(Path::to_path_buf);
            return Ok((manifest, path));
        }
        return Err(crate::error::Error::Cli(CliError::new(format!(
            "manifest.yaml not found at {}",
            path.display()
        ))));
    }
    let cwd = std::env::current_dir().map_err(crate::error::Error::Io)?;
    Manifest::discover(&cwd)?
        .map(|manifest| {
            let path = manifest.path().map(Path::to_path_buf);
            (manifest, path)
        })
        .ok_or_else(|| crate::error::Error::Cli(CliError::new("manifest.yaml not found")))
}

fn collect_sources(manifest: &Manifest, manifest_dir: &Path) -> crate::error::Result<Vec<PathBuf>> {
    let mut roots: Vec<PathBuf> = manifest
        .derived_source_roots()
        .into_iter()
        .map(|root| manifest_dir.join(root.path))
        .collect();
    roots.retain(|root| root.exists());

    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            files.push(root);
            continue;
        }
        collect_cl_files(&root, &mut files)?;
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(crate::error::Error::Cli(CliError::new(format!(
            "no Chic source files found under {} (expected src/ or configured sources in manifest.yaml)",
            manifest_dir.display()
        ))));
    }
    Ok(files)
}

fn collect_cl_files(root: &Path, files: &mut Vec<PathBuf>) -> crate::error::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_cl_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "cl") {
            files.push(path);
        }
    }
    Ok(())
}

fn default_banner(manifest_dir: &Path) -> String {
    let manifest = manifest_dir.join("manifest.yaml");
    format!(
        "<!--\nThis file is auto-generated from Chic XML doc comments.\nDo not edit by hand. To regenerate, run:\n\n    chic doc {}\n-->\n",
        manifest.display()
    )
}
