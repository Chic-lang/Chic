use super::helpers::*;

#[test]
fn variance_keywords_emit_in_generics_descriptors() {
    let mut module = Module::new(Some("Root".into()));
    let mut result_param = GenericParam::type_param("TResult", None);
    result_param.as_type_mut().expect("type param").variance = Variance::Covariant;
    let mut argument_param = GenericParam::type_param("TArgument", None);
    argument_param.as_type_mut().expect("type param").variance = Variance::Contravariant;
    let interface = InterfaceDecl {
        visibility: Visibility::Public,
        name: "IFunction".into(),
        bases: Vec::new(),
        members: vec![InterfaceMember::Method(FunctionDecl {
            visibility: Visibility::Public,
            name: "Invoke".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::In,
                    binding_nullable: false,
                    name: "value".into(),
                    name_span: None,
                    ty: TypeExpr::simple("TArgument"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                }],
                return_type: TypeExpr::simple("TResult"),
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
        })],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: Some(GenericParams::new(None, vec![result_param, argument_param])),
        attributes: Vec::new(),
    };
    module.push_item(Item::Interface(interface));

    let tables = DescriptorQuery::collect(&module);
    let iface = tables
        .types
        .iter()
        .find(|ty| ty.name == "Root::IFunction")
        .expect("missing interface descriptor");
    assert_eq!(
        iface
            .generic_arguments
            .iter()
            .map(|arg| arg.name.clone())
            .collect::<Vec<_>>(),
        vec!["out TResult".to_string(), "in TArgument".to_string()],
        "variance keywords should surface in the metadata generics list"
    );
}
