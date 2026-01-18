use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::chic_kind::ChicKind;
use crate::codegen::{self, Backend, FfiConfig, FileRole, wasm};
use crate::error::{Error, Result};
use crate::header::{HeaderOptions, generate_header};
use crate::lint;
use crate::mir::FunctionKind;
use crate::runtime_package::{ResolvedRuntime, RuntimeKind, RuntimeMetadata, resolve_runtime};
use crate::target::{Target, TargetRuntime};
use tracing::info;

use super::defines::resolve_conditional_defines;
use super::incremental::{self, ArtifactState, BuildManifest, FileState};
use super::pipeline::CompilerPipelineBuilder;
use super::report::{
    ModuleArtifact, archive_entry_name, default_artifact_path, slice_mir_module, unit_object_path,
};
use super::types::{BuildRequest, resolve_trace_enabled, trait_solver_metrics_enabled};
use super::{FrontendReport, GeneratedModuleIr};

pub(crate) fn execute(request: BuildRequest) -> Result<FrontendReport> {
    if request.inputs.is_empty() {
        return Err(crate::error::Error::internal(
            "no input sources provided for build",
        ));
    }

    let BuildRequest {
        inputs,
        manifest,
        workspace,
        target,
        kind,
        backend,
        runtime_backend: _,
        output,
        emit_wat_text,
        emit_object,
        coverage,
        cpu_isa,
        emit_header,
        emit_library_pack,
        cc1_args,
        cc1_keep_temps,
        load_stdlib,
        trace_pipeline,
        trait_solver_metrics,
        defines,
        log_level,
        ffi,
        configuration,
        framework: _,
        artifacts_path,
        obj_dir,
        bin_dir,
        no_dependencies,
        no_restore,
        no_incremental,
        rebuild,
        incremental_validate: _,
        clean_only,
        disable_build_servers: _,
        source_root,
        properties: _,
        verbosity: _,
        telemetry: _,
        version_suffix,
        nologo: _,
        force,
        interactive: _,
        self_contained: _,
        ..
    } = request;

    let ffi_config = if !ffi.search_paths.is_empty() || ffi.default_pattern.is_some() {
        Some(FfiConfig {
            search_paths: ffi
                .search_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            default_pattern: ffi.default_pattern.clone(),
        })
    } else {
        None
    };
    let ffi_package_files = ffi.packages.clone();

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_kind = match target.runtime() {
        TargetRuntime::NativeNoStd => RuntimeKind::NoStd,
        _ => RuntimeKind::Native,
    };
    let manifest_runtime = manifest.as_ref().and_then(|m| m.runtime());
    let require_runtime = manifest
        .as_ref()
        .map(|m| m.require_native_runtime(kind))
        .unwrap_or_else(|| !kind.is_library());
    let runtime_resolution = if let Some(selection) = manifest_runtime {
        match resolve_runtime(Some(selection), runtime_kind, &project_root) {
            Ok(resolution) => Some(resolution),
            Err(err) if require_runtime => return Err(err),
            Err(err) => {
                eprintln!(
                    "[chic-warning] runtime resolution failed (require_native_runtime=false): {err}"
                );
                None
            }
        }
    } else if manifest.is_some() {
        if require_runtime && !emit_object {
            return Err(Error::Cli(crate::cli::CliError::new(
                "toolchain.runtime must be specified in manifest.yaml",
            )));
        }
        None
    } else if require_runtime && !emit_object {
        Some(resolve_runtime(None, runtime_kind, &project_root)?)
    } else {
        None
    };
    if let Some(runtime) = runtime_resolution.as_ref() {
        ensure_target_runtime_matches(&runtime.resolved, target.runtime(), kind)?;
    }
    let runtime_metadata = runtime_resolution
        .as_ref()
        .map(|res| RuntimeMetadata::from(&res.resolved));
    let runtime_identity = runtime_metadata
        .as_ref()
        .map(|rt| rt.identity.clone())
        .unwrap_or_else(|| "no-runtime".to_string());

    let load_stdlib = if backend == Backend::Cc1 {
        false
    } else {
        load_stdlib
    };
    let lint_config = lint::discover(&inputs)?;

    let trace_enabled = resolve_trace_enabled(trace_pipeline, log_level);
    let solver_metrics_enabled = trait_solver_metrics_enabled(trace_enabled, trait_solver_metrics);
    let build_start = Instant::now();
    let inputs_summary = super::summarize_inputs(&inputs);
    let target_str = target.triple().to_string();
    let backend_name = backend.as_str().to_string();
    let kind_name = kind.as_str().to_string();
    let workspace_root = source_root.unwrap_or_else(|| {
        inputs
            .first()
            .and_then(|path| path.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    });
    if trace_enabled {
        info!(
            target: "pipeline",
            stage = "driver.build.start",
            command = "build",
            status = "start",
            target = %target_str,
            backend = backend_name.as_str(),
            kind = kind_name.as_str(),
            input_count = inputs.len(),
            inputs = %inputs_summary,
            load_stdlib,
            emit_wat = emit_wat_text
        );
    }

    let frontend_start = Instant::now();
    let corelib_files = super::collect_core_files()?;
    let foundationlib_files = if no_dependencies {
        Vec::new()
    } else {
        super::collect_foundation_files()?
    };
    let alloclib_files = if no_dependencies {
        Vec::new()
    } else {
        super::collect_alloc_files()?
    };
    let nostd_runtime_files = if no_dependencies {
        Vec::new()
    } else if let Some(runtime) = runtime_resolution.as_ref() {
        if runtime.resolved.kind == RuntimeKind::NoStd {
            super::collect_runtime_package_files(&runtime.resolved)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let stdlib_files = if no_dependencies {
        Vec::new()
    } else {
        super::stdlib_files_for(kind, backend)?
    };
    let conditional_defines = resolve_conditional_defines(&target, &defines);
    crate::frontend::conditional::set_active_defines(conditional_defines.clone());
    let pipeline = CompilerPipelineBuilder::new("build", &inputs, &target, conditional_defines)
        .backend(backend)
        .kind(kind)
        .manifest(manifest)
        .workspace(workspace)
        .coverage_enabled(request.coverage)
        .restore_enabled(!no_restore)
        .load_stdlib(load_stdlib && !no_dependencies)
        .corelib_files(&corelib_files)
        .foundationlib_files(&foundationlib_files)
        .alloclib_files(&alloclib_files)
        .stdlib_files(&stdlib_files)
        .nostd_runtime_files(&nostd_runtime_files)
        .runtime(runtime_resolution.as_ref().map(|rt| rt.resolved.clone()))
        .trace_enabled(trace_enabled)
        .trait_solver_metrics(solver_metrics_enabled)
        .lint_config(lint_config)
        .doc_enforcement(request.doc_enforcement)
        .build();
    let frontend = pipeline.execute()?;
    if trace_enabled {
        info!(
            target: "pipeline",
            stage = "driver.build.frontend",
            command = "build",
            status = "ok",
            target = %target_str,
            backend = backend_name.as_str(),
            kind = kind_name.as_str(),
            input_count = inputs.len(),
            inputs = %inputs_summary,
            elapsed_ms = frontend_start.elapsed().as_millis() as u64
        );
    }

    if std::env::var("CHIC_SKIP_CODEGEN").is_ok() {
        let module_artifacts = vec![ModuleArtifact::default(); frontend.modules.len()];
        return Ok(frontend.into_report(
            Some(workspace_root),
            None,
            None,
            Vec::new(),
            module_artifacts,
        ));
    }

    if !frontend.mir_lowering_diagnostics.is_empty() {
        if std::env::var("CHIC_DEBUG_MIR_DIAGNOSTICS").is_ok() {
            eprintln!(
                "[chic-debug] MIR lowering produced {} diagnostic(s):",
                frontend.mir_lowering_diagnostics.len()
            );
            for diagnostic in &frontend.mir_lowering_diagnostics {
                if let Some(span) = diagnostic.span {
                    if let Some(file) = frontend.files.path(span.file_id) {
                        if let Some(loc) = frontend.files.line_col(span.file_id, span.start) {
                            eprintln!(
                                "  {}:{}:{}: {}",
                                file.display(),
                                loc.line,
                                loc.column,
                                diagnostic.message
                            );
                            continue;
                        }
                        eprintln!(
                            "  [file_id {:?} offset {}-{}]: {}",
                            span.file_id, span.start, span.end, diagnostic.message
                        );
                        continue;
                    }
                }
                eprintln!("  (no span) {}", diagnostic.message);
            }
        }

        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        for diagnostic in &frontend.mir_lowering_diagnostics {
            let is_warning = diagnostic.message.trim_start().starts_with("warning:");
            let formatted = if let Some(span) = diagnostic.span {
                if let Some(file) = frontend.files.path(span.file_id) {
                    if let Some(loc) = frontend.files.line_col(span.file_id, span.start) {
                        format!(
                            "{}:{}:{}: {}",
                            file.display(),
                            loc.line,
                            loc.column,
                            diagnostic.message
                        )
                    } else {
                        diagnostic.message.clone()
                    }
                } else {
                    diagnostic.message.clone()
                }
            } else {
                diagnostic.message.clone()
            };

            if is_warning {
                warnings.push(formatted);
            } else {
                errors.push(formatted);
            }
        }

        if !warnings.is_empty() {
            eprintln!("{}", warnings.join("\n"));
        }
        if !errors.is_empty() {
            return Err(crate::error::Error::codegen(errors.join("\n")));
        }
    }

    if std::env::var("CHIC_DEBUG_ASYNC_RESULT").is_ok() {
        for func in &frontend.mir_module.functions {
            if !func.is_async {
                continue;
            }
            eprintln!(
                "[chic-debug] async function {} async_result={:?} locals={}",
                func.name,
                func.async_result.as_ref().map(|ty| ty.canonical_name()),
                func.body.locals.len()
            );
        }
    }

    let mut generated_ir = Vec::new();
    let mut module_artifacts;
    let mut library_pack = None;
    let mut header_path = None;
    let final_artifact_path;

    let default_input = inputs
        .first()
        .expect("checked earlier: inputs is not empty");
    let artifacts_root = artifacts_path
        .as_deref()
        .or_else(|| Some(workspace_root.as_path()));
    let layout = resolve_artifact_path(
        default_input,
        &target,
        kind,
        backend,
        emit_object,
        &output,
        obj_dir.as_deref(),
        bin_dir.as_deref(),
        artifacts_root,
        runtime_identity.as_str(),
        &configuration,
        version_suffix.as_deref(),
    );
    let runtime_abi = runtime_metadata
        .as_ref()
        .map(|rt| rt.abi.clone())
        .unwrap_or_else(|| "none".to_string());
    let runtime_manifest_hash = runtime_metadata
        .as_ref()
        .map(|rt| rt.manifest_hash.clone())
        .unwrap_or_else(|| "none".to_string());
    fs::create_dir_all(&layout.obj_dir)?;
    if let Some(parent) = layout.artifact_path.parent()
        && !parent.as_os_str().is_empty()
        && !emit_object
    {
        fs::create_dir_all(parent)?;
    }
    let cache_manifest_path = layout.cache_manifest_path();
    let previous_manifest = incremental::load_manifest(&cache_manifest_path);

    if clean_only {
        purge_all_artifacts(&layout)?;
    }
    if no_incremental || rebuild {
        purge_incremental_artifacts(&layout)?;
    } else if force {
        let _ = fs::remove_file(&layout.artifact_path);
    }

    if let Some(prev) = &previous_manifest {
        let current_inputs: HashSet<String> = inputs
            .iter()
            .map(|path| {
                relative_to(&workspace_root, path)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        for file in &prev.files {
            if !current_inputs.contains(&file.path) {
                if let Some(object) = &file.object {
                    let stale = workspace_root.join(object);
                    let _ = fs::remove_file(stale);
                }
            }
        }
    }

    let artifact_path = layout.artifact_path.clone();
    let object_base = layout.object_base.clone();

    match backend {
        Backend::Wasm => {
            let mut options = codegen::CodegenOptions::default();
            options.backend = Backend::Wasm;
            options.emit_wat_text = emit_wat_text;
            options.coverage = coverage;
            options.cpu_isa = cpu_isa.clone();
            options.sve_vector_bits = cpu_isa.sve_vector_bits();
            options.ffi_config = ffi_config.clone();
            options.ffi_packages = ffi_package_files.clone();
            options.runtime = runtime_metadata.clone();

            let codegen_start = Instant::now();
            let codegen_artifact = codegen::compile_module(
                &frontend.workspace_source,
                &frontend.combined_ast,
                &frontend.mir_module,
                Some(&frontend.mir_module),
                &frontend.perf_metadata,
                &target,
                kind,
                &artifact_path,
                &options,
                &frontend.drop_glue,
                &frontend.hash_glue,
                &frontend.eq_glue,
                &frontend.type_metadata,
            )?;
            if trace_enabled {
                info!(
                    target: "pipeline",
                    stage = "driver.build.codegen.wasm",
                    command = "build",
                    status = "ok",
                    target = %target_str,
                    backend = backend_name.as_str(),
                    kind = kind_name.as_str(),
                    input_count = inputs.len(),
                    inputs = %inputs_summary,
                    elapsed_ms = codegen_start.elapsed().as_millis() as u64
                );
            }

            if emit_wat_text {
                let entry = wasm::find_entry_function(&frontend.combined_ast);
                let _ = wasm::ensure_wat_artifact(
                    &frontend.mir_module,
                    entry,
                    kind,
                    &codegen_artifact.artifact_path,
                    &frontend.type_metadata,
                    &frontend.hash_glue,
                    &frontend.eq_glue,
                    &frontend.perf_metadata,
                )?;
            }

            generated_ir.push(GeneratedModuleIr {
                input: artifact_path.clone(),
                textual: codegen_artifact.textual_ir.clone(),
            });
            module_artifacts = vec![ModuleArtifact::default(); frontend.modules.len()];
            library_pack = codegen_artifact.library_pack;
            final_artifact_path = Some(codegen_artifact.artifact_path);
        }
        Backend::Llvm => {
            let codegen_start = Instant::now();
            let mut link_inputs: Vec<PathBuf> = Vec::new();
            module_artifacts = Vec::with_capacity(frontend.modules.len());

            let testcase_unit_index = frontend
                .unit_functions
                .iter()
                .enumerate()
                .find(|(_, indices)| {
                    indices.iter().any(|&idx| {
                        frontend
                            .mir_module
                            .functions
                            .get(idx)
                            .is_some_and(|func| matches!(func.kind, FunctionKind::Testcase))
                    })
                })
                .map(|(idx, _)| idx);

            let entry_unit_index =
                crate::codegen::llvm::find_entry_function(&frontend.combined_ast)
                    .and_then(|entry_name| {
                        frontend
                            .mir_module
                            .functions
                            .iter()
                            .position(|func| func.name == entry_name)
                    })
                    .and_then(|entry_function_index| {
                        frontend
                            .unit_functions
                            .iter()
                            .position(|indices| indices.contains(&entry_function_index))
                    });
            let startup_unit_index = entry_unit_index.or(testcase_unit_index).unwrap_or(0usize);

            let mut user_module_index = 0usize;
            let codegen_unit_count = frontend
                .modules
                .iter()
                .filter(|module| !(module.is_stdlib && !module.requires_codegen))
                .count();
            for (index, module) in frontend.modules.iter().enumerate() {
                if module.is_stdlib && !module.requires_codegen {
                    module_artifacts.push(ModuleArtifact::default());
                    continue;
                }
                let object_path = if emit_object && codegen_unit_count == 1 {
                    object_base.clone()
                } else {
                    unit_object_path(&object_base, module.input.as_path(), user_module_index)
                };
                user_module_index += 1;
                let mut options = codegen::CodegenOptions::default();
                options.backend = Backend::Llvm;
                options.emit_wat_text = false;
                options.coverage = coverage;
                options.cpu_isa = cpu_isa.clone();
                options.sve_vector_bits = cpu_isa.sve_vector_bits();
                options.link_final_artifact = false;
                options.ffi_config = ffi_config.clone();
                options.ffi_packages = ffi_package_files.clone();
                options.runtime = runtime_metadata.clone();

                let mut mir_unit =
                    slice_mir_module(&frontend.mir_module, &frontend.unit_functions[index]);
                if kind == ChicKind::Executable {
                    mir_unit.attributes.suppress_startup_descriptor = index != startup_unit_index;
                } else if Some(index) == testcase_unit_index {
                    mir_unit.attributes.suppress_startup_descriptor = false;
                } else if module.is_stdlib && module.requires_codegen {
                    mir_unit.attributes.suppress_startup_descriptor = true;
                }
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    eprintln!(
                        "[chic-debug] codegen unit {} (module {}) has {} functions",
                        user_module_index,
                        module.input.display(),
                        mir_unit.functions.len()
                    );
                    for func in &mir_unit.functions {
                        eprintln!(
                            "[chic-debug]   fn {} async={} machine={} async_result={:?}",
                            func.name,
                            func.is_async,
                            func.body.async_machine.is_some(),
                            func.async_result
                        );
                    }
                    for plan in &mir_unit.async_plans {
                        eprintln!(
                            "[chic-debug]   async plan {} frame={} suspend_points={} resume_states={}",
                            plan.function_name,
                            plan.frame_type,
                            plan.suspend_points.len(),
                            plan.resume_states.len()
                        );
                    }
                }
                let module_ref = module.parse.module_ref();
                let artifact = codegen::compile_module(
                    &module.source,
                    &module_ref,
                    &mir_unit,
                    Some(&frontend.mir_module),
                    &frontend.perf_metadata,
                    &target,
                    kind,
                    &object_path,
                    &options,
                    &frontend.drop_glue,
                    &frontend.hash_glue,
                    &frontend.eq_glue,
                    &frontend.type_metadata,
                )?;
                drop(module_ref);

                link_inputs.push(artifact.object_path.clone());
                if let Some(metadata) = &artifact.metadata_path {
                    if !module.is_stdlib {
                        link_inputs.push(metadata.clone());
                    }
                }

                let textual_ir = artifact.textual_ir.clone();
                generated_ir.push(GeneratedModuleIr {
                    input: module.input.clone(),
                    textual: textual_ir.clone(),
                });

                module_artifacts.push(ModuleArtifact {
                    object_path: Some(artifact.object_path),
                    metadata_path: artifact.metadata_path,
                    textual_ir: Some(textual_ir),
                    assembly_path: None,
                });
            }

            if trace_enabled {
                info!(
                    target: "pipeline",
                    stage = "driver.build.codegen.llvm",
                    command = "build",
                    status = "ok",
                    target = %target_str,
                    backend = backend_name.as_str(),
                    kind = kind_name.as_str(),
                    input_count = inputs.len(),
                    inputs = %inputs_summary,
                    elapsed_ms = codegen_start.elapsed().as_millis() as u64
                );
            }

            if emit_object {
                final_artifact_path = module_artifacts
                    .iter()
                    .find_map(|artifact| artifact.object_path.clone())
                    .or_else(|| link_inputs.first().cloned())
                    .or_else(|| Some(artifact_path.clone()));
            } else {
                let link_refs: Vec<&Path> = link_inputs.iter().map(PathBuf::as_path).collect();
                let link_start = Instant::now();
                let mut link_codegen_options = codegen::CodegenOptions::default();
                link_codegen_options.backend = Backend::Llvm;
                link_codegen_options.ffi_config = ffi_config.clone();
                link_codegen_options.runtime = runtime_metadata.clone();
                let link_options = codegen::collect_native_link_options(
                    &frontend.mir_module,
                    &link_codegen_options,
                );
                codegen::link_artifact(&target, kind, &link_refs, &artifact_path, &link_options)?;
                if trace_enabled {
                    info!(
                        target: "pipeline",
                        stage = "driver.build.link",
                        command = "build",
                        status = "ok",
                        target = %target_str,
                        backend = backend_name.as_str(),
                        kind = kind_name.as_str(),
                        input_count = inputs.len(),
                        inputs = %inputs_summary,
                        elapsed_ms = link_start.elapsed().as_millis() as u64
                    );
                }

                if kind.is_library() && emit_library_pack {
                    let archive_path = artifact_path.with_extension("clrlib");
                    let mut object_entries: Vec<(String, PathBuf)> = Vec::new();
                    let mut metadata_entries: Vec<(String, PathBuf)> = Vec::new();
                    let mut extra_entries: Vec<(String, FileRole, PathBuf)> = Vec::new();

                    for artifact in &module_artifacts {
                        if let Some(object) = &artifact.object_path {
                            object_entries
                                .push((archive_entry_name("objects", object), object.clone()));
                            let ir_path = object.with_extension("ll");
                            if ir_path.exists() {
                                extra_entries.push((
                                    archive_entry_name("ir", &ir_path),
                                    FileRole::LlvmIr,
                                    ir_path,
                                ));
                            }
                        }
                        if let Some(metadata) = &artifact.metadata_path {
                            metadata_entries
                                .push((archive_entry_name("objects", metadata), metadata.clone()));
                        }
                    }

                    let object_pairs: Vec<(&str, &Path)> = object_entries
                        .iter()
                        .map(|(name, path)| (name.as_str(), path.as_path()))
                        .collect();
                    let metadata_pairs: Vec<(&str, &Path)> = metadata_entries
                        .iter()
                        .map(|(name, path)| (name.as_str(), path.as_path()))
                        .collect();
                    let extra_pairs: Vec<(&str, FileRole, &Path)> = extra_entries
                        .iter()
                        .map(|(name, role, path)| (name.as_str(), role.clone(), path.as_path()))
                        .collect();

                    let pack_start = Instant::now();
                    let pack = codegen::package_clrlib_archive(
                        &frontend.combined_ast,
                        &frontend.mir_module,
                        &target,
                        kind,
                        &archive_path,
                        &object_pairs,
                        &metadata_pairs,
                        &extra_pairs,
                    )?;
                    library_pack = Some(pack);
                    if trace_enabled {
                        info!(
                            target: "pipeline",
                            stage = "driver.build.package.clrlib",
                            command = "build",
                            status = "ok",
                            target = %target_str,
                            backend = backend_name.as_str(),
                            kind = kind_name.as_str(),
                            input_count = inputs.len(),
                            inputs = %inputs_summary,
                            elapsed_ms = pack_start.elapsed().as_millis() as u64
                        );
                    }
                }

                final_artifact_path = Some(artifact_path.clone());
            }
        }
        Backend::Cc1 => {
            if frontend.modules.len() != 1 {
                return Err(crate::error::Error::internal(
                    "cc1 backend currently supports a single input module",
                ));
            }
            let mut options = codegen::CodegenOptions::default();
            options.backend = Backend::Cc1;
            options.keep_object = true;
            options.link_final_artifact = false;
            options.cc1 = Some(codegen::cc1::Cc1Options {
                extra_args: cc1_args.clone(),
                keep_preprocessed: cc1_keep_temps,
            });
            options.runtime = runtime_metadata.clone();

            let codegen_start = Instant::now();
            let codegen_artifact = codegen::compile_module(
                &frontend.workspace_source,
                &frontend.combined_ast,
                &frontend.mir_module,
                Some(&frontend.mir_module),
                &frontend.perf_metadata,
                &target,
                kind,
                &artifact_path,
                &options,
                &frontend.drop_glue,
                &frontend.hash_glue,
                &frontend.eq_glue,
                &frontend.type_metadata,
            )?;
            if trace_enabled {
                info!(
                    target: "pipeline",
                    stage = "driver.build.codegen.cc1",
                    command = "build",
                    status = "ok",
                    target = %target_str,
                    backend = backend_name.as_str(),
                    kind = kind_name.as_str(),
                    input_count = inputs.len(),
                    inputs = %inputs_summary,
                    elapsed_ms = codegen_start.elapsed().as_millis() as u64
                );
            }

            generated_ir.push(GeneratedModuleIr {
                input: artifact_path.clone(),
                textual: codegen_artifact.textual_ir.clone(),
            });
            module_artifacts = vec![ModuleArtifact {
                object_path: None,
                metadata_path: None,
                textual_ir: Some(codegen_artifact.textual_ir),
                assembly_path: Some(codegen_artifact.artifact_path.clone()),
            }];
            library_pack = None;
            final_artifact_path = Some(codegen_artifact.artifact_path);
        }
    }

    if let Some(path) = final_artifact_path.as_ref() {
        package_ffi_assets(path, &ffi.packages)?;
    } else {
        package_ffi_assets(&artifact_path, &ffi.packages)?;
    }

    if emit_header {
        if !kind.is_library() {
            return Err(crate::error::Error::internal(
                "--emit-header requires a library crate type (--crate-type lib or dylib)",
            ));
        }
        let options = HeaderOptions::default();
        let header = generate_header(&frontend.combined_ast, &options).map_err(|err| {
            crate::error::Error::codegen(format!("header generation failed: {err}"))
        })?;
        let header_output = artifact_path.with_extension("h");
        if header_output == artifact_path {
            return Err(crate::error::Error::internal(
                "header output path matches the compiled artifact; adjust --output when using --emit-header",
            ));
        }
        if let Some(parent) = header_output.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(&header_output, header)?;
        header_path = Some(header_output);
    }

    let previous_files: HashMap<String, FileState> = previous_manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|file| (file.path.clone(), file.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut object_by_input: HashMap<PathBuf, PathBuf> = HashMap::new();
    for (module, artifact) in frontend.modules.iter().zip(&module_artifacts) {
        if module.is_stdlib {
            continue;
        }
        if let Some(object) = &artifact.object_path {
            object_by_input.insert(module.input.clone(), object.clone());
        }
    }

    let mut manifest_files = Vec::new();
    for input in &inputs {
        let rel = relative_to(&workspace_root, input);
        let display = rel.to_string_lossy().to_string();
        let previous = previous_files.get(&display);
        let mut state = incremental::snapshot_file(display.clone(), input, previous)?;
        if let Some(object) = object_by_input.get(input) {
            state.object = Some(
                relative_to(&workspace_root, object)
                    .to_string_lossy()
                    .to_string(),
            );
        }
        manifest_files.push(state);
    }

    let artifact_state = ArtifactState {
        objects: module_artifacts
            .iter()
            .filter_map(|artifact| artifact.object_path.as_ref())
            .map(|path| {
                relative_to(&workspace_root, path)
                    .to_string_lossy()
                    .to_string()
            })
            .collect(),
        outputs: final_artifact_path
            .as_ref()
            .map(|path| {
                vec![
                    relative_to(&workspace_root, path)
                        .to_string_lossy()
                        .to_string(),
                ]
            })
            .unwrap_or_else(|| {
                vec![
                    relative_to(&workspace_root, &artifact_path)
                        .to_string_lossy()
                        .to_string(),
                ]
            }),
    };

    let mut option_parts = Vec::new();
    option_parts.push(configuration.as_str().to_string());
    option_parts.push(kind.as_str().to_string());
    option_parts.push(backend.as_str().to_string());
    option_parts.push(target.triple().to_string());
    option_parts.push(if emit_object { "emit_object" } else { "link" }.to_string());
    option_parts.push(if emit_header { "header" } else { "noheader" }.to_string());
    option_parts.push(if emit_library_pack { "pack" } else { "nopack" }.to_string());
    option_parts.push(if load_stdlib { "stdlib" } else { "nostd" }.to_string());
    option_parts.push(
        if no_dependencies {
            "nodeps"
        } else {
            "withdeps"
        }
        .to_string(),
    );
    option_parts.push(format!("runtime:{}", runtime_identity));
    option_parts.push(format!("runtime-abi:{}", runtime_abi));
    option_parts.push(format!("runtime-hash:{}", runtime_manifest_hash));
    option_parts.push(format!("cpuisa:{cpu_isa:?}"));
    if let Some(suffix) = version_suffix.as_ref() {
        option_parts.push(format!("suffix:{suffix}"));
    }
    for define in &defines {
        let formatted = if let Some(value) = &define.value {
            format!("{}={}", define.name, value)
        } else {
            define.name.clone()
        };
        option_parts.push(format!("define:{formatted}"));
    }
    let options_fingerprint = incremental::options_fingerprint(&option_parts);

    let manifest = BuildManifest {
        schema_version: incremental::MANIFEST_VERSION,
        toolchain: crate::codegen::compiler_cache_identity().to_string(),
        target: target.triple().to_string(),
        backend: backend.as_str().to_string(),
        profile: configuration.clone(),
        options_fingerprint,
        runtime_identity: runtime_identity.clone(),
        runtime_abi,
        runtime_manifest_hash,
        files: manifest_files,
        artifacts: artifact_state,
    };
    incremental::write_manifest(&cache_manifest_path, &manifest)?;

    if trace_enabled {
        info!(
            target: "pipeline",
            stage = "driver.build.complete",
            command = "build",
            status = "ok",
            target = %target_str,
            backend = backend_name.as_str(),
            kind = kind_name.as_str(),
            input_count = inputs.len(),
            inputs = %inputs_summary,
            load_stdlib,
            emit_wat = emit_wat_text,
            artifact = final_artifact_path
                .as_ref()
                .map(|path| path.display().to_string()),
            library_pack = library_pack.as_ref().map(|path| path.display().to_string()),
            header = header_path.as_ref().map(|path| path.display().to_string()),
            elapsed_ms = build_start.elapsed().as_millis() as u64
        );
    }

    Ok(frontend.into_report(
        final_artifact_path,
        library_pack,
        header_path,
        generated_ir,
        module_artifacts,
    ))
}

fn package_ffi_assets(artifact: &Path, packages: &[PathBuf]) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }
    let deps_dir = artifact.with_extension("deps");
    fs::create_dir_all(&deps_dir)?;
    for package in packages {
        let Some(name) = package.file_name() else {
            return Err(Error::internal(format!(
                "ffi package `{}` is missing a filename",
                package.display()
            )));
        };
        let dest = deps_dir.join(name);
        fs::copy(package, &dest)?;
    }
    Ok(())
}

fn resolve_artifact_path(
    default_input: &Path,
    target: &Target,
    kind: ChicKind,
    backend: Backend,
    emit_object: bool,
    explicit_output: &Option<PathBuf>,
    obj_root_override: Option<&Path>,
    bin_root_override: Option<&Path>,
    artifacts_root: Option<&Path>,
    runtime_identity: &str,
    configuration: &str,
    version_suffix: Option<&str>,
) -> ArtifactLayout {
    let mut base = default_artifact_path(default_input, target, kind, backend);
    if emit_object {
        base.set_extension("o");
    }

    let filename = add_version_suffix(&base, version_suffix);
    let file_name_only = filename
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| filename.clone());

    if artifacts_root.is_none() {
        let artifact_path = if let Some(output) = explicit_output {
            output.clone()
        } else {
            filename.clone()
        };
        let object_base = artifact_path.with_extension("o");
        let parent = artifact_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        return ArtifactLayout {
            artifact_path,
            object_base,
            obj_dir: parent.clone(),
            bin_dir: parent,
        };
    }

    let obj_root = obj_root_override
        .map(PathBuf::from)
        .or_else(|| artifacts_root.map(|root| root.join("obj")))
        .unwrap_or_else(|| PathBuf::from("obj"));
    let bin_root = bin_root_override
        .map(PathBuf::from)
        .or_else(|| artifacts_root.map(|root| root.join("bin")))
        .unwrap_or_else(|| PathBuf::from("bin"));

    let mut partition = PathBuf::from(target.triple())
        .join(configuration)
        .join(backend.as_str());
    if runtime_identity != "no-runtime" {
        let runtime_component: String = runtime_identity
            .chars()
            .map(|ch| if ch == '/' || ch == '\\' { '_' } else { ch })
            .collect();
        partition = partition.join(runtime_component);
    }
    let obj_dir = obj_root.join(&partition);
    let bin_dir = bin_root.join(&partition);

    let artifact_path = if let Some(output) = explicit_output {
        output.clone()
    } else if emit_object {
        obj_dir.join(&file_name_only)
    } else {
        bin_dir.join(&file_name_only)
    };

    ArtifactLayout {
        artifact_path,
        object_base: obj_dir.join(&file_name_only),
        obj_dir,
        bin_dir,
    }
}

fn add_version_suffix(path: &Path, suffix: Option<&str>) -> PathBuf {
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return path.to_path_buf();
    };
    let suffix = match suffix {
        Some(value) if !value.trim().is_empty() => value.trim(),
        _ => return path.to_path_buf(),
    };
    let mut name = stem.to_string();
    name.push('-');
    name.push_str(suffix);
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        PathBuf::from(format!("{name}.{ext}"))
    } else {
        PathBuf::from(name)
    }
}

fn purge_incremental_artifacts(layout: &ArtifactLayout) -> Result<()> {
    if layout.artifact_path.exists() {
        let _ = fs::remove_file(&layout.artifact_path);
    }
    if layout.obj_dir.exists() {
        for entry in fs::read_dir(&layout.obj_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let _ = fs::remove_file(path);
        }
    }
    let cache_path = layout.cache_manifest_path();
    if cache_path.exists() {
        let _ = fs::remove_file(cache_path);
    }
    Ok(())
}

fn purge_all_artifacts(layout: &ArtifactLayout) -> Result<()> {
    if layout.obj_dir.exists() {
        let _ = fs::remove_dir_all(&layout.obj_dir);
    }
    if layout.bin_dir.exists() {
        let _ = fs::remove_dir_all(&layout.bin_dir);
    }
    Ok(())
}

fn relative_to(base: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn ensure_target_runtime_matches(
    runtime: &ResolvedRuntime,
    target_runtime: &TargetRuntime,
    kind: ChicKind,
) -> Result<()> {
    match target_runtime {
        TargetRuntime::NativeNoStd if runtime.kind != RuntimeKind::NoStd => {
            Err(Error::internal(format!(
                "target requests native no_std runtime but manifest selected `{}`",
                runtime.kind.as_str()
            )))
        }
        TargetRuntime::NativeStd | TargetRuntime::Llvm if runtime.kind != RuntimeKind::Native => {
            if runtime.kind == RuntimeKind::NoStd && kind.is_library() {
                return Ok(());
            }
            Err(Error::internal(format!(
                "target requests native runtime but manifest selected `{}`",
                runtime.kind.as_str()
            )))
        }
        _ => Ok(()),
    }
}

struct ArtifactLayout {
    artifact_path: PathBuf,
    object_base: PathBuf,
    obj_dir: PathBuf,
    bin_dir: PathBuf,
}

impl ArtifactLayout {
    fn cache_manifest_path(&self) -> PathBuf {
        self.obj_dir.join("cache").join("cache_manifest.json")
    }
}
