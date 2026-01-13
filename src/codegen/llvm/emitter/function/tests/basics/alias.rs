use super::super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AliasContract, BasicBlock, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, MirModule, Operand, ParamMode, Place, ProjectionElem, Rvalue, Statement,
    StatementKind, Ty,
};
use crate::mir::{PointerQualifiers, PointerTy};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn restrict_parameters_emit_alias_metadata() {
    let module = alias_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name == "Demo::PtrOps::CopyOne")
        .expect("alias function");
    let sig = signatures
        .get(&function.name)
        .expect("alias function signature");
    let mut externals = BTreeSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut out = String::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut externals,
        &HashSet::new(),
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit alias function");
    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("!alias.scope"),
        "pointer dereference should include alias scope metadata: {body}"
    );
    assert!(
        body.contains("!noalias"),
        "restrict pointers should be marked mutually noalias: {body}"
    );
}

fn alias_module() -> MirModule {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let pointer_ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
        Ty::named("int"),
        true,
        PointerQualifiers {
            restrict: true,
            noalias: false,
            readonly: false,
            expose_address: false,
            alignment: None,
        },
    )));
    let mut dest = LocalDecl::new(
        Some("dest".into()),
        pointer_ty.clone(),
        true,
        None,
        LocalKind::Arg(0),
    )
    .with_param_mode(ParamMode::Value)
    .with_alias_contract(AliasContract {
        restrict: true,
        noalias: true,
        ..AliasContract::default()
    });
    dest.mutable = true;
    body.locals.push(dest);
    let mut src = LocalDecl::new(
        Some("src".into()),
        pointer_ty.clone(),
        false,
        None,
        LocalKind::Arg(1),
    )
    .with_param_mode(ParamMode::Value)
    .with_alias_contract(AliasContract {
        restrict: true,
        noalias: true,
        ..AliasContract::default()
    });
    src.mutable = false;
    body.locals.push(src);
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));

    let mut block = BasicBlock::new(body.entry(), None);
    let mut load_place = Place::new(LocalId(2));
    load_place.projection.push(ProjectionElem::Deref);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(load_place.clone())),
        },
    });
    let mut store_place = Place::new(LocalId(1));
    store_place.projection.push(ProjectionElem::Deref);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: store_place,
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });
    block.terminator = Some(crate::mir::Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::PtrOps::CopyOne".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![pointer_ty.clone(), pointer_ty],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
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

    let mut module = MirModule::default();
    module.functions.push(function);
    module
}
