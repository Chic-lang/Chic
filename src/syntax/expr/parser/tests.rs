use super::*;
use crate::frontend::diagnostics::Span;
use crate::mir::{BinOp, ConstValue};
use crate::syntax::expr::builders::{
    AssignOp, CallArgumentModifier, CallArgumentName, InlineAsmOperandMode, InlineAsmRegister,
    InlineAsmRegisterClass, InlineAsmTemplateOperandRef, InlineAsmTemplatePiece, InlineBindingKind,
    LambdaBody, LiteralConst, NewInitializer,
};
use crate::syntax::pattern::PatternNode;

#[test]
fn parses_lambda_with_expression_body() {
    let expr =
        parse_expression("(int value) => value + delta").expect("lambda expression should parse");
    match expr {
        ExprNode::Lambda(lambda) => {
            assert!(!lambda.is_async, "lambda should not be async");
            assert!(lambda.captures.is_empty(), "captures collected later");
            assert_eq!(lambda.params.len(), 1, "expected single parameter");
            let param = &lambda.params[0];
            assert_eq!(param.name, "value");
            assert_eq!(param.ty.as_deref(), Some("int"));
            match &lambda.body {
                LambdaBody::Expression(body) => match body.as_ref() {
                    ExprNode::Binary { op, .. } => {
                        assert!(matches!(op, BinOp::Add));
                    }
                    other => panic!("unexpected lambda body node: {other:?}", other = other),
                },
                other => panic!("expected expression body, found {other:?}", other = other),
            }
        }
        other => panic!("expected lambda expression, found {other:?}", other = other),
    }
}

#[test]
fn parses_async_lambda_with_block_body() {
    let expr = parse_expression("async (Foo bar) => { return bar.Invoke(); }")
        .expect("async lambda should parse");
    match expr {
        ExprNode::Lambda(lambda) => {
            assert!(lambda.is_async, "lambda should be async");
            assert_eq!(lambda.params.len(), 1);
            let param = &lambda.params[0];
            assert_eq!(param.name, "bar");
            assert_eq!(param.ty.as_deref(), Some("Foo"));
            match &lambda.body {
                LambdaBody::Block(block) => {
                    assert!(block.text.trim_start().starts_with('{'));
                    assert!(block.text.trim_end().ends_with('}'));
                }
                other => panic!("expected block body, found {other:?}", other = other),
            }
        }
        other => panic!("expected lambda expression, found {other:?}", other = other),
    }
}

#[test]
fn lambda_parameters_record_default_expression_nodes() {
    let expr =
        parse_expression("(int count = 3) => count").expect("lambda with default should parse");
    let ExprNode::Lambda(lambda) = expr else {
        panic!("expected lambda expression");
    };
    assert_eq!(lambda.params.len(), 1);
    let param = &lambda.params[0];
    let default = param
        .default
        .as_ref()
        .expect("expected default expression on lambda parameter");
    assert_eq!(default.text.trim(), "3");
    let Some(node) = default.node.as_ref() else {
        panic!("default should carry parsed node");
    };
    match node {
        ExprNode::Literal(LiteralConst {
            value: ConstValue::Int(value),
            ..
        }) => assert_eq!(*value, 3),
        other => panic!("expected literal node, found {other:?}", other = other),
    }
}

#[test]
fn parses_await_expression_into_node() {
    let expr = parse_expression("await future").expect("await expression should parse");
    match expr {
        ExprNode::Await { expr: inner } => match inner.as_ref() {
            ExprNode::Identifier(name) if name == "future" => {}
            other => panic!(
                "expected await operand identifier, found {other:?}",
                other = other
            ),
        },
        other => panic!("expected await node, found {other:?}", other = other),
    }
}

#[test]
fn parenthesized_expression_is_not_lambda() {
    let expr = parse_expression("(value)").expect("group expression should parse");
    match expr {
        ExprNode::Parenthesized(inner) => match inner.as_ref() {
            ExprNode::Identifier(name) if name == "value" => {}
            other => panic!("unexpected inner expression: {other:?}", other = other),
        },
        other => panic!(
            "expected parenthesized expression, found {other:?}",
            other = other
        ),
    }
}

#[test]
fn parses_multi_index_expression() {
    let expr = parse_expression("matrix[row, column]").expect("index expression should parse");
    match expr {
        ExprNode::Index {
            base,
            indices,
            null_conditional,
        } => {
            assert!(
                !null_conditional,
                "index expression should not be null-conditional"
            );
            assert_eq!(indices.len(), 2, "expected two indices");
            assert!(matches!(
                base.as_ref(),
                ExprNode::Identifier(name) if name == "matrix"
            ));
            assert!(matches!(
                indices[0],
                ExprNode::Identifier(ref name) if name == "row"
            ));
            assert!(matches!(
                indices[1],
                ExprNode::Identifier(ref name) if name == "column"
            ));
        }
        other => panic!("expected index expression, found {other:?}", other = other),
    }
}

#[test]
fn parses_range_expression_with_start_and_end() {
    let expr = parse_expression("start..end").expect("range expression should parse");
    match expr {
        ExprNode::Range(range) => {
            let start = range.start.expect("start endpoint missing");
            let end = range.end.expect("end endpoint missing");
            assert!(!range.inclusive, "range should be exclusive");
            assert!(!start.from_end);
            assert!(!end.from_end);
            assert!(matches!(*start.expr, ExprNode::Identifier(ref name) if name == "start"));
            assert!(matches!(*end.expr, ExprNode::Identifier(ref name) if name == "end"));
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn parses_index_from_end_operand() {
    let expr = parse_expression("^1").expect("index-from-end should parse");
    match expr {
        ExprNode::IndexFromEnd(inner) => match inner.expr.as_ref() {
            ExprNode::Literal(literal) => match &literal.value {
                ConstValue::Int(value) => assert_eq!(*value, 1),
                other => panic!("expected integer literal, found {other:?}"),
            },
            other => panic!("unexpected operand: {other:?}", other = other),
        },
        other => panic!(
            "expected index-from-end node, found {other:?}",
            other = other
        ),
    }
}

#[test]
fn parses_range_with_index_from_end_bounds() {
    let expr = parse_expression("^2..^1").expect("range with from-end bounds should parse");
    match expr {
        ExprNode::Range(range) => {
            let start = range.start.expect("start endpoint missing");
            let end = range.end.expect("end endpoint missing");
            assert!(start.from_end);
            assert!(end.from_end);
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn parses_inclusive_range() {
    let expr = parse_expression("0..=count").expect("inclusive range should parse");
    match expr {
        ExprNode::Range(range) => {
            assert!(range.inclusive, "inclusive flag should be set");
            let end = range.end.expect("end missing");
            assert!(matches!(*end.expr, ExprNode::Identifier(ref name) if name == "count"));
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn parses_open_range() {
    let expr = parse_expression("..").expect("open range should parse");
    match expr {
        ExprNode::Range(range) => {
            assert!(range.start.is_none());
            assert!(range.end.is_none());
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn rejects_multiple_range_operators() {
    let result = parse_expression("a..b..c");
    assert!(result.is_err(), "expected parse failure for chained ranges");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("multiple `..`"),
        "expected diagnostic about multiple range operators, got {}",
        err.message
    );
}

#[test]
fn range_precedence_respects_binary_binding() {
    let expr = parse_expression("1 + offset..^1").expect("range should parse");
    match expr {
        ExprNode::Range(range) => {
            let start = range.start.expect("start endpoint required");
            assert!(matches!(
                *start.expr,
                ExprNode::Binary { op: BinOp::Add, .. }
            ));
            let end = range.end.expect("end endpoint required");
            assert!(end.from_end, "end should be from-end");
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn parses_open_inclusive_range() {
    let expr = parse_expression("..=end").expect("inclusive open range should parse");
    match expr {
        ExprNode::Range(range) => {
            assert!(range.inclusive, "inclusive flag should be set");
            assert!(range.start.is_none(), "no explicit start expected");
            assert!(range.end.is_some(), "end should be captured");
        }
        other => panic!("expected range node, found {other:?}", other = other),
    }
}

#[test]
fn parses_result_propagation_postfix_operator() {
    let expr = parse_expression("maybeValue?").expect("result propagation should parse");
    match expr {
        ExprNode::TryPropagate { expr, .. } => match expr.as_ref() {
            ExprNode::Identifier(name) if name == "maybeValue" => {}
            other => panic!(
                "expected identifier operand, found {other:?}",
                other = other
            ),
        },
        other => panic!(
            "expected result propagation node, found {other:?}",
            other = other
        ),
    }
}

#[test]
fn parses_call_argument_modifiers() {
    let expr = parse_expression("Foo(ref value, name: out result, in data)")
        .expect("call with modifiers should parse");
    match expr {
        ExprNode::Call { args, .. } => {
            assert_eq!(args.len(), 3, "expected three call arguments");
            assert!(matches!(args[0].modifier, Some(CallArgumentModifier::Ref)));
            assert!(args[0].name.is_none());
            assert!(matches!(args[1].modifier, Some(CallArgumentModifier::Out)));
            assert!(matches!(
                args[1].name,
                Some(CallArgumentName { ref text, .. }) if text == "name"
            ));
            assert!(matches!(args[2].modifier, Some(CallArgumentModifier::In)));
        }
        other => panic!("expected call expression, found {other:?}"),
    }
}

#[test]
fn parses_out_var_inline_binding() {
    let expr =
        parse_expression("Foo(out var slice)").expect("call with inline binding should parse");
    match expr {
        ExprNode::Call { args, .. } => {
            assert_eq!(args.len(), 1);
            let arg = &args[0];
            assert!(matches!(arg.modifier, Some(CallArgumentModifier::Out)));
            assert!(matches!(
                arg.value,
                ExprNode::Identifier(ref name) if name == "slice"
            ));
            let binding = arg
                .inline_binding
                .as_ref()
                .expect("inline binding should be present");
            assert_eq!(binding.name, "slice");
            assert!(matches!(binding.kind, InlineBindingKind::Var));
        }
        other => panic!("expected call expression, found {other:?}"),
    }
}

#[test]
fn parses_out_typed_inline_binding_with_initializer() {
    let expr =
        parse_expression("Foo(out Int32 value = 0)").expect("typed inline binding should parse");
    match expr {
        ExprNode::Call { args, .. } => {
            assert_eq!(args.len(), 1);
            let arg = &args[0];
            assert!(matches!(arg.modifier, Some(CallArgumentModifier::Out)));
            assert!(matches!(
                arg.value,
                ExprNode::Identifier(ref name) if name == "value"
            ));
            let binding = arg
                .inline_binding
                .as_ref()
                .expect("inline binding should be present");
            assert_eq!(binding.name, "value");
            match &binding.kind {
                InlineBindingKind::Typed { type_name, .. } => {
                    assert_eq!(type_name, "Int32");
                }
                other => panic!("expected typed binding, found {other:?}"),
            }
            let initializer = binding
                .initializer
                .as_ref()
                .expect("initializer should be captured");
            assert!(matches!(
                initializer,
                ExprNode::Literal(LiteralConst {
                    value: ConstValue::Int(0),
                    ..
                })
            ));
        }
        other => panic!("expected call expression, found {other:?}"),
    }
}

#[test]
fn rejects_duplicate_argument_modifiers() {
    let err = parse_expression("Foo(ref ref value)").expect_err("duplicate modifiers");
    assert!(
        err.message.contains("duplicate argument modifier"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn new_expression_records_spans() {
    let source = "new Demo.Widget<int>(capacity: 4) { Value = 1 }";
    let expr = parse_expression(source).expect("new expression should parse");
    let new_expr = match expr {
        ExprNode::New(new_expr) => new_expr,
        other => panic!("expected new expression, found {other:?}", other = other),
    };
    assert_eq!(new_expr.type_name, "Demo.Widget<int>");
    let snippet = |span: Option<Span>| {
        span.map(|sp| &source[sp.start..sp.end])
            .unwrap_or("<missing>")
            .to_string()
    };
    assert_eq!(snippet(new_expr.keyword_span), "new");
    assert_eq!(snippet(new_expr.type_span), "Demo.Widget<int>");
    assert_eq!(snippet(new_expr.arguments_span), "(capacity: 4)");
    assert_eq!(snippet(new_expr.span), source);
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let fields = match initializer {
        NewInitializer::Object { fields, .. } => fields,
        other => panic!(
            "expected object initializer, found {other:?}",
            other = other
        ),
    };
    assert_eq!(fields.len(), 1);
    let field = &fields[0];
    assert_eq!(snippet(field.name_span), "Value");
    assert_eq!(snippet(field.value_span), "1");
    assert_eq!(snippet(field.span), "Value = 1");
}

#[test]
fn parses_quote_expression_without_interpolation() {
    let expr = parse_expression("quote(foo.Bar + delta)").expect("quote expression should parse");
    match expr {
        ExprNode::Quote(literal) => {
            assert_eq!(literal.source, "foo.Bar + delta");
            assert_eq!(literal.sanitized, "foo.Bar + delta");
            assert!(literal.interpolations.is_empty());
            let span = literal
                .content_span
                .expect("content span should be recorded");
            assert_eq!(span.len(), literal.source.len());
        }
        other => panic!("expected quote expression, found {other:?}"),
    }
}

#[test]
fn parses_quote_expression_with_interpolation() {
    let expr = parse_expression("quote(x + ${quote(y)} + 5)").expect("quote should parse");
    match expr {
        ExprNode::Quote(literal) => {
            assert_eq!(literal.sanitized, "x + __chic_quote_slot0 + 5");
            assert_eq!(literal.interpolations.len(), 1);
            let interpolation = &literal.interpolations[0];
            assert_eq!(interpolation.placeholder, "__chic_quote_slot0");
            assert_eq!(interpolation.expression_text, "quote(y)");
            assert!(matches!(interpolation.expression, ExprNode::Quote(_)));
            let span = interpolation
                .span
                .expect("interpolation span should be recorded");
            assert!(
                span.len() >= "quote(y)".len(),
                "span should cover interpolation expression"
            );
        }
        other => panic!("expected quote expression, found {other:?}"),
    }
}

#[test]
fn new_expression_records_collection_initializer() {
    let source = "new Numbers { 1, seed }";
    let expr = parse_expression(source).expect("new expression should parse");
    let new_expr = match expr {
        ExprNode::New(new_expr) => new_expr,
        other => panic!("expected new expression, found {other:?}", other = other),
    };
    assert_eq!(new_expr.type_name, "Numbers");
    assert!(
        new_expr.keyword_span.is_some(),
        "keyword span should be recorded"
    );
    let initializer = new_expr
        .initializer
        .as_ref()
        .expect("initializer should be captured");
    let elements = match initializer {
        NewInitializer::Collection { elements, .. } => elements,
        other => panic!("expected collection initializer, found {other:?}"),
    };
    assert_eq!(elements.len(), 2);
    assert!(matches!(elements[0], ExprNode::Literal(_)));
    assert!(matches!(elements[1], ExprNode::Identifier(ref name) if name == "seed"));
}

#[test]
fn parses_conditional_expression() {
    let expr = parse_expression("hasSimd ? fast(lhs, rhs) : slow(lhs, rhs)")
        .expect("conditional expression should parse");
    match expr {
        ExprNode::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            match condition.as_ref() {
                ExprNode::Identifier(name) if name == "hasSimd" => {}
                other => panic!("unexpected condition: {other:?}"),
            }
            match then_branch.as_ref() {
                ExprNode::Call { callee, .. } => match callee.as_ref() {
                    ExprNode::Identifier(name) if name == "fast" => {}
                    other => panic!("unexpected then callee: {other:?}"),
                },
                other => panic!("unexpected then branch: {other:?}"),
            }
            match else_branch.as_ref() {
                ExprNode::Call { callee, .. } => match callee.as_ref() {
                    ExprNode::Identifier(name) if name == "slow" => {}
                    other => panic!("unexpected else callee: {other:?}"),
                },
                other => panic!("unexpected else branch: {other:?}"),
            }
        }
        other => panic!("expected conditional expression, found {other:?}"),
    }
}

#[test]
fn parses_conditional_with_index_then_branch() {
    let expr = parse_expression("idx < effectiveKey.Length ? effectiveKey[idx] : 0u8")
        .expect("conditional expression should parse");
    match expr {
        ExprNode::Conditional { condition, .. } => {
            assert!(
                matches!(condition.as_ref(), ExprNode::Binary { .. }),
                "expected binary condition"
            );
        }
        other => panic!("expected conditional expression, found {other:?}"),
    }
}

#[test]
fn parses_switch_expression_with_guards() {
    let expr = parse_expression("value switch { 0 => 1, Record { X: let a } when a > 0 => a }")
        .expect("switch expression should parse");
    let ExprNode::Switch(switch_expr) = expr else {
        panic!("expected switch expression node");
    };
    assert!(
        matches!(*switch_expr.value, ExprNode::Identifier(ref name) if name == "value"),
        "switch discriminant should be preserved"
    );
    assert_eq!(switch_expr.arms.len(), 2, "expected two switch arms");
    let first = &switch_expr.arms[0];
    assert!(
        matches!(first.pattern.node, PatternNode::Literal(ConstValue::Int(0))),
        "first arm should parse literal pattern"
    );
    assert!(matches!(first.expression, ExprNode::Literal(_)));
    let second = &switch_expr.arms[1];
    assert!(
        matches!(
            second.pattern.node,
            PatternNode::Record(_) | PatternNode::Struct { .. }
        ),
        "second arm should parse record/struct pattern"
    );
    assert_eq!(second.guards.len(), 1, "expected a single guard");
}

#[test]
fn switch_expression_reports_missing_arrow() {
    let err = parse_expression("value switch { _ 1 }").expect_err("parse should fail");
    assert!(
        err.message.contains("expected `=>`"),
        "unexpected error message: {}",
        err.message
    );
}

#[test]
fn parses_new_expression_with_arguments() {
    let expr =
        parse_expression("new Std::Sync::Mutex<int>(value)").expect("new expression should parse");
    match expr {
        ExprNode::New(new_expr) => {
            assert_eq!(new_expr.type_name, "Std::Sync::Mutex<int>");
            assert_eq!(
                new_expr.args.len(),
                1,
                "expected single constructor argument"
            );
            assert!(
                new_expr.initializer.is_none(),
                "unexpected initializer for simple new"
            );
            let arg = &new_expr.args[0];
            match &arg.value {
                ExprNode::Identifier(name) if name == "value" => {}
                other => panic!("unexpected constructor argument: {other:?}"),
            }
        }
        other => panic!("expected new expression, found {other:?}"),
    }
}

#[test]
fn parses_array_creation_with_length() {
    let expr = parse_expression("new int[3]").expect("array creation should parse");
    match expr {
        ExprNode::New(new_expr) => {
            assert_eq!(new_expr.type_name, "int[]");
            let lengths = new_expr
                .array_lengths
                .as_ref()
                .expect("array lengths should be recorded");
            assert_eq!(lengths.len(), 1, "expected single length expression");
            assert!(
                matches!(
                    lengths[0],
                    ExprNode::Literal(LiteralConst {
                        value: ConstValue::Int(3),
                        ..
                    })
                ),
                "unexpected length expression: {:?}",
                lengths[0]
            );
            assert!(
                new_expr.initializer.is_none(),
                "unexpected initializer for length-only array creation"
            );
        }
        other => panic!("expected new expression, found {other:?}"),
    }
}

#[test]
fn parses_array_literal_basic() {
    let expr = parse_expression("[1, 2, 3]").expect("array literal should parse");
    match expr {
        ExprNode::ArrayLiteral(array) => {
            assert!(array.explicit_type.is_none());
            assert_eq!(array.elements.len(), 3);
            assert!(!array.trailing_comma);
        }
        other => panic!("expected array literal, found {other:?}"),
    }
}

#[test]
fn parses_array_literal_with_trailing_comma() {
    let expr = parse_expression("[a, b,]").expect("array literal should parse");
    match expr {
        ExprNode::ArrayLiteral(array) => {
            assert_eq!(array.elements.len(), 2);
            assert!(array.trailing_comma);
        }
        other => panic!("expected array literal, found {other:?}"),
    }
}

#[test]
fn parses_typed_array_literal() {
    let expr =
        parse_expression("string[] [\"a\", \"b\"]").expect("typed array literal should parse");
    match expr {
        ExprNode::ArrayLiteral(array) => {
            assert_eq!(array.explicit_type.as_deref(), Some("string[]"));
            assert_eq!(array.elements.len(), 2);
        }
        other => panic!("expected typed array literal, found {other:?}"),
    }
}

#[test]
fn parses_array_length_call_expression() {
    let expr = parse_expression("new IPAddress[NumericUnchecked.ToInt32(len)]")
        .expect("array creation with call length should parse");
    match expr {
        ExprNode::New(new_expr) => {
            assert_eq!(new_expr.type_name, "IPAddress[]");
            let lengths = new_expr
                .array_lengths
                .as_ref()
                .expect("array lengths should be recorded");
            assert_eq!(lengths.len(), 1);
            match &lengths[0] {
                ExprNode::Call { callee, args, .. } => {
                    match callee.as_ref() {
                        ExprNode::Member { base, member, .. } => {
                            assert_eq!(member, "ToInt32");
                            match base.as_ref() {
                                ExprNode::Identifier(name) => assert_eq!(name, "NumericUnchecked"),
                                other => panic!("unexpected call base: {other:?}"),
                            }
                        }
                        other => panic!("unexpected callee: {other:?}"),
                    }
                    assert_eq!(args.len(), 1);
                }
                other => panic!("unexpected length expression: {other:?}"),
            }
        }
        other => panic!("expected new expression, found {other:?}"),
    }
}

#[test]
fn parses_array_creation_with_initializer() {
    let expr = parse_expression("new int[] { 1, 2, 3 }").expect("array initializer should parse");
    match expr {
        ExprNode::New(new_expr) => {
            assert_eq!(new_expr.type_name, "int[]");
            let init = new_expr
                .initializer
                .expect("initializer should be captured");
            match init {
                NewInitializer::Collection { elements, .. } => {
                    assert_eq!(elements.len(), 3, "expected three initializer elements");
                    assert!(matches!(elements[0], ExprNode::Literal(_)));
                }
                other => panic!("unexpected initializer kind: {other:?}"),
            }
        }
        other => panic!("expected new expression, found {other:?}"),
    }
}

#[test]
fn parses_multi_rank_array_for_diagnostics() {
    let expr = parse_expression("new int[2, 3]").expect("multi-rank array parse should succeed");
    match expr {
        ExprNode::New(new_expr) => {
            assert_eq!(new_expr.type_name, "int[,]");
            let lengths = new_expr
                .array_lengths
                .as_ref()
                .expect("array lengths should be recorded");
            assert_eq!(
                lengths.len(),
                2,
                "expected both rank lengths preserved for diagnostics"
            );
        }
        other => panic!("expected new expression, found {other:?}"),
    }
}

#[test]
fn parses_is_pattern_with_multiple_guards() {
    let expr = parse_expression("value is Foo when cond when other")
        .expect("pattern guard chain should parse");
    match expr {
        ExprNode::IsPattern { guards, .. } => {
            assert_eq!(guards.len(), 2, "expected two guard expressions");
            assert!(matches!(&guards[0].expr, ExprNode::Identifier(name) if name == "cond"));
            assert!(matches!(&guards[1].expr, ExprNode::Identifier(name) if name == "other"));
        }
        other => panic!("expected `is` pattern with guards, found {other:?}"),
    }
}

#[test]
fn is_pattern_guard_respects_outer_boolean_precedence() {
    let expr = parse_expression("value is Foo when cond && extra")
        .expect("pattern guard with trailing && should parse");
    match expr {
        ExprNode::Binary {
            op: BinOp::And,
            left,
            right,
        } => {
            assert!(
                matches!(right.as_ref(), ExprNode::Identifier(name) if name == "extra"),
                "unexpected rhs: {right:?}",
                right = right
            );
            match *left {
                ExprNode::IsPattern { guards, .. } => {
                    assert_eq!(guards.len(), 1, "guard should stop before outer &&");
                    assert!(
                        matches!(&guards[0].expr, ExprNode::Identifier(name) if name == "cond")
                    );
                }
                other => panic!("expected `is` pattern on left-hand side, found {other:?}"),
            }
        }
        other => panic!("expected binary && expression, found {other:?}"),
    }
}

#[test]
fn is_pattern_guard_requires_expression() {
    let err =
        parse_expression("value is Foo when").expect_err("missing guard expression should error");
    assert!(
        err.message.contains("requires an expression"),
        "unexpected error: {err:?}",
        err = err
    );
}

#[test]
fn parses_inline_asm_with_operands_and_options() {
    let expr = parse_expression(
        r#"asm!("mov {0}, {src}", out(reg) dst, in(reg64) src, options(volatile, alignstack), clobber("xmm0"))"#,
    )
    .expect("inline asm should parse");
    let ExprNode::InlineAsm(asm) = expr else {
        panic!("expected inline asm node");
    };
    assert!(asm.options.volatile);
    assert!(asm.options.alignstack);
    assert!(!asm.options.intel_syntax);
    assert_eq!(asm.clobbers.len(), 1);
    assert!(matches!(
        asm.clobbers[0],
        InlineAsmRegister::Explicit(ref reg) if reg == "xmm0"
    ));
    assert_eq!(asm.template.pieces.len(), 4);
    assert!(matches!(
        asm.template.pieces[1],
        InlineAsmTemplatePiece::Placeholder {
            operand: InlineAsmTemplateOperandRef::Position(0),
            ..
        }
    ));
    assert!(matches!(
        asm.template.pieces[3],
        InlineAsmTemplatePiece::Placeholder {
            operand: InlineAsmTemplateOperandRef::Named(ref name),
            ..
        } if name == "src"
    ));
    assert_eq!(asm.operands.len(), 2);
    assert!(matches!(
        asm.operands[0].mode,
        InlineAsmOperandMode::Out { .. }
    ));
    assert!(matches!(
        asm.operands[0].reg,
        InlineAsmRegister::Class(InlineAsmRegisterClass::Reg)
    ));
    assert!(matches!(
        asm.operands[1].mode,
        InlineAsmOperandMode::In { .. }
    ));
}

#[test]
fn inline_asm_rejects_interpolated_templates() {
    let err = parse_expression(r#"asm!($"mov {0}")"#).expect_err("interpolated asm should fail");
    assert!(
        err.message.contains("do not support string interpolation"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn parses_null_conditional_member_assignment() {
    let expr = parse_expression("customer?.Order = next")
        .expect("null-conditional assignment should parse");
    match expr {
        ExprNode::Assign { target, op, value } => {
            assert!(matches!(op, AssignOp::Assign));
            assert!(matches!(
                *target,
                ExprNode::Member {
                    null_conditional: true,
                    ..
                }
            ));
            assert!(matches!(*value, ExprNode::Identifier(ref name) if name == "next"));
        }
        other => panic!("expected assignment node, found {other:?}", other = other),
    }
}

#[test]
fn parses_null_conditional_index_compound_assignment() {
    let expr =
        parse_expression("items?[i] += delta").expect("null-conditional compound should parse");
    match expr {
        ExprNode::Assign { target, op, .. } => {
            assert!(matches!(op, AssignOp::AddAssign));
            match *target {
                ExprNode::Index {
                    null_conditional,
                    indices,
                    ..
                } => {
                    assert!(null_conditional, "index target should be null-conditional");
                    assert_eq!(indices.len(), 1);
                }
                other => panic!("expected index target, found {other:?}", other = other),
            }
        }
        other => panic!("expected assignment node, found {other:?}", other = other),
    }
}
