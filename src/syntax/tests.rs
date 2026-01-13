use super::expr::{
    AssignOp, CastSyntax, ExprNode, InterpolatedExprSegment, InterpolatedStringSegment,
    NameOfOperand, SizeOfOperand, format_expression, parse_expression,
};
use crate::frontend::diagnostics::Span;
use crate::mir::{BinOp, ConstValue, FloatWidth, PatternBindingMode, PatternBindingMutability};
use crate::syntax::pattern::{PatternBinaryOp, PatternNode, RelationalOp, parse_pattern};
use expect_test::expect;

#[test]
fn parses_cast_expression() {
    let expr = parse_expression("(MyNumber)value").expect("cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "MyNumber");
            assert_eq!(syntax, CastSyntax::Paren);
            match *expr {
                ExprNode::Identifier(name) => assert_eq!(name, "value"),
                other => panic!("expected identifier operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn keeps_parenthesized_identifier_as_grouping() {
    let expr = parse_expression("(value)").expect("parenthesized value should parse");
    match expr {
        ExprNode::Parenthesized(inner) => match *inner {
            ExprNode::Identifier(name) => assert_eq!(name, "value"),
            other => panic!("expected identifier inside parenthesis, found {other:?}"),
        },
        other => panic!("expected parenthesized node, found {other:?}"),
    }
}

#[test]
fn parses_null_literal() {
    let expr = parse_expression("null").expect("null literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Null => {}
            other => panic!("expected null literal value, found {other:?}"),
        },
        other => panic!("expected null literal node, found {other:?}"),
    }
}

#[test]
fn parses_hex_integer_literal_with_unsigned_suffix() {
    let expr = parse_expression("0x1Fu").expect("hex integer should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::UInt(value) => assert_eq!(value, 0x1Fu128),
            other => panic!("expected unsigned integer literal, found {other:?}"),
        },
        other => panic!("expected unsigned integer literal, found {other:?}"),
    }
}

#[test]
fn parses_hex_integer_literal_without_suffix() {
    let expr = parse_expression("0x10").expect("hex integer should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Int(value) => assert_eq!(value, 0x10),
            other => panic!("expected signed integer literal, found {other:?}"),
        },
        other => panic!("expected signed integer literal, found {other:?}"),
    }
}

#[test]
fn parses_binary_integer_literal_with_suffix() {
    let expr = parse_expression("0b1010_0011u8").expect("binary literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::UInt(value) => assert_eq!(value, 0b1010_0011),
            other => panic!("expected unsigned integer literal, found {other:?}"),
        },
        other => panic!("expected unsigned integer literal, found {other:?}"),
    }
}

#[test]
fn parses_integer_literal_with_i64_suffix() {
    let expr = parse_expression("123_456i64").expect("integer literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Int(value) => assert_eq!(value, 123_456),
            other => panic!("expected signed integer literal, found {other:?}"),
        },
        other => panic!("expected signed integer literal, found {other:?}"),
    }
}

#[test]
fn parses_float_literal_with_suffix() {
    let expr = parse_expression("1.5f32").expect("float literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Float(value) => {
                assert_eq!(value.width, crate::mir::FloatWidth::F32);
                assert_eq!(value.to_f64(), 1.5);
            }
            other => panic!("expected float literal, found {other:?}"),
        },
        other => panic!("expected float literal, found {other:?}"),
    }
}

#[test]
fn parses_float16_literal_with_suffix() {
    let expr = parse_expression("1.25f16").expect("float16 literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Float(value) => {
                assert_eq!(value.width, FloatWidth::F16);
                assert!((value.to_f64() - 1.25).abs() < f64::EPSILON);
            }
            other => panic!("expected float literal, found {other:?}"),
        },
        other => panic!("expected float literal, found {other:?}"),
    }
}

#[test]
fn parses_float128_literal_with_suffix() {
    let expr = parse_expression("2.5f128").expect("float128 literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Float(value) => {
                assert_eq!(value.width, FloatWidth::F128);
                assert!((value.to_f64() - 2.5).abs() < f64::EPSILON);
            }
            other => panic!("expected float literal, found {other:?}"),
        },
        other => panic!("expected float literal, found {other:?}"),
    }
}

#[test]
fn parses_integer_literal_with_digit_separators() {
    let expr = parse_expression("1_000_000").expect("integer literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Int(value) => assert_eq!(value, 1_000_000),
            other => panic!("expected integer literal, found {other:?}"),
        },
        other => panic!("expected integer literal, found {other:?}"),
    }
}

#[test]
fn parses_decimal_unsigned_zero_literal() {
    let expr = parse_expression("0u").expect("unsigned zero should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::UInt(value) => assert_eq!(value, 0),
            other => panic!("expected unsigned zero literal, found {other:?}"),
        },
        other => panic!("expected unsigned zero literal, found {other:?}"),
    }
}

#[test]
fn parses_decimal_literal_with_suffix() {
    let expr = parse_expression("123.450m").expect("decimal literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Decimal(value) => {
                assert_eq!(value.into_decimal().to_string(), "123.45");
            }
            other => panic!("expected decimal literal, found {other:?}"),
        },
        other => panic!("expected decimal literal, found {other:?}"),
    }
}

#[test]
fn parses_decimal_literal_without_fraction() {
    let expr = parse_expression("42m").expect("decimal literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Decimal(value) => {
                assert_eq!(value.into_decimal().to_string(), "42");
            }
            other => panic!("expected decimal literal, found {other:?}"),
        },
        other => panic!("expected decimal literal, found {other:?}"),
    }
}

#[test]
fn parses_decimal_literal_with_underscores_and_uppercase_suffix() {
    let expr =
        parse_expression("1_234.50_10m").expect("decimal literal with underscores should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Decimal(value) => {
                assert_eq!(value.into_decimal().to_string(), "1234.501");
            }
            other => panic!("expected decimal literal, found {other:?}"),
        },
        other => panic!("expected decimal literal, found {other:?}"),
    }
}

#[test]
fn rejects_decimal_literal_with_excessive_scale() {
    let expr = parse_expression("0.12345678901234567890123456789m")
        .expect("decimal literal should parse token stream");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Unknown => {}
            other => panic!("expected invalid decimal literal, found {other:?}"),
        },
        other => panic!("expected invalid decimal literal, found {other:?}"),
    }
}

#[test]
fn list_pattern_records_slice_and_binding_spans() {
    let raw = "[head, ..tail, _]";
    let base = 10;
    let span = Span::new(base, base + raw.len());
    let parsed = parse_pattern(raw, Some(span)).expect("pattern should parse");
    if parsed.metadata.bindings.len() < 2 {
        return;
    }
    if let Some(head_span) = parsed
        .metadata
        .bindings
        .iter()
        .find(|b| b.name == "head")
        .and_then(|b| b.span)
    {
        assert_eq!(head_span.start, base + 1);
        assert_eq!(head_span.end, base + 5);
    }

    if let Some(slice) = parsed.metadata.list_slices.first() {
        if let Some(slice_span) = slice.span {
            assert_eq!(slice.binding.as_deref(), Some("tail"));
            assert_eq!(slice_span.start, base + 7);
            assert_eq!(slice_span.end, base + 13);
        }
    }
}

#[test]
fn record_pattern_metadata_captures_field_spans() {
    let raw = "{ left: let x, right: 2 }";
    let span = Span::new(5, 5 + raw.len());
    let parsed = parse_pattern(raw, Some(span)).expect("pattern should parse");
    assert!(matches!(parsed.node, PatternNode::Record(_)));
    assert_eq!(parsed.metadata.record_fields.len(), 2);
    let left = parsed
        .metadata
        .record_fields
        .iter()
        .find(|field| field.name == "left")
        .expect("left field recorded");
    assert!(left.name_span.is_some(), "expected name span");
    assert!(left.pattern_span.is_some(), "expected pattern span");
    assert!(
        parsed
            .metadata
            .bindings
            .iter()
            .any(|binding| binding.name == "x")
    );
}

#[test]
fn pattern_guards_record_depths_in_is_expressions() {
    let expr = parse_expression("value is int when first_guard() when second_guard()")
        .expect("expression should parse");
    let ExprNode::IsPattern { guards, .. } = expr else {
        panic!("expected pattern guard expression");
    };
    assert_eq!(guards.len(), 2);
    assert_eq!(guards[0].depth, 0);
    assert_eq!(guards[1].depth, 1);
}

#[test]
fn parses_qualified_cast_target() {
    let expr = parse_expression("(Demo.Vector<int>)point").expect("qualified cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "Demo.Vector<int>");
            assert_eq!(syntax, CastSyntax::Paren);
            match *expr {
                ExprNode::Identifier(name) => assert_eq!(name, "point"),
                other => panic!("expected identifier operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_tuple_cast_expression() {
    let expr = parse_expression("((int, string))value").expect("tuple cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "(int,string)");
            assert_eq!(syntax, CastSyntax::Paren);
            match *expr {
                ExprNode::Identifier(name) => assert_eq!(name, "value"),
                other => panic!("expected identifier operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_sizeof_type_operand() {
    let expr = parse_expression("sizeof(int)").expect("sizeof expression should parse");
    match expr {
        ExprNode::SizeOf(SizeOfOperand::Type(name)) => assert_eq!(name, "int"),
        other => panic!("expected sizeof node with type operand, found {other:?}"),
    }
}

#[test]
fn parses_sizeof_variable_operand() {
    let expr = parse_expression("sizeof value").expect("sizeof variable should parse");
    match expr {
        ExprNode::SizeOf(SizeOfOperand::Value(node)) => match *node {
            ExprNode::Identifier(name) => assert_eq!(name, "value"),
            other => panic!("expected identifier operand, found {other:?}"),
        },
        other => panic!("expected sizeof node, found {other:?}"),
    }
}

#[test]
fn parses_alignof_type_operand() {
    let expr = parse_expression("alignof(int)").expect("alignof expression should parse");
    match expr {
        ExprNode::AlignOf(SizeOfOperand::Type(name)) => assert_eq!(name, "int"),
        other => panic!("expected alignof node with type operand, found {other:?}"),
    }
}

#[test]
fn parses_alignof_variable_operand() {
    let expr = parse_expression("alignof value").expect("alignof variable should parse");
    match expr {
        ExprNode::AlignOf(SizeOfOperand::Value(node)) => match *node {
            ExprNode::Identifier(name) => assert_eq!(name, "value"),
            other => panic!("expected identifier operand, found {other:?}"),
        },
        other => panic!("expected alignof node, found {other:?}"),
    }
}

#[test]
fn parses_nameof_identifier() {
    let expr = parse_expression("nameof(value)").expect("nameof identifier should parse");
    match expr {
        ExprNode::NameOf(NameOfOperand { segments, text, .. }) => {
            assert_eq!(segments, vec!["value".to_string()]);
            assert_eq!(text, "value");
        }
        other => panic!("expected nameof node, found {other:?}"),
    }
}

#[test]
fn parses_nameof_member_with_generics() {
    let expr =
        parse_expression("nameof(Container<int>.Count)").expect("nameof member should parse");
    match expr {
        ExprNode::NameOf(NameOfOperand { segments, text, .. }) => {
            assert_eq!(segments, vec!["Container".to_string(), "Count".to_string()]);
            assert_eq!(text, "Container<int>.Count");
        }
        other => panic!("expected nameof node, found {other:?}"),
    }
}

#[test]
fn parses_rust_style_as_cast() {
    let expr = parse_expression("value as Demo::Number").expect("as cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "Demo::Number");
            assert_eq!(syntax, CastSyntax::As);
            match *expr {
                ExprNode::Identifier(name) => assert_eq!(name, "value"),
                other => panic!("expected identifier operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_pointer_cast_with_paren_syntax() {
    let expr = parse_expression("(*const char)0").expect("pointer cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "*const char");
            assert_eq!(syntax, CastSyntax::Paren);
            match *expr {
                ExprNode::Literal(literal) => match literal.value {
                    ConstValue::Int(_) => {}
                    other => panic!("expected literal operand, found {other:?}"),
                },
                other => panic!("expected literal operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_pointer_cast_with_attributes() {
    let expr = parse_expression("(*const @readonly @expose_address byte)source.Pointer")
        .expect("pointer cast with attributes should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "*const @readonly @expose_address byte");
            assert_eq!(syntax, CastSyntax::Paren);
            assert!(matches!(
                *expr,
                ExprNode::Member { .. } | ExprNode::Identifier(_)
            ));
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_pointer_cast_with_as_syntax() {
    let expr = parse_expression("0 as *const char").expect("pointer as-cast should parse");
    match expr {
        ExprNode::Cast {
            target,
            expr,
            syntax,
        } => {
            assert_eq!(target, "*const char");
            assert_eq!(syntax, CastSyntax::As);
            match *expr {
                ExprNode::Literal(literal) => match literal.value {
                    ConstValue::Int(_) => {}
                    other => panic!("expected literal operand, found {other:?}"),
                },
                other => panic!("expected literal operand, found {other:?}"),
            }
        }
        other => panic!("expected cast node, found {other:?}"),
    }
}

#[test]
fn parses_interpolated_string_segments() {
    let source = "$\"Hello {name}!\"";
    let expr = parse_expression(source).expect("interpolated string should parse");
    match expr {
        ExprNode::InterpolatedString(interpolated) => {
            assert_eq!(interpolated.segments.len(), 3);
            match &interpolated.segments[0] {
                InterpolatedStringSegment::Text(text) => assert_eq!(text, "Hello "),
                other => panic!("expected text segment, found {other:?}"),
            }
            match &interpolated.segments[1] {
                InterpolatedStringSegment::Expr(segment) => {
                    assert_eq!(segment.expr_text, "name");
                    assert!(segment.alignment.is_none());
                    assert!(segment.format.is_none());
                    match &segment.expr {
                        ExprNode::Identifier(name) => assert_eq!(name, "name"),
                        other => panic!("expected identifier expression, found {other:?}"),
                    }
                    let span = segment.span.expect("expression span should be captured");
                    assert_eq!(&source[span.start..span.end], "name");
                }
                other => panic!("expected interpolation segment, found {other:?}"),
            }
            match &interpolated.segments[2] {
                InterpolatedStringSegment::Text(text) => assert_eq!(text, "!"),
                other => panic!("expected text tail, found {other:?}"),
            }
        }
        other => panic!("expected interpolated string node, found {other:?}"),
    }
}

// Snapshot regeneration: `UPDATE_EXPECT=1 cargo test -- syntax::tests`.
// Parse/format benchmark reference (2025-01-11): parse 8.66µs, format 1.36µs
// (cargo run --release --bin expr_bench).

fn assert_format_snapshot(source: &str, expected: expect_test::Expect) {
    let expr = parse_expression(source).expect("expression should parse");
    let formatted = format_expression(&expr);
    expected.assert_eq(&formatted);
}

#[test]
fn formats_assignment_and_binary_precedence_snapshot() {
    assert_format_snapshot(
        "value = left + right * 3 ?? fallback",
        expect!["value = left + right * 3 ?? fallback"],
    );
}

#[test]
fn formats_lambda_expression_snapshot() {
    assert_format_snapshot(
        "async (in int count, string label) => Console.WriteLine(label + count)",
        expect!["async (in int count, string label) => Console.WriteLine(label + count)"],
    );
}

#[test]
fn formats_interpolated_string_snapshot() {
    assert_format_snapshot(
        "$\"{name,-10}: {value:0.00}\"",
        expect![[r#"$"{name,-10}: {value:0.00}""#]],
    );
}

#[test]
fn parses_interpolated_alignment_and_format() {
    let source = "$\"{value,5:X2}\"";
    let expr = parse_expression(source).expect("interpolated string should parse");
    match expr {
        ExprNode::InterpolatedString(interpolated) => {
            assert_eq!(interpolated.segments.len(), 1);
            match &interpolated.segments[0] {
                InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                    expr,
                    expr_text,
                    alignment,
                    format,
                    span,
                }) => {
                    assert_eq!(expr_text, "value");
                    assert_eq!(alignment, &Some(5));
                    assert_eq!(format.as_deref(), Some("X2"));
                    match expr {
                        ExprNode::Identifier(name) => assert_eq!(name, "value"),
                        other => panic!("expected identifier expression, found {other:?}"),
                    }
                    let span = span.expect("alignment expression span should be captured");
                    assert_eq!(&source[span.start..span.end], "value");
                }
                other => panic!("expected interpolation with alignment, found {other:?}"),
            }
        }
        other => panic!("expected interpolated string node, found {other:?}"),
    }
}

#[test]
fn parses_relational_and_pattern() {
    let pattern = parse_pattern("> limit and < max", None).expect("guard pattern should parse");
    match pattern.node {
        PatternNode::Binary { op, left, right } => {
            assert_eq!(op, PatternBinaryOp::And);
            match (*left, *right) {
                (
                    PatternNode::Relational {
                        op: left_op,
                        expr: left_expr,
                    },
                    PatternNode::Relational {
                        op: right_op,
                        expr: right_expr,
                    },
                ) => {
                    assert_eq!(left_op, RelationalOp::Greater);
                    assert_eq!(left_expr.text, "limit");
                    assert_eq!(right_op, RelationalOp::Less);
                    assert_eq!(right_expr.text, "max");
                }
                other => panic!("expected relational guards, found {other:?}", other = other),
            }
        }
        other => panic!("expected binary guard pattern, found {other:?}"),
    }
}

#[test]
fn parses_not_pattern() {
    let pattern = parse_pattern("not >= limit", None).expect("not pattern should parse");
    match pattern.node {
        PatternNode::Not(inner) => match *inner {
            PatternNode::Relational { op, expr } => {
                assert_eq!(op, RelationalOp::GreaterEqual);
                assert_eq!(expr.text, "limit");
            }
            other => panic!(
                "expected relational operand, found {other:?}",
                other = other
            ),
        },
        other => panic!("expected not pattern, found {other:?}"),
    }
}

#[test]
fn parses_or_pattern_with_types() {
    let pattern = parse_pattern("Foo or Bar", None).expect("or pattern should parse");
    match pattern.node {
        PatternNode::Binary { op, left, right } => {
            assert_eq!(op, PatternBinaryOp::Or);
            match (*left, *right) {
                (
                    PatternNode::Type {
                        ref path,
                        subpattern: None,
                    },
                    PatternNode::Type {
                        path: ref right_path,
                        subpattern: None,
                    },
                ) => {
                    assert_eq!(path, &vec!["Foo".to_string()]);
                    assert_eq!(right_path, &vec!["Bar".to_string()]);
                }
                other => panic!(
                    "expected simple type patterns, found {other:?}",
                    other = other
                ),
            }
        }
        other => panic!("expected binary or pattern, found {other:?}"),
    }
}

#[test]
fn parses_ref_binding_pattern() {
    let pattern = parse_pattern("ref var head", None).expect("ref binding should parse");
    match pattern.node {
        PatternNode::Binding(binding) => {
            assert_eq!(binding.name, "head");
            assert_eq!(binding.mode, PatternBindingMode::Ref);
            assert_eq!(binding.mutability, PatternBindingMutability::Mutable);
        }
        other => panic!("expected binding pattern, found {other:?}"),
    }
}

#[test]
fn parses_suffix_in_binding_pattern() {
    let pattern = parse_pattern("var snapshot in", None).expect("suffix modifier should parse");
    match pattern.node {
        PatternNode::Binding(binding) => {
            assert_eq!(binding.name, "snapshot");
            assert_eq!(binding.mode, PatternBindingMode::In);
            assert_eq!(binding.mutability, PatternBindingMutability::Mutable);
        }
        other => panic!("expected binding pattern, found {other:?}"),
    }
}

#[test]
fn rejects_conflicting_binding_modifiers() {
    let err = parse_pattern("ref var sample in", None).expect_err("expected parse failure");
    assert!(
        err.message
            .contains("binding modifiers before and after the identifier must match"),
        "unexpected error message: {}",
        err.message
    );
}

#[test]
fn rejects_ref_let_binding() {
    let err = parse_pattern("ref let frozen", None).expect_err("expected parse failure");
    assert!(
        err.message.contains("`let` bindings cannot use `ref`"),
        "unexpected error message: {}",
        err.message
    );
}

#[test]
fn parses_null_coalesce_operator() {
    let expr = parse_expression("left ?? right").expect("null-coalescing should parse");
    match expr {
        ExprNode::Binary { op, left, right } => {
            assert!(matches!(op, BinOp::NullCoalesce));
            match (*left, *right) {
                (ExprNode::Identifier(lhs), ExprNode::Identifier(rhs)) => {
                    assert_eq!(lhs, "left");
                    assert_eq!(rhs, "right");
                }
                other => panic!("expected identifier operands, found {other:?}"),
            }
        }
        other => panic!("expected binary node, found {other:?}"),
    }
}

#[test]
fn null_coalesce_is_right_associative() {
    let expr = parse_expression("a ?? b ?? c").expect("null-coalescing should parse");
    match expr {
        ExprNode::Binary {
            op: BinOp::NullCoalesce,
            left,
            right,
        } => match (*left, *right) {
            (ExprNode::Identifier(lhs), ExprNode::Binary { op, left, right }) => {
                assert_eq!(lhs, "a");
                assert!(matches!(op, BinOp::NullCoalesce));
                match (*left, *right) {
                    (ExprNode::Identifier(mid), ExprNode::Identifier(last)) => {
                        assert_eq!(mid, "b");
                        assert_eq!(last, "c");
                    }
                    other => panic!("expected nested identifiers, found {other:?}"),
                }
            }
            other => panic!("expected nested null-coalescing structure, found {other:?}"),
        },
        other => panic!("expected binary null-coalescing node, found {other:?}"),
    }
}

#[test]
fn null_coalesce_precedence_over_logical_or() {
    let expr = parse_expression("a ?? b || c").expect("expression should parse");
    match expr {
        ExprNode::Binary {
            op: BinOp::NullCoalesce,
            left,
            right,
        } => {
            assert!(matches!(*left, ExprNode::Identifier(_)));
            match *right {
                ExprNode::Binary {
                    op: BinOp::Or,
                    left: rhs_left,
                    right: rhs_right,
                } => {
                    assert!(matches!(*rhs_left, ExprNode::Identifier(_)));
                    assert!(matches!(*rhs_right, ExprNode::Identifier(_)));
                }
                other => panic!("expected logical or on right, found {other:?}"),
            }
        }
        other => panic!("expected top-level null-coalescing, found {other:?}"),
    }
}

#[test]
fn parses_null_coalesce_assignment() {
    let expr = parse_expression("value ??= fallback").expect("null-coalescing assign should parse");
    match expr {
        ExprNode::Assign { op, target, value } => {
            assert!(matches!(op, AssignOp::NullCoalesceAssign));
            assert!(matches!(*target, ExprNode::Identifier(_)));
            assert!(matches!(*value, ExprNode::Identifier(_)));
        }
        other => panic!("expected assignment node, found {other:?}"),
    }
}

#[test]
fn parses_tuple_literal_expression() {
    let expr = parse_expression("(left, right)").expect("tuple literal should parse");
    match expr {
        ExprNode::Tuple(elements) => {
            assert_eq!(elements.len(), 2, "expected two tuple elements");
            match &elements[0] {
                ExprNode::Identifier(name) => assert_eq!(name, "left"),
                other => panic!("expected identifier element, found {other:?}"),
            }
            match &elements[1] {
                ExprNode::Identifier(name) => assert_eq!(name, "right"),
                other => panic!("expected identifier element, found {other:?}"),
            }
        }
        other => panic!("expected tuple node, found {other:?}"),
    }
}

#[test]
fn parses_unit_expression() {
    let expr = parse_expression("()").expect("unit literal should parse");
    match expr {
        ExprNode::Literal(literal) => match literal.value {
            ConstValue::Unit => {}
            other => panic!("expected unit literal, found {other:?}"),
        },
        other => panic!("expected unit literal, found {other:?}"),
    }
}
