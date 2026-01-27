use super::common::*;

#[test]
fn ensure_active_block_spawns_successor_after_return() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            builder.set_terminator(Some(Span::new(0, 1)), Terminator::Return);
            let new_block = builder.ensure_active_block();
            assert_eq!(new_block, BlockId(1), "expected successor block");

            let (body, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert_eq!(
                body.blocks.len(),
                2,
                "orphan block should terminate with Return after finalize"
            );
            assert!(
                matches!(body.blocks[1].terminator, Some(Terminator::Return)),
                "fresh block should terminate with Return after finalize"
            );
        },
    );
}

#[test]
fn drop_to_scope_depth_emits_storage_dead() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            builder.push_scope();
            let temp = builder.create_temp(Some(Span::new(2, 3)));
            builder.drop_to_scope_depth(1, Some(Span::new(4, 5)));

            let (body, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            let dead = body.blocks[0]
                .statements
                .iter()
                .any(|stmt| matches!(stmt.kind, MirStatementKind::StorageDead(id) if id == temp));
            assert!(dead, "drop lowering should emit StorageDead for the temp");
        },
    );
}

#[test]
fn pending_statement_records_detail_and_kind() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let ast = Statement::new(Some(Span::new(10, 12)), AstStatementKind::Empty);
            builder.push_pending(&ast, PendingStatementKind::Try);

            let (body, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            let pending = body.blocks[0]
                .statements
                .iter()
                .find_map(|stmt| match &stmt.kind {
                    MirStatementKind::Pending(detail) => Some(detail.clone()),
                    _ => None,
                })
                .expect("pending statement not emitted");
            assert_eq!(pending.kind, PendingStatementKind::Try);
            assert_eq!(
                pending.detail.as_deref(),
                Some("Try pending lowering"),
                "detail should mention pending lowering kind"
            );
        },
    );
}

#[test]
fn readonly_field_assignment_requires_constructor_context() {
    with_state_builder(
        FunctionKind::Function,
        insert_readonly_layout,
        |mut builder| {
            let local = builder.push_local(LocalDecl::new(
                Some("self".into()),
                Ty::named("State::Readonly"),
                true,
                Some(Span::new(0, 1)),
                LocalKind::Arg(0),
            ));
            let mut place = Place::new(local);
            place
                .projection
                .push(ProjectionElem::FieldNamed("Value".into()));
            let statement = MirStatement {
                span: Some(Span::new(2, 3)),
                kind: MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("readonly field")),
                "readonly assignments outside constructors should emit diagnostics, got {diagnostics:?}"
            );
        },
    );
}

#[test]
fn match_binding_registration_tracks_unique_names() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let first = builder.create_temp(Some(Span::new(1, 2)));
            let second = builder.create_temp(Some(Span::new(2, 3)));
            let first_name = builder.register_match_binding(first);
            let second_name = builder.register_match_binding(second);
            assert_ne!(first_name, second_name, "bindings should be unique");
            assert_eq!(builder.lookup_name(&first_name), Some(first));
            assert_eq!(builder.lookup_name(&second_name), Some(second));
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
        },
    );
}

#[test]
fn try_finally_lowering_builds_mir_graph() {
    let source = r"
namespace StateIntegration;

public class Runner
{
    public int Execute(int limit)
    {
        while (limit > 0)
        {
            limit -= 1;
        }

        try
        {
            limit += 1;
        }
        finally
        {
            limit = 0;
        }

        return limit;
    }
}
";
    let parsed = parse_module(source).require("parse integration module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {0:?}",
        lowering.diagnostics
    );
    let func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "StateIntegration::Runner::Execute")
        .expect("missing Execute function");
    assert!(
        func.body.blocks.len() > 2,
        "loop + try/finally should introduce multiple blocks"
    );
    assert!(
        func.body.blocks.len() > 2,
        "loop + try/finally should introduce multiple blocks"
    );
}

#[test]
fn readonly_struct_diagnostics_surface_in_integration_lowering() {
    let source = r"
namespace StateIntegration;

public readonly struct Sensor
{
    public readonly int Value;

    public init(int value)
    {
        self.Value = value;
    }

    public void Update(int value)
    {
        self.Value = value;
    }
}
";
    let parsed = parse_module(source).require("parse readonly struct module");
    let lowering = lower_module(&parsed.module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("readonly field")),
        "expected readonly assignment diagnostic, got {0:?}",
        lowering.diagnostics
    );
}

#[test]
fn const_environment_tracks_bindings_across_scopes() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            builder.bind_const("PI", ConstValue::Int(3));
            builder.push_scope();
            builder.bind_const("E", ConstValue::Int(2));
            match builder.lookup_const("PI") {
                Some(ConstValue::Int(value)) => assert_eq!(value, 3),
                other => panic!("expected PI constant, found {other:?}"),
            }
            match builder.lookup_const("E") {
                Some(ConstValue::Int(value)) => assert_eq!(value, 2),
                other => panic!("expected E constant, found {other:?}"),
            }
            let env = builder.const_environment();
            match env.get("PI") {
                Some(ConstValue::Int(value)) => assert_eq!(*value, 3),
                other => panic!("expected PI entry, found {other:?}"),
            }
            match env.get("E") {
                Some(ConstValue::Int(value)) => assert_eq!(*value, 2),
                other => panic!("expected E entry, found {other:?}"),
            }
            builder.pop_scope();
            assert!(
                builder.lookup_const("E").is_none(),
                "popped scope should remove constants"
            );
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "const bookkeeping should not emit diagnostics: {diagnostics:?}"
            );
        },
    );
}

#[test]
fn constructor_allows_readonly_self_assignment() {
    with_named_state_builder(
        "State::Readonly::.ctor",
        FunctionKind::Constructor,
        insert_readonly_layout,
        |mut builder| {
            let self_local = builder.push_local(LocalDecl::new(
                Some("self".into()),
                Ty::named("State::Readonly"),
                true,
                Some(Span::new(0, 1)),
                LocalKind::Arg(0),
            ));
            let mut place = Place::new(self_local);
            place
                .projection
                .push(ProjectionElem::FieldNamed("Value".into()));
            let statement = MirStatement {
                span: Some(Span::new(2, 3)),
                kind: MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(7)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "assigning readonly fields within constructors should succeed"
            );
        },
    );
}

#[test]
fn temp_assignment_without_owner_is_allowed() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let temp = builder.create_temp(Some(Span::new(1, 2)));
            let statement = MirStatement {
                span: Some(Span::new(2, 3)),
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(9)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "non-struct locals should accept assignments"
            );
        },
    );
}

#[test]
fn readonly_field_index_assignment_rejected() {
    with_state_builder(
        FunctionKind::Function,
        insert_readonly_layout,
        |mut builder| {
            let local = builder.push_local(LocalDecl::new(
                Some("this".into()),
                Ty::named("State::Readonly"),
                true,
                Some(Span::new(0, 1)),
                LocalKind::Arg(0),
            ));
            let mut place = Place::new(local);
            place.projection.push(ProjectionElem::Field(0));
            let statement = MirStatement {
                span: Some(Span::new(4, 5)),
                kind: MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(5)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("readonly field")),
                "readonly fields accessed by numeric index should still reject assignments"
            );
        },
    );
}

#[test]
fn non_field_projection_skips_readonly_validation() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let buffer = builder.push_local(LocalDecl::new(
                Some("buffer".into()),
                Ty::named("Span<int>"),
                true,
                Some(Span::new(0, 1)),
                LocalKind::Local,
            ));
            let mut place = Place::new(buffer);
            place.projection.push(ProjectionElem::Index(LocalId(0)));
            let statement = MirStatement {
                span: Some(Span::new(3, 4)),
                kind: MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "non-field projections should bypass readonly validation"
            );
        },
    );
}

#[test]
fn readonly_struct_assignment_rejected_outside_constructor() {
    let module_source = r"
namespace State;

public readonly struct Immutable
{
    public int Value;
}
";
    let parsed = parse_module(module_source).require("parse readonly struct module");
    let symbol_index = SymbolIndex::build(&parsed.module);
    with_named_state_builder_with_index(
        "State::Immutable::Mutate",
        FunctionKind::Function,
        |layouts| {
            let field = FieldLayout {
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
            };
            let layout = StructLayout {
                name: "State::Immutable".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![field],
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: true,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride {
                    thread_safe: None,
                    shareable: None,
                    copy: None,
                },
                mmio: None,
                dispose: None,
                class: None,
            };
            layouts
                .types
                .insert("State::Immutable".into(), TypeLayout::Struct(layout));
        },
        symbol_index,
        |mut builder| {
            let receiver = builder.push_local(LocalDecl::new(
                Some("this".into()),
                Ty::named("State::Immutable"),
                true,
                Some(Span::new(0, 1)),
                LocalKind::Arg(0),
            ));
            let mut place = Place::new(receiver);
            place
                .projection
                .push(ProjectionElem::FieldNamed("Value".into()));
            let statement = MirStatement {
                span: Some(Span::new(4, 5)),
                kind: MirStatementKind::Assign {
                    place,
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(10)))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("readonly struct")),
                "readonly structs should reject assignments outside constructors"
            );
        },
    );
}

#[test]
fn temp_assignment_skips_borrow_escape_constraint() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(1, 2));
            let param = builder.push_local(
                LocalDecl::new(
                    Some("value".into()),
                    Ty::named("string"),
                    true,
                    span,
                    LocalKind::Arg(0),
                )
                .with_param_mode(ParamMode::In),
            );
            builder.record_local(param, span);
            let temp = builder.create_temp(span);
            let statement = MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Use(Operand::Move(Place::new(param))),
                },
            };
            builder.push_statement(statement);
            let (_, diagnostics, constraints, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            let borrow_constraints: Vec<_> = constraints
                .iter()
                .filter(|constraint| matches!(constraint.kind, ConstraintKind::BorrowEscape { .. }))
                .collect();
            assert!(
                borrow_constraints.is_empty(),
                "borrow escape constraints should ignore temp assignments: {borrow_constraints:?}"
            );
        },
    );
}

#[test]
fn borrow_escape_to_return_records_constraint() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(1, 2));
            let param = builder.push_local(
                LocalDecl::new(
                    Some("value".into()),
                    Ty::named("string"),
                    true,
                    span,
                    LocalKind::Arg(0),
                )
                .with_param_mode(ParamMode::In),
            );
            builder.record_local(param, span);

            builder.record_borrow_escape_from_assignment(
                &Place::new(LocalId(0)),
                &Place::new(param),
                AssignmentSourceKind::Borrow,
                span,
            );

            let (_, diagnostics, constraints, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
            assert!(
                constraints.iter().any(|constraint| matches!(
                    constraint.kind,
                    ConstraintKind::BorrowEscape { .. }
                )),
                "expected borrow escape constraint when storing borrowed param into return"
            );
        },
    );
}
