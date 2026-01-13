use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, CallDispatch, ConstOperand,
    ConstValue, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction,
    MirModule, Operand, ParamMode, Place, StructLayout, Terminator, TraitObjectDispatch,
    TraitObjectTy, TraitVTable, Ty, TypeLayout, TypeRepr, VTableSlot,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn emit_call_handles_trait_object_dispatch() {
    let module = trait_dispatch_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("llvm signatures");
    let vtable_symbols: HashSet<String> = module
        .trait_vtables
        .iter()
        .map(|table| table.symbol.clone())
        .collect();
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Render"))
        .expect("render function");
    let sig = signatures.get(&function.name).expect("render signature");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &vtable_symbols,
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
    .expect("emit render");
    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("getelementptr inbounds [1 x ptr]"),
        "expected vtable slot load in dyn call IR: {body}"
    );
    assert!(
        body.contains("call void"),
        "expected indirect call for trait dispatch: {body}"
    );
}

#[test]
fn trait_dispatch_missing_metadata_surfaces_error() {
    let mut module = trait_dispatch_module();
    module.trait_vtables.clear();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("llvm signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::Render"))
        .expect("render function");
    let sig = signatures.get(&function.name).expect("render signature");
    let mut metadata = MetadataRegistry::new();
    let err = emit_function(
        &mut String::new(),
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
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
    .expect_err("missing vtable metadata should fail trait dispatch");
    assert!(
        format!("{err}").contains("does not have vtable metadata"),
        "expected trait dispatch vtable metadata error, got {err:?}"
    );
}

fn trait_dispatch_module() -> MirModule {
    let mut module = MirModule::default();
    register_class_layout(&mut module, "Demo::Widget");
    module.trait_vtables.push(TraitVTable {
        symbol: "__vtable_Demo__Formatter__Demo__Widget".into(),
        trait_name: "Demo::Formatter".into(),
        impl_type: "Demo::Widget".into(),
        slots: vec![VTableSlot {
            method: "Format".into(),
            symbol: "Demo::Widget::Formatter::Format".into(),
        }],
    });
    module.functions.push(trait_impl_function());
    module.functions.push(render_function());
    module
}

fn register_class_layout(module: &mut MirModule, name: &str) {
    module.type_layouts.types.insert(
        name.to_string(),
        TypeLayout::Class(StructLayout {
            name: name.to_string(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(0),
            align: Some(1),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

fn trait_impl_function() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Widget::Formatter::Format".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("Demo::Widget")],
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
    }
}

fn render_function() -> MirFunction {
    let trait_ty = Ty::TraitObject(TraitObjectTy::new(vec!["Demo::Formatter".into()]));
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("fmt".into()),
        trait_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Unit)),
            args: vec![Operand::Copy(Place::new(LocalId(1)))],
            arg_modes: vec![ParamMode::Value],
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Trait(TraitObjectDispatch {
                trait_name: "Demo::Formatter".into(),
                method: "Format".into(),
                slot_index: 0,
                slot_count: 1,
                receiver_index: 0,
                impl_type: None,
            })),
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Render".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![trait_ty],
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
    }
}
