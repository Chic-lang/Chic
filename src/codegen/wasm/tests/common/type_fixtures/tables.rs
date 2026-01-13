use super::shared::simple_tuple_ty;
use crate::mir::{
    Abi, AggregateKind, AutoTraitOverride, AutoTraitSet, EnumLayout, EnumVariantLayout, FnSig,
    FunctionKind, LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand, Rvalue, Statement,
    StatementKind, TupleTy, Ty, TypeLayout, TypeLayoutTable, TypeRepr, UnionFieldLayout,
    UnionFieldMode, UnionLayout,
};
use crate::mir::{BasicBlock, BlockId, Terminator};

pub(crate) fn tuple_layout_table() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let tuple = simple_tuple_ty();
    layouts.ensure_tuple_layout(&tuple);
    layouts
}

fn tuple_locals(count: usize) -> MirBody {
    let tuple = simple_tuple_ty();
    let tuple_ty = Ty::Tuple(tuple.clone());
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    for index in 0..count {
        let kind = if index == 0 {
            LocalKind::Local
        } else {
            LocalKind::Local
        };
        body.locals.push(LocalDecl::new(
            Some(format!("tuple{index}")),
            tuple_ty.clone(),
            true,
            None,
            kind,
        ));
    }
    body
}

pub(crate) fn tuple_aggregate_fixture() -> (TypeLayoutTable, MirFunction) {
    let tuple = simple_tuple_ty();
    let mut layouts = super::super::wasm_layouts();
    layouts.ensure_tuple_layout(&tuple);

    let mut body = tuple_locals(1);
    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(1)),
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: crate::mir::Place::new(LocalId(1)),
            value: Rvalue::Aggregate {
                kind: AggregateKind::Tuple,
                fields: vec![
                    Operand::Const(crate::mir::ConstOperand::new(crate::mir::ConstValue::Int(
                        1,
                    ))),
                    Operand::Const(crate::mir::ConstOperand::new(crate::mir::ConstValue::Int(
                        2,
                    ))),
                ],
            },
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Tuples::Aggregate".into(),
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

pub(crate) fn tuple_copy_fixture() -> (TypeLayoutTable, MirFunction) {
    let tuple = simple_tuple_ty();
    let mut layouts = super::super::wasm_layouts();
    layouts.ensure_tuple_layout(&tuple);
    let tuple_ty = Ty::Tuple(tuple.clone());

    let mut body = tuple_locals(2);
    body.locals[1].ty = tuple_ty.clone();
    body.locals[2].ty = tuple_ty.clone();
    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(1)),
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(2)),
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: crate::mir::Place::new(LocalId(1)),
            value: Rvalue::Aggregate {
                kind: AggregateKind::Tuple,
                fields: vec![
                    Operand::Const(crate::mir::ConstOperand::new(crate::mir::ConstValue::Int(
                        3,
                    ))),
                    Operand::Const(crate::mir::ConstOperand::new(crate::mir::ConstValue::Int(
                        4,
                    ))),
                ],
            },
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: crate::mir::Place::new(LocalId(2)),
            value: Rvalue::Use(Operand::Copy(crate::mir::Place::new(LocalId(1)))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Tuples::Copy".into(),
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

pub(crate) fn tuple_param_fixture() -> (TypeLayoutTable, MirFunction) {
    let tuple = simple_tuple_ty();
    let mut layouts = super::super::wasm_layouts();
    layouts.ensure_tuple_layout(&tuple);
    let tuple_ty = Ty::Tuple(tuple.clone());

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("pair".into()),
        tuple_ty,
        true,
        None,
        LocalKind::Arg(0),
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Tuples::UseParam".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Tuple(simple_tuple_ty())],
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

pub(crate) fn enum_layout_table() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let variant = EnumVariantLayout {
        name: "First".into(),
        index: 0,
        discriminant: 0,
        fields: vec![crate::mir::FieldLayout {
            name: "Value".into(),
            ty: Ty::named("int"),
            index: 0,
            offset: Some(0),
            span: None,
            mmio: None,
            display_name: None,
            is_required: false,
            is_nullable: false,
            is_readonly: false,
            view_of: None,
        }],
        positional: Vec::new(),
    };
    let enum_layout = EnumLayout {
        name: "Demo::Choice".into(),
        repr: TypeRepr::Default,
        packing: None,
        underlying: Ty::named("int"),
        underlying_info: Some(crate::mir::casts::IntInfo {
            bits: 32,
            signed: true,
        }),
        explicit_underlying: false,
        variants: vec![variant],
        size: Some(4),
        align: Some(4),
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        is_flags: false,
    };
    layouts
        .types
        .insert("Demo::Choice".into(), TypeLayout::Enum(enum_layout));

    let flag_variants = vec![
        EnumVariantLayout {
            name: "None".into(),
            index: 0,
            discriminant: 0,
            fields: Vec::new(),
            positional: Vec::new(),
        },
        EnumVariantLayout {
            name: "Read".into(),
            index: 1,
            discriminant: 1,
            fields: Vec::new(),
            positional: Vec::new(),
        },
        EnumVariantLayout {
            name: "Write".into(),
            index: 2,
            discriminant: 2,
            fields: Vec::new(),
            positional: Vec::new(),
        },
        EnumVariantLayout {
            name: "Execute".into(),
            index: 3,
            discriminant: 4,
            fields: Vec::new(),
            positional: Vec::new(),
        },
        EnumVariantLayout {
            name: "All".into(),
            index: 4,
            discriminant: 7,
            fields: Vec::new(),
            positional: Vec::new(),
        },
    ];
    let flag_layout = EnumLayout {
        name: "Flags::Permissions".into(),
        repr: TypeRepr::Default,
        packing: None,
        underlying: Ty::named("int"),
        underlying_info: Some(crate::mir::casts::IntInfo {
            bits: 32,
            signed: true,
        }),
        explicit_underlying: false,
        variants: flag_variants,
        size: Some(4),
        align: Some(4),
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        is_flags: true,
    };
    layouts
        .types
        .insert("Flags::Permissions".into(), TypeLayout::Enum(flag_layout));
    layouts
}

pub(crate) fn union_layout_table() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let union_layout = UnionLayout {
        name: "Demo::UnionValue".into(),
        repr: TypeRepr::Default,
        packing: None,
        views: vec![UnionFieldLayout {
            name: "Data".into(),
            ty: Ty::named("int"),
            index: 0,
            mode: UnionFieldMode::Value,
            span: None,
            is_nullable: false,
        }],
        size: Some(4),
        align: Some(4),
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
    };
    layouts
        .types
        .insert("Demo::UnionValue".into(), TypeLayout::Union(union_layout));
    layouts
}

pub(crate) fn enum_projection_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = enum_layout_table();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("choice".into()),
        Ty::named("Demo::Choice"),
        true,
        None,
        LocalKind::Local,
    ));
    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);
    let function = MirFunction {
        name: "Demo::EnumUse".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
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

pub(crate) fn union_projection_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = union_layout_table();
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("union_value".into()),
        Ty::named("Demo::UnionValue"),
        true,
        None,
        LocalKind::Local,
    ));
    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);
    let function = MirFunction {
        name: "Demo::UnionUse".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_layout_is_registered() {
        let layouts = tuple_layout_table();
        let canonical =
            Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")])).canonical_name();
        assert!(layouts.types.contains_key(&canonical));
    }

    #[test]
    fn enum_layout_flags_marked_as_flags() {
        let layouts = enum_layout_table();
        let layout = layouts
            .types
            .get("Flags::Permissions")
            .expect("flags layout");
        match layout {
            TypeLayout::Enum(flag_layout) => assert!(flag_layout.is_flags),
            other => panic!("expected enum layout, got {other:?}"),
        }
    }

    #[test]
    fn union_layout_has_value_view() {
        let layouts = union_layout_table();
        let layout = layouts.types.get("Demo::UnionValue").expect("union layout");
        match layout {
            TypeLayout::Union(union_layout) => {
                assert_eq!(union_layout.views.len(), 1);
                assert_eq!(union_layout.views[0].name, "Data");
            }
            other => panic!("expected union layout, got {other:?}"),
        }
    }

    #[test]
    fn tuple_fixtures_build_expected_signatures() {
        let (_, aggregate_fn) = tuple_aggregate_fixture();
        assert_eq!(aggregate_fn.signature.ret, Ty::Unit);

        let (_, copy_fn) = tuple_copy_fixture();
        assert_eq!(copy_fn.body.locals.len(), 3);

        let (_, param_fn) = tuple_param_fixture();
        assert_eq!(param_fn.signature.params.len(), 1);
    }

    #[test]
    fn enum_and_union_projection_fixtures_return_int() {
        let (_, enum_fn) = enum_projection_fixture();
        assert_eq!(enum_fn.signature.ret, Ty::named("int"));

        let (_, union_fn) = union_projection_fixture();
        assert_eq!(union_fn.signature.ret, Ty::named("int"));
    }
}
