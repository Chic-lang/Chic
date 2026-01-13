#![cfg(test)]

use super::common::contains_bytes;
use super::helpers::emit_body_using_layouts;
use crate::mir::{
    Abi, AtomicFenceScope, AtomicOrdering, AtomicRmwOp, BasicBlock, ConstOperand, ConstValue,
    FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand, Place,
    Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr,
    make_field,
};

#[test]
fn atomic_instructions_lower_to_wasm_bytes() {
    let (layouts, function) = atomic_function_fixture();
    let body = emit_body_using_layouts(layouts, function, |_| None);

    assert!(
        contains_bytes(&body, &[0xFE, 0x17, 0x00, 0x00]),
        "expected i32.atomic.store opcode in body"
    );
    assert!(
        contains_bytes(&body, &[0xFE, 0x10, 0x00, 0x00]),
        "expected i32.atomic.load opcode in body"
    );
    assert!(
        contains_bytes(&body, &[0xFE, 0x1E, 0x00, 0x00]),
        "expected i32.atomic.rmw.add opcode in body"
    );
    assert!(
        contains_bytes(&body, &[0xFE, 0x48, 0x00, 0x00]),
        "expected i32.atomic.rmw.cmpxchg opcode in body"
    );
    assert!(
        contains_bytes(&body, &[0xFE, 0x03, 0x00]),
        "expected atomic.fence opcode in body"
    );
}

fn atomic_function_fixture() -> (TypeLayoutTable, MirFunction) {
    let atom_type = "Std::Sync::AtomicInt";
    let mut layouts = super::common::wasm_layouts();
    layouts.types.insert(
        atom_type.into(),
        TypeLayout::Struct(StructLayout {
            name: atom_type.into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![make_field("value", Ty::named("int"), 0, 0)],
            positional: Vec::new(),
            list: None,
            size: Some(4),
            align: Some(4),
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
        Ty::named(atom_type),
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
            value: Operand::Const(ConstOperand::new(ConstValue::Int(7))),
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
                expected: Operand::Const(ConstOperand::new(ConstValue::Int(8))),
                desired: Operand::Const(ConstOperand::new(ConstValue::Int(9))),
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
        name: "Demo::Atomics::wasm".into(),
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

    (layouts, function)
}
