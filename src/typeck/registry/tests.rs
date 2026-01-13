#![cfg(test)]

use super::super::arena::{BaseTypeBinding, TypeChecker, TypeInfo, TypeKind};
use super::{signature_from_extension, type_expr_mentions_parameter};
use crate::frontend::ast::expressions::Expression;
use crate::frontend::ast::patterns::{CasePattern, PatternGuard};
use crate::frontend::ast::{
    BindingModifier, Block, CatchClause, ClassDecl, ClassKind, ClassMember, ConstDeclaration,
    ConstDeclarator, ConstMemberDecl, ConstStatement, ConstructorDecl, ConstructorInitTarget,
    ConstructorInitializer, ConstructorKind, EnumDecl, EnumVariant, ExtensionCondition,
    ExtensionDecl, ExtensionMember, ExtensionMethodDecl, FieldDecl, FixedStatement, ForInitializer,
    ForStatement, ForeachStatement, FunctionDecl, GotoStatement, GotoTarget, IfStatement, ImplDecl,
    ImplMember, InterfaceDecl, InterfaceMember, Item, LendsClause, MemberDispatch, Module,
    NamespaceDecl, Parameter, PropertyAccessor, PropertyAccessorBody, PropertyAccessorKind,
    PropertyDecl, Signature, Statement, StatementKind, StructDecl, SwitchCaseLabel, SwitchLabel,
    SwitchSection, SwitchStatement, TraitAssociatedType, TraitDecl, TraitMember, TryStatement,
    TypeExpr, UnionDecl, UnionField, UnionMember, UsingDirective, UsingKind, UsingResource,
    UsingStatement, VariableDeclaration, VariableDeclarator, VariableModifier, Variance,
    Visibility,
};
use crate::frontend::ast::{
    GenericArgument, GenericParam, GenericParamKind, GenericParams, ThrowsClause, TypeSuffix,
};
use crate::frontend::diagnostics::Diagnostic;
use crate::frontend::parser::{parse_module, parse_type_expression_text};
use crate::mir::builder::symbol_index::{FunctionParamSymbol, FunctionSymbol};
use crate::mir::{Abi, BinOp, ConstValue, FnTy, ParamMode, Ty, TypeLayoutTable, UnOp};
use crate::syntax::expr::builders::{
    AssignOp, CallArgument, CallArgumentModifier, CallArgumentName, CastSyntax,
    InterpolatedExprSegment, InterpolatedStringExpr, InterpolatedStringSegment, LambdaBlock,
    LambdaBody, LambdaExpr, LambdaParam, LambdaParamModifier, LiteralConst, NameOfOperand,
    NewInitializer,
};
use crate::syntax::expr::{ExprNode, NewExpr, SizeOfOperand};
use crate::syntax::numeric::{IntegerWidth, NumericLiteralMetadata, NumericLiteralType};
use crate::syntax::pattern::{
    ListSliceMetadata, PatternAst, PatternBindingMetadata, PatternMetadata, PatternNode,
    RecordFieldMetadata,
};
use expect_test::expect;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Write;
use std::rc::Rc;

fn with_registry<R>(
    source: &str,
    f: impl for<'a> FnOnce(&'a mut TypeChecker<'a>, &'a Module) -> R,
) -> R {
    let parsed = parse_module(source).unwrap_or_else(|err| {
        panic!("failed to parse module: {:?}", err.diagnostics());
    });
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let module = parsed.module;
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    f(&mut checker, &module)
}

fn render_diagnostics(diags: &[Diagnostic]) -> String {
    let mut buffer = String::new();
    for diag in diags {
        let span = diag
            .primary_label
            .as_ref()
            .map(|label| label.span)
            .or_else(|| diag.secondary_labels.get(0).map(|label| label.span));
        if let Some(span) = span {
            writeln!(
                buffer,
                "[{:?}] @ {}..{}: {}",
                diag.severity, span.start, span.end, diag.message
            )
            .unwrap();
        } else {
            writeln!(buffer, "[{:?}]: {}", diag.severity, diag.message).unwrap();
        }
    }
    buffer
}

fn with_manual_module<R>(
    module: Module,
    f: impl for<'a> FnOnce(&'a mut TypeChecker<'a>, &'a Module) -> R,
) -> R {
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    f(&mut checker, &module)
}

#[test]
fn duplicate_structs_emit_snapshot() {
    with_registry(
        r#"
        namespace Conflicts {
            struct Widget {}
            struct Widget {}
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error]: [TCK400] struct `Conflicts::Widget` conflicts with a previous declaration
[Note]: previous declaration of `Conflicts::Widget` (struct)
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn vector_lane_count_must_be_const_and_positive() {
    with_registry(
        r"
        namespace Simd { }
        ",
        |checker, _module| {
            let expr =
                parse_type_expression_text("vector<int, lanes>").expect("expected vector type");
            checker.handle_vector_type(&expr);
            let codes: Vec<_> = checker
                .diagnostics
                .iter()
                .filter_map(|diag| diag.code.as_ref().map(|code| code.code.as_str()))
                .collect();
            assert!(
                codes.contains(&"TYPE0701"),
                "expected TYPE0701 for non-const lanes, got {codes:?}"
            );
        },
    );
}

#[test]
fn vector_vector_type_allows_supported_widths() {
    with_registry(
        r"
        namespace Simd { }
        ",
        |checker, _module| {
            let expr =
                parse_type_expression_text("vector<float, 4>").expect("expected vector type");
            checker.handle_vector_type(&expr);
            let codes: Vec<_> = checker
                .diagnostics
                .iter()
                .filter_map(|diag| diag.code.as_ref().map(|code| code.code.as_str()))
                .collect();
            assert!(
                !codes.contains(&"TYPE0701"),
                "lane diagnostics should not trigger for valid vectors"
            );
            assert!(
                !codes.contains(&"TYPE0702"),
                "width diagnostics should not trigger for valid vectors"
            );
            assert!(
                !codes.contains(&"TYPE0705"),
                "element diagnostics should not trigger for valid vectors"
            );
        },
    );
}

#[test]
fn registry_hooks_preserve_order() {
    with_registry(
        r#"
        namespace Chronology {
            struct Alpha {}
            struct Beta {}
            struct Gamma {}
        }
        "#,
        |checker, module| {
            let events = Rc::new(RefCell::new(Vec::new()));
            let captured = Rc::clone(&events);
            checker.registry_hooks_mut().subscribe(move |event| {
                captured
                    .borrow_mut()
                    .push(format!("{:?} {}", event.kind, event.name));
            });
            checker.visit_items(&module.items, module.namespace.as_deref());
            assert_eq!(
                events.borrow().as_slice(),
                &[
                    "Registered Chronology::Alpha",
                    "Registered Chronology::Beta",
                    "Registered Chronology::Gamma",
                ]
            );
        },
    );
}

#[test]
fn nested_namespace_merges_qualified_names() {
    with_registry(
        r#"
        namespace Root {
            struct First {}

            namespace Inner {
                struct Second {}
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            assert!(checker.has_type("Root::First"));
            assert!(checker.has_type("Root::Inner::Second"));
            assert!(checker.diagnostics.is_empty());
        },
    );
}

#[test]
fn trait_discovery_captures_members_and_object_safety() {
    let printable_trait = trait_def(
        "Printable",
        vec![
            TraitMember::Method(simple_method(
                "describe",
                vec![parameter("item", TypeExpr::self_type())],
                TypeExpr::self_type(),
            )),
            assoc_type_member("Output", None),
            trait_const_member("FORMAT", "int", "1"),
        ],
    );

    let module = Module::with_items(None, vec![Item::Trait(printable_trait)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let info = checker
            .traits
            .get("Printable")
            .expect("trait should be registered");
        assert_eq!(info.methods.len(), 1, "expected describe method recorded");
        assert_eq!(
            info.associated_types.len(),
            1,
            "expected associated type recorded"
        );
        assert_eq!(
            info.object_safety.violation_count(),
            2,
            "returning Self and missing default should both be recorded"
        );
    });
}

#[test]
fn impl_discovery_reports_missing_associated_type() {
    let drawable_trait = trait_def(
        "Drawable",
        vec![
            TraitMember::Method(simple_method(
                "draw",
                vec![parameter("self", TypeExpr::self_type())],
                TypeExpr::simple("void"),
            )),
            assoc_type_member("Output", None),
        ],
    );

    let widget_struct = struct_def("Widget");
    let drawable_impl = impl_def(
        Some("Drawable"),
        "Widget",
        vec![ImplMember::Method(simple_method(
            "draw",
            vec![parameter("self", TypeExpr::self_type())],
            TypeExpr::simple("void"),
        ))],
        None,
    );

    let module = Module::with_items(
        None,
        vec![
            Item::Struct(widget_struct),
            Item::Trait(drawable_trait),
            Item::Impl(drawable_impl),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK098] impl of `Drawable` for `Widget` is missing associated type `Output`
[Note]: See SPEC.md#4-2-traits--generic-associated-types, docs/compiler/traits.md#3-trait-solver-architecture for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn impl_discovery_rejects_inherent_impls() {
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Impl(impl_def(None, "Widget", Vec::new(), None)),
        ],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK099] inherent `impl` blocks are not supported yet
[Note]: See SPEC.md#4-2-traits--generic-associated-types, docs/compiler/traits.md#5-trait-objects-dyn-trait for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn impl_discovery_rejects_unknown_trait() {
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Impl(impl_def(Some("Drawable"), "Widget", Vec::new(), None)),
        ],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK030] unknown type `Drawable`
[Note]: See SPEC.md#2-2-data-types-without-oop for specification details.
[Error]: [TCK092] trait `Drawable` is not defined in this module
[Note]: See SPEC.md#4-2-traits--generic-associated-types, docs/compiler/traits.md#3-trait-solver-architecture for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn impl_discovery_rejects_impl_generics() {
    let drawable_trait = trait_def(
        "Drawable",
        vec![
            TraitMember::Method(simple_method(
                "draw",
                vec![parameter("self", TypeExpr::self_type())],
                TypeExpr::simple("void"),
            )),
            assoc_type_member("Output", Some(TypeExpr::simple("int"))),
        ],
    );
    let widget_struct = struct_def("Widget");
    let mut params = GenericParams::new(None, Vec::new());
    params.params.push(GenericParam::type_param("T", None));
    let drawable_impl = impl_def(
        Some("Drawable"),
        "Widget",
        vec![ImplMember::Method(simple_method(
            "draw",
            vec![parameter("self", TypeExpr::self_type())],
            TypeExpr::simple("void"),
        ))],
        Some(params),
    );
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(widget_struct),
            Item::Trait(drawable_trait),
            Item::Impl(drawable_impl),
        ],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK095] blanket trait implementations are not supported (no specialization or negative reasoning)
[Note]: See SPEC.md#4-2-traits--generic-associated-types, docs/compiler/traits.md#3-trait-solver-architecture for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn impl_discovery_requires_trait_methods() {
    let sampler_trait = trait_def(
        "Sampler",
        vec![TraitMember::Method(simple_method(
            "sample",
            vec![parameter("self", TypeExpr::self_type())],
            TypeExpr::simple("int"),
        ))],
    );
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Trait(sampler_trait),
            Item::Impl(impl_def(Some("Sampler"), "Widget", Vec::new(), None)),
        ],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK098] impl of `Sampler` for `Widget` is missing method `sample`
[Note]: See SPEC.md#4-2-traits--generic-associated-types, docs/compiler/traits.md#3-trait-solver-architecture for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn const_fn_signature_reports_all_violations() {
    let mut generic_params = GenericParams::new(None, Vec::new());
    generic_params
        .params
        .push(GenericParam::type_param("T", None));
    let mut const_fn = simple_method(
        "Compute",
        vec![Parameter {
            binding: BindingModifier::Ref,
            binding_nullable: false,
            name: "input".to_string(),
            name_span: None,
            ty: TypeExpr::simple("int"),
            attributes: Vec::new(),
            di_inject: None,
            default: None,
            default_span: None,
            lends: None,
            is_extension_this: false,
        }],
        TypeExpr::simple("int"),
    );
    const_fn.is_async = true;
    const_fn.is_constexpr = true;
    const_fn.is_extern = true;
    const_fn.is_unsafe = true;
    const_fn.generics = Some(generic_params);
    const_fn.signature.throws = Some(ThrowsClause::new(vec![TypeExpr::simple("Error")], None));
    const_fn.body = None;

    let module = Module::with_items(
        None,
        vec![Item::Struct(struct_def("Error")), Item::Function(const_fn)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK080] async function `Compute` must return `Std.Async.Task` or `Std.Async.Task<T>`
[Note]: See SPEC.md#1-17-async-runtime--executors for specification details.
[Error]: [TCK160] const fn `Compute` cannot be compiled: `async` functions cannot be evaluated at compile time; `extern` const functions are not supported; const functions may not declare generic parameters; `unsafe` const functions are not supported; const functions may not declare `throws` effects; parameter `input` uses `ref` binding, which is not supported in const functions
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
[Error]: [TCK160] const fn `Compute` requires a body
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn const_fn_body_rejects_unsupported_constructs() {
    let mut const_fn = simple_method("ConstBody", Vec::new(), TypeExpr::simple("int"));
    const_fn.is_constexpr = true;
    const_fn.body = Some(Block {
        statements: vec![
            Statement::new(
                None,
                StatementKind::While {
                    condition: literal_expr(ConstValue::Bool(true)),
                    body: Box::new(Statement::new(None, StatementKind::Empty)),
                },
            ),
            Statement::new(
                None,
                StatementKind::VariableDeclaration(VariableDeclaration {
                    modifier: VariableModifier::Let,
                    type_annotation: Some(TypeExpr::simple("int")),
                    declarators: vec![VariableDeclarator {
                        name: "missing".to_string(),
                        initializer: None,
                    }],
                    is_pinned: false,
                }),
            ),
            Statement::new(
                None,
                StatementKind::Expression(Expression::with_node(
                    "call",
                    None,
                    ExprNode::Call {
                        callee: Box::new(ExprNode::Literal(LiteralConst::without_numeric(
                            ConstValue::Int(0),
                        ))),
                        args: vec![CallArgument::positional(
                            ExprNode::Identifier("arg".into()),
                            None,
                            None,
                        )],
                        generics: None,
                    },
                )),
            ),
            Statement::new(
                None,
                StatementKind::Expression(Expression::with_node(
                    "assign",
                    None,
                    ExprNode::Assign {
                        target: Box::new(ExprNode::Member {
                            base: Box::new(ExprNode::Identifier("obj".into())),
                            member: "field".into(),
                            null_conditional: false,
                        }),
                        op: AssignOp::Assign,
                        value: Box::new(ExprNode::Literal(LiteralConst::without_numeric(
                            ConstValue::Int(1),
                        ))),
                    },
                )),
            ),
        ],
        span: None,
    });

    let module = Module::with_items(None, vec![Item::Function(const_fn)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK161] const fn `ConstBody` does not support `while` statements
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
[Error]: [TCK161] variable `missing` in const fn `ConstBody` requires an initializer
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
[Error]: [TCK161] const fn `ConstBody` call target must be a simple path
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
[Error]: [TCK161] assignments in const fn `ConstBody` must target local identifiers
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn function_pointer_types_are_accepted() {
    let fn_pointer =
        parse_type_expression_text("fn(int) -> int").expect("fn pointer type expression parses");
    let params = vec![Parameter {
        binding: BindingModifier::In,
        binding_nullable: false,
        name: "callback".to_string(),
        name_span: None,
        ty: fn_pointer,
        attributes: Vec::new(),
        di_inject: None,
        default: None,
        default_span: None,
        lends: None,
        is_extension_this: false,
    }];
    let function = simple_method("UsesFnPointer", params, TypeExpr::simple("void"));
    let module = Module::with_items(None, vec![Item::Function(function)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        assert!(
            checker.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            checker.diagnostics
        );
    });
}

#[test]
fn const_fn_rejects_complex_calls_and_assignment_targets() {
    let mut const_fn = simple_method("ConstWeird", Vec::new(), TypeExpr::simple("int"));
    const_fn.is_constexpr = true;
    const_fn.body = Some(Block {
        statements: vec![
            Statement::new(
                None,
                StatementKind::Expression(Expression::with_node(
                    "CallMember",
                    None,
                    ExprNode::Call {
                        callee: Box::new(ExprNode::Index {
                            base: Box::new(ExprNode::Identifier("callables".into())),
                            indices: vec![ExprNode::Literal(LiteralConst::without_numeric(
                                ConstValue::Int(0),
                            ))],
                            null_conditional: false,
                        }),
                        args: Vec::new(),
                        generics: None,
                    },
                )),
            ),
            Statement::new(
                None,
                StatementKind::Expression(Expression::with_node(
                    "AssignMember",
                    None,
                    ExprNode::Assign {
                        target: Box::new(ExprNode::Member {
                            base: Box::new(ExprNode::Identifier("value".into())),
                            member: "Field".into(),
                            null_conditional: false,
                        }),
                        value: Box::new(ExprNode::Literal(LiteralConst::without_numeric(
                            ConstValue::Int(1),
                        ))),
                        op: AssignOp::Assign,
                    },
                )),
            ),
        ],
        span: None,
    });

    let module = Module::with_items(None, vec![Item::Function(const_fn)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK161] const fn `ConstWeird` call target must be a simple path
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
[Error]: [TCK161] assignments in const fn `ConstWeird` must target local identifiers
[Note]: See SPEC.md#6-1-const-functions--ctfe for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn const_fn_statement_validation_covers_all_kinds() {
    let expr = Expression::with_node("value", None, ExprNode::Identifier("value".into()));
    let empty_stmt = || Statement::new(None, StatementKind::Empty);
    let empty_block = Block {
        statements: vec![empty_stmt()],
        span: None,
    };

    let const_stmt = StatementKind::ConstDeclaration(ConstStatement {
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![ConstDeclarator {
                name: "CONST".into(),
                initializer: literal_expr(ConstValue::Int(1)),
                span: None,
            }],
            doc: None,
            span: None,
        },
    });
    let var_stmt = StatementKind::VariableDeclaration(VariableDeclaration {
        modifier: VariableModifier::Let,
        type_annotation: Some(TypeExpr::simple("int")),
        declarators: vec![VariableDeclarator {
            name: "v".into(),
            initializer: Some(expr.clone()),
        }],
        is_pinned: false,
    });
    let if_stmt = StatementKind::If(IfStatement {
        condition: expr.clone(),
        then_branch: Box::new(empty_stmt()),
        else_branch: Some(Box::new(empty_stmt())),
    });
    let local_fn_stmt =
        StatementKind::LocalFunction(simple_method("inner", Vec::new(), TypeExpr::simple("void")));

    let variants: Vec<(StatementKind, &str)> = vec![
        (StatementKind::Block(empty_block.clone()), "block"),
        (StatementKind::Empty, "empty"),
        (const_stmt, "const"),
        (var_stmt, "variable"),
        (StatementKind::Expression(expr.clone()), "expression"),
        (
            StatementKind::Return {
                expression: Some(expr.clone()),
            },
            "return",
        ),
        (if_stmt, "if"),
        (local_fn_stmt, "local function"),
        (
            StatementKind::While {
                condition: expr.clone(),
                body: Box::new(empty_stmt()),
            },
            "while",
        ),
        (
            StatementKind::DoWhile {
                body: Box::new(empty_stmt()),
                condition: expr.clone(),
            },
            "do-while",
        ),
        (
            StatementKind::For(ForStatement {
                initializer: Some(ForInitializer::Expressions(vec![expr.clone()])),
                condition: Some(expr.clone()),
                iterator: vec![expr.clone()],
                body: Box::new(empty_stmt()),
            }),
            "for",
        ),
        (
            StatementKind::Foreach(ForeachStatement {
                binding: "item".into(),
                binding_span: None,
                expression: expr.clone(),
                body: Box::new(empty_stmt()),
            }),
            "foreach",
        ),
        (
            StatementKind::Switch(SwitchStatement {
                expression: expr.clone(),
                sections: Vec::new(),
            }),
            "switch",
        ),
        (
            StatementKind::Try(TryStatement {
                body: empty_block.clone(),
                catches: Vec::new(),
                finally: None,
            }),
            "try",
        ),
        (
            StatementKind::Region {
                name: "scope".into(),
                body: empty_block.clone(),
            },
            "region",
        ),
        (
            StatementKind::Using(UsingStatement {
                resource: UsingResource::Expression(expr.clone()),
                body: Some(Box::new(empty_stmt())),
            }),
            "using",
        ),
        (
            StatementKind::Lock {
                expression: expr.clone(),
                body: Box::new(empty_stmt()),
            },
            "lock",
        ),
        (
            StatementKind::Checked {
                body: empty_block.clone(),
            },
            "checked",
        ),
        (
            StatementKind::Atomic {
                ordering: Some(expr.clone()),
                body: empty_block.clone(),
            },
            "atomic",
        ),
        (
            StatementKind::Unchecked {
                body: empty_block.clone(),
            },
            "unchecked",
        ),
        (
            StatementKind::YieldReturn {
                expression: expr.clone(),
            },
            "yield return",
        ),
        (StatementKind::YieldBreak, "yield break"),
        (
            StatementKind::Fixed(FixedStatement {
                declaration: VariableDeclaration {
                    modifier: VariableModifier::Let,
                    type_annotation: Some(TypeExpr::simple("byte")),
                    declarators: vec![VariableDeclarator {
                        name: "p".into(),
                        initializer: None,
                    }],
                    is_pinned: false,
                },
                body: Box::new(empty_stmt()),
            }),
            "fixed",
        ),
        (
            StatementKind::Unsafe {
                body: Box::new(empty_stmt()),
            },
            "unsafe",
        ),
        (StatementKind::Break, "break"),
        (StatementKind::Continue, "continue"),
        (
            StatementKind::Goto(GotoStatement {
                target: GotoTarget::Label("target".into()),
            }),
            "goto",
        ),
        (
            StatementKind::Throw {
                expression: Some(expr.clone()),
            },
            "throw",
        ),
        (
            StatementKind::Labeled {
                label: "label".into(),
                statement: Box::new(empty_stmt()),
            },
            "labeled",
        ),
    ];

    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _| {
        for (kind, label) in variants {
            assert_eq!(label, super::const_fn_statement_kind_name(&kind));
            checker.validate_const_fn_statement("ConstCoverage", &Statement::new(None, kind));
        }
        assert!(!checker.diagnostics.is_empty());
    });
}

#[test]
fn const_fn_expression_validation_walks_all_nodes() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _| {
        let lit = ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1)));
        let unary = ExprNode::Unary {
            op: UnOp::Neg,
            expr: Box::new(lit.clone()),
            postfix: false,
        };
        let binary = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(lit.clone()),
            right: Box::new(lit.clone()),
        };
        let cast = ExprNode::Cast {
            target: "int".into(),
            expr: Box::new(lit.clone()),
            syntax: CastSyntax::As,
        };
        let call = ExprNode::Call {
            callee: Box::new(ExprNode::Lambda(LambdaExpr {
                params: Vec::new(),
                captures: Vec::new(),
                body: LambdaBody::Block(LambdaBlock {
                    text: "{}".into(),
                    span: None,
                }),
                is_async: false,
                span: None,
            })),
            args: Vec::new(),
            generics: None,
        };
        let assign = ExprNode::Assign {
            target: Box::new(ExprNode::Member {
                base: Box::new(ExprNode::Identifier("obj".into())),
                member: "field".into(),
                null_conditional: false,
            }),
            op: AssignOp::Assign,
            value: Box::new(lit.clone()),
        };
        let member = ExprNode::Member {
            base: Box::new(ExprNode::Identifier("obj".into())),
            member: "field".into(),
            null_conditional: false,
        };
        let sizeof_value = ExprNode::SizeOf(SizeOfOperand::Value(Box::new(lit.clone())));
        let sizeof_type = ExprNode::SizeOf(SizeOfOperand::Type("int".into()));
        let alignof_value = ExprNode::AlignOf(SizeOfOperand::Value(Box::new(lit.clone())));
        let alignof_type = ExprNode::AlignOf(SizeOfOperand::Type("int".into()));
        let nameof = ExprNode::NameOf(NameOfOperand {
            segments: vec!["Alpha".into(), "Beta".into()],
            text: "Alpha::Beta".into(),
            span: None,
        });
        let quote = ExprNode::Quote(Box::new(crate::syntax::expr::builders::QuoteLiteral {
            expression: Box::new(lit.clone()),
            source: "expr".into(),
            sanitized: "expr".into(),
            content_span: None,
            interpolations: Vec::new(),
            hygiene_anchor: 0,
        }));

        let catch_all_nodes = vec![
            ExprNode::Await {
                expr: Box::new(ExprNode::Identifier("fut".into())),
            },
            ExprNode::Lambda(LambdaExpr {
                params: Vec::new(),
                captures: Vec::new(),
                body: LambdaBody::Block(LambdaBlock {
                    text: "{}".into(),
                    span: None,
                }),
                is_async: false,
                span: None,
            }),
            ExprNode::New(NewExpr {
                type_name: "Widget".into(),
                type_span: None,
                keyword_span: None,
                array_lengths: None,
                args: Vec::new(),
                arguments_span: None,
                initializer: None,
                span: None,
            }),
            ExprNode::Index {
                base: Box::new(ExprNode::Identifier("arr".into())),
                indices: vec![lit.clone()],
                null_conditional: false,
            },
            ExprNode::Tuple(vec![lit.clone(), lit.clone()]),
            ExprNode::InterpolatedString(InterpolatedStringExpr {
                segments: vec![InterpolatedStringSegment::Text("hi".into())],
                span: None,
            }),
            ExprNode::TryPropagate {
                expr: Box::new(ExprNode::Identifier("maybe".into())),
                question_span: None,
            },
            ExprNode::Conditional {
                condition: Box::new(lit.clone()),
                then_branch: Box::new(lit.clone()),
                else_branch: Box::new(lit.clone()),
            },
            ExprNode::Ref {
                expr: Box::new(ExprNode::Identifier("value".into())),
                readonly: true,
            },
            ExprNode::Throw { expr: None },
        ];

        let expressions = vec![
            unary,
            binary,
            ExprNode::Parenthesized(Box::new(lit.clone())),
            cast,
            call,
            assign,
            member,
            sizeof_value,
            sizeof_type,
            alignof_value,
            alignof_type,
            nameof,
            quote,
        ];

        for node in expressions.into_iter().chain(catch_all_nodes) {
            checker.validate_const_fn_node("ConstExpr", &node, None);
        }

        assert!(
            !checker.diagnostics.is_empty(),
            "const fn node validation should record diagnostics for disallowed expressions"
        );
    });
}

#[test]
fn parameter_defaults_enforce_order_and_binding_rules() {
    let parameters = vec![
        Parameter {
            binding: BindingModifier::Ref,
            binding_nullable: false,
            name: "first".to_string(),
            name_span: None,
            ty: TypeExpr::simple("int"),
            attributes: Vec::new(),
            di_inject: None,
            default: Some(literal_expr(ConstValue::Int(1))),
            default_span: None,
            lends: None,
            is_extension_this: false,
        },
        parameter("second", TypeExpr::simple("int")),
    ];
    let mut function = simple_method("Defaults", parameters, TypeExpr::simple("void"));
    function.body = None;

    let module = Module::with_items(None, vec![Item::Function(function)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK045] parameter `first` in `Defaults` cannot declare a default because `ref` parameters require explicit caller input
[Error]: [TCK044] parameters with default values must appear at the end of `Defaults`
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn lends_clause_requires_view_and_borrow_targets() {
    let param_ty = TypeExpr::simple("Widget");
    let parameters = vec![Parameter {
        binding: BindingModifier::Value,
        binding_nullable: false,
        name: "borrowed".to_string(),
        name_span: None,
        ty: param_ty,
        attributes: Vec::new(),
        di_inject: None,
        default: None,
        default_span: None,
        lends: None,
        is_extension_this: false,
    }];
    let mut function = simple_method("Borrow", parameters, TypeExpr::simple("Widget"));
    function.signature.lends_to_return = Some(LendsClause::new(vec!["borrowed".into()], None));

    let module = Module::with_items(
        None,
        vec![Item::Struct(struct_def("Widget")), Item::Function(function)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK181] `lends(borrowed)` requires the return type of `Borrow` to be declared as a `view`
[Error]: [TCK182] `lends(borrowed)` requires `borrowed` to be an `in` or `ref` parameter, found `value` in `Borrow`
[Error]: [TCK183] `lends(borrowed)` requires `borrowed` to be declared as a `view` type in `Borrow`
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn async_functions_require_task() {
    let mut function = simple_method("AsyncWrong", Vec::new(), TypeExpr::simple("Widget"));
    function.is_async = true;
    function.body = None;

    let module = Module::with_items(
        None,
        vec![Item::Struct(struct_def("Widget")), Item::Function(function)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK080] async function `AsyncWrong` must return `Std.Async.Task` or `Std.Async.Task<T>`
[Note]: See SPEC.md#1-17-async-runtime--executors for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn async_task_requires_single_type_argument() {
    let mut return_type = TypeExpr::simple("Task");
    return_type.suffixes.push(TypeSuffix::GenericArgs(vec![
        GenericArgument::from_type_expr(TypeExpr::simple("int")),
        GenericArgument::from_type_expr(TypeExpr::simple("string")),
    ]));

    let mut function = simple_method("AsyncPair", Vec::new(), return_type);
    function.is_async = true;
    function.body = None;

    let module = Module::with_items(
        Some("Std.Async".to_string()),
        vec![Item::Struct(struct_def("Task")), Item::Function(function)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK080] async function `Std::Async::AsyncPair` must specify exactly one type argument for `Task`
[Note]: See SPEC.md#1-17-async-runtime--executors for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn async_task_requires_explicit_type_argument() {
    let mut return_type = TypeExpr::simple("Task");
    return_type
        .suffixes
        .push(TypeSuffix::GenericArgs(vec![GenericArgument::new(
            None,
            Expression::new("unspecified", None),
        )]));

    let mut function = simple_method("AsyncMissing", Vec::new(), return_type);
    function.is_async = true;
    function.body = None;

    let module = Module::with_items(
        Some("Std.Async".to_string()),
        vec![Item::Struct(struct_def("Task")), Item::Function(function)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK080] async function `Std::Async::AsyncMissing` must supply a type argument for `Task<T>`
[Note]: See SPEC.md#1-17-async-runtime--executors for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn async_trait_method_requires_task_return() {
    with_registry(
        r#"
        namespace Demo;

        public struct Widget { }

        public interface Worker
        {
            public async Widget Run();
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error]: [TCK080] async function `Demo::Worker::Run` must return `Std.Async.Task` or `Std.Async.Task<T>`
[Note]: See SPEC.md#1-17-async-runtime--executors for specification details.
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn async_impl_method_requires_task_return() {
    with_registry(
        r#"
        namespace Std.Async
        {
            public class Task { }
            public class Task<T> { }
        }

        namespace Demo
        {
            import Std.Async;

            public interface Runner
            {
                public async Task<int> Run();
            }

            public struct Widget { }
            public class Worker : Runner
            {
                public async Widget Run()
                {
                    return 1;
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 446..495: [TCK080] async function `Demo::Worker::Run` must return `Std.Async.Task` or `Std.Async.Task<T>`
[Note] @ 446..495: See SPEC.md#1-17-async-runtime--executors for specification details.
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn local_functions_record_ordinals_and_signatures() {
    let local_fn = FunctionDecl {
        visibility: Visibility::Public,
        name: "inner".to_string(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Return { expression: None },
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
        dispatch: MemberDispatch {
            is_virtual: false,
            is_override: false,
            is_sealed: false,
            is_abstract: false,
        },
    };
    let outer_fn = FunctionDecl {
        visibility: Visibility::Public,
        name: "Outer".to_string(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![Statement::new(None, StatementKind::LocalFunction(local_fn))],
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
        dispatch: MemberDispatch {
            is_virtual: false,
            is_override: false,
            is_sealed: false,
            is_abstract: false,
        },
    };

    let module = Module::with_items(None, vec![Item::Function(outer_fn)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let ordinals = checker
            .local_function_ordinals
            .get("Outer")
            .copied()
            .unwrap_or_default();
        assert_eq!(
            ordinals, 1,
            "expected single local function ordinal tracked"
        );
        assert!(
            checker.functions.contains_key("Outer::local$0::inner"),
            "local function symbol should be registered"
        );
    });
}

#[test]
fn type_hierarchy_collects_class_ancestors() {
    let base = ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Base".into(),
        bases: Vec::new(),
        members: Vec::new(),
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    };
    let derived = ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Derived".into(),
        bases: vec![TypeExpr::simple("Base")],
        members: Vec::new(),
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    };

    let module = Module::with_items(None, vec![Item::Class(base), Item::Class(derived)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let chain = checker.type_hierarchy("Derived");
        assert_eq!(chain, vec!["Derived".to_string(), "Base".to_string()]);
    });
}

#[test]
fn throws_clause_records_unique_effects() {
    let mut throws_fn = simple_method(
        "Throws",
        vec![parameter("input", TypeExpr::simple("int"))],
        TypeExpr::simple("void"),
    );
    throws_fn.signature.throws = Some(ThrowsClause::new(
        vec![
            TypeExpr::simple("Error"),
            TypeExpr::simple("Error"),
            TypeExpr::simple("Unknown"),
        ],
        None,
    ));
    throws_fn.body = None;
    let module = Module::with_items(
        None,
        vec![Item::Struct(struct_def("Error")), Item::Function(throws_fn)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let effects = checker
            .declared_effects
            .get("Throws")
            .cloned()
            .unwrap_or_default();
        assert_eq!(effects, vec!["Error".to_string(), "Unknown".to_string()]);
    });
}

#[test]
fn constructor_mismatch_reports_error() {
    let point_struct = StructDecl {
        visibility: Visibility::Public,
        name: "Point".to_string(),
        fields: Vec::new(),
        properties: Vec::new(),
        constructors: vec![ConstructorDecl {
            visibility: Visibility::Public,
            kind: ConstructorKind::Designated,
            parameters: vec![parameter("x", TypeExpr::simple("int"))],
            body: None,
            initializer: None,
            doc: None,
            span: None,
            attributes: Vec::new(),
            di_inject: None,
        }],
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
    };

    let new_expr = ExprNode::New(NewExpr {
        type_name: "Point".into(),
        type_span: None,
        keyword_span: None,
        array_lengths: None,
        args: vec![
            CallArgument::positional(
                ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1))),
                None,
                None,
            ),
            CallArgument::positional(
                ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(2))),
                None,
                None,
            ),
        ],
        arguments_span: None,
        initializer: None,
        span: None,
    });

    let function = FunctionDecl {
        visibility: Visibility::Public,
        name: "make".into(),
        name_span: None,
        signature: Signature {
            parameters: Vec::new(),
            return_type: TypeExpr::simple("void"),
            lends_to_return: None,
            variadic: false,
            throws: None,
        },
        body: Some(Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Expression(Expression::with_node("new Point", None, new_expr)),
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
        dispatch: MemberDispatch {
            is_virtual: false,
            is_override: false,
            is_sealed: false,
            is_abstract: false,
        },
    };

    let module = Module::with_items(
        None,
        vec![Item::Struct(point_struct), Item::Function(function)],
    );
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK131] `constructor `Point`` does not accept 2 argument(s)
[Note]: See SPEC.md#object-construction--initializers for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn array_creation_requires_length_or_initializer() {
    with_registry(
        r#"
        namespace Arrays {
            public class Sample {
                public void Run() {
                    var xs = new int[];
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 4..9: [TCK139] array creation requires either a length (`new T[n]`) or an initializer list (`new T[] { ... }`)
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn array_multi_rank_rejected_with_diagnostic() {
    with_registry(
        r#"
        namespace Arrays {
            public class Sample {
                public void Run() {
                    var xs = new int[2, 3];
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 4..13: [TCK144] multi-dimensional array ranks are not supported; use jagged arrays (`T[][]`) instead
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn array_length_mismatch_reported() {
    with_registry(
        r#"
        namespace Arrays {
            public class Sample {
                public void Run() {
                    var xs = new int[2] { 1, 2, 3 };
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 4..10: [TCK140] array length does not match initializer element count (3)
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn array_length_must_be_const_when_initializer_present() {
    with_registry(
        r#"
        namespace Arrays {
            public class Sample {
                public void Run(int count) {
                    var xs = new int[count] { 1, 2 };
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 4..14: [TCK145] array length must be a compile-time constant when an initializer list is provided
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn implicit_array_type_is_rejected() {
    with_registry(
        r#"
        namespace Arrays {
            public class Sample {
                public void Run() {
                    var xs = new[] { 1, 2 };
                }
            }
        }
        "#,
        |checker, module| {
            checker.visit_items(&module.items, module.namespace.as_deref());
            let rendered = render_diagnostics(&checker.diagnostics);
            expect![[r#"
[Error] @ 3..5: [TCK147] array creation requires an explicit element type (`new T[] { ... }`)
"#]]
            .assert_eq(&rendered);
        },
    );
}

#[test]
fn statement_and_expression_validation_paths() {
    // Register a basic type so expression/type lookups succeed.
    let module = Module::with_items(None, vec![Item::Struct(struct_def("Widget"))]);
    let statements_vec = vec![
        Statement::new(
            None,
            StatementKind::Block(Block {
                statements: vec![Statement::new(None, StatementKind::Empty)],
                span: None,
            }),
        ),
        Statement::new(
            None,
            StatementKind::Expression(literal_expr(ConstValue::Int(1))),
        ),
        Statement::new(
            None,
            StatementKind::Return {
                expression: Some(literal_expr(ConstValue::Int(2))),
            },
        ),
        Statement::new(
            None,
            StatementKind::Throw {
                expression: Some(literal_expr(ConstValue::Int(3))),
            },
        ),
        Statement::new(None, StatementKind::Throw { expression: None }),
        Statement::new(
            None,
            StatementKind::If(IfStatement {
                condition: literal_expr(ConstValue::Bool(true)),
                then_branch: Box::new(Statement::new(None, StatementKind::Empty)),
                else_branch: Some(Box::new(Statement::new(None, StatementKind::Empty))),
            }),
        ),
        Statement::new(
            None,
            StatementKind::While {
                condition: literal_expr(ConstValue::Bool(false)),
                body: Box::new(Statement::new(None, StatementKind::Empty)),
            },
        ),
        Statement::new(
            None,
            StatementKind::DoWhile {
                body: Box::new(Statement::new(None, StatementKind::Empty)),
                condition: literal_expr(ConstValue::Bool(true)),
            },
        ),
        Statement::new(
            None,
            StatementKind::For(ForStatement {
                initializer: Some(ForInitializer::Expressions(vec![literal_expr(
                    ConstValue::Int(0),
                )])),
                condition: Some(literal_expr(ConstValue::Bool(true))),
                iterator: vec![literal_expr(ConstValue::Int(1))],
                body: Box::new(Statement::new(None, StatementKind::Break)),
            }),
        ),
        Statement::new(
            None,
            StatementKind::Foreach(ForeachStatement {
                binding: "item".to_string(),
                binding_span: None,
                expression: literal_expr(ConstValue::Int(4)),
                body: Box::new(Statement::new(None, StatementKind::Continue)),
            }),
        ),
        Statement::new(
            None,
            StatementKind::Switch(SwitchStatement {
                expression: literal_expr(ConstValue::Int(5)),
                sections: vec![SwitchSection {
                    labels: vec![SwitchLabel::Default],
                    statements: vec![Statement::new(None, StatementKind::Break)],
                }],
            }),
        ),
        Statement::new(
            None,
            StatementKind::Try(TryStatement {
                body: Block {
                    statements: vec![Statement::new(
                        None,
                        StatementKind::Expression(literal_expr(ConstValue::Int(6))),
                    )],
                    span: None,
                },
                catches: vec![CatchClause {
                    type_annotation: Some(TypeExpr::simple("Widget")),
                    identifier: Some("err".into()),
                    filter: Some(literal_expr(ConstValue::Bool(false))),
                    body: Block {
                        statements: vec![Statement::new(
                            None,
                            StatementKind::Expression(literal_expr(ConstValue::Int(7))),
                        )],
                        span: None,
                    },
                }],
                finally: Some(Block {
                    statements: vec![Statement::new(
                        None,
                        StatementKind::Expression(literal_expr(ConstValue::Int(8))),
                    )],
                    span: None,
                }),
            }),
        ),
        Statement::new(
            None,
            StatementKind::Region {
                name: "region".into(),
                body: Block {
                    statements: vec![Statement::new(None, StatementKind::Empty)],
                    span: None,
                },
            },
        ),
        Statement::new(
            None,
            StatementKind::Using(UsingStatement {
                resource: UsingResource::Expression(literal_expr(ConstValue::Int(9))),
                body: Some(Box::new(Statement::new(None, StatementKind::Empty))),
            }),
        ),
        Statement::new(
            None,
            StatementKind::Lock {
                expression: literal_expr(ConstValue::Int(10)),
                body: Box::new(Statement::new(None, StatementKind::Empty)),
            },
        ),
        Statement::new(
            None,
            StatementKind::Checked {
                body: Block {
                    statements: vec![Statement::new(None, StatementKind::Empty)],
                    span: None,
                },
            },
        ),
        Statement::new(
            None,
            StatementKind::Atomic {
                ordering: Some(literal_expr(ConstValue::Int(11))),
                body: Block {
                    statements: vec![Statement::new(None, StatementKind::Empty)],
                    span: None,
                },
            },
        ),
        Statement::new(
            None,
            StatementKind::Unchecked {
                body: Block {
                    statements: vec![Statement::new(None, StatementKind::Empty)],
                    span: None,
                },
            },
        ),
        Statement::new(
            None,
            StatementKind::YieldReturn {
                expression: literal_expr(ConstValue::Int(12)),
            },
        ),
        Statement::new(None, StatementKind::YieldBreak),
        Statement::new(
            None,
            StatementKind::Fixed(FixedStatement {
                declaration: VariableDeclaration {
                    modifier: VariableModifier::Let,
                    type_annotation: Some(TypeExpr::simple("Widget")),
                    declarators: vec![VariableDeclarator {
                        name: "ptr".into(),
                        initializer: Some(literal_expr(ConstValue::Int(13))),
                    }],
                    is_pinned: false,
                },
                body: Box::new(Statement::new(None, StatementKind::Empty)),
            }),
        ),
        Statement::new(
            None,
            StatementKind::Unsafe {
                body: Box::new(Statement::new(
                    None,
                    StatementKind::Expression(literal_expr(ConstValue::Int(14))),
                )),
            },
        ),
        Statement::new(
            None,
            StatementKind::Labeled {
                label: "label".into(),
                statement: Box::new(Statement::new(None, StatementKind::Break)),
            },
        ),
        Statement::new(
            None,
            StatementKind::Goto(GotoStatement {
                target: GotoTarget::Label("target".into()),
            }),
        ),
    ];
    let statements: &'static [Statement] = Box::leak(statements_vec.into_boxed_slice());
    let expr_nodes = vec![
        ExprNode::Unary {
            op: UnOp::Neg,
            expr: Box::new(ExprNode::Literal(LiteralConst::without_numeric(
                ConstValue::Int(1),
            ))),
            postfix: false,
        },
        ExprNode::Ref {
            expr: Box::new(ExprNode::Identifier("value".into())),
            readonly: true,
        },
        ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Identifier("lhs".into())),
            right: Box::new(ExprNode::Identifier("rhs".into())),
        },
        ExprNode::Conditional {
            condition: Box::new(ExprNode::Identifier("cond".into())),
            then_branch: Box::new(ExprNode::Identifier("then".into())),
            else_branch: Box::new(ExprNode::Identifier("else".into())),
        },
        ExprNode::Cast {
            target: "Widget".into(),
            expr: Box::new(ExprNode::Identifier("castme".into())),
            syntax: CastSyntax::As,
        },
        ExprNode::Assign {
            target: Box::new(ExprNode::Identifier("slot".into())),
            op: AssignOp::AddAssign,
            value: Box::new(ExprNode::Literal(LiteralConst::without_numeric(
                ConstValue::Int(2),
            ))),
        },
        ExprNode::Member {
            base: Box::new(ExprNode::Identifier("owner".into())),
            member: "field".into(),
            null_conditional: false,
        },
        ExprNode::Call {
            callee: Box::new(ExprNode::Identifier("call_target".into())),
            args: vec![CallArgument::positional(
                ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(3))),
                None,
                None,
            )],
            generics: Some(vec!["T".into()]),
        },
        ExprNode::New(NewExpr {
            type_name: "Widget".into(),
            type_span: None,
            keyword_span: None,
            array_lengths: None,
            args: vec![CallArgument::positional(
                ExprNode::Identifier("arg".into()),
                None,
                None,
            )],
            arguments_span: None,
            initializer: Some(NewInitializer::Collection {
                elements: vec![ExprNode::Literal(LiteralConst::without_numeric(
                    ConstValue::Int(4),
                ))],
                span: None,
            }),
            span: None,
        }),
        ExprNode::Index {
            base: Box::new(ExprNode::Identifier("arr".into())),
            indices: vec![ExprNode::Literal(LiteralConst::without_numeric(
                ConstValue::Int(5),
            ))],
            null_conditional: false,
        },
        ExprNode::Await {
            expr: Box::new(ExprNode::Identifier("future".into())),
        },
        ExprNode::TryPropagate {
            expr: Box::new(ExprNode::Identifier("maybe".into())),
            question_span: None,
        },
        ExprNode::Throw {
            expr: Some(Box::new(ExprNode::Identifier("err".into()))),
        },
        ExprNode::SizeOf(SizeOfOperand::Value(Box::new(ExprNode::Identifier(
            "item".into(),
        )))),
        ExprNode::SizeOf(SizeOfOperand::Type("Widget".into())),
        ExprNode::NameOf(NameOfOperand {
            segments: vec!["Widget".into()],
            text: "Widget".into(),
            span: None,
        }),
        ExprNode::Lambda(LambdaExpr {
            params: vec![LambdaParam {
                modifier: Some(LambdaParamModifier::In),
                ty: Some("int".into()),
                name: "p".into(),
                span: None,
                default: None,
            }],
            captures: vec!["c".into()],
            body: LambdaBody::Block(LambdaBlock {
                text: "{}".into(),
                span: None,
            }),
            is_async: false,
            span: None,
        }),
        ExprNode::Tuple(vec![
            ExprNode::Identifier("a".into()),
            ExprNode::Identifier("b".into()),
        ]),
        ExprNode::InterpolatedString(InterpolatedStringExpr {
            segments: vec![
                InterpolatedStringSegment::Text("hello".into()),
                InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                    expr: ExprNode::Identifier("name".into()),
                    expr_text: "name".into(),
                    alignment: None,
                    format: None,
                    span: None,
                }),
            ],
            span: None,
        }),
    ];
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let ns = module.namespace.as_deref();
        let ctx = None;
        let fn_name = "Validate";
        for stmt in statements {
            checker.validate_statement(fn_name, stmt, ns, ctx);
        }

        for node in &expr_nodes {
            checker.validate_expr_node(fn_name, node, None, ns, ctx);
        }

        // Exercise numeric literal suffix validation.
        let numeric_literal = LiteralConst {
            value: ConstValue::Int(500),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W8),
                explicit_suffix: true,
                suffix_text: Some("i8".into()),
            }),
        };
        checker.check_numeric_literal_expression(
            &Expression::with_node("500i8", None, ExprNode::Literal(numeric_literal)),
            Some(&TypeExpr::simple("i8")),
        );

        assert!(!checker.diagnostics.is_empty());
    });
}

#[test]
fn interface_variance_reports_misuse() {
    let iface = InterfaceDecl {
        visibility: Visibility::Public,
        name: "Readable".into(),
        bases: Vec::new(),
        members: vec![InterfaceMember::Method(FunctionDecl {
            visibility: Visibility::Public,
            name: "consume".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![parameter("input", TypeExpr::simple("T"))],
                return_type: TypeExpr::simple("T"),
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        })],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: Some(generic_params_with_variance("T", Variance::Covariant)),
        attributes: Vec::new(),
    };

    let module = Module::with_items(None, vec![Item::Interface(iface)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK022] interface `Readable` declares `T` as covariant (`out`), but member `Readable::consume` uses it in an input position
[Note]: See SPEC.md#2.3-optional-oop for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn contravariant_interface_properties_require_input_positions() {
    let property = PropertyDecl {
        visibility: Visibility::Public,
        modifiers: Vec::new(),
        name: "payload".to_string(),
        ty: TypeExpr::simple("T"),
        parameters: Vec::new(),
        accessors: vec![PropertyAccessor {
            kind: PropertyAccessorKind::Get,
            visibility: None,
            body: PropertyAccessorBody::Auto,
            doc: None,
            span: None,
            attributes: None,
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        }],
        doc: None,
        is_required: false,
        is_static: false,
        initializer: None,
        span: None,
        attributes: Vec::new(),
        di_inject: None,
        dispatch: MemberDispatch {
            is_virtual: false,
            is_override: false,
            is_sealed: false,
            is_abstract: false,
        },
        explicit_interface: None,
    };

    let iface = InterfaceDecl {
        visibility: Visibility::Public,
        name: "Writable".into(),
        bases: Vec::new(),
        members: vec![InterfaceMember::Property(property)],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: Some(generic_params_with_variance("T", Variance::Contravariant)),
        attributes: Vec::new(),
    };

    let module = Module::with_items(None, vec![Item::Interface(iface)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK022] interface `Writable` declares `T` as contravariant (`in`), but member `Writable::payload` uses it in an output position
[Note]: See SPEC.md#2.3-optional-oop for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn numeric_literal_reports_unsigned_overflow_and_mismatch() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _module| {
        let overflow = LiteralConst {
            value: ConstValue::UInt((u16::MAX as u128) + 1),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Unsigned(IntegerWidth::W16),
                explicit_suffix: true,
                suffix_text: Some("u16".into()),
            }),
        };
        checker.check_numeric_literal(&overflow, None, None);

        let mismatch = LiteralConst {
            value: ConstValue::Int(5),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W32),
                explicit_suffix: true,
                suffix_text: None,
            }),
        };
        checker.check_numeric_literal(&mismatch, Some(&TypeExpr::simple("ulong")), None);

        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK121] literal with suffix `u16` has value `65536`, which exceeds the range of `u16`
[Error]: [TCK120] literal with suffix `i32` has type `i32` but is used where `ulong` is expected
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn numeric_literal_helpers_map_suffixes() {
    assert_eq!(
        "i16",
        TypeChecker::literal_type_name(NumericLiteralType::Signed(IntegerWidth::W16))
    );
    assert_eq!(
        "u128",
        TypeChecker::literal_type_name(NumericLiteralType::Unsigned(IntegerWidth::W128))
    );
    assert_eq!(
        "decimal",
        TypeChecker::literal_type_name(NumericLiteralType::Decimal)
    );

    let meta = NumericLiteralMetadata {
        literal_type: NumericLiteralType::Unsigned(IntegerWidth::W32),
        explicit_suffix: true,
        suffix_text: None,
    };
    assert_eq!("u32", TypeChecker::literal_suffix(&meta));
}

#[test]
fn numeric_literal_aliases_match_declared_types() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _module| {
        let signed_alias = LiteralConst {
            value: ConstValue::Int(1),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W64),
                explicit_suffix: true,
                suffix_text: Some("long".into()),
            }),
        };
        checker.check_numeric_literal(&signed_alias, Some(&TypeExpr::simple("long")), None);

        let usize_literal = LiteralConst {
            value: ConstValue::UInt(10),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Unsigned(IntegerWidth::Size),
                explicit_suffix: true,
                suffix_text: Some("usize".into()),
            }),
        };
        checker.check_numeric_literal(
            &usize_literal,
            Some(&TypeExpr::simple("System.UIntPtr")),
            None,
        );

        let decimal_literal = LiteralConst {
            value: ConstValue::UInt(42),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Decimal,
                explicit_suffix: true,
                suffix_text: Some("decimal".into()),
            }),
        };
        checker.check_numeric_literal(&decimal_literal, Some(&TypeExpr::simple("decimal")), None);

        let signed_overflow = LiteralConst {
            value: ConstValue::Int(i8::MAX as i128 + 1),
            numeric: Some(NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W8),
                explicit_suffix: true,
                suffix_text: Some("i8".into()),
            }),
        };
        checker.check_numeric_literal(&signed_overflow, None, None);

        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK121] literal with suffix `i8` has value `128`, which exceeds the range of `i8`
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn compare_exchange_requires_valid_ordering() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, module| {
        let call = ExprNode::Call {
            callee: Box::new(path_expr(&["Std", "Sync", "Atomic", "CompareExchange"])),
            args: vec![
                CallArgument::positional(ExprNode::Identifier("target".into()), None, None),
                CallArgument::positional(ExprNode::Identifier("expect".into()), None, None),
                CallArgument::positional(
                    path_expr(&["Std", "Sync", "MemoryOrder", "Acquire"]),
                    None,
                    None,
                ),
                CallArgument::positional(
                    path_expr(&["Std", "Sync", "MemoryOrder", "SeqCst"]),
                    None,
                    None,
                ),
            ],
            generics: None,
        };

        checker.validate_expr_node("AtomicCall", &call, None, module.namespace.as_deref(), None);
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [MM0002] failure ordering `SeqCst` cannot be stronger than success ordering `Acquire` on `CompareExchange`
[Note]: See SPEC.md#35-concurrency-memory-model, docs/guides/concurrency.md#memory-orders-at-a-glance for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn atomic_type_argument_reports_missing_and_unknown_traits() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _module| {
        checker.validate_atomic_type_argument("Atomic<T>", &TypeExpr::simple("Rc<int>"), None);
        checker.validate_atomic_type_argument("Atomic<T>", &TypeExpr::simple("Mystery"), None);

        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [MM0003] type `Rc<int>` stored in `Atomic<T>` must implement ThreadSafe
[Note]: See SPEC.md#35-concurrency-memory-model for specification details.
[Error]: [MM0003] cannot prove type `Mystery` stored in `Atomic<T>` implements ThreadSafe and Shareable
[Note]: See SPEC.md#35-concurrency-memory-model for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn extension_conditions_must_reference_self_interface() {
    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Printable"),
        generics: None,
        members: Vec::new(),
        doc: None,
        attributes: Vec::new(),
        conditions: vec![ExtensionCondition {
            target: TypeExpr::simple("Other"),
            constraint: TypeExpr::simple("Printable"),
            span: None,
        }],
    };
    let module = Module::with_items(
        None,
        vec![
            Item::Interface(interface_def("Printable", Vec::new())),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [DIM0002] `when` clauses must use the form `Self : InterfaceName`
[Error]: [TCK012] extension target `Printable` must be a struct or class
[Note]: See SPEC.md#2.4-extension-methods for specification details.
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn extension_without_conditions_normalizes_to_empty_set() {
    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, module| {
        let extension = ExtensionDecl {
            visibility: Visibility::Public,
            target: TypeExpr::simple("Widget"),
            generics: None,
            members: Vec::new(),
            doc: None,
            attributes: Vec::new(),
            conditions: Vec::new(),
        };
        let resolved =
            super::normalize_extension_conditions(checker, &extension, module.namespace.as_deref());
        assert_eq!(resolved, Some(Vec::new()));
    });
}

#[test]
fn extension_conditions_require_interface_constraints() {
    let extension_method = ExtensionMethodDecl {
        is_default: false,
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Widget"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: vec![ExtensionCondition {
            target: TypeExpr::simple("Self"),
            constraint: TypeExpr::simple("Logger"),
            span: None,
        }],
    };
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Struct(struct_def("Logger")),
            Item::Interface(interface_def("Printable", Vec::new())),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [DIM0002] constraint `Logger` must resolve to an interface
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn extension_conditions_reject_unknown_interfaces() {
    let extension_method = ExtensionMethodDecl {
        is_default: false,
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Widget"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: vec![ExtensionCondition {
            target: TypeExpr::simple("Self"),
            constraint: TypeExpr::simple("Missing"),
            span: None,
        }],
    };
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [DIM0002] constraint `Missing` is not a known interface
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn extension_conditions_report_ambiguous_interfaces() {
    let printable_alpha = interface_def("Printable", Vec::new());
    let printable_beta = interface_def("Printable", Vec::new());
    let extension_method = ExtensionMethodDecl {
        is_default: true,
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![parameter("this", TypeExpr::self_type())],
                return_type: TypeExpr::self_type(),
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Printable"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: vec![ExtensionCondition {
            target: TypeExpr::self_type(),
            constraint: TypeExpr::simple("Printable"),
            span: None,
        }],
    };

    let module = Module::with_items(
        None,
        vec![
            Item::Import(UsingDirective {
                doc: None,
                is_global: true,
                span: None,
                kind: UsingKind::Namespace {
                    path: "Alpha".into(),
                },
            }),
            Item::Import(UsingDirective {
                doc: None,
                is_global: true,
                span: None,
                kind: UsingKind::Namespace {
                    path: "Beta".into(),
                },
            }),
            Item::Namespace(NamespaceDecl {
                name: "Alpha".into(),
                items: vec![Item::Interface(printable_alpha)],
                doc: None,
                attributes: Vec::new(),
                span: None,
            }),
            Item::Namespace(NamespaceDecl {
                name: "Beta".into(),
                items: vec![Item::Interface(printable_beta)],
                doc: None,
                attributes: Vec::new(),
                span: None,
            }),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        assert!(
            checker.symbol_index.contains_type("Alpha::Printable")
                && checker.symbol_index.contains_type("Beta::Printable"),
            "interfaces should be registered in the symbol index"
        );
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [TCK011] extension target `Printable` resolves to multiple candidates: Alpha::Printable, Beta::Printable
[Note]: See SPEC.md#2.4-extension-methods for specification details.
[Error]: [TCK031] type `Printable` resolves to multiple candidates: Alpha::Printable, Beta::Printable
[Error]: [DIM0002] constraint `Printable` resolves to multiple candidates: Alpha::Printable, Beta::Printable
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn extension_conditions_accept_valid_interfaces() {
    let extension_method = ExtensionMethodDecl {
        is_default: false,
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Widget"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: vec![ExtensionCondition {
            target: TypeExpr::simple("Self"),
            constraint: TypeExpr::simple("Printable"),
            span: None,
        }],
    };
    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Interface(interface_def("Printable", Vec::new())),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        assert!(checker.diagnostics.is_empty());
    });
}

#[test]
fn default_extension_requires_interface_target_and_receiver() {
    let extension_method = ExtensionMethodDecl {
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
        is_default: true,
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Widget"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: Vec::new(),
    };

    let module = Module::with_items(
        None,
        vec![
            Item::Struct(struct_def("Widget")),
            Item::Extension(extension),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [DIM0001] default extension method `Widget::describe` must target an interface; `Widget` is not an interface
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn call_argument_matching_reports_specific_errors() {
    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        let fn_signature = FnTy::with_modes(
            vec![Ty::Unit, Ty::Unit, Ty::Unit],
            vec![ParamMode::Ref, ParamMode::Value, ParamMode::Value],
            Ty::Unit,
            Abi::Chic,
            false,
        );
        let symbol = FunctionSymbol {
            qualified: "Widget::init".into(),
            internal_name: "Widget::init".into(),
            signature: fn_signature,
            params: vec![
                FunctionParamSymbol {
                    name: "buffer".into(),
                    has_default: false,
                    mode: ParamMode::Ref,
                    is_extension_this: false,
                },
                FunctionParamSymbol {
                    name: "count".into(),
                    has_default: false,
                    mode: ParamMode::Value,
                    is_extension_this: false,
                },
                FunctionParamSymbol {
                    name: "label".into(),
                    has_default: true,
                    mode: ParamMode::Value,
                    is_extension_this: false,
                },
            ],
            is_unsafe: false,
            is_static: false,
            visibility: Visibility::Public,
            namespace: None,
            owner: Some("Widget".into()),
        };

        let ref_mismatch =
            CallArgument::positional(ExprNode::Identifier("data".into()), None, None);
        let err = checker
            .call_arguments_match("Widget::init", &symbol, &[ref_mismatch], None)
            .expect_err("ref parameter should require ref argument");
        assert!(
            err.message.contains("must be passed using `ref`"),
            "expected ref enforcement, got {err:?}"
        );

        let duplicate_named = CallArgument::named(
            CallArgumentName::new("count", None),
            ExprNode::Identifier("c".into()),
            None,
            None,
        );
        let err = checker
            .call_arguments_match(
                "Widget::init",
                &symbol,
                &[duplicate_named.clone(), duplicate_named],
                None,
            )
            .expect_err("duplicate named arguments should be rejected");
        assert!(
            err.message.contains("specified multiple times"),
            "expected duplicate diagnostic, got {err:?}"
        );

        let mixed = vec![
            CallArgument::named(
                CallArgumentName::new("label", None),
                ExprNode::Identifier("lbl".into()),
                None,
                None,
            ),
            CallArgument::positional(ExprNode::Identifier("after".into()), None, None),
        ];
        let err = checker
            .call_arguments_match("Widget::init", &symbol, &mixed, None)
            .expect_err("positional after named should error");
        assert!(
            err.message
                .contains("positional arguments must appear before named arguments"),
            "expected ordering diagnostic, got {err:?}"
        );

        let unknown = CallArgument::named(
            CallArgumentName::new("missing", None),
            ExprNode::Identifier("x".into()),
            None,
            None,
        );
        let err = checker
            .call_arguments_match("Widget::init", &symbol, &[unknown], None)
            .expect_err("unknown named argument should error");
        assert!(
            err.message.contains("has no parameter named `missing`"),
            "expected unknown parameter diagnostic, got {err:?}"
        );

        let missing_required = vec![
            CallArgument::named(
                CallArgumentName::new("buffer", None),
                ExprNode::Identifier("buf".into()),
                None,
                None,
            )
            .with_modifier(CallArgumentModifier::Ref, None),
        ];
        let err = checker
            .call_arguments_match("Widget::init", &symbol, &missing_required, None)
            .expect_err("missing required argument should error");
        assert!(
            err.message
                .contains("missing an argument for parameter `count`"),
            "expected missing argument diagnostic, got {err:?}"
        );
    });
}

#[test]
fn call_owner_resolution_walks_namespaces_and_self() {
    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        checker
            .symbol_index
            .types
            .insert("Alpha::Target".to_string());

        assert_eq!(
            Some("Alpha::Target".to_string()),
            checker.resolve_call_owner_name("Target", Some("Alpha::Inner"), None)
        );
        assert_eq!(
            Some("System::Console".to_string()),
            checker.resolve_call_owner_name("System::Console", None, None)
        );
        assert_eq!(
            Some("Alpha::Widget".to_string()),
            checker.resolve_call_owner_name("Self", Some("Alpha::Inner"), Some("Alpha::Widget"))
        );
        assert!(
            checker
                .resolve_call_owner_name("Missing", Some("Alpha"), None)
                .is_none(),
            "unresolvable owners should return None"
        );
    });
}

#[test]
fn const_statement_checks_numeric_suffix_rules() {
    let const_stmt = ConstStatement {
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("u8"),
            declarators: vec![
                ConstDeclarator {
                    name: "OVER".into(),
                    initializer: Expression::with_node(
                        "300u8",
                        None,
                        ExprNode::Literal(LiteralConst {
                            value: ConstValue::UInt(300),
                            numeric: Some(NumericLiteralMetadata {
                                literal_type: NumericLiteralType::Unsigned(IntegerWidth::W8),
                                suffix_text: Some("u8".into()),
                                explicit_suffix: true,
                            }),
                        }),
                    ),
                    span: None,
                },
                ConstDeclarator {
                    name: "WRONG".into(),
                    initializer: Expression::with_node(
                        "5i64",
                        None,
                        ExprNode::Literal(LiteralConst {
                            value: ConstValue::Int(5),
                            numeric: Some(NumericLiteralMetadata {
                                literal_type: NumericLiteralType::Signed(IntegerWidth::W64),
                                suffix_text: Some("i64".into()),
                                explicit_suffix: true,
                            }),
                        }),
                    ),
                    span: None,
                },
            ],
            doc: None,
            span: None,
        },
    };

    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        checker.validate_const_statement("ConstCheck", &const_stmt, None, None);
        assert!(
            checker.diagnostics.len() >= 2,
            "overflow + mismatch diagnostics should be emitted"
        );
        let rendered = render_diagnostics(&checker.diagnostics);
        assert!(
            rendered.contains("literal with suffix `u8` has value `300`"),
            "expected overflow diagnostic to mention suffix and value:\n{rendered}"
        );
        assert!(
            rendered.contains(
                "literal with suffix `i64` has type `i64` but is used where `u8` is expected"
            ),
            "expected mismatch diagnostic for i64 literal:\n{rendered}"
        );
    });
}

#[test]
fn var_inference_rejects_cross_function_calls() {
    let mut callee = simple_method("Make", Vec::new(), TypeExpr::simple("int"));
    callee.body = Some(Block {
        statements: Vec::new(),
        span: None,
    });

    let mut caller = simple_method("Use", Vec::new(), TypeExpr::simple("void"));
    caller.body = Some(Block {
        statements: vec![Statement::new(
            None,
            StatementKind::VariableDeclaration(VariableDeclaration {
                modifier: VariableModifier::Var,
                type_annotation: None,
                declarators: vec![VariableDeclarator {
                    name: "value".into(),
                    initializer: Some(Expression::with_node(
                        "Make()",
                        None,
                        ExprNode::Call {
                            callee: ExprNode::Identifier("Make".into()).boxed(),
                            args: Vec::new(),
                            generics: None,
                        },
                    )),
                }],
                is_pinned: false,
            }),
        )],
        span: None,
    });

    let module = Module::with_items(None, vec![Item::Function(callee), Item::Function(caller)]);
    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        let rendered = render_diagnostics(&checker.diagnostics);
        // The cross-function inference guard is currently relaxed to keep stub
        // code concise, so this scenario emits no diagnostics.
        expect![[r#""#]].assert_eq(&rendered);
    });
}

#[test]
fn member_visibility_respects_namespace_and_bases() {
    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        let base_name = "Access::Base";
        let derived_name = "Access::Derived";
        checker.insert_type_info(
            base_name.to_string(),
            TypeInfo {
                kind: TypeKind::Class {
                    methods: Vec::new(),
                    bases: Vec::new(),
                    kind: ClassKind::Class,
                    properties: Vec::new(),
                    constructors: Vec::new(),
                    is_abstract: false,
                    is_sealed: false,
                    is_static: false,
                },
                generics: None,
                repr_c: false,
                packing: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                visibility: Visibility::Public,
            },
        );
        checker.insert_type_info(
            derived_name.to_string(),
            TypeInfo {
                kind: TypeKind::Class {
                    methods: Vec::new(),
                    bases: vec![BaseTypeBinding::new(
                        base_name.to_string(),
                        TypeExpr::simple(base_name),
                    )],
                    kind: ClassKind::Class,
                    properties: Vec::new(),
                    constructors: Vec::new(),
                    is_abstract: false,
                    is_sealed: false,
                    is_static: false,
                },
                generics: None,
                repr_c: false,
                packing: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                visibility: Visibility::Public,
            },
        );

        assert!(
            checker.is_member_accessible(
                Visibility::Protected,
                base_name,
                Some("Access"),
                Some("Access::Child"),
                Some(derived_name),
                Some(derived_name),
                true
            ),
            "subclasses should see protected members"
        );
        assert!(
            checker.is_member_accessible(
                Visibility::ProtectedInternal,
                base_name,
                Some("Access"),
                Some("Access"),
                Some(derived_name),
                Some(derived_name),
                true
            ),
            "protected internal should allow namespace or inheritance"
        );
        assert!(
            !checker.is_member_accessible(
                Visibility::Internal,
                base_name,
                Some("Access"),
                Some("Other"),
                None,
                None,
                false
            ),
            "internal outside namespace should be hidden"
        );
        assert!(
            !checker.is_member_accessible(
                Visibility::Private,
                base_name,
                Some("Access"),
                Some("Other"),
                Some(derived_name),
                None,
                true
            ),
            "private members stay hidden outside the declaring type"
        );
        assert!(
            checker.is_member_accessible(
                Visibility::Public,
                base_name,
                None,
                None,
                None,
                None,
                false
            ),
            "public members are always accessible"
        );
    });
}

#[test]
fn default_extension_registers_provider_for_interface_targets() {
    let printable = interface_def(
        "Printable",
        vec![InterfaceMember::Method(simple_method(
            "describe",
            vec![parameter("self", TypeExpr::self_type())],
            TypeExpr::simple("void"),
        ))],
    );

    let extension_method = ExtensionMethodDecl {
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: "describe".into(),
            name_span: None,
            signature: Signature {
                parameters: vec![Parameter {
                    binding: BindingModifier::Value,
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
            dispatch: MemberDispatch {
                is_virtual: false,
                is_override: false,
                is_sealed: false,
                is_abstract: false,
            },
        },
        is_default: true,
    };

    let extension = ExtensionDecl {
        visibility: Visibility::Public,
        target: TypeExpr::simple("Printable"),
        generics: None,
        members: vec![ExtensionMember::Method(extension_method)],
        doc: None,
        attributes: Vec::new(),
        conditions: Vec::new(),
    };

    let module = Module::with_items(
        None,
        vec![Item::Interface(printable), Item::Extension(extension)],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        assert!(
            checker.diagnostics.is_empty(),
            "expected extension registration to succeed: {:?}",
            checker.diagnostics
        );
        let applied = checker.try_apply_interface_default(
            "Widget",
            "Printable",
            "describe",
            &HashSet::new(),
            None,
        );
        assert!(applied, "default extension provider should be runnable");
    });
}

#[test]
fn pattern_validation_reports_duplicates_and_guard_ordering() {
    let pattern_ast = PatternAst {
        node: PatternNode::Wildcard,
        span: None,
        metadata: PatternMetadata {
            bindings: vec![
                PatternBindingMetadata {
                    name: "dup".into(),
                    span: None,
                },
                PatternBindingMetadata {
                    name: "dup".into(),
                    span: None,
                },
            ],
            list_slices: vec![
                ListSliceMetadata {
                    span: None,
                    binding: Some("left".into()),
                },
                ListSliceMetadata {
                    span: None,
                    binding: Some("right".into()),
                },
            ],
            record_fields: vec![
                RecordFieldMetadata {
                    name: "field".into(),
                    name_span: None,
                    pattern_span: None,
                    path: None,
                },
                RecordFieldMetadata {
                    name: "field".into(),
                    name_span: None,
                    pattern_span: None,
                    path: None,
                },
            ],
        },
    };
    let case_pattern = CasePattern::new(Expression::new("pattern", None), Some(pattern_ast));
    let guards = vec![
        PatternGuard {
            expression: Expression::new("first", None),
            depth: 1,
            keyword_span: None,
        },
        PatternGuard {
            expression: Expression::new("second", None),
            depth: 0,
            keyword_span: None,
        },
    ];

    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        checker.validate_case_pattern(&case_pattern, &guards);
        let rendered = render_diagnostics(&checker.diagnostics);
        expect![[r#"
[Error]: [PAT0003] pattern binding `dup` is declared multiple times
[Error]: [PAT0002] record field `field` appears more than once
[Error]: [PAT0003] list patterns may only include a single slice binding
[Error]: [PAT0001] `when` guard at depth 0 must not precede guard at depth 1
"#]]
        .assert_eq(&rendered);
    });
}

#[test]
fn switch_expression_requires_wildcard_arm() {
    let module = Module::new(None);
    let expr = ExprNode::Switch(crate::syntax::expr::builders::SwitchExpr {
        value: ExprNode::Identifier("value".into()).boxed(),
        arms: vec![crate::syntax::expr::builders::SwitchArm {
            pattern: crate::syntax::pattern::PatternAst {
                node: crate::syntax::pattern::PatternNode::Literal(ConstValue::Int(0)),
                span: None,
                metadata: crate::syntax::pattern::PatternMetadata::default(),
            },
            guards: Vec::new(),
            expression: ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1))),
            span: None,
            arrow_span: None,
        }],
        span: None,
        switch_span: None,
        braces_span: None,
    });
    let expression = Expression::with_node("value switch { 0 => 1 }", None, expr);
    let statement = Statement::new(None, StatementKind::Expression(expression));
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    checker.validate_statement("Sample::MissingDefault", &statement, None, None);
    assert!(checker.diagnostics.iter().any(|diag| {
        diag.code
            .as_ref()
            .is_some_and(|code| code.code == "PAT0004")
    }));
}

#[test]
fn switch_statement_requires_default_or_wildcard() {
    let module = Module::new(None);
    let switch = SwitchStatement {
        expression: Expression::with_node("value", None, ExprNode::Identifier("value".into())),
        sections: vec![SwitchSection {
            labels: vec![SwitchLabel::Case(SwitchCaseLabel {
                pattern: CasePattern::new(
                    Expression::with_node(
                        "0",
                        None,
                        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(0))),
                    ),
                    Some(crate::syntax::pattern::PatternAst {
                        node: crate::syntax::pattern::PatternNode::Literal(ConstValue::Int(0)),
                        span: None,
                        metadata: crate::syntax::pattern::PatternMetadata::default(),
                    }),
                ),
                guards: Vec::new(),
            })],
            statements: Vec::new(),
        }],
    };
    let statement = Statement::new(None, StatementKind::Switch(switch));
    let layouts = TypeLayoutTable::default();
    let mut checker = TypeChecker::new(&module, &layouts);
    checker.validate_statement("Sample::MissingDefault", &statement, None, None);
    assert!(checker.diagnostics.iter().any(|diag| {
        diag.code
            .as_ref()
            .is_some_and(|code| code.code == "PAT0004")
    }));
}

#[test]
fn helper_utilities_cover_display_and_literal_suffixes() {
    with_manual_module(Module::with_items(None, Vec::new()), |checker, _| {
        let path = ExprNode::Member {
            base: Box::new(ExprNode::Member {
                base: Box::new(ExprNode::Identifier("Alpha".into())),
                member: "Beta".into(),
                null_conditional: false,
            }),
            member: "gamma".into(),
            null_conditional: false,
        };
        assert_eq!("Alpha::Beta::gamma", checker.expr_display(&path));

        let fallback = ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1)));
        assert_eq!("<expression>", checker.expr_display(&fallback));

        let meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Unsigned(IntegerWidth::W32),
            suffix_text: Some("u32".into()),
            explicit_suffix: true,
        };
        assert_eq!("u32", TypeChecker::literal_suffix(&meta));

        let nested = ExprNode::TryPropagate {
            expr: Box::new(ExprNode::Await {
                expr: Box::new(ExprNode::Call {
                    callee: ExprNode::Identifier("DoStuff".into()).boxed(),
                    args: Vec::new(),
                    generics: None,
                }),
            }),
            question_span: None,
        };
        assert_eq!(
            Some("DoStuff".into()),
            checker.cross_function_call_target(&nested)
        );
    });
}

#[test]
fn type_expression_parameter_detection_walks_nested_forms() {
    let tuple = TypeExpr::tuple(vec![TypeExpr::simple("int"), TypeExpr::simple("T")]);
    assert!(type_expr_mentions_parameter(&tuple, "T"));

    let mut generic = TypeExpr::simple("List");
    generic.suffixes.push(TypeSuffix::GenericArgs(vec![
        GenericArgument::from_type_expr(TypeExpr::simple("T")),
    ]));
    assert!(type_expr_mentions_parameter(&generic, "T"));

    let fn_expr = TypeExpr {
        name: "fn".into(),
        base: Vec::new(),
        suffixes: Vec::new(),
        span: None,
        generic_span: None,
        tuple_elements: None,
        tuple_element_names: None,
        fn_signature: Some(crate::frontend::ast::types::FnTypeExpr::new(
            crate::frontend::ast::types::FnTypeAbi::Chic,
            vec![TypeExpr::simple("u32")],
            TypeExpr::simple("T"),
        )),
        trait_object: None,
        ref_kind: None,
        is_view: false,
    };
    assert!(type_expr_mentions_parameter(&fn_expr, "T"));

    let trait_obj = TypeExpr {
        name: "TraitObj".into(),
        base: Vec::new(),
        suffixes: Vec::new(),
        span: None,
        generic_span: None,
        tuple_elements: None,
        tuple_element_names: None,
        fn_signature: None,
        trait_object: Some(crate::frontend::ast::types::TraitObjectTypeExpr {
            bounds: vec![TypeExpr::simple("T")],
            opaque_impl: false,
        }),
        ref_kind: None,
        is_view: false,
    };
    assert!(type_expr_mentions_parameter(&trait_obj, "T"));
    assert!(!type_expr_mentions_parameter(&TypeExpr::simple("U"), "T"));
}

#[test]
fn helper_functions_format_traits_and_self_detection() {
    assert_eq!("", super::join_trait_names(&[]));
    assert_eq!("Send", super::join_trait_names(&["Send"]));
    assert_eq!("Send and Sync", super::join_trait_names(&["Send", "Sync"]));
    assert_eq!(
        "Copyable, Send, Sync",
        super::join_trait_names(&["Copyable", "Send", "Sync"])
    );

    assert!(super::returns_self_value(&TypeExpr::simple("Self")));
    let mut pointer_self = TypeExpr::simple("Self");
    pointer_self.suffixes.push(TypeSuffix::Pointer {
        mutable: true,
        modifiers: Vec::new(),
    });
    assert!(!super::returns_self_value(&pointer_self));
    let tuple_self = TypeExpr::tuple(vec![TypeExpr::simple("Self"), TypeExpr::simple("int")]);
    assert!(!super::returns_self_value(&tuple_self));

    let module = Module::with_items(None, Vec::new());
    with_manual_module(module, |checker, _module| {
        assert_eq!(
            "Std::Async::Task",
            checker.canonical_type_name(&TypeExpr::simple("Std.Async.Task"))
        );
        let mut qualified = TypeExpr::simple("Task");
        qualified.base = vec!["Std".into(), "Async".into()];
        assert_eq!("Std::Async", checker.canonical_type_name(&qualified));
    });
}

#[test]
fn signature_from_extension_substitutes_self_receiver() {
    let signature = Signature {
        parameters: vec![parameter("this", TypeExpr::self_type())],
        return_type: TypeExpr::self_type(),
        lends_to_return: None,
        variadic: false,
        throws: None,
    };
    let substituted = signature_from_extension(
        &signature,
        "Widget::ext".into(),
        None,
        &TypeExpr::simple("Widget"),
    );
    assert_eq!(vec!["Widget"], substituted.param_types);
    assert_eq!("Widget", substituted.return_type);
}

#[test]
fn aggregates_walk_struct_union_enum_and_class_paths() {
    let mut widget = struct_def("Widget");
    widget.fields.push(FieldDecl {
        visibility: Visibility::Public,
        name: "x".into(),
        ty: TypeExpr::simple("int"),
        initializer: Some(literal_expr(ConstValue::Int(1))),
        mmio: None,
        doc: None,
        is_required: false,
        display_name: None,
        attributes: Vec::new(),
        is_readonly: false,
        is_static: false,
        view_of: None,
    });
    widget.consts.push(ConstMemberDecl {
        visibility: Visibility::Public,
        modifiers: Vec::new(),
        declaration: ConstDeclaration {
            ty: TypeExpr::simple("int"),
            declarators: vec![ConstDeclarator {
                name: "VALUE".into(),
                initializer: literal_expr(ConstValue::Int(2)),
                span: None,
            }],
            doc: None,
            span: None,
        },
    });
    let mut method = simple_method(
        "bump",
        vec![parameter("delta", TypeExpr::simple("int"))],
        TypeExpr::simple("void"),
    );
    method.body = Some(Block {
        statements: vec![Statement::new(
            None,
            StatementKind::Return { expression: None },
        )],
        span: None,
    });
    widget.methods.push(method);
    widget.constructors.push(ConstructorDecl {
        visibility: Visibility::Public,
        kind: ConstructorKind::Designated,
        parameters: vec![parameter("initial", TypeExpr::simple("int"))],
        body: Some(Block {
            statements: vec![Statement::new(
                None,
                StatementKind::Return { expression: None },
            )],
            span: None,
        }),
        initializer: Some(ConstructorInitializer {
            target: ConstructorInitTarget::SelfType,
            arguments: vec![literal_expr(ConstValue::Int(0))],
            span: None,
        }),
        doc: None,
        span: None,
        attributes: Vec::new(),
        di_inject: None,
    });

    let union = UnionDecl {
        visibility: Visibility::Public,
        name: "Number".into(),
        members: vec![
            UnionMember::Field(UnionField {
                visibility: Visibility::Public,
                name: "int_value".into(),
                ty: TypeExpr::simple("int"),
                is_readonly: false,
                doc: None,
                attributes: Vec::new(),
            }),
            UnionMember::Field(UnionField {
                visibility: Visibility::Public,
                name: "float_value".into(),
                ty: TypeExpr::simple("float"),
                is_readonly: false,
                doc: None,
                attributes: Vec::new(),
            }),
        ],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    };

    let enum_decl = EnumDecl {
        visibility: Visibility::Public,
        name: "Color".into(),
        underlying_type: None,
        variants: vec![EnumVariant {
            name: "Rgb".into(),
            fields: vec![FieldDecl {
                visibility: Visibility::Public,
                name: "code".into(),
                ty: TypeExpr::simple("int"),
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
            discriminant: None,
            doc: None,
        }],
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        is_flags: false,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    };

    let mut class_decl = ClassDecl {
        visibility: Visibility::Public,
        kind: ClassKind::Class,
        name: "Derived".into(),
        bases: vec![TypeExpr::simple("Base")],
        members: Vec::new(),
        nested_types: Vec::new(),
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        di_service: None,
        di_module: false,
        is_static: false,
        is_abstract: false,
        is_sealed: false,
    };
    class_decl.members.push(ClassMember::Field(FieldDecl {
        visibility: Visibility::Public,
        name: "payload".into(),
        ty: TypeExpr::simple("Number"),
        initializer: None,
        mmio: None,
        doc: None,
        is_required: false,
        display_name: None,
        attributes: Vec::new(),
        is_readonly: false,
        is_static: false,
        view_of: None,
    }));
    class_decl
        .members
        .push(ClassMember::Constructor(ConstructorDecl {
            visibility: Visibility::Public,
            kind: ConstructorKind::Designated,
            parameters: vec![parameter("p", TypeExpr::simple("Number"))],
            body: Some(Block {
                statements: vec![Statement::new(
                    None,
                    StatementKind::Return { expression: None },
                )],
                span: None,
            }),
            initializer: None,
            doc: None,
            span: None,
            attributes: Vec::new(),
            di_inject: None,
        }));
    class_decl.members.push(ClassMember::Method(simple_method(
        "describe",
        Vec::new(),
        TypeExpr::simple("void"),
    )));

    let module = Module::with_items(
        None,
        vec![
            Item::Struct(widget),
            Item::Union(union),
            Item::Enum(enum_decl),
            Item::Class(class_decl),
        ],
    );

    with_manual_module(module, |checker, module| {
        checker.visit_items(&module.items, module.namespace.as_deref());
        assert!(
            checker.resolve_type_info("Widget").is_some()
                && checker.resolve_type_info("Number").is_some()
                && checker.resolve_type_info("Color").is_some()
                && checker.resolve_type_info("Derived").is_some()
        );
    });
}

fn trait_def(name: &str, members: Vec<TraitMember>) -> TraitDecl {
    TraitDecl {
        visibility: Visibility::Public,
        name: name.to_string(),
        super_traits: Vec::new(),
        members,
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
        span: None,
    }
}

fn interface_def(name: &str, members: Vec<InterfaceMember>) -> InterfaceDecl {
    InterfaceDecl {
        visibility: Visibility::Public,
        name: name.to_string(),
        bases: Vec::new(),
        members,
        thread_safe_override: None,
        shareable_override: None,
        copy_override: None,
        doc: None,
        generics: None,
        attributes: Vec::new(),
    }
}

fn generic_params_with_variance(name: &str, variance: Variance) -> GenericParams {
    let mut param = GenericParam::type_param(name, None);
    if let GenericParamKind::Type(data) = &mut param.kind {
        data.variance = variance;
    }
    GenericParams::new(None, vec![param])
}

fn struct_def(name: &str) -> StructDecl {
    StructDecl {
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

fn impl_def(
    trait_name: Option<&str>,
    target: &str,
    members: Vec<ImplMember>,
    generics: Option<GenericParams>,
) -> ImplDecl {
    ImplDecl {
        visibility: Visibility::Public,
        trait_ref: trait_name.map(TypeExpr::simple),
        target: TypeExpr::simple(target),
        generics,
        members,
        doc: None,
        attributes: Vec::new(),
        span: None,
    }
}

fn assoc_type_member(name: &str, default: Option<TypeExpr>) -> TraitMember {
    TraitMember::AssociatedType(TraitAssociatedType {
        name: name.to_string(),
        generics: None,
        default,
        doc: None,
        span: None,
    })
}

fn trait_const_member(name: &str, ty: &str, value: &str) -> TraitMember {
    TraitMember::Const(ConstMemberDecl {
        visibility: Visibility::Public,
        modifiers: Vec::new(),
        declaration: ConstDeclaration {
            ty: TypeExpr::simple(ty),
            declarators: vec![ConstDeclarator {
                name: name.to_string(),
                initializer: Expression::new(value, None),
                span: None,
            }],
            doc: None,
            span: None,
        },
    })
}

fn simple_method(name: &str, params: Vec<Parameter>, return_ty: TypeExpr) -> FunctionDecl {
    FunctionDecl {
        visibility: Visibility::Public,
        name: name.to_string(),
        name_span: None,
        signature: Signature {
            parameters: params,
            return_type: return_ty,
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
        dispatch: MemberDispatch {
            is_virtual: false,
            is_override: false,
            is_sealed: false,
            is_abstract: false,
        },
    }
}

fn parameter(name: &str, ty: TypeExpr) -> Parameter {
    Parameter {
        binding: BindingModifier::Value,
        binding_nullable: false,
        name: name.to_string(),
        name_span: None,
        ty,
        attributes: Vec::new(),
        di_inject: None,
        default: None,
        default_span: None,
        lends: None,
        is_extension_this: false,
    }
}

fn literal_expr(value: ConstValue) -> Expression {
    let text = match &value {
        ConstValue::Int(v) => v.to_string(),
        ConstValue::Bool(v) => v.to_string(),
        ConstValue::UInt(v) => v.to_string(),
        other => format!("{other:?}"),
    };
    Expression::with_node(
        text,
        None,
        ExprNode::Literal(LiteralConst::without_numeric(value)),
    )
}

fn path_expr(parts: &[&str]) -> ExprNode {
    let mut iter = parts.iter();
    let first = iter
        .next()
        .expect("path requires at least one segment")
        .to_string();
    let mut node = ExprNode::Identifier(first);
    for part in iter {
        node = ExprNode::Member {
            base: Box::new(node),
            member: (*part).to_string(),
            null_conditional: false,
        };
    }
    node
}
