use super::super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy,
    Statement, StatementKind, StructLayout, Terminator, Ty, TypeLayout, TypeLayoutTable, TypeRepr,
};
use std::collections::{BTreeSet, HashMap, HashSet};

fn zero_init_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("slot".into()),
        Ty::named("Sample::Holder"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::ZeroInit {
            place: Place::new(LocalId(1)),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let mut module = MirModule::default();
    module.type_layouts = holder_layouts();
    let function = MirFunction {
        name: "Demo::ZeroInit".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
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
    module
}

fn zero_init_raw_module() -> MirModule {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    let pointer_ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true)));
    body.locals.push(
        LocalDecl::new(
            Some("ptr".into()),
            pointer_ty,
            true,
            None,
            LocalKind::Arg(0),
        )
        .with_param_mode(ParamMode::Value),
    );
    body.locals.push(
        LocalDecl::new(
            Some("len".into()),
            Ty::named("usize"),
            false,
            None,
            LocalKind::Arg(1),
        )
        .with_param_mode(ParamMode::Value),
    );

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::ZeroInitRaw {
            pointer: Operand::Copy(Place::new(LocalId(1))),
            length: Operand::Copy(Place::new(LocalId(2))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let mut module = MirModule::default();
    module.type_layouts = holder_layouts();
    let function = MirFunction {
        name: "Demo::ZeroInitRaw".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![
                Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
                Ty::named("usize"),
            ],
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
    module
}

fn holder_layouts() -> TypeLayoutTable {
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Sample::Holder".into(),
        TypeLayout::Struct(StructLayout {
            name: "Sample::Holder".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(16),
            align: Some(8),
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
    layouts
}

fn emit_function_ir(module: &MirModule, index: usize) -> String {
    let target = test_target();
    let signatures = build_signatures(module, None, &target).expect("signatures");
    let function = &module.functions[index];
    let sig = signatures
        .get(&function.name)
        .unwrap_or_else(|| panic!("missing signature for {}", function.name));
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
    .expect("emit function");
    function_ir(&out, &sig.symbol).to_string()
}

#[test]
fn zero_init_emits_memset_call() {
    let module = zero_init_module();
    let body = emit_function_ir(&module, 0);
    assert!(
        body.contains("@llvm.memset.p0.i64"),
        "expected ZeroInit to lower to llvm.memset: {body}"
    );
}

#[test]
fn zero_init_raw_calls_runtime_when_length_dynamic() {
    let module = zero_init_raw_module();
    let body = emit_function_ir(&module, 0);
    assert!(
        body.contains("@chic_rt_zero_init"),
        "ZeroInitRaw should call the runtime helper when the length is dynamic: {body}"
    );
}

#[test]
fn zero_init_runtime_helper_is_declared_for_cpu_dispatch() {
    use crate::codegen::llvm::emitter::dispatch::emit_external_declarations;

    let mut externals = BTreeSet::new();
    externals.insert("chic_rt_zero_init");
    let mut out = String::new();
    emit_external_declarations(&mut out, &externals);
    assert!(
        out.contains("declare void @chic_rt_zero_init"),
        "expected cpu helper declarations to include zero-init runtime helper: {out}"
    );
}
