use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::chic_kind::ChicKind;
use crate::clone_glue::{SynthesisedCloneGlue, synthesise_clone_glue};
use crate::codegen::Backend;
use crate::diagnostics::DiagnosticCode;
use crate::drop_glue::{SynthesisedDropGlue, synthesise_drop_glue};
use crate::eq_glue::{SynthesisedEqGlue, synthesise_eq_glue};
use crate::error::Result;
use crate::frontend::ast::{CrateStdSetting, Item, Module as AstModule, NamespaceDecl};
use crate::frontend::cfg::apply_cfg;
use crate::frontend::conditional::{ConditionalDefines, preprocess};
use crate::frontend::diagnostics::{Diagnostic, FileCache, Span};
use crate::frontend::import_resolver::ImportResolver;
use crate::frontend::lexer::{TokenKind, lex};
use crate::frontend::macro_expander::{MacroRegistry, expand_module as expand_macros};
use crate::frontend::parser::{ParseResult, parse_module_in_file};
use crate::hash_glue::{SynthesisedHashGlue, synthesise_hash_glue};
use crate::language::{features_from_defines, set_language_features};
use crate::lint::{LintConfig, LintDiagnostic, LintModuleInfo, run_lints};
use crate::manifest::{DependencySource, Manifest, MissingDocsRule, WorkspaceConfig};
use crate::mir::normalise_cost_model;
use crate::mir::{
    BorrowCheckResult, InterfaceDefaultImpl, LoweringDiagnostic, LoweringResult, MirModule,
    borrow_check_module, check_fallible_values, check_unreachable_code, configure_pointer_width,
    intern_raw_strings, lower_module_with_units_and_hook, verify_body,
};
use crate::monomorphize::{MonomorphizationSummary, analyse_module};
use crate::package::resolver::{ResolveOptions, resolve_dependencies};
use crate::perf::PerfMetadata;
use crate::primitives::PrimitiveDescriptor;
use crate::runtime_package::ResolvedRuntime;
use crate::target::Target;
use crate::threading::{ThreadRuntimeMode, configure_thread_runtime};
use crate::type_metadata::{SynthesisedTypeMetadata, synthesise_type_metadata};
use crate::typeck::{
    AsyncSignatureInfo, PackageContext, TraitSolverMetrics, TypeCheckResult, TypeConstraint,
    check_module_with_context as type_check_module,
};
use tracing::info;

use super::report::{ModuleArtifact, ModuleReport, slice_mir_module};
use super::{FrontendReport, GeneratedModuleIr};

mod crate_attributes;
mod docs;
mod package_rules;
use package_rules::{
    append_external_diagnostics, attach_manifest_issues, attach_package_resolution_status,
    enforce_namespace_rules, validate_package_imports,
};
mod logging;
mod module_loader;
mod package_std;
mod trim;

pub(crate) struct CompilerPipelineBuilder<'a> {
    command: &'static str,
    inputs: &'a [PathBuf],
    manifest: Option<Manifest>,
    workspace: Option<WorkspaceConfig>,
    target: &'a Target,
    kind: ChicKind,
    backend: Backend,
    load_stdlib: bool,
    coverage_enabled: bool,
    corelib_files: &'a [PathBuf],
    foundationlib_files: &'a [PathBuf],
    alloclib_files: &'a [PathBuf],
    stdlib_files: &'a [PathBuf],
    nostd_runtime_files: &'a [PathBuf],
    runtime: Option<ResolvedRuntime>,
    trace_enabled: bool,
    trait_solver_metrics: bool,
    defines: ConditionalDefines,
    lint_config: LintConfig,
    extra_primitives_hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
    doc_enforcement: MissingDocsRule,
    restore_enabled: bool,
}

impl<'a> CompilerPipelineBuilder<'a> {
    pub fn new(
        command: &'static str,
        inputs: &'a [PathBuf],
        target: &'a Target,
        defines: ConditionalDefines,
    ) -> Self {
        Self {
            command,
            inputs,
            manifest: None,
            workspace: None,
            target,
            kind: ChicKind::Executable,
            backend: Backend::Llvm,
            load_stdlib: false,
            coverage_enabled: false,
            corelib_files: &[],
            foundationlib_files: &[],
            alloclib_files: &[],
            stdlib_files: &[],
            nostd_runtime_files: &[],
            runtime: None,
            trace_enabled: false,
            trait_solver_metrics: false,
            defines,
            lint_config: LintConfig::default(),
            extra_primitives_hook: None,
            doc_enforcement: MissingDocsRule::default(),
            restore_enabled: true,
        }
    }

    pub fn kind(mut self, kind: ChicKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn coverage_enabled(mut self, enabled: bool) -> Self {
        self.coverage_enabled = enabled;
        self
    }

    pub fn restore_enabled(mut self, enabled: bool) -> Self {
        self.restore_enabled = enabled;
        self
    }

    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    pub fn manifest(mut self, manifest: Option<Manifest>) -> Self {
        self.manifest = manifest;
        self
    }

    pub fn workspace(mut self, workspace: Option<WorkspaceConfig>) -> Self {
        self.workspace = workspace;
        self
    }

    pub fn load_stdlib(mut self, load: bool) -> Self {
        self.load_stdlib = load;
        self
    }

    pub fn stdlib_files(mut self, files: &'a [PathBuf]) -> Self {
        self.stdlib_files = files;
        self
    }

    pub fn nostd_runtime_files(mut self, files: &'a [PathBuf]) -> Self {
        self.nostd_runtime_files = files;
        self
    }

    pub fn runtime(mut self, runtime: Option<ResolvedRuntime>) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn alloclib_files(mut self, files: &'a [PathBuf]) -> Self {
        self.alloclib_files = files;
        self
    }

    pub fn foundationlib_files(mut self, files: &'a [PathBuf]) -> Self {
        self.foundationlib_files = files;
        self
    }

    pub fn corelib_files(mut self, files: &'a [PathBuf]) -> Self {
        self.corelib_files = files;
        self
    }

    pub fn trace_enabled(mut self, enabled: bool) -> Self {
        self.trace_enabled = enabled;
        self
    }

    pub fn trait_solver_metrics(mut self, enabled: bool) -> Self {
        self.trait_solver_metrics = enabled;
        self
    }

    pub fn lint_config(mut self, lint_config: LintConfig) -> Self {
        self.lint_config = lint_config;
        self
    }

    pub fn doc_enforcement(mut self, rule: MissingDocsRule) -> Self {
        self.doc_enforcement = rule;
        self
    }

    #[allow(dead_code)]
    pub fn extra_primitives_hook(mut self, hook: Option<fn() -> Vec<PrimitiveDescriptor>>) -> Self {
        self.extra_primitives_hook = hook;
        self
    }

    pub fn build(self) -> CompilerPipeline<'a> {
        let mut defines = self.defines;
        defines.set_string("BACKEND", self.backend.as_str());
        defines.set_string("backend", self.backend.as_str());
        defines.set_string("KIND", self.kind.as_str());
        defines.set_string("kind", self.kind.as_str());
        CompilerPipeline {
            config: PipelineConfig {
                command: self.command,
                inputs: self.inputs,
                manifest: self.manifest,
                workspace: self.workspace,
                target: self.target,
                kind: self.kind,
                backend: self.backend,
                load_stdlib: self.load_stdlib,
                coverage_enabled: self.coverage_enabled,
                corelib_files: self.corelib_files,
                foundationlib_files: self.foundationlib_files,
                alloclib_files: self.alloclib_files,
                stdlib_files: self.stdlib_files,
                nostd_runtime_files: self.nostd_runtime_files,
                runtime: self.runtime,
                trace_enabled: self.trace_enabled,
                trait_solver_metrics: self.trait_solver_metrics,
                defines,
                lint_config: self.lint_config,
                extra_primitives_hook: self.extra_primitives_hook,
                doc_enforcement: self.doc_enforcement,
                restore_enabled: self.restore_enabled,
            },
        }
    }
}

struct PipelineConfig<'a> {
    command: &'static str,
    inputs: &'a [PathBuf],
    manifest: Option<Manifest>,
    workspace: Option<WorkspaceConfig>,
    target: &'a Target,
    kind: ChicKind,
    backend: Backend,
    load_stdlib: bool,
    coverage_enabled: bool,
    corelib_files: &'a [PathBuf],
    foundationlib_files: &'a [PathBuf],
    alloclib_files: &'a [PathBuf],
    stdlib_files: &'a [PathBuf],
    nostd_runtime_files: &'a [PathBuf],
    runtime: Option<ResolvedRuntime>,
    trace_enabled: bool,
    trait_solver_metrics: bool,
    defines: ConditionalDefines,
    lint_config: LintConfig,
    extra_primitives_hook: Option<fn() -> Vec<PrimitiveDescriptor>>,
    doc_enforcement: MissingDocsRule,
    restore_enabled: bool,
}

pub(crate) struct CompilerPipeline<'a> {
    config: PipelineConfig<'a>,
}

impl<'a> CompilerPipeline<'a> {
    pub fn execute(&self) -> Result<FrontendState> {
        if self.config.inputs.is_empty() {
            return Err(crate::error::Error::internal(
                "no input sources provided for frontend",
            ));
        }

        let thread_mode = if self.config.backend == Backend::Wasm {
            ThreadRuntimeMode::Unsupported {
                backend: self.config.backend.as_str(),
            }
        } else {
            ThreadRuntimeMode::Supported
        };
        configure_thread_runtime(thread_mode);
        let language_features = features_from_defines(&self.config.defines);
        set_language_features(language_features);

        let trace_enabled = self.config.trace_enabled;
        let frontend_start = Instant::now();
        let metadata = logging::PipelineLogMetadata::new(
            self.config.command,
            self.config.target.triple().to_string(),
            self.config.backend.as_str(),
            self.config.kind.as_str(),
            self.config.inputs.len(),
            self.config.load_stdlib,
            self.config.trait_solver_metrics,
        );
        if trace_enabled {
            info!(
                target: "pipeline",
                stage = "frontend.start",
                command = metadata.command,
                status = "start",
                target = %metadata.target,
                backend = metadata.backend,
                kind = metadata.kind,
                input_count = metadata.input_count,
                load_stdlib = metadata.load_stdlib
            );
        }

        let macro_registry = MacroRegistry::with_builtins();
        let mut files = FileCache::default();
        let mut modules: Vec<FrontendModuleState> = Vec::new();
        let mut loaded_modules: HashSet<PathBuf> = HashSet::new();
        let mut override_manifests: HashMap<PathBuf, Manifest> = HashMap::new();

        let async_override_env = std::env::var_os("CHIC_ASYNC_STDLIB_OVERRIDE");
        if !self.config.load_stdlib
            && async_override_env.is_some()
            && matches!(self.config.backend, Backend::Llvm | Backend::Wasm)
            && (matches!(self.config.kind, ChicKind::Executable)
                || (self.config.backend == Backend::Wasm && self.config.kind.is_library()))
        {
            let startup_override = std::env::var_os("CHIC_STARTUP_STDLIB_OVERRIDE")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("packages/std/src/native_startup.cl")
                });
            let async_override = async_override_env.map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/std/src/async.cl")
            });
            let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let async_manifest = Manifest::discover(&repo_root.join("packages/std.async"))?
                .ok_or_else(|| {
                    crate::error::Error::internal(
                        "missing manifest for packages/std.async (needed for async stdlib override)",
                    )
                })?;
            let runtime_manifest = Manifest::discover(&repo_root.join("packages/std.runtime"))?
                .ok_or_else(|| {
                    crate::error::Error::internal(
                        "missing manifest for packages/std.runtime (needed for startup stdlib override)",
                    )
                })?;
            override_manifests.insert(async_override.clone(), async_manifest);
            override_manifests.insert(startup_override.clone(), runtime_manifest);

            if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                eprintln!(
                    "[chic-debug] injecting native startup + async override: {} and {}",
                    startup_override.display(),
                    async_override.display()
                );
            }

            let mut injected: HashSet<PathBuf> = HashSet::new();
            for path in [startup_override, async_override] {
                if !injected.insert(path.clone()) {
                    continue;
                }
                if self.config.inputs.iter().any(|input| *input == path) {
                    continue;
                }
                let canonical = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if !loaded_modules.insert(canonical) {
                    continue;
                }
                let read_start = Instant::now();
                let mut source = fs::read_to_string(&path)?;
                logging::log_stage_with_path(
                    trace_enabled,
                    &metadata,
                    "frontend.stdlib.read_source",
                    &path,
                    read_start,
                );

                let parse_start = Instant::now();
                let mut preprocess_result = preprocess(&source, &self.config.defines);
                if let Some(rewritten) = preprocess_result.rewritten {
                    source = rewritten;
                }
                let file_id = files.add_file(path.clone(), source.clone());
                module_loader::stamp_file_id(&mut preprocess_result.diagnostics, file_id);
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
                    apply_cfg(&mut module, &self.config.defines)
                };
                parse.diagnostics.append(&mut cfg_diags);
                logging::log_stage_with_path(
                    trace_enabled,
                    &metadata,
                    "frontend.stdlib.parse",
                    &path,
                    parse_start,
                );

                let macro_start = Instant::now();
                let expansion = {
                    let mut module = parse.module_mut();
                    expand_macros(&mut module, &macro_registry)
                };
                parse.diagnostics.extend(expansion.diagnostics);
                let mut cfg_diags = {
                    let mut module = parse.module_mut();
                    apply_cfg(&mut module, &self.config.defines)
                };
                parse.diagnostics.append(&mut cfg_diags);
                parse.module = parse.module_owned();
                logging::log_stage_with_path(
                    trace_enabled,
                    &metadata,
                    "frontend.stdlib.expand_macros",
                    &path,
                    macro_start,
                );

                let manifest = override_manifests.get(&path).cloned();
                modules.push(FrontendModuleState {
                    input: path,
                    source,
                    parse,
                    manifest,
                    is_stdlib: true,
                    requires_codegen: true,
                });
            }
        }

        let mut workspace_source = String::new();
        for module in &modules {
            module_loader::append_workspace_source(
                &mut workspace_source,
                &module.input,
                &module.source,
            );
        }

        modules.reserve(self.config.inputs.len());

        for path in self.config.inputs {
            let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            if !loaded_modules.insert(canonical) {
                continue;
            }
            let read_start = Instant::now();
            let mut source = fs::read_to_string(path)?;
            logging::log_stage_with_path(
                trace_enabled,
                &metadata,
                "frontend.read_source",
                path,
                read_start,
            );

            let parse_start = Instant::now();
            let mut preprocess_result = preprocess(&source, &self.config.defines);
            if let Some(rewritten) = preprocess_result.rewritten {
                source = rewritten;
            }
            let file_id = files.add_file(path.clone(), source.clone());
            module_loader::stamp_file_id(&mut preprocess_result.diagnostics, file_id);
            let mut parse = match parse_module_in_file(&source, file_id) {
                Ok(parsed) => parsed,
                Err(err) => {
                    log_stdlib_parse_error(path, &source, &err);
                    return Err(err.with_file(path.clone(), source).into());
                }
            };
            parse.diagnostics.extend(preprocess_result.diagnostics);
            let mut cfg_diags = {
                let mut module = parse.module_mut();
                apply_cfg(&mut module, &self.config.defines)
            };
            parse.diagnostics.append(&mut cfg_diags);
            logging::log_stage_with_path(
                trace_enabled,
                &metadata,
                "frontend.parse",
                path,
                parse_start,
            );

            let macro_start = Instant::now();
            let expansion = {
                let mut module = parse.module_mut();
                expand_macros(&mut module, &macro_registry)
            };
            parse.diagnostics.extend(expansion.diagnostics);
            let mut cfg_diags = {
                let mut module = parse.module_mut();
                apply_cfg(&mut module, &self.config.defines)
            };
            parse.diagnostics.append(&mut cfg_diags);
            parse.module = parse.module_owned();
            logging::log_stage_with_path(
                trace_enabled,
                &metadata,
                "frontend.expand_macros",
                path,
                macro_start,
            );

            modules.push(FrontendModuleState {
                input: path.clone(),
                source,
                parse,
                manifest: override_manifests
                    .get(path)
                    .cloned()
                    .or_else(|| self.config.manifest.clone()),
                is_stdlib: false,
                requires_codegen: true,
            });
        }

        let workspace_crate_attributes = crate_attributes::resolve_workspace_crate_attributes(
            &mut modules,
            self.config.manifest.as_ref(),
        );
        let is_no_std_crate = matches!(
            workspace_crate_attributes.std_setting,
            CrateStdSetting::NoStd { .. }
        );
        let manifest_declares_std = self
            .config
            .manifest
            .as_ref()
            .map(package_std::declares_std_dependency)
            .unwrap_or(false);
        let is_std_package = self
            .config
            .manifest
            .as_ref()
            .and_then(|manifest| manifest.package())
            .and_then(|pkg| pkg.name.as_deref())
            .map(package_std::is_std_name)
            .unwrap_or(false);
        let should_load_stdlib =
            self.config.load_stdlib && !manifest_declares_std && !is_std_package;
        let enable_alloc_env = std::env::var("CHIC_ENABLE_ALLOC")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let enable_alloc_flag = self.config.defines.is_true("ENABLE_ALLOC");
        let enable_alloc = enable_alloc_env || enable_alloc_flag;
        let should_load_alloc = should_load_stdlib && (!is_no_std_crate || enable_alloc);
        let should_load_foundation = should_load_stdlib && (!is_no_std_crate || enable_alloc);
        let should_load_no_std_runtime =
            should_load_stdlib && is_no_std_crate && !self.config.nostd_runtime_files.is_empty();

        if should_load_stdlib && !self.config.corelib_files.is_empty() {
            let core_modules = module_loader::load_standard_library(
                "core",
                self.config.backend,
                self.config.kind,
                &macro_registry,
                &mut files,
                &mut loaded_modules,
                trace_enabled,
                self.config.corelib_files,
                &self.config.defines,
                &metadata,
            )?;
            modules.extend(core_modules);
        }

        if should_load_alloc && !self.config.alloclib_files.is_empty() {
            let alloc_modules = module_loader::load_standard_library(
                "alloc",
                self.config.backend,
                self.config.kind,
                &macro_registry,
                &mut files,
                &mut loaded_modules,
                trace_enabled,
                self.config.alloclib_files,
                &self.config.defines,
                &metadata,
            )?;
            modules.extend(alloc_modules);
        }

        if should_load_no_std_runtime {
            let nostd_modules = module_loader::load_standard_library(
                "no_std_runtime",
                self.config.backend,
                self.config.kind,
                &macro_registry,
                &mut files,
                &mut loaded_modules,
                trace_enabled,
                self.config.nostd_runtime_files,
                &self.config.defines,
                &metadata,
            )?;
            modules.extend(nostd_modules);
        }

        if should_load_foundation && !self.config.foundationlib_files.is_empty() {
            let foundation_modules = module_loader::load_standard_library(
                "foundation",
                self.config.backend,
                self.config.kind,
                &macro_registry,
                &mut files,
                &mut loaded_modules,
                trace_enabled,
                self.config.foundationlib_files,
                &self.config.defines,
                &metadata,
            )?;
            modules.extend(foundation_modules);
        }

        if should_load_stdlib && !is_no_std_crate {
            let stdlib_modules = module_loader::load_standard_library(
                "stdlib",
                self.config.backend,
                self.config.kind,
                &macro_registry,
                &mut files,
                &mut loaded_modules,
                trace_enabled,
                self.config.stdlib_files,
                &self.config.defines,
                &metadata,
            )?;
            modules.extend(stdlib_modules);
        }

        if let Some(manifest) = &self.config.manifest {
            attach_manifest_issues(manifest, &mut modules);
            validate_package_imports(manifest, &mut modules);
            enforce_namespace_rules(manifest, self.config.workspace.as_ref(), &mut modules);
            package_std::enforce_std_dependency(manifest, self.config.load_stdlib, &mut modules);
        }

        let used_packages = trim::collect_used_packages(&modules);
        let mut resolver_diagnostics = Vec::new();
        if self.config.restore_enabled {
            if let Some(manifest) = &self.config.manifest {
                if let Some(manifest_path) = manifest.path() {
                    let lockfile = manifest_path.parent().map(|dir| dir.join("manifest.lock"));
                    let options = ResolveOptions::from_env(lockfile);
                    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
                        let deps: Vec<String> = manifest
                            .dependencies()
                            .iter()
                            .map(|dep| match &dep.source {
                                DependencySource::Path(path) => {
                                    format!(
                                        "{} (path {} absolute={})",
                                        dep.name,
                                        path.display(),
                                        path.is_absolute()
                                    )
                                }
                                DependencySource::Git { repo, .. } => {
                                    format!("{} (git {repo})", dep.name)
                                }
                                DependencySource::Registry { registry } => {
                                    format!("{} (registry {:?})", dep.name, registry)
                                }
                            })
                            .collect();
                        eprintln!("[chic-debug] manifest dependencies: {:?}", deps);
                    }
                    let outcome = resolve_dependencies(manifest, manifest_path, &options);
                    resolver_diagnostics.extend(outcome.diagnostics);
                    let mut resolved: HashMap<_, _> = outcome
                        .packages
                        .into_iter()
                        .map(|pkg| (pkg.name.clone(), pkg))
                        .collect();
                    let resolved_snapshot = resolved.clone();
                    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
                        let mut names: Vec<_> = resolved.keys().cloned().collect();
                        names.sort();
                        eprintln!(
                            "[chic-debug] resolved packages: {:?} (used imports: {:?})",
                            names, used_packages
                        );
                        for pkg in resolved.values() {
                            let manifest_path = pkg
                                .manifest
                                .path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| "<none>".to_string());
                            eprintln!(
                                "[chic-debug]   package {} root={} sources={} manifest={}",
                                pkg.name,
                                pkg.root.display(),
                                pkg.manifest.source_roots().len(),
                                manifest_path
                            );
                        }
                    }
                    let reachable = trim::compute_reachable_packages(&used_packages, &resolved);
                    let mut names: Vec<_> = resolved.keys().cloned().collect();
                    names.sort();
                    for name in names {
                        if !reachable.contains(&name) {
                            if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
                                eprintln!("[chic-debug] skipping unreachable package {name}");
                            }
                            continue;
                        }
                        if let Some(package) = resolved.remove(&name) {
                            let mut dep_modules = module_loader::parse_dependency_modules(
                                &package,
                                &mut files,
                                &self.config.defines,
                                &macro_registry,
                                trace_enabled,
                                &metadata,
                            )?;
                            attach_manifest_issues(&package.manifest, &mut dep_modules);
                            validate_package_imports(&package.manifest, &mut dep_modules);
                            enforce_namespace_rules(&package.manifest, None, &mut dep_modules);
                            modules.extend(dep_modules);
                        }
                    }
                    if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
                        let mut reachable_sorted: Vec<_> = reachable.iter().cloned().collect();
                        reachable_sorted.sort();
                        eprintln!("[chic-debug] reachable packages: {:?}", reachable_sorted);
                    }
                    attach_package_resolution_status(
                        Some(manifest),
                        &resolved_snapshot,
                        &mut modules,
                    );
                }
            }
        }

        append_external_diagnostics(&mut modules, resolver_diagnostics);

        workspace_source.clear();
        for module in &modules {
            module_loader::append_workspace_source(
                &mut workspace_source,
                &module.input,
                &module.source,
            );
        }

        let assemble_start = Instant::now();
        let (mut combined_ast, item_units) = assemble_workspace_module(&modules)?;
        if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
            eprintln!("[chic-debug] item_units: {:?}", item_units);
        }
        combined_ast.crate_attributes = workspace_crate_attributes;
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.assemble_workspace",
            assemble_start,
        );

        let unit_packages: Vec<Option<String>> = modules
            .iter()
            .map(|module| {
                module
                    .manifest
                    .as_ref()
                    .and_then(|manifest| manifest.package())
                    .and_then(|pkg| pkg.name.clone())
            })
            .collect();

        let lower_start = Instant::now();
        let (pointer_size, pointer_align) = if self.config.backend == Backend::Wasm {
            (4usize, 4usize)
        } else {
            (8usize, 8usize)
        };
        configure_pointer_width(pointer_size, pointer_align);
        let LoweringResult {
            module: mut mir_module,
            diagnostics: mut mir_lowering_diagnostics,
            constraints: type_constraints,
            unit_slices,
            pass_metrics: _,
            cache_metrics: _,
            mut perf_metadata,
        } = lower_module_with_units_and_hook(
            &combined_ast,
            Some(&item_units),
            Some(&unit_packages),
            self.config.extra_primitives_hook,
        );
        if std::env::var_os("CHIC_DEBUG_TESTCASE_RET_FLOW").is_some() {
            for name in [
                "Std::Async::cancel_token_respects_deadline",
                "Std::Random::rng_sequence_is_deterministic",
            ] {
                if let Some(func) = mir_module.functions.iter().find(|f| f.name == name) {
                    eprintln!(
                        "[testcase-ret-flow] stage=after_lower name={} kind={:?} ret={:?}",
                        name, func.kind, func.signature.ret
                    );
                }
            }
        }
        if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
            eprintln!("[chic-debug] unit_slices: {:?}", unit_slices);
        }
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.lower_module",
            lower_start,
        );
        intern_raw_strings(&mut mir_module);

        let mut unit_functions = vec![Vec::new(); modules.len()];
        for slice in unit_slices {
            if let Some(bucket) = unit_functions.get_mut(slice.unit) {
                bucket.extend(slice.range);
            }
        }
        if !unit_functions.is_empty() {
            let mut assigned = vec![false; mir_module.functions.len()];
            for bucket in &unit_functions {
                for &index in bucket {
                    if let Some(flag) = assigned.get_mut(index) {
                        *flag = true;
                    }
                }
            }
            if let Some(default_bucket) = unit_functions.first_mut() {
                for (index, flag) in assigned.iter().enumerate() {
                    if !flag {
                        default_bucket.push(index);
                    }
                }
            }
        }

        let non_user_buckets_empty = unit_functions
            .iter()
            .take(unit_functions.len().saturating_sub(1))
            .all(|b| b.is_empty());
        let needs_reassign =
            !self.config.load_stdlib && self.config.manifest.is_none() && non_user_buckets_empty;
        if needs_reassign {
            // If we injected native_startup/async override without full stdlib, spread functions
            // into buckets by namespace so stub layouts participate in codegen.
            let startup_idx = modules
                .iter()
                .position(|module| {
                    module
                        .parse
                        .module_ref()
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| ns.starts_with("Std.Runtime.Startup"))
                })
                .unwrap_or(0);
            let async_idx = modules
                .iter()
                .position(|module| {
                    module
                        .parse
                        .module_ref()
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| ns.starts_with("Std.Async"))
                })
                .unwrap_or(startup_idx);
            let user_idx = (0..modules.len())
                .find(|idx| *idx != startup_idx && *idx != async_idx)
                .unwrap_or_else(|| modules.len().saturating_sub(1));

            let mut buckets = vec![Vec::new(); modules.len()];
            for (idx, func) in mir_module.functions.iter().enumerate() {
                let name = func.name.as_str();
                let bucket_idx = if name.starts_with("Std::Runtime::Startup::") {
                    startup_idx
                } else if name.starts_with("Std::Async::") {
                    async_idx
                } else {
                    user_idx
                };
                if let Some(bucket) = buckets.get_mut(bucket_idx) {
                    bucket.push(idx);
                }
            }
            // Merge async bucket into the user bucket so cross-unit calls resolve without
            // additional signature plumbing.
            if buckets.len() > 2 && async_idx != user_idx {
                let async_funcs = buckets[async_idx].clone();
                buckets[user_idx].extend(async_funcs);
                buckets[user_idx].sort_unstable();
                buckets[user_idx].dedup();
                buckets[async_idx].clear();
            }
            if buckets.iter().all(|b| b.is_empty()) {
                // fallback to original if we somehow lost everything
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    eprintln!("[chic-debug] async bucket reassignment skipped (all empty)");
                }
            } else {
                if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                    let sizes: Vec<_> = buckets.iter().map(|b| b.len()).collect();
                    eprintln!(
                        "[chic-debug] reassigned unit buckets by namespace (startup={startup_idx}, async={async_idx}, user={user_idx}): {:?}",
                        sizes
                    );
                }
                unit_functions = buckets;
            }
        }

        if !self.config.load_stdlib && self.config.manifest.is_none() {
            let startup_idx = modules
                .iter()
                .position(|module| {
                    module
                        .parse
                        .module_ref()
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| ns.starts_with("Std.Runtime.Startup"))
                })
                .unwrap_or(0);
            let async_idx = modules
                .iter()
                .position(|module| {
                    module
                        .parse
                        .module_ref()
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| ns.starts_with("Std.Async"))
                })
                .unwrap_or(startup_idx);
            let user_idx = (0..modules.len())
                .find(|idx| *idx != startup_idx && *idx != async_idx)
                .unwrap_or_else(|| modules.len().saturating_sub(1));

            let mut buckets = vec![Vec::new(); modules.len()];
            for (idx, func) in mir_module.functions.iter().enumerate() {
                let name = func.name.as_str();
                let bucket_idx = if name.starts_with("Std::Runtime::Startup::") {
                    startup_idx
                } else if name.starts_with("Std::Async::") {
                    async_idx
                } else {
                    user_idx
                };
                if let Some(bucket) = buckets.get_mut(bucket_idx) {
                    bucket.push(idx);
                }
            }
            if buckets.len() > 1 && async_idx != user_idx {
                let async_funcs = std::mem::take(&mut buckets[async_idx]);
                if let Some(user_bucket) = buckets.get_mut(user_idx) {
                    user_bucket.extend(async_funcs);
                    user_bucket.sort_unstable();
                    user_bucket.dedup();
                }
            }
            unit_functions = buckets;
            if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
                let sizes: Vec<_> = unit_functions.iter().map(|b| b.len()).collect();
                eprintln!(
                    "[chic-debug] rebuilt buckets for stdlibless compile (startup={startup_idx}, async={async_idx}, user={user_idx}): {:?}",
                    sizes
                );
            }
        }

        if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
            eprintln!(
                "[chic-debug] unit_functions buckets: total_modules={}, functions={}",
                unit_functions.len(),
                mir_module.functions.len()
            );
            for (idx, bucket) in unit_functions.iter().enumerate() {
                let names: Vec<_> = bucket
                    .iter()
                    .filter_map(|i| mir_module.functions.get(*i))
                    .map(|f| {
                        format!(
                            "{} (async={}, machine={})",
                            f.name,
                            f.is_async,
                            f.body.async_machine.is_some()
                        )
                    })
                    .collect();
                eprintln!("[chic-debug]   module[{idx}] -> {} functions", names.len());
                for name in names {
                    eprintln!("[chic-debug]     {name}");
                }
            }
        }

        let typeck_start = Instant::now();
        let unit_import_resolvers: Vec<ImportResolver> = modules
            .iter()
            .map(|module| {
                let module_ref = module.parse.module_ref();
                ImportResolver::build(&module_ref)
            })
            .collect();
        let package_context = PackageContext {
            item_units: Some(item_units.clone()),
            unit_packages: unit_packages.clone(),
            unit_import_resolvers: Some(unit_import_resolvers),
        };
        let TypeCheckResult {
            diagnostics: mut type_diagnostics,
            async_signatures,
            interface_defaults,
            trait_solver_metrics,
        } = type_check_module(
            &combined_ast,
            &type_constraints,
            &mir_module.type_layouts,
            package_context,
        );
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.type_check",
            typeck_start,
        );
        if self.config.trait_solver_metrics {
            logging::log_trait_solver_metrics(&metadata, &trait_solver_metrics);
        }
        attach_async_metadata(&mut mir_module, &async_signatures);
        mir_module.interface_defaults = interface_defaults
            .into_iter()
            .map(|binding| InterfaceDefaultImpl {
                implementer: binding.implementer,
                interface: binding.interface,
                method: binding.method,
                symbol: binding.symbol,
            })
            .collect();

        if std::env::var_os("CHIC_DEBUG_FN_SIG").is_some() {
            for function in &mir_module.functions {
                if function.name.contains("CancellationTokenSource::Create") {
                    eprintln!(
                        "[chic-debug] fn {} params={} ret={:?}",
                        function.name,
                        function.signature.params.len(),
                        function.signature.ret
                    );
                }
            }
        }

        // Allow tests/tools to bypass MIR verification and related diagnostics when the stdlib
        // bodies are known to violate current verifier expectations.
        let suppress_bootstrap_diagnostics = std::env::var_os("CHIC_SKIP_MIR_VERIFY")
            .map(|v| v != "0")
            .unwrap_or(false);

        let verify_start = Instant::now();
        let mut mir_verification = Vec::new();
        if !suppress_bootstrap_diagnostics {
            for function in &mir_module.functions {
                if let Err(errors) = verify_body(&function.body) {
                    mir_verification.push(super::MirVerificationIssue {
                        function: function.name.clone(),
                        errors,
                    });
                }
            }
        }
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.verify_bodies",
            verify_start,
        );
        let reachability_start = Instant::now();
        let reachability_diagnostics = if suppress_bootstrap_diagnostics {
            Vec::new()
        } else {
            check_unreachable_code(&mir_module)
        };
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.reachability",
            reachability_start,
        );
        normalise_cost_model(&mut perf_metadata, &mir_module);

        let borrow_start = Instant::now();
        let BorrowCheckResult {
            diagnostics: borrow_diagnostics,
        } = if suppress_bootstrap_diagnostics {
            BorrowCheckResult {
                diagnostics: Vec::new(),
            }
        } else {
            borrow_check_module(&mir_module)
        };
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.borrow_check",
            borrow_start,
        );
        if trace_enabled {
            info!(
                target: "pipeline",
                stage = "frontend.complete",
                command = metadata.command,
                status = "ok",
                target = %metadata.target,
                backend = metadata.backend,
                kind = metadata.kind,
                input_count = metadata.input_count,
                load_stdlib = metadata.load_stdlib,
                elapsed_ms = frontend_start.elapsed().as_millis() as u64
            );
        }

        let fallible_start = Instant::now();
        let fallible_diagnostics = if suppress_bootstrap_diagnostics {
            Vec::new()
        } else {
            check_fallible_values(&mir_module)
        };
        logging::log_stage(
            trace_enabled,
            &metadata,
            "frontend.fallible_drop",
            fallible_start,
        );

        let monomorphization = analyse_module(&mir_module);
        let drop_glue = synthesise_drop_glue(&mut mir_module, &monomorphization);
        if std::env::var_os("CHIC_DEBUG_TESTCASE_RET_FLOW").is_some() {
            for name in [
                "Std::Async::cancel_token_respects_deadline",
                "Std::Random::rng_sequence_is_deterministic",
            ] {
                if let Some(func) = mir_module.functions.iter().find(|f| f.name == name) {
                    eprintln!(
                        "[testcase-ret-flow] stage=after_drop_glue name={} kind={:?} ret={:?}",
                        name, func.kind, func.signature.ret
                    );
                }
            }
        }
        if !drop_glue.is_empty() {
            if let Some(bucket) = unit_functions.first_mut() {
                for entry in &drop_glue {
                    bucket.push(entry.function_index);
                }
                bucket.sort_unstable();
                bucket.dedup();
            }
        }
        let clone_glue = synthesise_clone_glue(&mut mir_module, &monomorphization);
        if std::env::var_os("CHIC_DEBUG_TESTCASE_RET_FLOW").is_some() {
            for name in [
                "Std::Async::cancel_token_respects_deadline",
                "Std::Random::rng_sequence_is_deterministic",
            ] {
                if let Some(func) = mir_module.functions.iter().find(|f| f.name == name) {
                    eprintln!(
                        "[testcase-ret-flow] stage=after_clone_glue name={} kind={:?} ret={:?}",
                        name, func.kind, func.signature.ret
                    );
                }
            }
        }
        if !clone_glue.is_empty() {
            if let Some(bucket) = unit_functions.first_mut() {
                for entry in &clone_glue {
                    bucket.push(entry.function_index);
                }
                bucket.sort_unstable();
                bucket.dedup();
            }
        }
        let hash_glue = synthesise_hash_glue(&mut mir_module, &monomorphization);
        if !hash_glue.is_empty() {
            if let Some(bucket) = unit_functions.first_mut() {
                for entry in &hash_glue {
                    bucket.push(entry.function_index);
                }
                bucket.sort_unstable();
                bucket.dedup();
            }
        }
        let eq_glue = synthesise_eq_glue(&mut mir_module, &monomorphization);
        if !eq_glue.is_empty() {
            if let Some(bucket) = unit_functions.first_mut() {
                for entry in &eq_glue {
                    bucket.push(entry.function_index);
                }
                bucket.sort_unstable();
                bucket.dedup();
            }
        }
        let type_metadata = synthesise_type_metadata(&mir_module, &drop_glue);
        let lint_modules: Vec<_> = modules
            .iter()
            .map(|module| LintModuleInfo {
                path: module.input.as_path(),
                is_stdlib: module.is_stdlib,
            })
            .collect();
        let lint_diagnostics = if suppress_bootstrap_diagnostics {
            Vec::new()
        } else {
            run_lints(
                &self.config.lint_config,
                &combined_ast,
                &lint_modules,
                &unit_functions,
                &mir_module,
                &drop_glue,
                &clone_glue,
            )
        };
        let doc_diagnostics = if suppress_bootstrap_diagnostics {
            Vec::new()
        } else {
            docs::enforce_missing_docs(&modules, &self.config.doc_enforcement)
        };

        let trim_stats = if self.config.manifest.is_some() {
            Some(trim::trim_unreachable_package_exports(
                self.config.coverage_enabled,
                self.config.manifest.as_ref(),
                &modules,
                &mut mir_module,
                &mut unit_functions,
            ))
        } else {
            None
        };
        if let Some(stats) = &trim_stats {
            if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some()
                && (stats.trimmed_functions > 0 || stats.trimmed_exports > 0)
            {
                eprintln!(
                    "[chic-debug] package trim removed {} functions and {} exports",
                    stats.trimmed_functions, stats.trimmed_exports
                );
            }
        }

        if suppress_bootstrap_diagnostics {
            type_diagnostics.clear();
            mir_lowering_diagnostics.clear();
        }

        Ok(FrontendState {
            modules,
            files,
            combined_ast,
            workspace_source,
            target: self.config.target.clone(),
            kind: self.config.kind,
            runtime: self.config.runtime.clone(),
            mir_module,
            mir_lowering_diagnostics,
            mir_verification,
            reachability_diagnostics,
            borrow_diagnostics,
            fallible_diagnostics,
            type_constraints,
            type_diagnostics,
            unit_functions,
            monomorphization,
            drop_glue,
            clone_glue,
            hash_glue,
            eq_glue,
            type_metadata,
            trait_solver_metrics,
            perf_metadata,
            lint_diagnostics,
            doc_diagnostics,
        })
    }
}

fn assemble_workspace_module(modules: &[FrontendModuleState]) -> Result<(AstModule, Vec<usize>)> {
    let mut shared_namespace: Option<String> = None;
    let mut namespace_conflict = false;

    for module in modules {
        if module.is_stdlib {
            continue;
        }
        let module_ref = module.parse.module_ref();
        let module_ns = module_ref.namespace.clone();
        match (&shared_namespace, module_ns.as_ref()) {
            (None, Some(ns)) => shared_namespace = Some(ns.clone()),
            (Some(existing), Some(ns)) if existing != ns => {
                namespace_conflict = true;
                shared_namespace = None;
                break;
            }
            (Some(_), None) => {
                namespace_conflict = true;
                shared_namespace = None;
                break;
            }
            _ => {}
        }
    }

    let has_stdlib = modules.iter().any(|module| module.is_stdlib);
    if has_stdlib {
        shared_namespace = None;
    }
    debug_assert!(!has_stdlib || shared_namespace.is_none());

    let mut combined = AstModule::new(shared_namespace.clone());
    let mut item_units = Vec::new();

    for (index, module) in modules.iter().enumerate() {
        let module_ref = module.parse.module_ref();
        if std::env::var_os("CHIC_DEBUG_PACKAGE_TRIM").is_some() {
            eprintln!(
                "[chic-debug] assembling module {index} {} (items: {})",
                module.input.display(),
                module_ref.items.len()
            );
        }
        combined
            .namespace_attributes
            .extend(module_ref.namespace_attributes.clone());

        let wrap_namespace = if namespace_conflict {
            module_ref.namespace.is_some()
        } else if module.is_stdlib {
            module_ref.namespace.is_some()
        } else {
            match (&shared_namespace, module_ref.namespace.as_ref()) {
                (Some(shared), Some(ns)) => shared != ns,
                (Some(_), None) => false,
                (None, Some(_)) => true,
                (None, None) => false,
            }
        };

        if wrap_namespace {
            let ns_name = module_ref.namespace.clone().unwrap();
            let namespace_item = Item::Namespace(NamespaceDecl {
                name: ns_name,
                items: module_ref.items.clone(),
                doc: None,
                attributes: Vec::new(),
                span: None,
            });
            combined.items.push(namespace_item);
            item_units.push(index);
            continue;
        }

        for item in &module_ref.items {
            combined.items.push(item.clone());
            item_units.push(index);
        }
    }

    if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
        eprintln!(
            "[chic-debug] assemble_workspace_module modules={} items={} item_units len={}",
            modules.len(),
            combined.items.len(),
            item_units.len()
        );
        for (idx, module) in modules.iter().enumerate() {
            eprintln!(
                "[chic-debug]   module[{idx}] is_stdlib={} requires_codegen={} namespace={:?} items={}",
                module.is_stdlib,
                module.requires_codegen,
                module.parse.module_ref().namespace,
                module.parse.module_ref().items.len()
            );
        }
    }

    combined.rebuild_overloads();
    Ok((combined, item_units))
}

fn log_stdlib_parse_error(path: &Path, source: &str, err: &crate::frontend::parser::ParseError) {
    if std::env::var_os("CHIC_DEBUG_PARSE_ERRORS").is_none() {
        return;
    }
    eprintln!(
        "[chic-debug] parse failure in stdlib file {}: {}",
        path.display(),
        err
    );
    for diagnostic in err.diagnostics() {
        eprintln!("[chic-debug]   {}", diagnostic);
        let span = diagnostic
            .primary_label
            .as_ref()
            .map(|label| label.span)
            .or_else(|| diagnostic.secondary_labels.get(0).map(|label| label.span));
        if let Some(span) = span {
            let start = span.start.saturating_sub(32).min(source.len());
            let end = span.end.min(source.len());
            let snippet = &source[start..end];
            eprintln!(
                "[chic-debug]   snippet @{}..{}: {}",
                span.start,
                span.end,
                snippet.replace('\n', "\\n")
            );
        }
    }
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy() == "char.cl")
    {
        let lex_output = lex(source);
        if !lex_output.diagnostics.is_empty() {
            eprintln!(
                "[chic-debug]   lexer diagnostics: {:?}",
                lex_output.diagnostics
            );
        }
        for diagnostic in err.diagnostics() {
            let span = diagnostic
                .primary_label
                .as_ref()
                .map(|label| label.span)
                .or_else(|| diagnostic.secondary_labels.get(0).map(|label| label.span));
            if let Some(span) = span {
                eprintln!(
                    "[chic-debug]   token dump around parser diagnostic @{}..{}:",
                    span.start, span.end
                );
                for (idx, token) in lex_output.tokens.iter().enumerate() {
                    let overlaps = token.span.start <= span.end && token.span.end >= span.start;
                    if overlaps {
                        eprintln!(
                            "[chic-debug]     #{idx:03} {:?} `{}` @{}..{}",
                            token.kind, token.lexeme, token.span.start, token.span.end
                        );
                    }
                }
            }
        }
        if err
            .diagnostics()
            .iter()
            .all(|diag| diag.primary_label.is_none() && diag.secondary_labels.is_empty())
        {
            eprintln!("[chic-debug]   parser produced no spans; dumping first 32 tokens:");
            for (idx, token) in lex_output.tokens.iter().take(32).enumerate() {
                if matches!(
                    token.kind,
                    TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment
                ) {
                    continue;
                }
                eprintln!(
                    "[chic-debug]     #{idx:03} {:?} `{}` @{}..{}",
                    token.kind, token.lexeme, token.span.start, token.span.end
                );
            }
        }
    }
}

impl FrontendState {
    pub(crate) fn into_report(
        self,
        artifact: Option<PathBuf>,
        library_pack: Option<PathBuf>,
        header: Option<PathBuf>,
        generated: Vec<GeneratedModuleIr>,
        module_outputs: Vec<ModuleArtifact>,
    ) -> FrontendReport {
        let module_count = self.modules.len();
        debug_assert_eq!(module_count, self.unit_functions.len());
        debug_assert_eq!(module_count, module_outputs.len());
        let modules = self
            .modules
            .into_iter()
            .zip(self.unit_functions.into_iter())
            .zip(module_outputs.into_iter())
            .filter_map(|((module, indices), artifact)| {
                if module.is_stdlib {
                    return None;
                }
                let mir = slice_mir_module(&self.mir_module, &indices);
                Some(ModuleReport {
                    input: module.input,
                    parse: module.parse,
                    mir,
                    generated: artifact.textual_ir,
                    object_path: artifact.object_path,
                    metadata_path: artifact.metadata_path,
                    assembly_path: artifact.assembly_path,
                })
            })
            .collect();

        FrontendReport {
            modules,
            files: self.files,
            target: self.target,
            kind: self.kind,
            runtime: self.runtime,
            artifact,
            library_pack,
            header,
            generated,
            mir_module: self.mir_module,
            perf_metadata: self.perf_metadata,
            mir_lowering_diagnostics: self.mir_lowering_diagnostics,
            mir_verification: self.mir_verification,
            reachability_diagnostics: self.reachability_diagnostics,
            borrow_diagnostics: self.borrow_diagnostics,
            fallible_diagnostics: self.fallible_diagnostics,
            type_constraints: self.type_constraints,
            type_diagnostics: self.type_diagnostics,
            lint_diagnostics: self.lint_diagnostics,
            format_diagnostics: Vec::new(),
            doc_diagnostics: self.doc_diagnostics,
            monomorphization: self.monomorphization,
            drop_glue: self.drop_glue,
            clone_glue: self.clone_glue,
            hash_glue: self.hash_glue,
            eq_glue: self.eq_glue,
            type_metadata: self.type_metadata,
            trait_solver_metrics: self.trait_solver_metrics,
        }
    }
}

fn attach_async_metadata(mir_module: &mut MirModule, async_signatures: &[AsyncSignatureInfo]) {
    use crate::mir::Ty;

    if async_signatures.is_empty() {
        return;
    }

    let mut grouped: HashMap<&str, Vec<&AsyncSignatureInfo>> = HashMap::new();
    for info in async_signatures {
        grouped.entry(info.name.as_str()).or_default().push(info);
    }

    for function in &mut mir_module.functions {
        if !function.is_async {
            continue;
        }
        let Some(candidates) = grouped.get(function.name.as_str()) else {
            continue;
        };
        let param_names = function
            .signature
            .params
            .iter()
            .map(|ty| ty.canonical_name())
            .collect::<Vec<_>>();
        let selected = candidates
            .iter()
            .copied()
            .find(|candidate| signature_matches(candidate, &param_names))
            .or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|candidate| candidate.param_types.len() == param_names.len())
            });
        if let Some(info) = selected {
            function.async_result = info.result.as_ref().map(|expr| Ty::from_type_expr(expr));
        }
    }
}

fn signature_matches(candidate: &AsyncSignatureInfo, params: &[String]) -> bool {
    if candidate.param_types.len() != params.len() {
        return false;
    }
    candidate
        .param_types
        .iter()
        .zip(params)
        .all(|(lhs, rhs)| type_names_equivalent(lhs, rhs))
}

fn type_names_equivalent(lhs: &str, rhs: &str) -> bool {
    if lhs == rhs {
        return true;
    }
    let lhs_last = lhs.rsplit(['.', ':']).next().unwrap_or(lhs);
    let rhs_last = rhs.rsplit(['.', ':']).next().unwrap_or(rhs);
    lhs_last == rhs_last
}

pub(crate) struct FrontendModuleState {
    pub(crate) input: PathBuf,
    pub(crate) source: String,
    pub(crate) parse: ParseResult,
    pub(crate) manifest: Option<Manifest>,
    pub(crate) is_stdlib: bool,
    pub(crate) requires_codegen: bool,
}

pub(crate) struct FrontendState {
    pub(crate) modules: Vec<FrontendModuleState>,
    pub(crate) files: FileCache,
    pub(crate) combined_ast: AstModule,
    pub(crate) workspace_source: String,
    pub(crate) target: Target,
    pub(crate) kind: ChicKind,
    pub(crate) runtime: Option<ResolvedRuntime>,
    pub(crate) mir_module: MirModule,
    pub(crate) mir_lowering_diagnostics: Vec<LoweringDiagnostic>,
    pub(crate) mir_verification: Vec<super::MirVerificationIssue>,
    pub(crate) reachability_diagnostics: Vec<Diagnostic>,
    pub(crate) borrow_diagnostics: Vec<Diagnostic>,
    pub(crate) fallible_diagnostics: Vec<Diagnostic>,
    pub(crate) type_constraints: Vec<TypeConstraint>,
    pub(crate) type_diagnostics: Vec<Diagnostic>,
    pub(crate) unit_functions: Vec<Vec<usize>>,
    pub(crate) monomorphization: MonomorphizationSummary,
    pub(crate) drop_glue: Vec<SynthesisedDropGlue>,
    pub(crate) clone_glue: Vec<SynthesisedCloneGlue>,
    pub(crate) hash_glue: Vec<SynthesisedHashGlue>,
    pub(crate) eq_glue: Vec<SynthesisedEqGlue>,
    pub(crate) type_metadata: Vec<SynthesisedTypeMetadata>,
    pub(crate) trait_solver_metrics: TraitSolverMetrics,
    pub(crate) perf_metadata: PerfMetadata,
    pub(crate) lint_diagnostics: Vec<LintDiagnostic>,
    pub(crate) doc_diagnostics: Vec<Diagnostic>,
}

const PKG_FRIEND_SELF_PREFIX: &str = "PKG0001";
const PKG_FRIEND_INVALID_PREFIX: &str = "PKG0002";
const PKG_NAMESPACE_OUT_OF_SCOPE: &str = "PKG0003";
const PKG_FRIEND_DUPLICATE: &str = "PKG0004";
const PKG_PACKAGE_UNKNOWN: &str = "PKG0100";
const PKG_PACKAGE_UNRESOLVED: &str = "PKG0101";
const PKG_PACKAGE_VERSION_MISMATCH: &str = "PKG0102";
const PKG_PACKAGE_STD_MISSING: &str = "PKG0103";
const PKG_PACKAGE_DIRECTIVES_DISALLOWED: &str = "PKG0104";

fn package_error(code: &'static str, message: impl Into<String>, span: Option<Span>) -> Diagnostic {
    let mut message = message.into();
    if !message.starts_with('[') {
        message = format!("[{code}] {message}");
    }
    Diagnostic::error(message, span).with_code(DiagnosticCode::new(
        code.to_string(),
        Some("package".into()),
    ))
}

#[cfg(test)]
mod tests;
