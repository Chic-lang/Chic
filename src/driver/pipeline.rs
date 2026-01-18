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
mod execute;
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
