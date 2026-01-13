use super::helpers::*;

#[test]
fn captures_class_trait_impl_and_extension_descriptors() {
    let service_field = FieldDecl {
        visibility: Visibility::Public,
        name: "Endpoint".into(),
        ty: TypeExpr::simple("string"),
        initializer: None,
        mmio: None,
        doc: doc("service endpoint"),
        is_required: true,
        display_name: Some("EndpointUrl".into()),
        attributes: vec![attr("Http")],
        is_readonly: false,
        is_static: false,
        view_of: None,
    };
    let property = PropertyDecl {
        visibility: Visibility::Public,
        modifiers: vec!["cached".into()],
        name: "Name".into(),
        ty: TypeExpr::simple("string"),
        parameters: Vec::new(),
        accessors: vec![
            PropertyAccessor {
                kind: PropertyAccessorKind::Get,
                visibility: Some(Visibility::Public),
                body: PropertyAccessorBody::Auto,
                doc: doc("getter"),
                span: None,
                attributes: Some(vec![attr("Getter")]),
                dispatch: MemberDispatch::default(),
            },
            PropertyAccessor {
                kind: PropertyAccessorKind::Set,
                visibility: Some(Visibility::Internal),
                body: PropertyAccessorBody::Auto,
                doc: None,
                span: None,
                attributes: None,
                dispatch: MemberDispatch::default(),
            },
        ],
        doc: doc("service name"),
        is_required: true,
        is_static: false,
        initializer: None,
        span: None,
        attributes: vec![attr("Data")],
        di_inject: None,
        dispatch: MemberDispatch::default(),
        explicit_interface: None,
    };
    let constructor = ConstructorDecl {
        visibility: Visibility::Public,
        kind: ConstructorKind::Designated,
        parameters: vec![Parameter {
            binding: BindingModifier::Value,
            binding_nullable: false,
            name: "endpoint".into(),
            name_span: None,
            ty: TypeExpr::simple("string"),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        }],
        body: None,
        initializer: None,
        doc: doc("ctor"),
        span: None,
        attributes: Vec::new(),
        di_inject: None,
    };
    let mut method = FunctionDecl {
        visibility: Visibility::Public,
        name: "Send".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Ref,
                binding_nullable: true,
                name: "payload".into(),
                name_span: None,
                ty: TypeExpr::simple("string"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("bool"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: None,
        is_async: true,
        is_constexpr: false,
        doc: doc("send payload"),
        modifiers: vec!["async".into()],
        is_unsafe: false,
        attributes: vec![attr("Rpc")],
        is_extern: false,
        extern_abi: None,
        extern_options: None,
        link_name: None,
        link_library: None,
        operator: None,
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    };
    method.signature.throws = Some(ThrowsClause::new(vec![TypeExpr::simple("Error")], None));
    let const_member = ConstMemberDecl {
        visibility: Visibility::Public,
        modifiers: vec!["readonly".into()],
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![ConstDeclarator {
                name: "Timeout".into(),
                initializer: Expression::new("30", None),
                span: None,
            }],
            doc: doc("default timeout"),
            span: None,
        },
    };
    let mut class_generics = GenericParams::new(None, vec![GenericParam::type_param("T", None)]);
    if let Some(data) = class_generics.params[0].as_type_mut() {
        data.constraints
            .push(GenericConstraint::new(GenericConstraintKind::Class, None));
        data.constraints.push(GenericConstraint::new(
            GenericConstraintKind::AutoTrait(AutoTraitConstraint::ThreadSafe),
            None,
        ));
    }
    let class = ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Service".into(),
        bases: vec![TypeExpr::simple("BaseService")],
        members: vec![
            ClassMember::Field(service_field),
            ClassMember::Property(property),
            ClassMember::Constructor(constructor),
            ClassMember::Method(method),
            ClassMember::Const(const_member.clone()),
        ],
        nested_types: Vec::new(),
        thread_safe_override: Some(true),
        shareable_override: None,
        copy_override: None,
        doc: doc("service class"),
        generics: Some(class_generics),
        attributes: vec![attr("Service")],
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    };

    let trait_decl = TraitDecl {
        visibility: Visibility::Public,
        name: "IService".into(),
        super_traits: vec![TypeExpr::simple("IDisposable")],
        members: vec![
            TraitMember::Method(FunctionDecl {
                visibility: Visibility::Public,
                name: "Execute".into(),
                name_span: None,
                signature: Signature {
                    parameters: Vec::new(),
                    return_type: TypeExpr::simple("void"),
                    lends_to_return: None,
                    variadic: false,
                    throws: None,
                },
                body: None,
                is_async: false,
                is_constexpr: false,
                doc: None,
                modifiers: Vec::new(),
                is_unsafe: false,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_name: None,
                link_library: None,
                operator: None,
                generics: None,
                vectorize_hint: None,
                dispatch: MemberDispatch::default(),
            }),
            TraitMember::AssociatedType(TraitAssociatedType {
                name: "Output".into(),
                generics: None,
                default: Some(TypeExpr::simple("string")),
                doc: doc("result type"),
                span: None,
            }),
            TraitMember::Const(const_member.clone()),
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: doc("service contract"),
        generics: None,
        attributes: vec![attr("Contract")],
        span: None,
    };

    let impl_decl = ImplDecl {
        visibility: Visibility::Public,
        trait_ref: Some(TypeExpr::simple("IService")),
        target: TypeExpr::simple("Service"),
        generics: None,
        members: vec![
            ImplMember::Method(FunctionDecl {
                visibility: Visibility::Public,
                name: "Execute".into(),
                name_span: None,
                signature: Signature {
                    parameters: Vec::new(),
                    return_type: TypeExpr::simple("void"),
                    lends_to_return: None,
                    variadic: false,
                    throws: None,
                },
                body: None,
                is_async: false,
                is_constexpr: false,
                doc: None,
                modifiers: Vec::new(),
                is_unsafe: false,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_name: None,
                link_library: None,
                operator: None,
                generics: None,
                vectorize_hint: None,
                dispatch: MemberDispatch::default(),
            }),
            ImplMember::AssociatedType(TraitAssociatedType {
                name: "Output".into(),
                generics: None,
                default: Some(TypeExpr::simple("int")),
                doc: None,
                span: None,
            }),
            ImplMember::Const(const_member.clone()),
        ],
        doc: None,
        attributes: Vec::new(),
        span: None,
    };

    let extension_decl = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Service"),
        generics: None,
        members: vec![ExtensionMember::Method(ExtensionMethodDecl {
            function: FunctionDecl {
                visibility: Visibility::Public,
                name: "Decorate".into(),
                name_span: None,
                signature: Signature {
                    parameters: vec![Parameter {
                        binding: BindingModifier::Value,
                        binding_nullable: false,
                        name: "this".into(),
                        name_span: None,
                        ty: TypeExpr::simple("Self"),
                        attributes: Vec::new(),
                        di_inject: None,
                        default: None,
                        default_span: None,
                        lends: None,
                        is_extension_this: true,
                    }],
                    return_type: TypeExpr::simple("Service"),
                    lends_to_return: None,
                    variadic: false,
                    throws: None,
                },
                body: None,
                is_async: false,
                is_constexpr: false,
                doc: None,
                modifiers: Vec::new(),
                is_unsafe: false,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_name: None,
                link_library: None,
                operator: None,
                generics: None,
                vectorize_hint: None,
                dispatch: MemberDispatch::default(),
            },
            is_default: true,
        })],
        doc: doc("extension helpers"),
        attributes: vec![attr("Extension")],
        conditions: vec![ExtensionCondition {
            target: TypeExpr::simple("Service"),
            constraint: TypeExpr::simple("IDecorator"),
            span: None,
        }],
    };

    let const_item = ConstItemDecl {
        visibility: Visibility::Public,
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![
                ConstDeclarator {
                    name: "Meaning".into(),
                    initializer: Expression::new("42", None),
                    span: None,
                },
                ConstDeclarator {
                    name: "RetryCount".into(),
                    initializer: Expression::new("3", None),
                    span: None,
                },
            ],
            doc: doc("global constants"),
            span: None,
        },
    };

    let namespace = NamespaceDecl {
        name: "Outer.Inner".into(),
        items: vec![
            Item::Class(class),
            Item::Trait(trait_decl),
            Item::Impl(impl_decl),
            Item::Extension(extension_decl),
            Item::Const(const_item),
        ],
        doc: None,
        attributes: Vec::new(),
        span: None,
    };

    let mut module = Module::new(Some("Root".into()));
    module.push_item(Item::Namespace(namespace));

    let tables = DescriptorQuery::collect(&module);
    let service = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Outer::Inner::Service")
        .expect("class descriptor missing");
    assert_eq!(service.generic_arguments.len(), 1);
    assert!(
        service
            .members
            .iter()
            .any(|member| member.kind == MemberKind::Constructor)
    );
    let property_member = service
        .members
        .iter()
        .find(|member| member.kind == MemberKind::Property)
        .expect("property missing");
    let property = property_member
        .property
        .as_ref()
        .expect("property descriptor");
    assert!(property.has_getter);
    assert!(property.has_setter);

    let trait_descriptor = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Outer::Inner::IService")
        .expect("trait descriptor missing");
    assert_eq!(
        trait_descriptor.members.len(),
        3,
        "trait members should include methods, associated types, and consts"
    );

    let impl_descriptor = tables
        .types
        .iter()
        .find(|ty| ty.kind == TypeKind::Impl)
        .expect("impl descriptor missing");
    assert!(
        impl_descriptor.name.contains("IService"),
        "impl descriptor should include trait name"
    );

    let extension = tables
        .types
        .iter()
        .find(|ty| ty.kind == TypeKind::Extension)
        .expect("extension descriptor missing");
    assert!(
        extension.name.contains("Extension<"),
        "extension descriptor should be named after target"
    );

    let const_descriptor = tables
        .types
        .iter()
        .find(|ty| ty.kind == TypeKind::Const && ty.name.ends_with("RetryCount"))
        .expect("const descriptor missing");
    assert_eq!(
        const_descriptor
            .underlying_type
            .as_ref()
            .map(|ty| ty.name.as_str()),
        Some("int")
    );
}
