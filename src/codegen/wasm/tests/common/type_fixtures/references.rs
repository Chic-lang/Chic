use crate::mir::TypeLayoutTable;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, Operand, Place, ProjectionElem, Rvalue, Statement,
    StatementKind, Terminator, Ty,
};

pub(crate) use super::shared::{sample_class_layout, sample_pair_layout};

pub(crate) fn struct_projection_fixture() -> (TypeLayoutTable, MirFunction) {
    let layouts = sample_pair_layout();

    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("pair".into()),
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("local_pair".into()),
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("field".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut block0 = BasicBlock::new(BlockId(0), None);
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(2)),
    });
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(3)),
    });

    let mut assign_place = Place::new(LocalId(2));
    assign_place.projection.push(ProjectionElem::Field(0));
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: assign_place,
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(7)))),
        },
    });

    let mut load_place = Place::new(LocalId(1));
    load_place.projection.push(ProjectionElem::Field(0));
    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::Use(Operand::Copy(load_place)),
        },
    });

    block0.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Copy(Place::new(LocalId(3)))),
        },
    });

    block0.terminator = Some(Terminator::Return);
    body.blocks.push(block0);

    let function = MirFunction {
        name: "Demo::UsePair".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("Demo::Pair")],
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
    use crate::mir::TypeLayout;

    #[test]
    fn class_layout_has_vtable_offset() {
        let layouts = sample_class_layout();
        let TypeLayout::Class(class_layout) =
            layouts.types.get("Demo::Window").expect("class layout")
        else {
            panic!("expected class layout");
        };

        assert_eq!(
            class_layout.class.as_ref().and_then(|c| c.vtable_offset),
            Some(0)
        );
    }

    #[test]
    fn pair_layout_defines_two_fields() {
        let layouts = sample_pair_layout();
        let TypeLayout::Struct(struct_layout) =
            layouts.types.get("Demo::Pair").expect("pair layout")
        else {
            panic!("expected struct layout");
        };
        assert_eq!(struct_layout.fields.len(), 2);
        assert_eq!(struct_layout.size, Some(8));
    }

    #[test]
    fn struct_projection_fixture_assigns_and_returns_field() {
        let (_, function) = struct_projection_fixture();
        assert_eq!(function.name, "Demo::UsePair");
        assert_eq!(function.signature.params.len(), 1);
        assert_eq!(function.body.blocks.len(), 1);
    }
}
