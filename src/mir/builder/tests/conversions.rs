use super::*;
use crate::frontend::ast::{
    BindingModifier, Block, ClassDecl, ClassKind, ClassMember, ConversionKind, Expression,
    FunctionDecl, Item, MemberDispatch, Module as AstModule, Parameter, Signature, Statement,
    StatementKind, TypeExpr, Visibility,
};
use crate::syntax::expr::{CastSyntax, ExprNode};

fn conversion_name(kind: ConversionKind, target: &str) -> String {
    let fragment = target
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    match kind {
        ConversionKind::Implicit => format!("op_Implicit_{fragment}"),
        ConversionKind::Explicit => format!("op_Explicit_{fragment}"),
    }
}

fn conversion_method(
    _owner: &str,
    kind: ConversionKind,
    source: &str,
    target: &str,
) -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: conversion_name(kind, target),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple(source),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple(target),
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
        operator: Some(crate::frontend::ast::OperatorDecl {
            kind: crate::frontend::ast::OperatorKind::Conversion(kind),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn value_class(include_implicit: bool, include_explicit: bool) -> ClassDecl {
    let mut members = Vec::new();
    if include_implicit {
        members.push(ClassMember::Method(conversion_method(
            "Value",
            ConversionKind::Implicit,
            "Input",
            "Value",
        )));
    }
    if include_explicit {
        members.push(ClassMember::Method(conversion_method(
            "Value",
            ConversionKind::Explicit,
            "Input",
            "Value",
        )));
    }
    ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Value".into(),
        bases: Vec::new(),
        members,
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }
}

fn input_class(include_conversion: bool) -> ClassDecl {
    let mut members = Vec::new();
    if include_conversion {
        members.push(ClassMember::Method(conversion_method(
            "Input",
            ConversionKind::Implicit,
            "Input",
            "Value",
        )));
    }
    ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Input".into(),
        bases: Vec::new(),
        members,
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        attributes: Vec::new(),
        generics: None,
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    }
}

fn return_function(name: &str, expr: Expression) -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: name.into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "input".into(),
                name_span: None,
                ty: TypeExpr::simple("Input"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("Value"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Return {
                    expression: Some(expr),
                },
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
        operator: None,
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn identifier_expr(name: &str) -> Expression {
    Expression::with_node(name, None, ExprNode::Identifier(name.into()))
}

fn cast_expr(target: &str, operand: &str) -> Expression {
    Expression::with_node(
        format!("({target}){operand}"),
        None,
        ExprNode::Cast {
            target: target.into(),
            expr: Box::new(ExprNode::Identifier(operand.into())),
            syntax: CastSyntax::Paren,
        },
    )
}

#[test]
fn lowers_implicit_conversion_on_return() {
    let mut module = AstModule::new(Some("Numbers".into()));
    module.items.push(Item::Class(value_class(true, false)));
    module.items.push(Item::Class(input_class(false)));
    module.items.push(Item::Function(return_function(
        "ConvertImplicit",
        identifier_expr("input"),
    )));

    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Numbers::ConvertImplicit")
        .expect("missing ConvertImplicit function");

    let call = func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, .. }) => Some(func),
            _ => None,
        })
        .expect("expected conversion call");

    let pending = match call {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand for conversion call, found {other:?}"),
    };
    assert_eq!(
        pending.repr, "Numbers::Value::op_Implicit_Value",
        "expected implicit conversion call"
    );
}

#[test]
fn lowers_explicit_conversion_for_cast() {
    let mut module = AstModule::new(Some("Numbers".into()));
    module.items.push(Item::Class(value_class(false, true)));
    module.items.push(Item::Class(input_class(false)));
    module.items.push(Item::Function(return_function(
        "ConvertExplicit",
        cast_expr("Value", "input"),
    )));

    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        lowering.diagnostics
    );

    let func = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Numbers::ConvertExplicit")
        .expect("missing ConvertExplicit function");

    let call = func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call { func, .. }) => Some(func),
            _ => None,
        })
        .expect("expected conversion call");

    let pending = match call {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand for conversion call, found {other:?}"),
    };
    assert_eq!(
        pending.repr, "Numbers::Value::op_Explicit_Value",
        "expected explicit conversion call"
    );
}

#[test]
fn reports_missing_implicit_conversion_without_cast() {
    let mut module = AstModule::new(Some("Numbers".into()));
    module.items.push(Item::Class(value_class(false, true)));
    module.items.push(Item::Class(input_class(false)));
    module.items.push(Item::Function(return_function(
        "ConvertImplicit",
        identifier_expr("input"),
    )));

    let lowering = lower_module(&module);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("requires a cast")),
        "expected cast-required diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn reports_ambiguous_conversions_when_multiple_exist() {
    let mut module = AstModule::new(Some("Numbers".into()));
    module.items.push(Item::Class(value_class(true, false)));
    module.items.push(Item::Class(input_class(true)));
    module.items.push(Item::Function(return_function(
        "ConvertImplicit",
        identifier_expr("input"),
    )));

    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("ambiguous conversion from `Numbers::Input` to `Numbers::Value`")),
        "expected ambiguous conversion diagnostic, found {:?}",
        lowering.diagnostics
    );
}
