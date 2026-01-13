#![cfg(test)]

use crate::const_eval_config::{self, ConstEvalConfig};
use crate::frontend::ast::expressions::Expression;
use crate::frontend::ast::{
    BindingModifier, ClassDecl, ClassKind, ClassMember, ConstructorDecl, ConstructorKind,
    FieldDecl, FunctionDecl, GenericArgument, GenericConstraint, GenericConstraintKind,
    GenericParam, GenericParams, ImportDirective, ImportKind, Item, MemberDispatch, Module,
    NamespaceDecl, OperatorDecl, OperatorKind, Parameter, Signature, StructDecl, TypeExpr,
    TypeSuffix, Visibility,
};
use crate::frontend::parser::parse_module;
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr,
    lower_module,
};
use crate::typeck::arena::{
    AutoTraitConstraintOrigin, AutoTraitKind, ConstraintKind, TypeCheckResult, TypeConstraint,
    check_module,
};

pub(super) fn make_type_expr(name: &str) -> TypeExpr {
    TypeExpr::simple(name)
}

pub(super) fn module_with_imports(use_alias: bool) -> Module {
    let mut items = Vec::new();
    items.push(Item::Import(ImportDirective {
        doc: None,
        is_global: false,
        span: None,
        kind: ImportKind::Namespace {
            path: "Alpha".to_string(),
        },
    }));
    if use_alias {
        items.push(Item::Import(ImportDirective {
            doc: None,
            is_global: false,
            span: None,
            kind: ImportKind::Alias {
                alias: "Alias".to_string(),
                target: "Alpha".to_string(),
            },
        }));
    }
    let mut alpha_items = Vec::new();
    alpha_items.push(Item::Struct(StructDecl {
        visibility: Visibility::Public,
        name: "Widget".to_string(),
        fields: vec![FieldDecl {
            visibility: Visibility::Public,
            name: "Value".to_string(),
            ty: make_type_expr("int"),
            initializer: None,
            mmio: None,
            doc: None,
            is_required: false,
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
        layout: None,
        is_intrinsic: false,
        inline_attr: None,
        is_record: false,
        record_positional_fields: Vec::new(),
    }));
    items.push(Item::Namespace(NamespaceDecl {
        name: "Alpha".to_string(),
        items: alpha_items,
        doc: None,
        attributes: Vec::new(),
        span: None,
    }));
    Module::with_namespace_items(None, None, Vec::new(), Vec::new(), items)
}

pub(super) fn parse_and_check(source: &str) -> (Module, TypeCheckResult) {
    const_eval_config::set_global(ConstEvalConfig::default());
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.module;
    let result = check_module(&module, &[], &TypeLayoutTable::default());
    (module, result)
}

pub(super) fn parse_lower_and_check(source: &str) -> (Module, TypeCheckResult) {
    const_eval_config::set_global(ConstEvalConfig::default());
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("parse failed: {:?}", err.diagnostics());
    });
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.module;
    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected lowering diagnostics: {:?}",
        lowering.diagnostics
    );
    let result = check_module(
        &module,
        &lowering.constraints,
        &lowering.module.type_layouts,
    );
    (module, result)
}

pub(super) fn simple_struct_layout(
    name: &str,
    traits: AutoTraitSet,
    overrides: AutoTraitOverride,
) -> TypeLayout {
    TypeLayout::Struct(StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: traits,
        overrides,
        mmio: None,
        dispose: None,
        class: None,
    })
}

pub(super) fn module_with_struct(full_name: &str) -> Module {
    if let Some(idx) = full_name.rfind("::") {
        let namespace = &full_name[..idx];
        let name = &full_name[idx + 2..];
        let mut module = Module::new(Some(namespace.to_string()));
        module.push_item(Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: name.to_string(),
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
            doc: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            mmio: None,
            generics: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }));
        module.rebuild_overloads();
        module
    } else {
        let mut module = Module::new(None);
        module.push_item(Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: full_name.to_string(),
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
            doc: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            mmio: None,
            generics: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }));
        module.rebuild_overloads();
        module
    }
}

pub(super) fn result_contains(result: &TypeCheckResult, needle: &str) -> bool {
    result
        .diagnostics
        .iter()
        .any(|diag| diag.message.contains(needle))
}

pub(super) fn simple_class(
    name: &str,
    members: Vec<ClassMember>,
    generics: Option<GenericParams>,
) -> ClassDecl {
    ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: name.to_string(),
        bases: Vec::new(),
        members,
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }
}

pub(super) fn needs_ctor_generics() -> GenericParams {
    let mut param = GenericParam::type_param("T", None);
    param
        .as_type_mut()
        .expect("type parameter")
        .constraints
        .push(GenericConstraint::new(
            GenericConstraintKind::DefaultConstructor,
            None,
        ));
    GenericParams {
        span: None,
        params: vec![param],
    }
}

pub(super) fn needs_ctor_class() -> ClassDecl {
    simple_class("NeedsCtor", Vec::new(), Some(needs_ctor_generics()))
}

pub(super) fn usage_class(argument: TypeExpr) -> ClassDecl {
    let mut field_ty = TypeExpr::simple("NeedsCtor");
    field_ty.suffixes.push(TypeSuffix::GenericArgs(vec![
        GenericArgument::from_type_expr(argument),
    ]));
    simple_class(
        "Usage",
        vec![ClassMember::Field(FieldDecl {
            visibility: Visibility::Public,
            name: "Field".into(),
            ty: field_ty,
            initializer: None,
            mmio: None,
            doc: None,
            attributes: Vec::new(),
            is_required: false,
            display_name: None,
            is_readonly: false,
            is_static: false,
            view_of: None,
        })],
        None,
    )
}

pub(super) fn const_generic_argument(text: &str) -> GenericArgument {
    GenericArgument::new(None, Expression::new(text.to_string(), None))
}

pub(super) fn parameterless_ctor(visibility: Visibility) -> ClassMember {
    ClassMember::Constructor(ConstructorDecl {
        visibility,
        kind: ConstructorKind::Designated,
        parameters: Vec::new(),
        body: None,
        initializer: None,
        doc: None,
        span: None,
        attributes: Vec::new(),
        di_inject: None,
    })
}

pub(super) fn operator_function(
    kind: OperatorKind,
    param_types: &[&str],
    return_type: &str,
) -> FunctionDecl {
    let params = param_types
        .iter()
        .enumerate()
        .map(|(index, ty)| Parameter {
            binding: BindingModifier::Value,
            binding_nullable: false,
            name: format!("p{index}"),
            name_span: None,
            ty: TypeExpr::simple(*ty),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        })
        .collect::<Vec<_>>();
    FunctionDecl {
        visibility: Visibility::Public,
        name: "Op".into(),
        name_span: None,
        signature: Signature {
            parameters: params,
            return_type: TypeExpr::simple(return_type),
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
        operator: Some(OperatorDecl { kind, span: None }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

pub(super) fn layouts_with_struct(
    name: &str,
    traits: AutoTraitSet,
    overrides: AutoTraitOverride,
) -> TypeLayoutTable {
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        name.to_string(),
        simple_struct_layout(name, traits, overrides),
    );
    layouts
}

pub(super) fn with_constraints<'a>(
    constraints: impl IntoIterator<Item = TypeConstraint>,
    module: &'a Module,
    layouts: &'a TypeLayoutTable,
) -> TypeCheckResult {
    let constraints: Vec<_> = constraints.into_iter().collect();
    check_module(module, &constraints, layouts)
}

pub(super) fn requires_auto_trait_constraint(
    function: &str,
    target: &str,
    ty: &str,
    trait_kind: AutoTraitKind,
) -> TypeConstraint {
    TypeConstraint::new(
        ConstraintKind::RequiresAutoTrait {
            function: function.into(),
            target: target.into(),
            ty: ty.into(),
            trait_kind,
            origin: AutoTraitConstraintOrigin::Generic,
        },
        None,
    )
}
