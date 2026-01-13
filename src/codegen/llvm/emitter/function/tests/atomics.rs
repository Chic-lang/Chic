use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AtomicFenceScope, AtomicOrdering, AtomicRmwOp, BasicBlock, ConstOperand, ConstValue,
    FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand,
    Place, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn atomic_operations_lower_to_llvm_ir() {
    let module = atomic_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("llvm signatures");
    let function = &module.functions[0];
    let sig = signatures
        .get(&function.name)
        .unwrap_or_else(|| panic!("missing signature for {}", function.name));
    let mut externals = BTreeSet::new();
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
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
    .expect("emit atomic function");

    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("store atomic i32"),
        "expected atomic store lowering: {body}"
    );
    assert!(
        body.contains("load atomic i32"),
        "expected atomic load lowering: {body}"
    );
    assert!(
        body.contains("atomicrmw add"),
        "expected atomicrmw lowering: {body}"
    );
    assert!(
        body.contains("cmpxchg ptr"),
        "expected cmpxchg lowering: {body}"
    );
    assert!(
        body.contains("fence seq_cst"),
        "expected fence lowering: {body}"
    );
}

fn atomic_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("cell".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("loaded".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("rmw".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("cas".into()),
        Ty::named("bool"),
        false,
        None,
        LocalKind::Temp,
    ));

    let mut block = BasicBlock::new(body.entry(), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::AtomicStore {
            target: Place::new(LocalId(1)),
            value: Operand::Const(ConstOperand::new(ConstValue::Int(42))),
            order: AtomicOrdering::SeqCst,
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(2)),
            value: Rvalue::AtomicLoad {
                target: Place::new(LocalId(1)),
                order: AtomicOrdering::Acquire,
            },
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::AtomicRmw {
                op: AtomicRmwOp::Add,
                target: Place::new(LocalId(1)),
                value: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                order: AtomicOrdering::AcqRel,
            },
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(4)),
            value: Rvalue::AtomicCompareExchange {
                target: Place::new(LocalId(1)),
                expected: Operand::Const(ConstOperand::new(ConstValue::Int(42))),
                desired: Operand::Const(ConstOperand::new(ConstValue::Int(7))),
                success: AtomicOrdering::SeqCst,
                failure: AtomicOrdering::Acquire,
                weak: false,
            },
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::AtomicFence {
            order: AtomicOrdering::SeqCst,
            scope: AtomicFenceScope::Full,
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::Atomics::demo".into(),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    module
}
