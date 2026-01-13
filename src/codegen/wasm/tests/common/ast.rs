use crate::frontend::ast::{
    Block as AstBlock, FunctionDecl, Item as AstItem, MemberDispatch, Module as AstModule,
    NamespaceDecl, Signature, TypeExpr, Visibility,
};

pub(crate) fn simple_ast_module_with_main() -> AstModule {
    let mut module = AstModule::new(None);
    let function = FunctionDecl {
        visibility: Visibility::Public,
        name: "Main".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(AstBlock {
            statements: Vec::new(),
            span: None,
        }),
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
    };
    module.push_item(AstItem::Function(function));
    module
}

pub(crate) fn nested_namespace_ast_module() -> AstModule {
    let mut inner = NamespaceDecl {
        name: "Inner".into(),
        items: Vec::new(),
        doc: None,
        attributes: Vec::new(),
        span: None,
    };
    inner.items.push(AstItem::Function(FunctionDecl {
        visibility: Visibility::Public,
        name: "Main".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(AstBlock {
            statements: Vec::new(),
            span: None,
        }),
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

    let mut outer = NamespaceDecl {
        name: "Outer".into(),
        items: Vec::new(),
        doc: None,
        attributes: Vec::new(),
        span: None,
    };
    outer.items.push(AstItem::Namespace(inner));

    let mut module = AstModule::new(None);
    module.push_item(AstItem::Namespace(outer));
    module
}

pub(crate) fn ast_module_without_main() -> AstModule {
    let mut module = AstModule::new(None);
    let helper = FunctionDecl {
        visibility: Visibility::Public,
        name: "Helper".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(AstBlock {
            statements: Vec::new(),
            span: None,
        }),
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
    };
    module.push_item(AstItem::Function(helper));
    module
}
