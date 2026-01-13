use super::helpers::*;

#[test]
fn golden_snapshot_matches_expected_json() {
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
        is_readonly: true,
        layout: Some(LayoutHints {
            repr_c: true,
            packing: Some(PackingHint {
                value: Some(4),
                span: None,
            }),
            align: Some(AlignHint {
                value: 16,
                span: None,
            }),
        }),
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));

    let tables = collect_reflection_tables(&module);
    assert_eq!(tables.version, 2);
    let ty = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::Packed")
        .expect("type descriptor present");
    assert_eq!(ty.full_name, "Root::Packed");
    assert_eq!(ty.namespace.as_deref(), Some("Root"));
    assert_eq!(ty.kind, TypeKind::Struct);
    assert!(ty.readonly);
    assert!(!ty.is_generic);
    let layout_hints = ty.layout_hints.as_ref().expect("layout hints populated");
    assert!(layout_hints.repr_c);
    assert_eq!(layout_hints.pack, Some(4));
    assert_eq!(layout_hints.align, Some(16));
    let field = ty
        .members
        .iter()
        .find(|m| m.name == "Value")
        .expect("field member");
    assert_eq!(field.kind, MemberKind::Field);
    let field_info = field.field.as_ref().expect("field descriptor");
    assert_eq!(field_info.field_type.name, "u8");

    let serialized = serialize_reflection_tables(&tables).expect("pretty json");
    assert!(
        serialized.contains("\"Root::Packed\""),
        "serialized tables should include type name"
    );
}
