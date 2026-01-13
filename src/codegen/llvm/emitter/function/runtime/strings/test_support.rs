#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::codegen::isa::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::LlvmFunctionSignature;
use crate::mir::{
    ClassVTable, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction, StaticVar, StrId,
    TraitVTable, Ty, TypeLayoutTable,
};
use crate::target::TargetArch;

/// Build a minimal FunctionEmitter for string runtime tests.
pub(crate) fn with_emitter<F, R>(
    locals: Vec<Ty>,
    str_literals: impl IntoIterator<Item = (StrId, StrLiteralInfo)>,
    mut f: F,
) -> (R, BTreeSet<&'static str>)
where
    F: FnMut(&mut FunctionEmitter<'_>, &TypeLayoutTable) -> R,
{
    let mut body = MirBody::new(0, None);
    for ty in locals {
        body.locals
            .push(LocalDecl::new(None, ty, false, None, LocalKind::Local));
    }

    let function = MirFunction {
        name: "strings".to_string(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };
    let mut externals: BTreeSet<&'static str> = BTreeSet::new();
    let mut metadata = MetadataRegistry::new();
    let signatures: HashMap<String, LlvmFunctionSignature> = HashMap::new();
    let vtable_symbols: HashSet<String> = HashSet::new();
    let trait_vtables: Vec<TraitVTable> = Vec::new();
    let class_vtables: Vec<ClassVTable> = Vec::new();
    let statics: Vec<StaticVar> = Vec::new();
    let str_literals: HashMap<StrId, StrLiteralInfo> = str_literals.into_iter().collect();
    let type_layouts = TypeLayoutTable::default();
    let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");

    let mut emitter = FunctionEmitter::new(
        &function,
        &signatures,
        &mut externals,
        &vtable_symbols,
        &trait_vtables,
        &class_vtables,
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        TargetArch::Aarch64,
        &target,
        &statics,
        &str_literals,
        &type_layouts,
        &mut metadata,
        None,
    );

    if !function.body.locals.is_empty() {
        let local_tys: Vec<Option<String>> = function
            .body
            .locals
            .iter()
            .map(|local| {
                crate::codegen::llvm::types::map_type_owned(&local.ty, Some(&type_layouts))
                    .ok()
                    .flatten()
            })
            .collect();
        emitter.set_local_types_for_tests(local_tys);
        emitter.local_ptrs = (0..function.body.locals.len())
            .map(|index| Some(format!("%loc{index}")))
            .collect();
    }

    let result = f(&mut emitter, &type_layouts);
    (result, externals)
}
