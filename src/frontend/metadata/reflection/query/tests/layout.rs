use super::helpers::*;

#[test]
fn layout_hints_surface_in_descriptor() {
    let mut module = Module::new(Some("Root".into()));
    module.push_item(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Packed".into(),
        fields: vec![FieldDecl {
            visibility: Visibility::Public,
            name: "Value".into(),
            ty: TypeExpr::simple("u8"),
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
        layout: Some(LayoutHints {
            repr_c: true,
            packing: Some(PackingHint {
                value: Some(2),
                span: None,
            }),
            align: Some(AlignHint {
                value: 8,
                span: None,
            }),
        }),
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));

    let tables = DescriptorQuery::collect(&module);
    let packed = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Packed")
        .expect("packed descriptor missing");
    let hints = packed.layout_hints.as_ref().expect("layout hints");
    assert_eq!(hints.pack, Some(2));
    assert_eq!(hints.align, Some(8));
}
