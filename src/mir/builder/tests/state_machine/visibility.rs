use super::common::*;

#[test]
fn visibility_and_namespace_helpers_cover_branches() {
    with_custom_state_builder(
        "Demo::Child::Method",
        FunctionKind::Method,
        Some("Demo"),
        |layouts| {
            layouts
                .types
                .insert("Demo::Child".into(), empty_layout("Demo::Child"));
            layouts
                .types
                .insert("Demo::Base".into(), empty_layout("Demo::Base"));
        },
        |bases| {
            bases.insert(
                "Demo::Child".into(),
                vec!["Demo::Base".into(), "Demo::Root".into()],
            );
        },
        SymbolIndex::default(),
        |mut builder| {
            let expected = [
                (Visibility::Public, "public"),
                (Visibility::Internal, "internal"),
                (Visibility::Protected, "protected"),
                (Visibility::Private, "private"),
                (Visibility::ProtectedInternal, "protected internal"),
                (Visibility::PrivateProtected, "private protected"),
            ];
            for (visibility, keyword) in expected {
                assert_eq!(
                    BodyBuilder::visibility_keyword(visibility),
                    keyword,
                    "visibility keyword for {visibility:?} should match"
                );
            }
            assert_eq!(BodyBuilder::namespace_root(None), None);
            assert_eq!(
                BodyBuilder::namespace_root(Some("Demo::Nested")),
                Some("Demo")
            );
            assert_eq!(
                builder.current_namespace_root().as_deref(),
                Some("Demo"),
                "method should infer namespace root from self type"
            );
            assert!(
                builder.inherits_from("Demo::Child", "Demo::Base"),
                "inheritance traversal should find base type"
            );
            assert!(
                builder.is_within_type("Demo::Child"),
                "should report true when owner matches current self type"
            );
            assert!(
                !builder.is_within_type("Demo::Base"),
                "different type should not be treated as self"
            );
            assert!(
                builder.namespaces_match(Some("Demo::Other")),
                "matching namespace roots should be treated as internal visibility"
            );
            assert!(
                builder.check_static_visibility(
                    "Demo::Child",
                    Some("Demo"),
                    Visibility::Private,
                    None,
                    "member"
                ),
                "private members on self type should be visible"
            );
            assert!(
                !builder.check_static_visibility(
                    "Other::Owner",
                    Some("Other"),
                    Visibility::Protected,
                    None,
                    "member"
                ),
                "protected members on unrelated types should be hidden"
            );
            assert!(
                !builder.check_static_visibility(
                    "Demo::Base",
                    Some("Other"),
                    Visibility::PrivateProtected,
                    None,
                    "member"
                ),
                "private protected should require both protected and internal visibility"
            );
            assert!(
                builder.check_static_visibility(
                    "Demo::Child",
                    Some("Demo"),
                    Visibility::Internal,
                    None,
                    "member"
                ),
                "internal visibility should match namespace roots"
            );
            assert!(
                builder.check_static_visibility(
                    "Demo::Base",
                    Some("Demo"),
                    Visibility::Protected,
                    None,
                    "member"
                ),
                "protected visibility should allow base types"
            );
            assert!(
                builder.check_static_visibility(
                    "Demo::Base",
                    Some("Demo"),
                    Visibility::ProtectedInternal,
                    None,
                    "member"
                ),
                "protected internal should succeed when either condition is met"
            );
        },
    );
}

#[test]
fn current_namespace_root_for_functions_without_namespace_is_none() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |builder| {
            assert_eq!(
                builder.current_namespace_root(),
                None,
                "free functions without namespace should not infer a root"
            );
        },
    );
}

#[test]
fn resolve_static_type_name_supports_self_and_namespace_chain() {
    with_custom_state_builder(
        "Demo::Widget::Init",
        FunctionKind::Method,
        Some("Demo"),
        |layouts| {
            layouts
                .types
                .insert("Demo::Widget".into(), empty_layout("Demo::Widget"));
            layouts.types.insert(
                "Demo::Util::Helper".into(),
                empty_layout("Demo::Util::Helper"),
            );
        },
        |_| {},
        SymbolIndex::default(),
        |builder| {
            let self_segments = vec!["self".to_string(), "Inner".to_string()];
            assert_eq!(
                builder.resolve_static_type_name(&self_segments),
                Some("Demo::Widget::Inner".into())
            );

            let util_segments = vec!["Util".to_string(), "Helper".to_string()];
            assert_eq!(
                builder.resolve_static_type_name(&util_segments),
                Some("Demo::Util::Helper".into())
            );
        },
    );
}

#[test]
fn static_lowering_error_paths_emit_diagnostics() {
    let span = Some(Span::new(10, 11));
    let mut symbol_index = SymbolIndex::default();
    symbol_index.type_fields.insert(
        "Demo::Config".into(),
        HashMap::from([(
            "Value".into(),
            FieldSymbol {
                ty: TypeExpr::simple("int"),
                visibility: Visibility::Public,
                is_static: true,
                is_readonly: false,
                is_required: false,
                span,
                namespace: Some("Demo".into()),
            },
        )]),
    );
    symbol_index.type_properties.insert(
        "Demo::Config".into(),
        HashMap::from([(
            "Name".into(),
            PropertySymbol {
                ty: "string".into(),
                is_static: false,
                accessors: HashMap::new(),
                span,
                is_required: false,
                is_nullable: false,
                visibility: Visibility::Public,
                namespace: Some("Demo".into()),
            },
        )]),
    );

    let property_symbol = symbol_index
        .type_properties
        .get("Demo::Config")
        .and_then(|props| props.get("Name"))
        .cloned()
        .expect("property symbol should exist");
    let field_symbol = symbol_index
        .type_fields
        .get("Demo::Config")
        .and_then(|fields| fields.get("Value"))
        .cloned()
        .expect("field symbol should exist");

    with_custom_state_builder(
        "Demo::Config::Init",
        FunctionKind::Function,
        Some("Demo"),
        |layouts| {
            layouts
                .types
                .insert("Demo::Config".into(), empty_layout("Demo::Config"));
        },
        |_| {},
        symbol_index,
        |mut builder| {
            let property_operand = builder.lower_static_member_operand(
                &ExprNode::Identifier("Demo::Config".into()),
                "Name",
                span,
            );
            assert!(
                property_operand.is_some(),
                "non-static property access should still return a pending operand"
            );

            let field_operand =
                builder.lower_static_field_value("Demo::Config", "Value", &field_symbol, span);
            assert!(
                field_operand.is_some(),
                "unregistered static field should return pending operand"
            );

            let readonly_field = FieldSymbol {
                is_readonly: true,
                ..field_symbol.clone()
            };
            let readonly_store = builder.emit_static_store(
                "Demo::Config",
                "Value",
                &readonly_field,
                Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                span,
            );
            assert!(
                !readonly_store,
                "readonly static field assignments should be rejected"
            );

            let compound_property = PropertySymbol {
                is_static: true,
                ..property_symbol.clone()
            };
            let compound_result = builder.lower_static_property_assignment(
                "Demo::Config",
                "Name",
                &compound_property,
                AssignOp::AddAssign,
                ExprNode::Identifier("value".into()),
                span,
            );
            assert!(
                matches!(compound_result, Some(false)),
                "compound assignments on static properties should fail"
            );

            assert!(
                !builder.validate_static_property_setter_context(
                    PropertyAccessorKind::Init,
                    "Demo::Config",
                    "Name",
                    span,
                    span
                ),
                "init-only setters outside constructors should be rejected"
            );

            let (_, diagnostics, ..) = builder.finish();
            let messages: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
            assert!(
                messages
                    .iter()
                    .any(|msg| msg.contains("property `Demo::Config.Name` is not static")),
                "expected non-static property diagnostic, got {messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .any(|msg| msg.contains("static field `Demo::Config.Value` is not registered")),
                "expected missing static registration diagnostic, got {messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .any(|msg| msg.contains("readonly") && msg.contains("Demo::Config.Value")),
                "expected readonly assignment diagnostic, got {messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .any(|msg| msg.contains("compound assignment on property")),
                "expected compound assignment diagnostic, got {messages:?}"
            );
            assert!(
                messages
                    .iter()
                    .any(|msg| msg.contains("init-only property `Demo::Config.Name`")),
                "expected init-only setter diagnostic, got {messages:?}"
            );
        },
    );
}

#[test]
fn atomic_order_from_expr_node_parses_memory_order() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let node = ExprNode::Member {
                base: Box::new(ExprNode::Member {
                    base: Box::new(ExprNode::Member {
                        base: Box::new(ExprNode::Identifier("Std".into())),
                        member: "Sync".into(),
                        null_conditional: false,
                    }),
                    member: "MemoryOrder".into(),
                    null_conditional: false,
                }),
                member: "SeqCst".into(),
                null_conditional: false,
            };
            let order = builder
                .atomic_order_from_expr_node(&node, Some(Span::new(10, 11)), "atomic")
                .expect("expected SeqCst order");
            assert_eq!(order, AtomicOrdering::SeqCst);
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics.is_empty(),
                "unexpected diagnostics: {diagnostics:?}"
            );
        },
    );
}

#[test]
fn atomic_order_from_operand_reports_error_for_invalid_constant() {
    with_state_builder(
        FunctionKind::Function,
        |_| {},
        |mut builder| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int(1)));
            let order =
                builder.atomic_order_from_operand(&operand, Some(Span::new(12, 13)), "atomic");
            assert!(order.is_none(), "invalid atomic order should return None");
            let (_, diagnostics, ..) = builder.finish();
            assert!(
                diagnostics
                    .iter()
                    .any(|diag| diag.message.contains("MemoryOrder")),
                "expected atomic ordering diagnostic, got {diagnostics:?}"
            );
        },
    );
}
