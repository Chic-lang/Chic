use crate::frontend::ast::{
    BindingModifier, Block, ClassDecl, ClassMember, EnumDecl, ExtensionDecl, ExtensionMember,
    ExtensionMethodDecl, FunctionDecl, ImplDecl, ImplMember, Item, MemberDispatch, OperatorDecl,
    OperatorKind, Parameter, Signature, Statement, StatementKind, StaticDeclaration,
    StaticDeclarator, StaticItemDecl, StaticMutability, StructDecl, TypeExpr, VariableDeclaration,
    VariableDeclarator, VariableModifier, Visibility,
};
use crate::frontend::diagnostics::Diagnostic;

use super::model::MacroInvocation;
use super::registry::{
    AttributeInput, AttributeOutput, AttributeTarget, DeriveInput, DeriveOutput, DeriveTarget,
};

pub fn derive_equatable(input: DeriveInput<'_>) -> DeriveOutput {
    match input.target {
        DeriveTarget::Struct(strct) => derive_equatable_for_struct(strct, input.invocation),
        DeriveTarget::Class(class) => derive_equatable_for_class(class, input.invocation),
        DeriveTarget::Enum(enm) => derive_equatable_for_enum(enm, input.invocation),
    }
}

pub fn derive_hashable(input: DeriveInput<'_>) -> DeriveOutput {
    match input.target {
        DeriveTarget::Struct(strct) => derive_hashable_for_struct(strct, input.invocation),
        DeriveTarget::Class(class) => derive_hashable_for_class(class, input.invocation),
        DeriveTarget::Enum(_enm) => DeriveOutput {
            new_items: Vec::new(),
            diagnostics: vec![Diagnostic::error(
                "`@derive(Hashable)` is not supported on enums yet",
                input.invocation.span,
            )],
        },
    }
}

pub fn derive_clone(input: DeriveInput<'_>) -> DeriveOutput {
    match input.target {
        DeriveTarget::Struct(strct) => derive_clone_for_struct(strct, input.invocation),
        DeriveTarget::Class(class) => derive_clone_for_class(class, input.invocation),
        DeriveTarget::Enum(_enm) => DeriveOutput {
            new_items: Vec::new(),
            diagnostics: vec![Diagnostic::error(
                "`@derive(Clone)` is not supported on enums yet",
                input.invocation.span,
            )],
        },
    }
}

fn derive_clone_for_struct(strct: &mut StructDecl, invocation: &MacroInvocation) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if strct
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Clone)` does not currently support generic structs",
            invocation.span,
        ));
        return output;
    }

    let fields = collect_struct_fields(strct);
    if fields.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Clone)` requires at least one public field or property",
            invocation.span,
        ));
        return output;
    }

    let impl_decl = build_clone_impl(&strct.name, convert_visibility(strct.visibility), &fields);
    output.new_items.push(Item::Impl(impl_decl));
    output
}

fn derive_clone_for_class(class: &mut ClassDecl, invocation: &MacroInvocation) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if class
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Clone)` does not currently support generic classes",
            invocation.span,
        ));
        return output;
    }

    if !class.bases.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Clone)` does not yet support classes with base types",
            invocation.span,
        ));
        return output;
    }

    let accessors = collect_class_accessors(class);
    if accessors.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Clone)` requires at least one public field or property",
            invocation.span,
        ));
        return output;
    }

    let impl_decl = build_clone_impl(
        &class.name,
        convert_visibility(class.visibility),
        &accessors,
    );
    output.new_items.push(Item::Impl(impl_decl));
    output
}

fn build_clone_impl(type_name: &str, visibility: Visibility, fields: &[String]) -> ImplDecl {
    let initializer = build_clone_expression(type_name, fields);
    ImplDecl {
        visibility,
        trait_ref: Some(TypeExpr::simple("Std.Clone")),
        target: TypeExpr::simple(type_name),
        generics: None,
        members: vec![ImplMember::Method(FunctionDecl {
            visibility,
            name: "Clone".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::In,
                    binding_nullable: false,
                    name: "this".into(),
                    name_span: None,
                    ty: TypeExpr::self_type(),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: true,
                }],
                return_type: TypeExpr::self_type(),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: Some(Block {
                statements: vec![make_return_statement(initializer)],
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
        })],
        doc: None,
        attributes: Vec::new(),
        span: None,
    }
}

fn build_clone_expression(type_name: &str, fields: &[String]) -> String {
    let assignments: Vec<String> = fields
        .iter()
        .map(|field| format!("{field} = Std.Clone.Runtime.CloneField(this.{field})"))
        .collect();
    if assignments.is_empty() {
        format!("new {type_name}()")
    } else {
        format!("new {type_name} {{ {} }}", assignments.join(", "))
    }
}

pub fn memoize_attribute(input: AttributeInput<'_>) -> AttributeOutput {
    match input.target {
        AttributeTarget::Function(func) => memoize_function(func, input.invocation),
        AttributeTarget::Method { .. } => AttributeOutput {
            new_items: Vec::new(),
            diagnostics: vec![Diagnostic::error(
                "`@memoize` on methods is not supported yet",
                input.invocation.span,
            )],
        },
    }
}

fn memoize_function(func: &mut FunctionDecl, invocation: &MacroInvocation) -> AttributeOutput {
    let mut output = AttributeOutput::empty();
    if func.body.is_none() {
        output.diagnostics.push(Diagnostic::error(
            "`@memoize` requires a function body",
            invocation.span,
        ));
        return output;
    }
    if func.signature.return_type.name.eq_ignore_ascii_case("void") {
        output.diagnostics.push(Diagnostic::error(
            "`@memoize` cannot be applied to functions that return void",
            invocation.span,
        ));
        return output;
    }
    if !func.signature.parameters.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@memoize` currently supports parameterless functions only",
            invocation.span,
        ));
        return output;
    }

    let hygiene = invocation.hygiene.value();
    let has_flag = format!("__memoized_{}_has_{hygiene}", func.name);
    let cached_value = format!("__memoized_{}_value_{hygiene}", func.name);
    let impl_name = format!("__memoized_impl_{}_{}", func.name, hygiene);
    let result_name = format!("__memo_result_{hygiene}");

    let cached_flag_static = Item::Static(StaticItemDecl {
        visibility: Visibility::Private,
        declaration: StaticDeclaration {
            mutability: StaticMutability::Mutable,
            ty: TypeExpr::simple("bool"),
            declarators: vec![StaticDeclarator {
                name: has_flag.clone(),
                initializer: Some(crate::frontend::ast::Expression::new("false", None)),
                span: invocation.span,
            }],
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_library: None,
            is_weak_import: false,
            doc: None,
            span: invocation.span,
        },
    });

    let cached_value_static = Item::Static(StaticItemDecl {
        visibility: Visibility::Private,
        declaration: StaticDeclaration {
            mutability: StaticMutability::Mutable,
            ty: func.signature.return_type.clone(),
            declarators: vec![StaticDeclarator {
                name: cached_value.clone(),
                initializer: None,
                span: invocation.span,
            }],
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_library: None,
            is_weak_import: false,
            doc: None,
            span: invocation.span,
        },
    });

    let mut helper = func.clone();
    helper.name = impl_name.clone();
    helper.visibility = Visibility::Private;
    helper.attributes.clear();
    helper.doc = None;

    let original_body = func.body.take().unwrap();
    helper.body = Some(original_body.clone());

    let mut new_statements = Vec::new();
    new_statements.push(Statement::new(
        None,
        StatementKind::If(crate::frontend::ast::expressions::IfStatement {
            condition: crate::frontend::ast::Expression::new(has_flag.clone(), None),
            then_branch: Box::new(Statement::new(
                None,
                StatementKind::Return {
                    expression: Some(crate::frontend::ast::Expression::new(
                        cached_value.clone(),
                        None,
                    )),
                },
            )),
            else_branch: None,
        }),
    ));

    let call_expr = crate::frontend::ast::Expression::new(format!("{impl_name}()"), None);
    let result_decl = VariableDeclaration {
        modifier: VariableModifier::Let,
        type_annotation: None,
        declarators: vec![VariableDeclarator {
            name: result_name.clone(),
            initializer: Some(call_expr),
        }],
        is_pinned: false,
    };
    new_statements.push(Statement::new(
        None,
        StatementKind::VariableDeclaration(result_decl),
    ));

    new_statements.push(Statement::new(
        None,
        StatementKind::Expression(crate::frontend::ast::Expression::new(
            format!("{cached_value} = {result_name}"),
            None,
        )),
    ));
    new_statements.push(Statement::new(
        None,
        StatementKind::Expression(crate::frontend::ast::Expression::new(
            format!("{has_flag} = true"),
            None,
        )),
    ));
    new_statements.push(Statement::new(
        None,
        StatementKind::Return {
            expression: Some(crate::frontend::ast::Expression::new(
                result_name.clone(),
                None,
            )),
        },
    ));

    func.body = Some(Block {
        statements: new_statements,
        span: None,
    });

    output.new_items.extend([
        cached_flag_static,
        cached_value_static,
        Item::Function(helper),
    ]);
    output
}

/// No-op attribute used to whitelist compiler-recognised annotations.
pub fn noop_attribute(_input: AttributeInput<'_>) -> AttributeOutput {
    AttributeOutput::empty()
}

fn derive_equatable_for_struct(
    strct: &mut StructDecl,
    invocation: &MacroInvocation,
) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if strct
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Equatable)` does not currently support generic structs",
            invocation.span,
        ));
        return output;
    }

    let type_name = strct.name.clone();
    let fields = collect_struct_fields(strct);
    let extension =
        build_equatable_extension(&type_name, convert_visibility(strct.visibility), &fields);
    output.new_items.push(Item::Extension(extension));
    output
}

fn derive_equatable_for_class(class: &mut ClassDecl, invocation: &MacroInvocation) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if class
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Equatable)` does not currently support generic classes",
            invocation.span,
        ));
        return output;
    }

    if !class.bases.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Equatable)` does not yet support classes with base types",
            invocation.span,
        ));
        return output;
    }

    let accessors = collect_class_accessors(class);
    if accessors.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Equatable)` requires at least one public field or property",
            invocation.span,
        ));
        return output;
    }

    let extension = build_equatable_extension(
        &class.name,
        convert_visibility(class.visibility),
        &accessors,
    );
    output.new_items.push(Item::Extension(extension));
    output
}

fn derive_equatable_for_enum(_enm: &mut EnumDecl, invocation: &MacroInvocation) -> DeriveOutput {
    DeriveOutput {
        new_items: Vec::new(),
        diagnostics: vec![Diagnostic::error(
            "`@derive(Equatable)` is not supported on enums yet",
            invocation.span,
        )],
    }
}

fn derive_hashable_for_struct(
    strct: &mut StructDecl,
    invocation: &MacroInvocation,
) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if strct
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Hashable)` does not currently support generic structs",
            invocation.span,
        ));
        return output;
    }

    let fields = collect_struct_fields(strct);
    let extension =
        build_hashable_extension(&strct.name, convert_visibility(strct.visibility), &fields);
    output.new_items.push(Item::Extension(extension));
    output
}

fn derive_hashable_for_class(class: &mut ClassDecl, invocation: &MacroInvocation) -> DeriveOutput {
    let mut output = DeriveOutput::empty();
    if class
        .generics
        .as_ref()
        .is_some_and(|params| !params.params.is_empty())
    {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Hashable)` does not currently support generic classes",
            invocation.span,
        ));
        return output;
    }

    if !class.bases.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Hashable)` does not yet support classes with base types",
            invocation.span,
        ));
        return output;
    }

    let accessors = collect_class_accessors(class);
    if accessors.is_empty() {
        output.diagnostics.push(Diagnostic::error(
            "`@derive(Hashable)` requires at least one public field or property",
            invocation.span,
        ));
        return output;
    }

    let extension = build_hashable_extension(
        &class.name,
        convert_visibility(class.visibility),
        &accessors,
    );
    output.new_items.push(Item::Extension(extension));
    output
}

fn collect_struct_fields(strct: &StructDecl) -> Vec<String> {
    strct
        .fields
        .iter()
        .map(|field| field.name.clone())
        .collect()
}

fn collect_class_accessors(class: &ClassDecl) -> Vec<String> {
    let mut accessors = Vec::new();
    for member in &class.members {
        match member {
            ClassMember::Field(field) if matches!(field.visibility, Visibility::Public) => {
                accessors.push(field.name.clone());
            }
            ClassMember::Property(prop) if matches!(prop.visibility, Visibility::Public) => {
                accessors.push(prop.name.clone());
            }
            _ => {}
        }
    }
    accessors
}

pub(super) fn build_equatable_extension(
    type_name: &str,
    visibility: Visibility,
    fields: &[String],
) -> ExtensionDecl {
    let mut equality_terms: Vec<String> = fields
        .iter()
        .map(|field| format!("left.{field} == right.{field}"))
        .collect();
    if equality_terms.is_empty() {
        equality_terms.push("true".to_string());
    }
    let equality_expression = equality_terms.join(" && ");

    let equality_method = FunctionDecl {
        visibility,
        name: "op_Equality".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "left".into(),
                    name_span: None,
                    ty: TypeExpr::simple(type_name),
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
                    ty: TypeExpr::simple(type_name),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("bool"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![make_return_statement(equality_expression)],
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
        operator: Some(OperatorDecl {
            kind: OperatorKind::Binary(crate::frontend::ast::BinaryOperator::Equal),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    };

    let inequality_method = FunctionDecl {
        visibility,
        name: "op_Inequality".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "left".into(),
                    name_span: None,
                    ty: TypeExpr::simple(type_name),
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
                    ty: TypeExpr::simple(type_name),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("bool"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![make_return_statement(
                "!op_Equality(left, right)".to_string(),
            )],
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
        operator: Some(OperatorDecl {
            kind: OperatorKind::Binary(crate::frontend::ast::BinaryOperator::NotEqual),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    };

    ExtensionDecl {
        visibility,
        target: extension_target(type_name),
        generics: None,
        members: vec![
            ExtensionMember::Method(ExtensionMethodDecl {
                function: equality_method,
                is_default: false,
            }),
            ExtensionMember::Method(ExtensionMethodDecl {
                function: inequality_method,
                is_default: false,
            }),
        ],
        doc: None,
        attributes: Vec::new(),
        conditions: Vec::new(),
    }
}

pub(super) fn build_hashable_extension(
    type_name: &str,
    visibility: Visibility,
    fields: &[String],
) -> ExtensionDecl {
    let mut terms: Vec<String> = fields
        .iter()
        .map(|field| format!("value.{field}.GetHashCode()"))
        .collect();
    if terms.is_empty() {
        terms.push("0".to_string());
    }
    let body_expression = terms.join(" ^ ");

    let method = FunctionDecl {
        visibility,
        name: "GetHashCode".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::In,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple(type_name),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: true,
            }],
            return_type: TypeExpr::simple("int"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![make_return_statement(body_expression)],
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

    ExtensionDecl {
        visibility,
        target: extension_target(type_name),
        generics: None,
        members: vec![ExtensionMember::Method(ExtensionMethodDecl {
            function: method,
            is_default: false,
        })],
        doc: None,
        attributes: Vec::new(),
        conditions: Vec::new(),
    }
}

fn extension_target(name: &str) -> TypeExpr {
    TypeExpr::simple(name)
}

fn make_return_statement(expr: String) -> Statement {
    Statement::new(
        None,
        StatementKind::Return {
            expression: Some(crate::frontend::ast::Expression::new(expr, None)),
        },
    )
}

fn convert_visibility(visibility: Visibility) -> Visibility {
    visibility
}

#[cfg(test)]
pub(crate) fn macro_attribute(name: &str, args: &[&str]) -> crate::frontend::ast::Attribute {
    let arguments = args
        .iter()
        .map(|value| crate::frontend::ast::AttributeArgument::new(None, *value, None))
        .collect::<Vec<_>>();
    crate::frontend::ast::Attribute::new(
        name,
        arguments,
        None,
        None,
        crate::frontend::ast::AttributeKind::Macro,
    )
    .with_macro_metadata(crate::frontend::ast::AttributeMacroMetadata::new(
        true,
        Vec::new(),
    ))
}

#[cfg(test)]
pub(crate) fn struct_with_fields(name: &str, fields: &[(&str, &str)]) -> StructDecl {
    StructDecl {
        visibility: Visibility::Public,
        name: name.into(),
        fields: fields
            .iter()
            .map(|(field_name, field_ty)| crate::frontend::ast::FieldDecl {
                visibility: Visibility::Public,
                name: (*field_name).into(),
                ty: TypeExpr::simple(*field_ty),
                initializer: None,
                attributes: Vec::new(),
                mmio: None,
                doc: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            })
            .collect(),
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
    }
}

#[cfg(test)]
pub(crate) fn make_return_statement_block(expr: &str) -> Block {
    Block {
        statements: vec![make_return_statement(expr.to_string())],
        span: None,
    }
}
