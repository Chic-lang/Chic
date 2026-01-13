use super::helpers::*;

#[test]
fn non_public_items_are_skipped() {
    let mut module = Module::new(Some("Root".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Private,
        name: "Hidden".into(),
        fields: Vec::new(),
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
        is_readonly: false,
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));
    module.push_item(Item::Function(FunctionDecl {
        visibility: Visibility::Private,
        name: "HiddenFn".into(),
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
    }));

    let tables = DescriptorQuery::collect(&module);
    assert!(
        tables.types.is_empty(),
        "non-public items should not emit descriptors: {:?}",
        tables.types
    );
}
