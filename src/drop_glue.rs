use std::collections::HashSet;

use crate::frontend::parser::parse_type_expression_text;
use crate::mir::builder::synthesise_drop_statements;
use crate::mir::{
    Abi, BasicBlock, BinOp, BlockId, ConstOperand, ConstValue, FnSig, FnTy, FunctionKind, LocalDecl,
    LocalId, LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy,
    ProjectionElem, Rvalue, Statement, StatementKind, Terminator, Ty, TypeLayout, TypeLayoutTable,
};
use crate::monomorphize::MonomorphizationSummary;
use crate::type_identity::type_identity_for_name;
use blake3::hash;

/// Metadata describing a synthesised drop glue thunk.
#[derive(Debug, Clone)]
pub struct SynthesisedDropGlue {
    pub type_name: String,
    pub symbol: String,
    pub function_index: usize,
    pub type_identity: u64,
}

pub fn drop_glue_symbol_for(ty_name: &str) -> String {
    let mut symbol = String::from("__cl_drop__");
    for ch in ty_name.chars() {
        match ch {
            ':' | '<' | '>' | ',' | ' ' | '[' | ']' | '?' | '.' => symbol.push('_'),
            ch if ch.is_alphanumeric() || ch == '_' => symbol.push(ch),
            _ => symbol.push('_'),
        }
    }
    symbol
}

#[must_use]
pub fn drop_type_identity(name: &str) -> u64 {
    let digest = hash(name.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest.as_bytes()[..8]);
    u64::from_le_bytes(bytes)
}

fn pointer_ty_for(ty: Ty) -> Ty {
    Ty::Pointer(Box::new(PointerTy::new(ty, true)))
}

fn parse_type_from_name(name: &str) -> Option<Ty> {
    if let Some(expr) = parse_type_expression_text(name) {
        return Some(Ty::from_type_expr(&expr));
    }
    if name.contains("::") {
        let substituted = name.replace("::", ".");
        if let Some(mut expr) = parse_type_expression_text(&substituted) {
            expr.name = name.to_string();
            return Some(Ty::from_type_expr(&expr));
        }
    }
    Some(Ty::named(name))
}

fn synthesize_fn_drop_glue(ty_name: &str, fn_ty: &FnTy) -> Option<MirFunction> {
    let name = drop_glue_symbol_for(ty_name);
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);
    let typed_pointer_ty = pointer_ty_for(Ty::Fn(fn_ty.clone()));

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        typed_pointer_ty,
        false,
        None,
        LocalKind::Arg(0),
    ));

    let mut drop_block = BasicBlock::new(BlockId(0), None);
    let context_place = Place {
        local: LocalId(1),
        projection: vec![
            ProjectionElem::Deref,
            ProjectionElem::FieldNamed("context".into()),
        ],
    };
    let drop_glue_place = Place {
        local: LocalId(1),
        projection: vec![
            ProjectionElem::Deref,
            ProjectionElem::FieldNamed("drop_glue".into()),
        ],
    };
    drop_block.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "chic_rt_drop_invoke".into(),
        ))),
        args: vec![
            Operand::Copy(drop_glue_place.clone()),
            Operand::Copy(context_place.clone()),
        ],
        arg_modes: vec![ParamMode::Value; 2],
        destination: None,
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });

    let mut free_block = BasicBlock::new(BlockId(1), None);
    let env_size_place = Place {
        local: LocalId(1),
        projection: vec![
            ProjectionElem::Deref,
            ProjectionElem::FieldNamed("env_size".into()),
        ],
    };
    let env_align_place = Place {
        local: LocalId(1),
        projection: vec![
            ProjectionElem::Deref,
            ProjectionElem::FieldNamed("env_align".into()),
        ],
    };
    free_block.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "chic_rt_closure_env_free".into(),
        ))),
        args: vec![
            Operand::Copy(context_place),
            Operand::Copy(env_size_place),
            Operand::Copy(env_align_place),
        ],
        arg_modes: vec![ParamMode::Value; 3],
        destination: None,
        target: BlockId(2),
        unwind: None,
        dispatch: None,
    });

    let mut ret_block = BasicBlock::new(BlockId(2), None);
    ret_block.terminator = Some(Terminator::Return);

    body.blocks.push(drop_block);
    body.blocks.push(free_block);
    body.blocks.push(ret_block);

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty],
            ret: Ty::Unit,
            abi: Abi::Extern("C".into()),
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
    })
}

fn synthesize_glue_function(ty_name: &str, layouts: &TypeLayoutTable) -> Option<MirFunction> {
    let name = drop_glue_symbol_for(ty_name);
    let ty = parse_type_from_name(ty_name)?;
    let typed_pointer_ty = pointer_ty_for(ty.clone());
    let raw_pointer_ty = pointer_ty_for(Ty::Unit);

    let is_class = layouts
        .layout_for_name(ty_name)
        .is_some_and(|layout| matches!(layout, TypeLayout::Class(_)));

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("ptr".into()),
        typed_pointer_ty.clone(),
        false,
        None,
        LocalKind::Arg(0),
    ));

    let slot_place = Place {
        local: LocalId(1),
        projection: vec![ProjectionElem::Deref],
    };
    let place = if is_class {
        Place {
            local: LocalId(1),
            projection: vec![ProjectionElem::Deref, ProjectionElem::Deref],
        }
    } else {
        slot_place.clone()
    };

    let mut statements =
        synthesise_drop_statements(layouts, place.clone(), ty.clone(), None, BlockId(0), false);
    if matches!(
        statements.last(),
        Some(Statement {
            kind: StatementKind::Drop { .. },
            ..
        })
    ) {
        statements.pop();
    }

    if !is_class {
        let mut block = BasicBlock::new(BlockId(0), None);
        block.statements.append(&mut statements);
        block.terminator = Some(Terminator::Return);
        body.blocks.push(block);
    } else {
        body.locals.push(LocalDecl::new(
            Some("is_null".into()),
            Ty::named("bool"),
            false,
            None,
            LocalKind::Temp,
        ));

        let mut entry = BasicBlock::new(BlockId(0), None);
        entry.statements.push(Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(2)),
                value: Rvalue::Binary {
                    op: BinOp::Eq,
                    lhs: Operand::Copy(slot_place.clone()),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::Null)),
                    rounding: None,
                },
            },
        });
        entry.terminator = Some(Terminator::SwitchInt {
            discr: Operand::Copy(Place::new(LocalId(2))),
            targets: vec![(1, BlockId(2))],
            otherwise: BlockId(1),
        });
        body.blocks.push(entry);

        let mut drop_block = BasicBlock::new(BlockId(1), None);
        drop_block.statements.append(&mut statements);
        drop_block.terminator = Some(Terminator::Return);
        body.blocks.push(drop_block);

        let mut ret_block = BasicBlock::new(BlockId(2), None);
        ret_block.terminator = Some(Terminator::Return);
        body.blocks.push(ret_block);
    }

    Some(MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![raw_pointer_ty],
            ret: Ty::Unit,
            abi: Abi::Extern("C".into()),
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
    })
}

/// Append per-type drop glue thunks to the MIR module, returning metadata about each emitted thunk.
pub fn synthesise_drop_glue(
    module: &mut MirModule,
    summary: &MonomorphizationSummary,
) -> Vec<SynthesisedDropGlue> {
    if summary.drop_candidates.is_empty() {
        return Vec::new();
    }

    let mut existing: HashSet<String> = module
        .functions
        .iter()
        .map(|function| function.name.clone())
        .collect();
    let mut synthesised = Vec::new();

    for ty_name in &summary.drop_candidates {
        let symbol = drop_glue_symbol_for(ty_name);
        if existing.contains(&symbol) {
            continue;
        }
        let Some(function) = (if let Some(Ty::Fn(fn_ty)) = parse_type_from_name(ty_name) {
            synthesize_fn_drop_glue(ty_name, &fn_ty)
        } else if let Some(sig) = module.type_layouts.delegate_signature(ty_name) {
            synthesize_fn_drop_glue(ty_name, sig)
        } else {
            synthesize_glue_function(ty_name, &module.type_layouts)
        }) else {
            continue;
        };
        let index = module.functions.len();
        existing.insert(symbol.clone());
        module.functions.push(function);
        synthesised.push(SynthesisedDropGlue {
            type_name: ty_name.clone(),
            symbol,
            function_index: index,
            type_identity: type_identity_for_name(&module.type_layouts, ty_name),
        });
    }

    synthesised
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        AutoTraitOverride, AutoTraitSet, FieldLayout, ProjectionElem, StatementKind, StructLayout,
        TypeLayout, TypeRepr, UnionFieldLayout, UnionFieldMode, UnionLayout,
    };

    #[test]
    fn synthesise_creates_glue_function() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::StringHolder".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::StringHolder".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Value".into(),
                    ty: Ty::String,
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
                size: None,
                align: None,
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
        let summary = MonomorphizationSummary {
            drop_candidates: vec!["Demo::StringHolder".into()],
            clone_candidates: Vec::new(),
            hash_candidates: Vec::new(),
            eq_candidates: Vec::new(),
        };
        let emitted = synthesise_drop_glue(&mut module, &summary);
        let symbol = drop_glue_symbol_for("Demo::StringHolder");
        assert!(module.functions.iter().any(|f| f.name == symbol));
        let entry = emitted
            .iter()
            .find(|item| item.symbol == symbol)
            .expect("drop glue metadata missing");
        assert_eq!(entry.type_name, "Demo::StringHolder");
        assert_eq!(
            entry.type_identity,
            drop_type_identity("Demo::StringHolder"),
            "drop glue metadata should record stable type identity"
        );
        let recorded_index = module
            .functions
            .iter()
            .position(|f| f.name == symbol)
            .expect("expected drop glue function index");
        assert_eq!(entry.function_index, recorded_index);
        let function = module
            .functions
            .iter()
            .find(|f| f.name == symbol)
            .expect("drop glue not generated");
        assert_eq!(
            function.signature.params,
            vec![pointer_ty_for(Ty::Unit)],
            "drop glue should accept a raw void pointer"
        );
    }

    #[test]
    fn glue_drops_struct_fields_without_recursing_on_self() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::Holder".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::Holder".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Name".into(),
                    ty: Ty::String,
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
                size: None,
                align: None,
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
        let summary = MonomorphizationSummary {
            drop_candidates: vec!["Demo::Holder".into()],
            clone_candidates: Vec::new(),
            hash_candidates: Vec::new(),
            eq_candidates: Vec::new(),
        };
        assert!(
            module
                .type_layouts
                .ty_requires_drop(&Ty::named("Demo::Holder")),
            "holder type should require drop"
        );
        assert!(
            module
                .type_layouts
                .layout_for_name("Demo::Holder")
                .is_some(),
            "holder layout should be registered"
        );
        synthesise_drop_glue(&mut module, &summary);
        let symbol = drop_glue_symbol_for("Demo::Holder");
        let function = module
            .functions
            .iter()
            .find(|f| f.name == symbol)
            .expect("drop glue not generated");
        let block = &function.body.blocks[0];
        assert!(
            !block.statements.is_empty(),
            "expected drop glue statements for Demo::Holder"
        );
        for statement in &block.statements {
            if let StatementKind::Drop { place, .. } = &statement.kind {
                assert!(
                    !place.projection.is_empty(),
                    "drop glue should not recurse on the root pointer"
                );
                let last = place
                    .projection
                    .last()
                    .expect("projection should contain at least one element");
                match last {
                    ProjectionElem::Field(index) => assert_eq!(*index, 0),
                    other => {
                        panic!("unexpected terminal projection {other:?} for holder drop glue")
                    }
                }
            }
        }
    }

    #[test]
    fn glue_invokes_user_deinit_before_nested_drops() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::Inner".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::Inner".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Payload".into(),
                    ty: Ty::String,
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
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: Some("Demo::Inner::dispose".into()),
                class: None,
            }),
        );
        let summary = MonomorphizationSummary {
            drop_candidates: vec!["Demo::Inner".into()],
            clone_candidates: Vec::new(),
            hash_candidates: Vec::new(),
            eq_candidates: Vec::new(),
        };
        synthesise_drop_glue(&mut module, &summary);
        let symbol = drop_glue_symbol_for("Demo::Inner");
        let function = module
            .functions
            .iter()
            .find(|f| f.name == symbol)
            .expect("drop glue not generated");
        let block = &function.body.blocks[0];
        let mut statements = block.statements.iter();
        let first = statements
            .next()
            .expect("expected statements for Demo::Inner glue");
        assert!(
            matches!(first.kind, StatementKind::Deinit(_)),
            "user dispose should be invoked before field drops"
        );
        assert!(
            statements.any(|stmt| matches!(stmt.kind, StatementKind::Drop { .. })),
            "expected nested drop after dispose"
        );
    }

    #[test]
    fn glue_synthesises_for_generic_struct_instances() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::Wrapper<int>".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::Wrapper<int>".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Value".into(),
                    ty: Ty::String,
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
                size: None,
                align: None,
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
        let summary = MonomorphizationSummary {
            drop_candidates: vec!["Demo::Wrapper<int>".into()],
            clone_candidates: Vec::new(),
            hash_candidates: Vec::new(),
            eq_candidates: Vec::new(),
        };
        let emitted = synthesise_drop_glue(&mut module, &summary);
        let symbol = drop_glue_symbol_for("Demo::Wrapper<int>");
        assert!(
            emitted.iter().any(|item| item.symbol == symbol),
            "drop glue should be generated for monomorphised generic structs"
        );
    }

    #[test]
    fn glue_synthesises_for_unions_with_drop_payloads() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::Status".into(),
            TypeLayout::Union(UnionLayout {
                name: "Demo::Status".into(),
                repr: TypeRepr::Default,
                packing: None,
                views: vec![UnionFieldLayout {
                    name: "Owned".into(),
                    ty: Ty::String,
                    index: 0,
                    mode: UnionFieldMode::Value,
                    span: None,
                    is_nullable: false,
                }],
                size: None,
                align: None,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
            }),
        );
        let summary = MonomorphizationSummary {
            drop_candidates: vec!["Demo::Status".into()],
            clone_candidates: Vec::new(),
            hash_candidates: Vec::new(),
            eq_candidates: Vec::new(),
        };
        let emitted = synthesise_drop_glue(&mut module, &summary);
        let symbol = drop_glue_symbol_for("Demo::Status");
        assert!(
            emitted.iter().any(|item| item.symbol == symbol),
            "drop glue should be generated for unions with droppable payloads"
        );
    }
}
