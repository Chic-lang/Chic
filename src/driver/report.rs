use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::chic_kind::ChicKind;
use crate::clone_glue::SynthesisedCloneGlue;
use crate::codegen;
use crate::codegen::Backend;
use crate::drop_glue::SynthesisedDropGlue;
use crate::eq_glue::SynthesisedEqGlue;
use crate::frontend::diagnostics::{Diagnostic, FileCache};
use crate::frontend::parser::ParseResult;
use crate::hash_glue::SynthesisedHashGlue;
use crate::lint::LintDiagnostic;
use crate::mir::{Abi, LoweringDiagnostic, MirModule, TestCaseMetadata, VerifyError};
use crate::monomorphize::MonomorphizationSummary;
use crate::perf::PerfMetadata;
use crate::runtime_package::ResolvedRuntime;
use crate::target::Target;
use crate::type_metadata::SynthesisedTypeMetadata;
use crate::typeck::{TraitSolverMetrics, TypeConstraint};

#[derive(Clone, Default)]
pub(crate) struct ModuleArtifact {
    pub(crate) object_path: Option<PathBuf>,
    pub(crate) metadata_path: Option<PathBuf>,
    pub(crate) textual_ir: Option<String>,
    pub(crate) assembly_path: Option<PathBuf>,
}

pub struct MirVerificationIssue {
    pub function: String,
    pub errors: Vec<VerifyError>,
}

pub struct GeneratedModuleIr {
    pub input: PathBuf,
    pub textual: String,
}

pub struct ModuleReport {
    pub input: PathBuf,
    pub parse: ParseResult,
    pub mir: MirModule,
    pub generated: Option<String>,
    pub object_path: Option<PathBuf>,
    pub metadata_path: Option<PathBuf>,
    pub assembly_path: Option<PathBuf>,
}

pub struct FrontendReport {
    pub modules: Vec<ModuleReport>,
    pub files: FileCache,
    pub target: Target,
    pub kind: ChicKind,
    pub runtime: Option<ResolvedRuntime>,
    pub artifact: Option<PathBuf>,
    pub library_pack: Option<PathBuf>,
    pub header: Option<PathBuf>,
    pub generated: Vec<GeneratedModuleIr>,
    pub mir_module: MirModule,
    pub perf_metadata: PerfMetadata,
    pub mir_lowering_diagnostics: Vec<LoweringDiagnostic>,
    pub mir_verification: Vec<MirVerificationIssue>,
    pub reachability_diagnostics: Vec<Diagnostic>,
    pub borrow_diagnostics: Vec<Diagnostic>,
    pub fallible_diagnostics: Vec<Diagnostic>,
    pub type_constraints: Vec<TypeConstraint>,
    pub type_diagnostics: Vec<Diagnostic>,
    pub lint_diagnostics: Vec<LintDiagnostic>,
    pub format_diagnostics: Vec<Diagnostic>,
    pub doc_diagnostics: Vec<Diagnostic>,
    pub monomorphization: MonomorphizationSummary,
    pub drop_glue: Vec<SynthesisedDropGlue>,
    pub clone_glue: Vec<SynthesisedCloneGlue>,
    pub hash_glue: Vec<SynthesisedHashGlue>,
    pub eq_glue: Vec<SynthesisedEqGlue>,
    pub type_metadata: Vec<SynthesisedTypeMetadata>,
    pub trait_solver_metrics: TraitSolverMetrics,
}

impl FrontendReport {
    #[must_use]
    pub fn has_diagnostics(&self) -> bool {
        self.modules
            .iter()
            .any(|module| !module.parse.diagnostics.is_empty())
            || !self.mir_lowering_diagnostics.is_empty()
            || !self.mir_verification.is_empty()
            || !self.reachability_diagnostics.is_empty()
            || !self.borrow_diagnostics.is_empty()
            || !self.fallible_diagnostics.is_empty()
            || !self.type_diagnostics.is_empty()
            || !self.lint_diagnostics.is_empty()
            || !self.format_diagnostics.is_empty()
            || !self.doc_diagnostics.is_empty()
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        let parse_errors = self.modules.iter().any(|module| {
            module
                .parse
                .diagnostics
                .iter()
                .any(|d| d.severity.is_error())
        });
        let lowering_errors = self
            .mir_lowering_diagnostics
            .iter()
            .any(|diagnostic| !diagnostic.message.starts_with("warning:"));
        let verification_errors = !self.mir_verification.is_empty();
        let reachability_errors = self
            .reachability_diagnostics
            .iter()
            .any(|d| d.severity.is_error());
        let borrow_errors = self
            .borrow_diagnostics
            .iter()
            .any(|d| d.severity.is_error());
        let fallible_errors = self
            .fallible_diagnostics
            .iter()
            .any(|d| d.severity.is_error());
        let type_errors = self.type_diagnostics.iter().any(|d| d.severity.is_error());
        let lint_errors = self
            .lint_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == crate::lint::LintLevel::Error);
        let format_errors = self
            .format_diagnostics
            .iter()
            .any(|d| d.severity.is_error());
        let doc_errors = self.doc_diagnostics.iter().any(|d| d.severity.is_error());

        parse_errors
            || lowering_errors
            || verification_errors
            || reachability_errors
            || borrow_errors
            || fallible_errors
            || type_errors
            || lint_errors
            || format_errors
            || doc_errors
    }

    #[must_use]
    pub fn has_doc_errors(&self) -> bool {
        self.doc_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_error())
    }
}

pub struct MirDumpResult {
    pub report: FrontendReport,
    pub rendered: String,
}

pub(crate) fn default_artifact_path(
    source: &Path,
    target: &Target,
    kind: ChicKind,
    backend: Backend,
) -> PathBuf {
    let extension = match backend {
        Backend::Wasm => "wasm".to_string(),
        Backend::Llvm => match kind {
            ChicKind::Executable => ChicKind::Executable.default_extension().to_string(),
            ChicKind::StaticLibrary => static_library_extension(target),
            ChicKind::DynamicLibrary => dynamic_library_extension(target),
        },
        Backend::Cc1 => "s".to_string(),
    };
    source.with_extension(extension)
}

pub(crate) fn static_library_extension(target: &Target) -> String {
    let triple = codegen::canonical_toolchain_triple(target);
    if triple.contains("windows") {
        "lib".into()
    } else {
        "a".into()
    }
}

pub(crate) fn dynamic_library_extension(target: &Target) -> String {
    let triple = codegen::canonical_toolchain_triple(target);
    if triple.contains("apple") {
        "dylib".into()
    } else if triple.contains("windows") {
        "dll".into()
    } else {
        "so".into()
    }
}

pub(crate) fn slice_mir_module(base: &MirModule, indices: &[usize]) -> MirModule {
    let mut functions = Vec::with_capacity(indices.len());
    let mut index_map = HashMap::new();
    for &index in indices {
        if let Some(function) = base.functions.get(index) {
            if function.extern_spec.is_some() {
                continue;
            }
            if matches!(function.signature.abi, Abi::Extern(_)) && function.body.blocks.is_empty() {
                continue;
            }
            index_map.insert(index, functions.len());
            functions.push(function.clone());
        }
    }
    let retained: HashSet<_> = functions.iter().map(|func| func.name.clone()).collect();
    let exports = base
        .exports
        .iter()
        .filter(|export| retained.contains(&export.function))
        .cloned()
        .collect();
    let test_cases = base
        .test_cases
        .iter()
        .filter_map(|meta| {
            index_map
                .get(&meta.function_index)
                .map(|new_index| TestCaseMetadata {
                    function_index: *new_index,
                    id: meta.id.clone(),
                    qualified_name: meta.qualified_name.clone(),
                    name: meta.name.clone(),
                    namespace: meta.namespace.clone(),
                    categories: meta.categories.clone(),
                    parameters: meta.parameters.clone(),
                    is_async: meta.is_async,
                    span: meta.span,
                })
        })
        .collect();
    MirModule {
        functions,
        test_cases,
        statics: base.statics.clone(),
        type_layouts: base.type_layouts.clone(),
        primitive_registry: base.primitive_registry.clone(),
        interned_strs: base.interned_strs.clone(),
        exports,
        attributes: base.attributes.clone(),
        trait_vtables: base.trait_vtables.clone(),
        class_vtables: base.class_vtables.clone(),
        interface_defaults: base.interface_defaults.clone(),
        default_arguments: base.default_arguments.clone(),
        type_variance: base.type_variance.clone(),
        async_plans: base
            .async_plans
            .iter()
            .filter_map(|plan| {
                index_map
                    .get(&plan.function_index)
                    .map(|new_index| AsyncLoweringArtifact {
                        function_index: *new_index,
                        function_name: plan.function_name.clone(),
                        frame_type: plan.frame_type.clone(),
                        context_local: plan.context_local,
                        policy: plan.policy.clone(),
                        metrics: plan.metrics.clone(),
                        frame_fields: plan.frame_fields.clone(),
                        suspend_points: plan.suspend_points.clone(),
                        resume_states: plan.resume_states.clone(),
                        state_count: plan.state_count,
                        poll_fn: plan.poll_fn.clone(),
                        drop_fn: plan.drop_fn.clone(),
                        implicit_promotion: plan.implicit_promotion,
                        captured_arguments: plan.captured_arguments.clone(),
                        captured_locals: plan.captured_locals.clone(),
                    })
            })
            .collect(),
    }
}

pub(crate) fn unit_object_path(artifact: &Path, input: &Path, index: usize) -> PathBuf {
    let base_name = artifact
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let input_stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module");
    let file_name = format!("{base_name}.{input_stem}.module{index}.o");
    artifact
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub(crate) fn archive_entry_name(prefix: &str, path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    format!("{prefix}/{name}")
}
use std::collections::HashMap;

use crate::mir::AsyncLoweringArtifact;
