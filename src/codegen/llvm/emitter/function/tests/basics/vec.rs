use super::super::helpers::{ensure_array_layout, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::codegen::llvm::emitter::literals::StrLiteralInfo;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule,
    Place, ProjectionElem, StrId, Ty, TypeLayoutTable,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn llvm_place_is_vec_handles_array_places() {
    let mut layouts = TypeLayoutTable::default();
    let element = Ty::named("int");
    let array_ty = ensure_array_layout(&mut layouts, element);

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("array".into()),
        array_ty.clone(),
        true,
        None,
        LocalKind::Arg(0),
    ));

    let mut module = MirModule::default();
    module.type_layouts = layouts;
    module.type_layouts.finalize_auto_traits();
    let function = MirFunction {
        name: "Demo::UseArray".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![array_ty],
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
    module.functions.push(function);

    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut externals = BTreeSet::new();
    let str_literals: HashMap<StrId, StrLiteralInfo> = HashMap::new();
    let symbol_set = HashSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut emitter = FunctionEmitter::new(
        &module.functions[0],
        &signatures,
        &mut externals,
        &symbol_set,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &str_literals,
        &module.type_layouts,
        &mut metadata,
        None,
    );
    let types = emitter.compute_local_types().expect("compute locals");
    emitter.set_local_types_for_tests(types);

    let place = Place::new(LocalId(1));
    assert!(
        emitter.place_is_vec(&place).expect("evaluate place"),
        "arrays should be treated as vec-like by LLVM emitter"
    );
}

#[test]
fn llvm_place_ptr_supports_string_index_projection() {
    let mut layouts = TypeLayoutTable::default();
    layouts.finalize_auto_traits();

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("char"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("text".into()),
        Ty::String,
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("index".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut module = MirModule::default();
    module.type_layouts = layouts;
    module.type_layouts.finalize_auto_traits();
    let function = MirFunction {
        name: "Demo::UseString".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::String],
            ret: Ty::named("char"),
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
    module.functions.push(function);
    let local_count = module.functions[0].body.locals.len();

    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut externals = BTreeSet::new();
    let str_literals: HashMap<StrId, StrLiteralInfo> = HashMap::new();
    let symbol_set = HashSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut emitter = FunctionEmitter::new(
        &module.functions[0],
        &signatures,
        &mut externals,
        &symbol_set,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &str_literals,
        &module.type_layouts,
        &mut metadata,
        None,
    );
    let types = emitter.compute_local_types().expect("compute locals");
    emitter.set_local_types_for_tests(types);
    emitter.local_ptrs.resize(local_count, None);
    emitter.local_ptrs[1] = Some("%text_ptr".into());
    emitter.local_tys[1] = Some("ptr".into());
    emitter.local_ptrs[2] = Some("%idx_ptr".into());
    emitter.local_tys[2] = Some("i32".into());

    let mut place = Place::new(LocalId(1));
    place.projection.push(ProjectionElem::Index(LocalId(2)));
    emitter
        .place_ptr(&place)
        .expect("string index projections should inline pointer math");
}

#[test]
fn llvm_place_ptr_supports_str_index_projection() {
    let mut layouts = TypeLayoutTable::default();
    layouts.finalize_auto_traits();

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("char"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("text".into()),
        Ty::Str,
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("index".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut module = MirModule::default();
    module.type_layouts = layouts;
    module.type_layouts.finalize_auto_traits();
    let function = MirFunction {
        name: "Demo::UseStr".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Str],
            ret: Ty::named("char"),
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
    module.functions.push(function);
    let local_count = module.functions[0].body.locals.len();

    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let mut externals = BTreeSet::new();
    let str_literals: HashMap<StrId, StrLiteralInfo> = HashMap::new();
    let symbol_set = HashSet::new();
    let mut metadata = MetadataRegistry::new();
    let mut emitter = FunctionEmitter::new(
        &module.functions[0],
        &signatures,
        &mut externals,
        &symbol_set,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &str_literals,
        &module.type_layouts,
        &mut metadata,
        None,
    );
    let types = emitter.compute_local_types().expect("compute locals");
    emitter.set_local_types_for_tests(types);
    emitter.local_ptrs.resize(local_count, None);
    emitter.local_ptrs[1] = Some("%str_ptr".into());
    emitter.local_tys[1] = Some("ptr".into());
    emitter.local_ptrs[2] = Some("%idx_ptr".into());
    emitter.local_tys[2] = Some("i32".into());

    let mut place = Place::new(LocalId(1));
    place.projection.push(ProjectionElem::Index(LocalId(2)));
    emitter
        .place_ptr(&place)
        .expect("str index projections should inline pointer math");
}
