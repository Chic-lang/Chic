use super::helpers::*;

#[test]
fn collects_public_static_items() {
    let module = Module::with_namespace_items(
        Some("Demo".into()),
        None,
        Vec::new(),
        Vec::new(),
        vec![Item::Static(StaticItemDecl {
            visibility: Visibility::Public,
            declaration: StaticDeclaration {
                mutability: StaticMutability::Const,
                ty: TypeExpr::simple("int"),
                declarators: vec![StaticDeclarator {
                    name: "Answer".into(),
                    initializer: Some(Expression::new("42", None)),
                    span: None,
                }],
                attributes: vec![attr("cold")],
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_library: None,
                is_weak_import: false,
                doc: doc("public answer"),
                span: None,
            },
        })],
    );

    let tables = collect_reflection_tables(&module);
    let static_ty = tables
        .types
        .iter()
        .find(|ty| ty.name == "Demo::Answer")
        .expect("missing static descriptor");
    assert_eq!(static_ty.kind, TypeKind::Static);
    assert_eq!(static_ty.visibility, VisibilityDescriptor::Public);
    assert_eq!(
        static_ty
            .underlying_type
            .as_ref()
            .map(|ty| ty.name.as_str()),
        Some("int")
    );
    assert!(static_ty.readonly);
    assert!(static_ty.attributes.iter().any(|attr| attr.name == "cold"));
}
