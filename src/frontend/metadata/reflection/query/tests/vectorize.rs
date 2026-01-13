use super::helpers::*;

#[test]
fn vectorize_hint_surfaces_in_metadata_flags() {
    let mut module = Module::new(Some("Root".into()));
    let method = FunctionDecl {
        visibility: Visibility::Public,
        name: "Accumulate".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "lhs".into(),
                    name_span: None,
                    ty: TypeExpr::simple("decimal"),
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
                    name: "rhs".into(),
                    name_span: None,
                    ty: TypeExpr::simple("decimal"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("decimal"),
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
        vectorize_hint: Some(VectorizeHint::Decimal),
        dispatch: MemberDispatch::default(),
    };

    let decl = StructDecl {
        visibility: Visibility::Public,
        name: "Surface".into(),
        fields: Vec::new(),
        properties: Vec::new(),
        constructors: Vec::new(),
        consts: Vec::new(),
        methods: vec![method],
        nested_types: Vec::new(),
        bases: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        mmio: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    };
    module.push_item(Item::Struct(decl));

    let tables = DescriptorQuery::collect(&module);
    let ty = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Surface")
        .expect("missing Surface descriptor");
    let method_descriptor = ty
        .members
        .iter()
        .find(|member| member.name == "Accumulate")
        .expect("missing method descriptor");
    assert!(
        method_descriptor
            .attributes
            .iter()
            .any(|attr| attr.name == "vectorize=decimal"),
        "vectorize flag missing: {:?}",
        method_descriptor.attributes
    );
}
