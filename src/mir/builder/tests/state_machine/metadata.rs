use super::common::*;

#[test]
fn parse_type_arguments_reports_invalid_expression() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(3, 4));
            let result = builder.parse_type_arguments("not<a", span);
            assert!(result.is_none(), "invalid type args should fail to parse");
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("not<a")),
                "expected diagnostic citing invalid type expression, got {diagnostics:?}"
            );
        },
    );
}

#[test]
fn type_identity_helpers_cover_edge_cases() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(7, 8));
            assert_eq!(
                builder.parse_type_arguments("int", span),
                Some(Vec::new()),
                "non-generic type text should return an empty args list"
            );
            let unknown = builder.type_id_operand_for_ty(&Ty::Unknown, span);
            let missing_param = builder.type_param_type_id_operand("T", span);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                unknown.is_none(),
                "unknown type identity should not resolve"
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("type identity")),
                "expected diagnostic for unknown type identity, got {diagnostics:?}"
            );
            assert!(
                missing_param.is_none(),
                "missing type param metadata should fail"
            );
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("type parameter `T`")),
                "expected diagnostic about missing type param metadata, got {diagnostics:?}"
            );
        },
    );
}

#[test]
fn runtime_type_metadata_call_emits_call() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(5, 6));
            let ty = Ty::named("Demo::Data");
            let result = builder.runtime_type_size_operand(&ty, span);
            assert!(
                result.is_some(),
                "runtime metadata call should produce operand"
            );

            let (body, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics from runtime type metadata call: {diagnostics:?}"
            );
            let call = body.blocks[0]
                .terminator
                .as_ref()
                .expect("runtime metadata should emit call terminator");
            match call {
                Terminator::Call { func, args, .. } => {
                    assert!(
                        matches!(
                            func,
                            Operand::Const(ConstOperand {
                                value: ConstValue::Symbol(symbol),
                                ..
                            }) if symbol == "chic_rt_type_size"
                        ),
                        "expected runtime size symbol, got {func:?}"
                    );
                    assert_eq!(args.len(), 1, "runtime metadata call should pass type id");
                    assert!(
                        matches!(
                            args[0],
                            Operand::Const(ConstOperand {
                                value: ConstValue::UInt(_),
                                ..
                            })
                        ),
                        "type id argument should be emitted as const uint, got {:?}",
                        args[0]
                    );
                }
                other => panic!("expected runtime metadata call terminator, got {other:?}"),
            }
        },
    );
}

#[test]
fn runtime_type_metadata_variants_emit_expected_symbols() {
    fn assert_symbol<F>(function_name: &str, mut invoke: F, expected: &str)
    where
        F: FnMut(&mut BodyBuilder<'_>, &Ty, Option<Span>) -> Option<Operand>,
    {
        with_state_builder(
            FunctionKind::Function,
            |_| {},
            |mut builder| {
                let span = Some(Span::new(6, 7));
                let ty = Ty::named(function_name);
                invoke(&mut builder, &ty, span).expect("runtime call should produce operand");
                let (body, diagnostics, ..) = builder.finish();
                assert!(
                    diagnostics.is_empty(),
                    "unexpected diagnostics for {function_name}: {diagnostics:?}"
                );
                let call = body.blocks[0]
                    .terminator
                    .as_ref()
                    .expect("runtime metadata should emit call terminator");
                match call {
                    Terminator::Call { func, .. } => match func {
                        Operand::Const(ConstOperand {
                            value: ConstValue::Symbol(symbol),
                            ..
                        }) => assert_eq!(symbol, expected, "runtime call symbol should match"),
                        other => panic!("unexpected function operand {other:?}"),
                    },
                    other => panic!("expected call terminator, got {other:?}"),
                }
            },
        );
    }

    assert_symbol(
        "Demo::Data",
        |builder, ty, span| builder.runtime_type_align_operand(ty, span),
        "chic_rt_type_align",
    );
    assert_symbol(
        "Demo::Cloneable",
        |builder, ty, span| builder.runtime_type_clone_operand(ty, span),
        "chic_rt_type_clone_glue",
    );
    assert_symbol(
        "Demo::Droppable",
        |builder, ty, span| builder.runtime_type_drop_operand(ty, span),
        "chic_rt_type_drop_glue",
    );
}

#[test]
fn describe_place_formats_projections() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let base = builder.push_local(LocalDecl::new(
                Some("item".into()),
                Ty::named("Record"),
                true,
                Some(Span::new(1, 2)),
                LocalKind::Local,
            ));
            let index_local = builder.push_local(LocalDecl::new(
                Some("idx".into()),
                Ty::named("int"),
                false,
                Some(Span::new(2, 3)),
                LocalKind::Local,
            ));
            let mut place = Place::new(base);
            place.projection.push(ProjectionElem::Field(1));
            place
                .projection
                .push(ProjectionElem::FieldNamed("Name".into()));
            place.projection.push(ProjectionElem::UnionField {
                index: 0,
                name: "Kind".into(),
            });
            place
                .projection
                .push(ProjectionElem::Index(LocalId(index_local.0)));
            place.projection.push(ProjectionElem::ConstantIndex {
                offset: 5,
                length: 1,
                from_end: false,
            });
            place.projection.push(ProjectionElem::Deref);
            place
                .projection
                .push(ProjectionElem::Downcast { variant: 2 });
            place
                .projection
                .push(ProjectionElem::Subslice { from: 1, to: 3 });

            let label = builder.describe_place(&place);
            assert_eq!(
                label, "item.1.Name.Kind[idx][5].*#2[1..3]",
                "projection formatting should include each projection element"
            );
        },
    );
}

#[test]
fn operand_to_place_creates_temps_for_consts_and_pending() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let span = Some(Span::new(4, 5));
            let const_place = builder
                .operand_to_place(Operand::Const(ConstOperand::new(ConstValue::Int(1))), span);
            let pending_place = builder.operand_to_place(
                Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                    repr: "pending".into(),
                    span,
                    info: None,
                }),
                span,
            );
            assert!(
                const_place.local != LocalId(0) && pending_place.local != LocalId(0),
                "const and pending operands should be materialised into temporaries"
            );
            assert!(const_place.projection.is_empty());
            assert!(pending_place.projection.is_empty());
        },
    );
}

#[test]
fn validate_required_initializer_reports_missing_members() {
    let span = Some(Span::new(20, 30));
    let mut symbol_index = SymbolIndex::default();
    symbol_index.type_properties.insert(
        "Demo::Derived".into(),
        HashMap::from([(
            "Prop".into(),
            PropertySymbol {
                ty: "int".into(),
                is_static: true,
                accessors: HashMap::new(),
                span,
                is_required: true,
                is_nullable: false,
                visibility: Visibility::Public,
                namespace: Some("Demo".into()),
            },
        )]),
    );

    with_custom_state_builder(
        "Demo::Derived::Init",
        FunctionKind::Constructor,
        Some("Demo"),
        |layouts| {
            layouts.types.insert(
                "Demo::Base".into(),
                required_layout(
                    "Demo::Base",
                    Vec::new(),
                    vec![required_field("hidden", 0, Some("PublicName"))],
                ),
            );
            layouts.types.insert(
                "Demo::Derived".into(),
                required_layout(
                    "Demo::Derived",
                    vec!["Demo.Base".into()],
                    vec![required_field("Own", 1, None)],
                ),
            );
        },
        |bases| {
            bases.insert("Demo::Derived".into(), vec!["Demo::Base".into()]);
        },
        symbol_index,
        |mut builder| {
            let expr = Expression::new("new Demo::Derived { Prop = 1 }", span);
            builder.validate_required_initializer(&expr);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("PublicName")),
                "expected diagnostic mentioning missing required member, got {diagnostics:?}"
            );
        },
    );
}

#[test]
fn impl_trait_bounds_extracts_opaque_bounds() {
    let expr = TypeExpr {
        trait_object: Some(TraitObjectTypeExpr {
            bounds: vec![TypeExpr::simple("First"), TypeExpr::simple("Second")],
            opaque_impl: true,
        }),
        ..TypeExpr::simple("impl")
    };
    let bounds =
        impl_trait_bounds_from_type_expr(&expr).expect("opaque impl trait should yield bounds");
    assert_eq!(bounds, vec!["First", "Second"]);
}
