use super::*;
use crate::frontend::ast::{BinaryOperator, UnaryOperator};
use crate::frontend::ast::{
    BindingModifier, Block, ClassDecl, ClassKind, ClassMember, Expression, ExtensionDecl,
    ExtensionMember, ExtensionMethodDecl, FunctionDecl, Item, MemberDispatch, Module as AstModule,
    OperatorDecl, OperatorKind, Parameter, Signature, Statement, StatementKind, TypeExpr,
    VariableDeclaration, VariableDeclarator, VariableModifier, Visibility,
};
use crate::mir::data::StatementKind as MirStatementKind;

fn build_operator_module(include_extension: bool) -> AstModule {
    let mut module = AstModule::new(Some("Numbers".into()));
    module.items.push(Item::Class(build_my_number_class()));
    if include_extension {
        module
            .items
            .push(Item::Extension(build_my_number_extension()));
    }
    module.items.push(Item::Function(build_add_function()));
    module.items.push(Item::Function(build_negate_function()));
    module
        .items
        .push(Item::Function(build_accumulate_function()));
    module
        .items
        .push(Item::Function(build_prefix_increment_function()));
    module
        .items
        .push(Item::Function(build_postfix_increment_function()));
    module
}

fn build_my_number_class() -> ClassDecl {
    ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "MyNumber".into(),
        bases: Vec::new(),
        members: vec![
            ClassMember::Method(build_binary_operator_method()),
            ClassMember::Method(build_unary_operator_method()),
            ClassMember::Method(build_increment_operator_method()),
        ],
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

fn build_my_number_extension() -> ExtensionDecl {
    ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("MyNumber"),
        generics: None,
        members: vec![ExtensionMember::Method(ExtensionMethodDecl {
            function: build_extension_operator_method(),
            is_default: false,
        })],
        doc: None,
        attributes: Vec::new(),
        conditions: Vec::new(),
    }
}

fn build_binary_operator_method() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "op_Addition".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "lhs".into(),
                    name_span: None,
                    ty: TypeExpr::simple("MyNumber"),
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
                    ty: TypeExpr::simple("MyNumber"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("lhs")],
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
            kind: OperatorKind::Binary(BinaryOperator::Add),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn build_unary_operator_method() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "op_UnaryNegation".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple("MyNumber"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("value")],
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
            kind: OperatorKind::Unary(UnaryOperator::Negate),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn build_increment_operator_method() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "op_Increment".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple("MyNumber"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("value")],
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
            kind: OperatorKind::Unary(UnaryOperator::Increment),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn build_extension_operator_method() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "op_Addition".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "lhs".into(),
                    name_span: None,
                    ty: TypeExpr::simple("MyNumber"),
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
                    ty: TypeExpr::simple("MyNumber"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("lhs")],
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
            kind: OperatorKind::Binary(BinaryOperator::Add),
            span: None,
        }),
        generics: None,
        vectorize_hint: None,
        dispatch: MemberDispatch::default(),
    }
}

fn build_add_function() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "Add".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![
                Parameter {
                    binding: BindingModifier::Value,
                    binding_nullable: false,
                    name: "lhs".into(),
                    name_span: None,
                    ty: TypeExpr::simple("MyNumber"),
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
                    ty: TypeExpr::simple("MyNumber"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("lhs + rhs")],
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

fn build_prefix_increment_function() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "PrefixIncrement".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple("MyNumber"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("++value")],
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

fn build_postfix_increment_function() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "PostfixIncrement".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple("MyNumber"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("value++")],
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

fn build_negate_function() -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: "Negate".into(),
        name_span: None,
        signature: Signature {
            parameters: vec![Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "value".into(),
                name_span: None,
                ty: TypeExpr::simple("MyNumber"),
                attributes: Vec::new(),
                di_inject: None,
                default: None,
                default_span: None,
                lends: None,
                is_extension_this: false,
            }],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![return_statement("-value")],
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

fn build_accumulate_function() -> FunctionDecl {
    FunctionDecl {
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
                    ty: TypeExpr::simple("MyNumber"),
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
                    ty: TypeExpr::simple("MyNumber"),
                    attributes: Vec::new(),
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: false,
                },
            ],
            return_type: TypeExpr::simple("MyNumber"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![
                Statement::new(
                    None,
                    StatementKind::VariableDeclaration(VariableDeclaration {
                        modifier: VariableModifier::Var,
                        type_annotation: Some(TypeExpr::simple("MyNumber")),
                        declarators: vec![VariableDeclarator {
                            name: "total".into(),
                            initializer: Some(Expression::new("lhs", None)),
                        }],
                        is_pinned: false,
                    }),
                ),
                Statement::new(
                    None,
                    StatementKind::Expression(Expression::new("total += rhs", None)),
                ),
                return_statement("total"),
            ],
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

fn return_statement(expr_text: &str) -> Statement {
    Statement::new(
        None,
        StatementKind::Return {
            expression: Some(Expression::new(expr_text, None)),
        },
    )
}

#[test]
fn lowers_user_defined_binary_operator_to_call() {
    let module = build_operator_module(false);
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
        .find(|func| func.name == "Numbers::Add")
        .expect("missing Add function");
    let body = &func.body;
    let (call_block_id, func_operand, args, destination, target) = body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                func,
                args,
                destination,
                target,
                arg_modes: _,
                ..
            }) => Some((block.id, func, args, destination, *target)),
            _ => None,
        })
        .expect("expected call terminator");

    let pending = match func_operand {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand, found {other:?}"),
    };
    assert_eq!(pending.repr, "Numbers::MyNumber::op_Addition");
    assert_eq!(args.len(), 2);
    assert!(matches!(args[0], Operand::Copy(_)));
    assert!(matches!(args[1], Operand::Copy(_)));
    let destination = destination
        .as_ref()
        .expect("operator call should yield destination");
    assert_ne!(
        destination.local.0, 0,
        "operator result should not write directly to return slot"
    );

    let continue_block = &body.blocks[target.0];
    assert!(
        continue_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. })),
        "expected continuation block to assign operator result"
    );

    let call_block = body
        .blocks
        .iter()
        .find(|block| block.id == call_block_id)
        .expect("missing call block");
    assert!(
        call_block
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, MirStatementKind::StorageLive(_))),
        "expected storage live before operator call"
    );
}

#[test]
fn lowers_user_defined_unary_operator_to_call() {
    let module = build_operator_module(false);
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
        .find(|func| func.name == "Numbers::Negate")
        .expect("missing Negate function");
    let body = &func.body;
    let (_id, func_operand, args, destination, _) = body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                func,
                args,
                destination,
                target: _,
                arg_modes: _,
                ..
            }) => Some((block.id, func, args, destination, block)),
            _ => None,
        })
        .expect("expected unary operator call");

    let pending = match func_operand {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand, found {other:?}"),
    };
    assert_eq!(pending.repr, "Numbers::MyNumber::op_UnaryNegation");
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0], Operand::Copy(_)));
    assert!(
        destination.is_some(),
        "unary operator should provide a destination local"
    );
}

#[test]
fn lowers_compound_assignment_with_operator() {
    let module = build_operator_module(false);
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
        .find(|func| func.name == "Numbers::Accumulate")
        .expect("missing Accumulate function");
    let body = &func.body;
    let call_block = body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Some(Terminator::Call { .. })))
        .expect("expected operator call");
    match &call_block.terminator {
        Some(Terminator::Call {
            func,
            args,
            destination,
            arg_modes: _,
            ..
        }) => {
            let pending = match func {
                Operand::Pending(pending) => pending,
                other => panic!("expected pending operand, found {other:?}"),
            };
            assert_eq!(pending.repr, "Numbers::MyNumber::op_Addition");
            assert_eq!(args.len(), 2);
            assert!(
                destination.is_some(),
                "compound assignment call should store result"
            );
        }
        _ => unreachable!("guard above ensures terminator is call"),
    }
}

#[test]
fn reports_missing_operator_overload() {
    let mut module = build_operator_module(false);
    if let Some(Item::Class(class)) = module
        .items
        .iter_mut()
        .find(|item| matches!(item, Item::Class(_)))
    {
        class.members.retain(
            |member| !matches!(member, ClassMember::Method(method) if method.operator.is_some()),
        );
    }
    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("operator `+` is not defined for operand types")),
        "expected missing operator diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn reports_ambiguous_operator_overload() {
    let module = build_operator_module(true);
    let lowering = lower_module(&module);
    assert!(
        lowering.diagnostics.iter().any(|diag| diag
            .message
            .contains("ambiguous operator `+` for operand types")),
        "expected ambiguous operator diagnostic, found {:?}",
        lowering.diagnostics
    );
}

#[test]
fn lowers_prefix_increment_operator_to_call() {
    let module = build_operator_module(false);
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
        .find(|func| func.name == "Numbers::PrefixIncrement")
        .expect("missing PrefixIncrement function");
    let (_id, func_operand, args, destination, _) = func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                func,
                args,
                destination,
                target: _,
                arg_modes: _,
                ..
            }) => Some((block.id, func, args, destination, block)),
            _ => None,
        })
        .expect("expected increment operator call");

    let pending = match func_operand {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand, found {other:?}"),
    };
    assert_eq!(pending.repr, "Numbers::MyNumber::op_Increment");
    assert_eq!(args.len(), 1);
    assert!(
        destination.is_some(),
        "increment call should have destination"
    );
}

#[test]
fn lowers_postfix_increment_operator_to_call() {
    let module = build_operator_module(false);
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
        .find(|func| func.name == "Numbers::PostfixIncrement")
        .expect("missing PostfixIncrement function");
    let (_id, func_operand, args, destination, _) = func
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Some(Terminator::Call {
                func,
                args,
                destination,
                target: _,
                arg_modes: _,
                ..
            }) => Some((block.id, func, args, destination, block)),
            _ => None,
        })
        .expect("expected postfix increment call");

    let pending = match func_operand {
        Operand::Pending(pending) => pending,
        other => panic!("expected pending operand, found {other:?}"),
    };
    assert_eq!(pending.repr, "Numbers::MyNumber::op_Increment");
    assert_eq!(args.len(), 1);
    assert!(
        destination.is_some(),
        "postfix increment should store result"
    );
}
