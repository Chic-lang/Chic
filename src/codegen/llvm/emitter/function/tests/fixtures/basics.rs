use crate::mir::{
    Abi, BasicBlock, BinOp, BlockId, ClassLayoutInfo, ClassLayoutKind, ConstOperand, ConstValue,
    FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand,
    Place, PointerTy, Rvalue, Statement, StatementKind, Terminator, Ty, TypeLayout,
    TypeLayoutTable, TypeRepr,
};
use crate::mir::{AutoTraitOverride, AutoTraitSet, StructLayout};

use super::super::helpers::flag_layouts;

fn push_demo_disposable_dispose(module: &mut MirModule) {
    if module
        .functions
        .iter()
        .any(|func| func.name == "Demo::Disposable::dispose")
    {
        return;
    }

    let mut dispose_body = MirBody::new(0, None);
    dispose_body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    dispose_body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        Ty::Pointer(Box::new(PointerTy::new(
            Ty::named("Demo::Disposable"),
            true,
        ))),
        false,
        None,
        LocalKind::Arg(0),
    ));
    let mut dispose_entry = BasicBlock::new(BlockId(0), None);
    dispose_entry.terminator = Some(Terminator::Return);
    dispose_body.blocks.push(dispose_entry);
    module.functions.push(MirFunction {
        name: "Demo::Disposable::dispose".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Demo::Disposable"),
                true,
            )))],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: dispose_body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    });

    if module
        .functions
        .iter()
        .any(|func| func.name == "__cl_drop__Demo__Disposable")
    {
        return;
    }

    let mut drop_body = MirBody::new(0, None);
    drop_body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    drop_body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        Ty::Pointer(Box::new(PointerTy::new(Ty::named("byte"), true))),
        false,
        None,
        LocalKind::Arg(0),
    ));
    let mut drop_block = BasicBlock::new(BlockId(0), None);
    drop_block.terminator = Some(Terminator::Return);
    drop_body.blocks.push(drop_block);
    module.functions.push(MirFunction {
        name: "__cl_drop__Demo__Disposable".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("byte"),
                true,
            )))],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body: drop_body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    });
}

pub(crate) fn flag_enum_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("Flags::Permissions"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("Flags::Permissions"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: None,
        span: None,
    };
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Binary {
                op: BinOp::BitOr,
                lhs: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                rhs: Operand::Const(ConstOperand::new(ConstValue::Int(2))),
                rounding: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Binary {
                op: BinOp::BitAnd,
                lhs: Operand::Copy(Place::new(LocalId(1))),
                rhs: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                rounding: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(1)))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Flags::Combine".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("Flags::Permissions"),
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

    push_demo_disposable_dispose(&mut module);
    module.type_layouts = flag_layouts();
    module
}

pub(crate) fn drop_with_deinit_module() -> MirModule {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("resource".into()),
        Ty::named("Demo::Disposable"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(1)),
    });
    let drop_place = Place::new(LocalId(1));
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Deinit(drop_place.clone()),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Drop {
            place: drop_place,
            target: BlockId(0),
            unwind: None,
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(LocalId(1)),
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let function = MirFunction {
        name: "Demo::Dropper".into(),
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

    let mut module = MirModule::default();
    module.functions.push(function);
    push_demo_disposable_dispose(&mut module);

    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Demo::Disposable".into(),
        TypeLayout::Class(StructLayout {
            name: "Demo::Disposable".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
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
            dispose: Some("Demo::Disposable::dispose".into()),
            class: Some(ClassLayoutInfo {
                kind: ClassLayoutKind::Class,
                bases: Vec::new(),
                vtable_offset: Some(0),
            }),
        }),
    );
    module.type_layouts = layouts;
    module
}
