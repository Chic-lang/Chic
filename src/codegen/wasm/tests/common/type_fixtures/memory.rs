use super::shared::register_array_layout;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, Operand, Place, ProjectionElem, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, TypeLayout, TypeLayoutTable,
    TypeRepr,
};

pub(crate) fn array_index_fixture() -> (TypeLayoutTable, MirFunction) {
    let mut layouts = super::super::wasm_layouts();
    let element = Ty::named("int");
    let array_ty = register_array_layout(&mut layouts, element.clone());

    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        element.clone(),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("data".into()),
        array_ty.clone(),
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("index".into()),
        Ty::named("usize"),
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        element.clone(),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    let mut element_place = Place::new(LocalId(1));
    element_place
        .projection
        .push(ProjectionElem::Index(LocalId(2)));
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(element_place)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::ArrayIndex".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![array_ty.clone(), Ty::named("usize")],
            ret: element,
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

pub(crate) fn string_index_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = super::super::wasm_layouts();
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
        Ty::named("usize"),
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("char"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    let mut char_place = Place::new(LocalId(1));
    char_place
        .projection
        .push(ProjectionElem::Index(LocalId(2)));
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(char_place)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::StringIndex".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::String, Ty::named("usize")],
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

    (layouts, function)
}

pub(crate) fn str_index_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = super::super::wasm_layouts();
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
        Ty::named("usize"),
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("char"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    let mut char_place = Place::new(LocalId(1));
    char_place
        .projection
        .push(ProjectionElem::Index(LocalId(2)));
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(char_place)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::StrIndex".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Str, Ty::named("usize")],
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

    (layouts, function)
}

pub(crate) fn struct_with_missing_offsets_fixture() -> (TypeLayoutTable, MirFunction) {
    let mut layouts = super::super::wasm_layouts();
    let struct_layout = StructLayout {
        name: "Demo::MissingOffsets".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![FieldLayout {
            name: "Field".into(),
            ty: Ty::named("int"),
            index: 0,
            offset: None,
            span: None,
            mmio: None,
            display_name: None,
            is_required: false,
            is_nullable: false,
            is_readonly: false,
            view_of: None,
        }],
        positional: Vec::new(),
        list: None,
        size: Some(4),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts.types.insert(
        "Demo::MissingOffsets".into(),
        TypeLayout::Struct(struct_layout),
    );

    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("aggregate".into()),
        Ty::named("Demo::MissingOffsets"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.terminator = Some(Terminator::Return);
    body.blocks.push(block0);

    let function = MirFunction {
        name: "Demo::NeedOffsets".into(),
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

pub(crate) fn struct_without_size_layout() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let struct_layout = StructLayout {
        name: "Demo::Incomplete".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: None,
        align: Some(4),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert("Demo::Incomplete".into(), TypeLayout::Struct(struct_layout));
    layouts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_layout_registers_indices_and_lengths() {
        let (layouts, _) = array_index_fixture();
        let key =
            Ty::Array(crate::mir::ArrayTy::new(Box::new(Ty::named("int")), 1)).canonical_name();
        let layout = layouts.types.get(&key).expect("array layout");
        match layout {
            TypeLayout::Struct(struct_layout) => {
                assert_eq!(struct_layout.list.as_ref().unwrap().length_index, Some(1));
                assert_eq!(struct_layout.fields.len(), 6);
            }
            other => panic!("expected struct layout, got {other:?}"),
        }
    }

    #[test]
    fn missing_offsets_fixture_exposes_incomplete_field_offsets() {
        let (layouts, _) = struct_with_missing_offsets_fixture();
        let layout = layouts
            .types
            .get("Demo::MissingOffsets")
            .expect("missing offsets layout");
        match layout {
            TypeLayout::Struct(struct_layout) => {
                assert!(struct_layout.fields[0].offset.is_none());
            }
            _ => panic!("expected struct layout"),
        }
    }

    #[test]
    fn struct_without_size_fixture_has_no_size() {
        let layouts = struct_without_size_layout();
        let layout = layouts
            .types
            .get("Demo::Incomplete")
            .expect("incomplete layout");
        match layout {
            TypeLayout::Struct(struct_layout) => {
                assert!(struct_layout.size.is_none());
            }
            _ => panic!("expected struct layout"),
        }
    }

    #[test]
    fn string_and_str_fixtures_return_char() {
        let (_, string_fn) = string_index_fixture();
        assert_eq!(string_fn.signature.ret, Ty::named("char"));
        let (_, str_fn) = str_index_fixture();
        assert_eq!(str_fn.signature.ret, Ty::named("char"));
    }
}
