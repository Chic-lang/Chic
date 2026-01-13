use super::helpers::*;

#[test]
fn collects_public_struct_members() {
    let mut module = Module::new(Some("Root".into()));
    let decl = StructDecl {
        visibility: Visibility::Public,
        name: "Widget".into(),
        fields: vec![FieldDecl {
            visibility: Visibility::Public,
            name: "Id".into(),
            ty: TypeExpr::simple("int"),
            initializer: None,
            mmio: None,
            doc: None,
            is_required: true,
            display_name: None,
            attributes: Vec::new(),
            is_readonly: false,
            is_static: false,
            view_of: None,
        }],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: vec![FunctionDecl {
            visibility: Visibility::Public,
            name: "Create".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "value".into(),
                    name_span: None,
                    ty: TypeExpr::simple("int"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                }],
                return_type: TypeExpr::simple("Widget"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: None,
            is_async: false,
            is_constexpr: false,
            doc: None,
            modifiers: vec!["static".into()],
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: Some(GenericParams::new(
                None,
                vec![GenericParam::type_param("T", None)],
            )),
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        }],
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        mmio: None,
        doc: None,
        generics: Some(GenericParams::new(
            None,
            vec![GenericParam::type_param("T", None)],
        )),
        attributes: vec![attr("Data")],
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    };
    module.push_item(Item::Struct(decl));
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Add".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "left".into(),
                    name_span: None,
                    ty: TypeExpr::simple("int"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "right".into(),
                    name_span: None,
                    ty: TypeExpr::simple("int"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("int"),
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
    }));

    let tables = DescriptorQuery::collect(&module);
    assert_eq!(tables.types.len(), 2);
    let ty = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Widget")
        .expect("missing struct descriptor");
    let generic_names: Vec<_> = ty
        .generic_arguments
        .iter()
        .map(|handle| handle.name.as_str())
        .collect();
    assert_eq!(generic_names, vec!["T"]);
    let attribute_names: Vec<_> = ty
        .attributes
        .iter()
        .map(|attr| attr.name.as_str())
        .collect();
    assert_eq!(attribute_names, vec!["Data"]);
    assert_eq!(ty.members.len(), 2);
    let field = &ty.members[0];
    assert_eq!(field.name, "Id");
    let field_info = field.field.as_ref().expect("field descriptor");
    assert_eq!(field_info.field_type.name, "int");

    let method = &ty.members[1];
    assert_eq!(method.name, "Create");
    let method_info = method.method.as_ref().expect("method descriptor");
    assert!(method_info.is_static);
    let parameter_names: Vec<_> = method_info
        .parameters
        .iter()
        .map(|param| param.name.as_str())
        .collect();
    assert_eq!(parameter_names, vec!["value"]);
    assert_eq!(method_info.parameters[0].parameter_type.name, "int");

    let add = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Add")
        .expect("missing function descriptor");
    assert!(matches!(add.kind, TypeKind::Function));
    let signature = add
        .members
        .iter()
        .find(|member| matches!(member.kind, MemberKind::Method))
        .and_then(|member| member.method.as_ref())
        .expect("function signature missing");
    let parameter_names: Vec<_> = signature
        .parameters
        .iter()
        .map(|param| param.name.as_str())
        .collect();
    assert_eq!(parameter_names, vec!["left", "right"]);
}

#[test]
fn record_structs_are_tagged_with_record_kind() {
    let mut module = Module::new(Some("Geom".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Point".into(),
        fields: vec![
            FieldDecl {
                visibility: Visibility::Public,
                name: "X".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                mmio: None,
                doc: None,
                is_required: true,
                display_name: None,
                attributes: Vec::new(),
                is_readonly: true,
                is_static: false,
                view_of: None,
            },
            FieldDecl {
                visibility: Visibility::Public,
                name: "Y".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                mmio: None,
                doc: None,
                is_required: true,
                display_name: None,
                attributes: Vec::new(),
                is_readonly: true,
                is_static: false,
                view_of: None,
            },
        ],
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: Vec::new(),
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        mmio: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        is_readonly: true,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: true,
        record_positional_fields: vec![
            RecordPositionalField {
                name: "X".into(),
                span: None,
            },
            RecordPositionalField {
                name: "Y".into(),
                span: None,
            },
        ],
    }));

    let tables = DescriptorQuery::collect(&module);
    let descriptor = tables
        .types
        .iter()
        .find(|ty| ty.name == "Geom::Point")
        .expect("missing record descriptor");
    assert!(
        matches!(descriptor.kind, TypeKind::Record),
        "expected record type kind, got {:?}",
        descriptor.kind
    );
    assert!(descriptor.readonly);
    assert_eq!(
        descriptor
            .members
            .iter()
            .filter(|member| matches!(member.kind, MemberKind::Field))
            .count(),
        2
    );
}
