use crate::const_eval_config::ConstEvalConfig;
use crate::decimal::Decimal128;
use crate::frontend::ast::expressions::{Block, Expression, Statement, StatementKind};
use crate::frontend::ast::items::{
    BindingModifier, FunctionDecl, GenericParam, GenericParamKind, GenericParams, MemberDispatch,
    Parameter, Signature, Visibility,
};
use crate::frontend::ast::types::TypeExpr;
use crate::frontend::diagnostics::Span;
use crate::frontend::parser::parse_module;
use crate::mir::ConstEvalContext;
use crate::mir::TypeLayoutTable;
use crate::mir::builder::const_eval::ConstEvalResult;
use crate::mir::builder::const_eval::diagnostics::ConstEvalError;
use crate::mir::builder::const_eval::environment::{ConstFnCacheKey, EvalEnv};
use crate::mir::builder::symbol_index::{FunctionDeclSymbol, SymbolIndex};
use crate::mir::data::{BinOp, ConstValue, StrId, UnOp};
use crate::syntax::expr::AssignOp;
use crate::syntax::expr::SizeOfOperand;
use crate::syntax::expr::builders::{
    CallArgument, CallArgumentModifier, CallArgumentName, CastSyntax, ExprNode, InlineBinding,
    InlineBindingKind, InterpolatedExprSegment, InterpolatedStringExpr, InterpolatedStringSegment,
    LambdaBlock, LambdaBody, LambdaExpr, LambdaParam, LambdaParamModifier, LiteralConst,
    NameOfOperand, NewExpr, NewInitializer, ObjectInitializerField, QuoteInterpolation,
    QuoteLiteral, QuoteSourceSpan,
};
use crate::syntax::pattern::{PatternAst, PatternMetadata, PatternNode};
use std::collections::HashMap;

fn default_ctx() -> (SymbolIndex, TypeLayoutTable) {
    (SymbolIndex::default(), TypeLayoutTable::default())
}

fn make_param(name: &str, ty: &str) -> Parameter {
    Parameter {
        binding: BindingModifier::Value,
        binding_nullable: false,
        name: name.into(),
        name_span: None,
        ty: TypeExpr::simple(ty),
        attributes: Vec::new(),
        di_inject: None,
        default: None,
        default_span: None,
        lends: None,
        is_extension_this: false,
    }
}

fn make_signature(params: Vec<Parameter>, return_type: &str) -> Signature {
    Signature {
        parameters: params,
        return_type: TypeExpr::simple(return_type),
        lends_to_return: None,
        variadic: false,
        throws: None,
    }
}

fn make_return_block(expr: ExprNode) -> Block {
    Block {
        statements: vec![Statement::new(
            None,
            StatementKind::Return {
                expression: Some(Expression::with_node("ret", None, expr)),
            },
        )],
        span: None,
    }
}

fn make_const_fn(
    name: &str,
    params: Vec<Parameter>,
    return_type: &str,
    body: Option<Block>,
) -> FunctionDeclSymbol {
    FunctionDeclSymbol {
        qualified: format!("Demo::{name}"),
        function: FunctionDecl {
            visibility: Visibility::Public,
            name: name.into(),
            name_span: None,
            signature: make_signature(params, return_type),
            body,
            is_async: false,
            is_constexpr: true,
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
        },
        owner: None,
        namespace: Some("Demo".into()),
        internal_name: format!("Demo::{name}#0"),
    }
}

fn descriptor_list_items(list: &ConstValue) -> Vec<ConstValue> {
    let mut current = list.clone();
    let mut items = Vec::new();
    loop {
        let ConstValue::Struct { fields, .. } = current else {
            break;
        };
        let mut is_empty = false;
        let mut head = None;
        let mut tail = None;
        for (name, value) in fields {
            match name.as_str() {
                "IsEmpty" => is_empty = matches!(value, ConstValue::Bool(true)),
                "Head" => head = Some(value),
                "Tail" => tail = Some(value),
                _ => {}
            }
        }
        if is_empty {
            break;
        }
        if let Some(value) = head {
            items.push(value);
        }
        if let Some(next) = tail {
            current = next;
        } else {
            break;
        }
    }
    items
}

fn quote_node_summary(node: &ConstValue) -> (String, Option<String>, Vec<ConstValue>) {
    let ConstValue::Struct { fields, .. } = node else {
        panic!("quote node should be a struct");
    };
    let mut kind = None;
    let mut value = None;
    let mut children = None;
    for (name, field) in fields {
        match name.as_str() {
            "Kind" => {
                if let ConstValue::Enum { variant, .. } = field {
                    kind = Some(variant);
                }
            }
            "Value" => match field {
                ConstValue::RawStr(text) => value = Some(Some(text)),
                ConstValue::Null => value = Some(None),
                _ => {}
            },
            "Children" => children = Some(descriptor_list_items(&field)),
            _ => {}
        }
    }
    (
        kind.expect("kind present").to_string(),
        value.expect("value present").cloned(),
        children.expect("children present"),
    )
}

#[test]
fn quote_capture_list_deduplicates_identifiers() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let expr = ExprNode::Binary {
        op: BinOp::Add,
        left: ExprNode::Identifier("x".into()).boxed(),
        right: ExprNode::Binary {
            op: BinOp::Mul,
            left: ExprNode::Identifier("x".into()).boxed(),
            right: ExprNode::Identifier("y".into()).boxed(),
        }
        .boxed(),
    };
    let captures = ctx.build_quote_capture_list(&expr);
    assert_eq!(captures, vec!["x".to_string(), "y".to_string()]);
}

#[test]
fn quote_capture_list_collects_inline_bindings_and_new_initializers() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let expr = ExprNode::Call {
        callee: ExprNode::Identifier("Func".into()).boxed(),
        args: vec![CallArgument {
            name: Some(CallArgumentName {
                text: "first".into(),
                span: None,
            }),
            value: ExprNode::New(NewExpr {
                type_name: "Demo.List".into(),
                type_span: None,
                keyword_span: None,
                array_lengths: None,
                args: Vec::new(),
                arguments_span: None,
                initializer: Some(NewInitializer::Collection {
                    elements: vec![ExprNode::Identifier("elem".into())],
                    span: None,
                }),
                span: None,
            }),
            span: None,
            value_span: None,
            modifier: None,
            modifier_span: None,
            inline_binding: Some(InlineBinding {
                kind: InlineBindingKind::Var,
                name: "binding".into(),
                keyword_span: None,
                name_span: None,
                initializer: Some(ExprNode::Identifier("init".into())),
                initializer_span: None,
            }),
        }],
        generics: None,
    };
    let captures = ctx.build_quote_capture_list(&expr);
    assert!(
        captures.contains(&"elem".to_string()) && captures.contains(&"init".to_string()),
        "captures missing elements: {:?}",
        captures
    );
}

#[test]
fn const_value_label_orders_struct_fields() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let value = ConstValue::Struct {
        type_name: "Demo.Point".into(),
        fields: vec![
            ("B".into(), ConstValue::Int(2)),
            ("A".into(), ConstValue::Int(1)),
        ],
    };
    let label = ctx.const_value_label(&value);
    assert_eq!(label, "struct:Demo.Point{A=i:1,B=i:2}");
}

#[test]
fn const_value_label_formats_scalars_and_unknown() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    assert_eq!(ctx.const_value_label(&ConstValue::Null), "null");
    assert_eq!(ctx.const_value_label(&ConstValue::Unit), "unit");
    assert_eq!(ctx.const_value_label(&ConstValue::Unknown), "unknown");
    assert_eq!(
        ctx.const_value_label(&ConstValue::Char('x' as u16)),
        "char:'x'"
    );
    assert_eq!(
        ctx.const_value_label(&ConstValue::Symbol("S".into())),
        "sym:S"
    );
    assert_eq!(
        ctx.const_value_label(&ConstValue::Str {
            id: StrId::new(1),
            value: "abc".into()
        }),
        "str:abc"
    );
    assert_eq!(
        ctx.const_value_label(&ConstValue::RawStr("body".into())),
        "raw:body"
    );
}

#[test]
fn const_fn_cache_key_handles_enum_and_decimal() {
    let parsed = parse_module(
        r#"
namespace Demo;

public const fn Call(Demo.Color color, decimal value) -> decimal { return value; }
"#,
    )
    .expect("module parses");
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = TypeLayoutTable::default();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let symbol = ctx
        .symbol_index
        .function_decls("Demo::Call")
        .and_then(|list| list.first())
        .cloned()
        .expect("function symbol present");
    let args = vec![
        (
            None,
            ConstEvalResult::new(ConstValue::Enum {
                type_name: "Demo.Color".into(),
                variant: "Red".into(),
                discriminant: 1,
            }),
        ),
        (
            None,
            ConstEvalResult::new(ConstValue::Decimal(Decimal128::zero())),
        ),
    ];
    let key = ctx.const_fn_cache_key(&symbol, &args);
    let expected = ConstFnCacheKey::new(
        "Demo::Call",
        vec!["enum:Demo.Color::Red#1".into(), "dec:Decimal128(0)".into()],
    );
    assert_eq!(key, expected);
}

#[test]
fn resolve_const_function_rejects_empty_segments() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let err = ctx
        .resolve_const_function(None, None, &[], &[], None)
        .expect_err("empty segments should fail");
    assert!(matches!(err, ConstEvalError { .. }));
    assert!(
        err.message.contains("not a valid path"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn resolve_const_function_prefers_constexpr_over_inferred() {
    let mut symbol_index = SymbolIndex::default();
    let const_decl = make_const_fn(
        "Target",
        vec![make_param("value", "int")],
        "int",
        Some(make_return_block(ExprNode::Identifier("value".into()))),
    );
    let mut inferred_decl = make_const_fn(
        "Target",
        vec![make_param("value", "int")],
        "int",
        Some(make_return_block(ExprNode::Identifier("value".into()))),
    );
    inferred_decl.function.is_constexpr = false;
    symbol_index
        .function_decls
        .insert("Demo::Target".into(), vec![inferred_decl, const_decl]);
    let mut layouts = TypeLayoutTable::default();
    let ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let args = vec![(None, ConstEvalResult::new(ConstValue::Int(1)))];
    let symbol = ctx
        .resolve_const_function(Some("Demo"), None, &["Target".into()], &args, None)
        .expect("resolve prefers constexpr");
    assert!(symbol.function.is_constexpr);
}

#[test]
fn resolve_const_function_reports_ambiguous_overloads() {
    let parsed = parse_module(
        r#"
namespace Demo;

public const fn Over(int value) -> int { return value; }
public const fn Over(int value) -> int { return value + 1; }
"#,
    )
    .expect("module parses");
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = TypeLayoutTable::default();
    let ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let args = vec![(None, ConstEvalResult::new(ConstValue::Int(2)))];
    let err = ctx
        .resolve_const_function(
            Some("Demo"),
            None,
            &["Over".into()],
            &args,
            Some(Span::new(0, 1)),
        )
        .expect_err("ambiguous overloads rejected");
    assert!(err.message.contains("ambiguous"), "{}", err.message);
}

#[test]
fn execute_const_function_rejects_ref_binding() {
    let parsed = parse_module(
        r#"
namespace Demo;

public const fn RefArg(ref int value) -> int { return value; }
"#,
    )
    .expect("module parses");
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = TypeLayoutTable::default();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let callee = ExprNode::Identifier("RefArg".into());
    let args = vec![CallArgument::positional(
        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(2))),
        None,
        None,
    )];
    let mut env = EvalEnv {
        namespace: Some("Demo"),
        owner: None,
        span: Some(Span::new(0, 1)),
        params: None,
        locals: None,
    };
    let err = ctx
        .evaluate_call(&callee, None, &args, &mut env)
        .expect_err("ref binding not allowed for const-eval");
    assert!(
        err.message.contains("cannot use `ref` binding"),
        "unexpected message: {}",
        err.message
    );
}

#[test]
fn execute_const_function_detects_recursion() {
    let parsed = parse_module(
        r#"
namespace Demo;

public const fn Loop(int value) -> int { return value; }
"#,
    )
    .expect("module parses");
    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = TypeLayoutTable::default();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    ctx.fn_stack.push("Demo::Loop".into());
    let callee = ExprNode::Identifier("Loop".into());
    let args = vec![CallArgument::positional(
        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1))),
        None,
        None,
    )];
    let mut env = EvalEnv {
        namespace: Some("Demo"),
        owner: None,
        span: Some(Span::new(5, 10)),
        params: None,
        locals: None,
    };
    let err = ctx
        .evaluate_call(&callee, None, &args, &mut env)
        .expect_err("cycle should be detected");
    assert!(err.message.contains("cycle detected"), "{}", err.message);
}

#[test]
fn execute_const_function_rejects_async_extern_and_generic() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let mut symbol = make_const_fn("Check", Vec::new(), "int", None);
    let mut env = EvalEnv {
        namespace: Some("Demo"),
        owner: None,
        span: Some(Span::new(0, 1)),
        params: None,
        locals: None,
    };

    symbol.function.is_async = true;
    let err = ctx
        .execute_const_function(symbol.clone(), Vec::new(), &mut env)
        .expect_err("async const-eval not allowed");
    assert!(err.message.contains("cannot be async"), "{}", err.message);

    symbol.function.is_async = false;
    symbol.function.is_extern = true;
    let err = ctx
        .execute_const_function(symbol.clone(), Vec::new(), &mut env)
        .expect_err("extern const-eval not allowed");
    assert!(err.message.contains("cannot be extern"), "{}", err.message);

    symbol.function.is_extern = false;
    symbol.function.generics = Some(GenericParams::new(
        None,
        vec![GenericParam {
            name: "T".into(),
            span: None,
            kind: GenericParamKind::Type(Default::default()),
        }],
    ));
    let err = ctx
        .execute_const_function(symbol, Vec::new(), &mut env)
        .expect_err("generic const-eval not allowed");
    assert!(err.message.contains("cannot be generic"), "{}", err.message);
}

#[test]
fn execute_const_function_validates_arguments_and_body() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let symbol = make_const_fn(
        "Named",
        vec![make_param("expected", "int")],
        "int",
        Some(make_return_block(ExprNode::Identifier("expected".into()))),
    );
    let mut env = EvalEnv {
        namespace: Some("Demo"),
        owner: None,
        span: Some(Span::new(3, 8)),
        params: None,
        locals: None,
    };
    let err = ctx
        .execute_const_function(symbol.clone(), Vec::new(), &mut env)
        .expect_err("argument count mismatch should fail");
    assert!(err.message.contains("expects 1 arguments but received 0"));

    let err = ctx
        .execute_const_function(
            symbol.clone(),
            vec![(
                Some("other".into()),
                ConstEvalResult::new(ConstValue::Int(1)),
            )],
            &mut env,
        )
        .expect_err("unknown named argument should fail");
    assert!(
        err.message.contains("does not have parameter `other`"),
        "{}",
        err.message
    );

    let mut no_body = symbol.clone();
    no_body.function.body = None;
    let err = ctx
        .execute_const_function(no_body, Vec::new(), &mut env)
        .expect_err("body required for const-eval");
    assert!(err.message.contains("requires a body"), "{}", err.message);
}

#[test]
fn evaluate_pure_function_body_requires_return_value() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let symbol = make_const_fn(
        "Empty",
        Vec::new(),
        "int",
        Some(Block {
            statements: Vec::new(),
            span: None,
        }),
    );
    let params = HashMap::new();
    let err = ctx
        .evaluate_pure_function_body(
            &symbol.function,
            symbol.function.body.as_ref().unwrap(),
            None,
            None,
            &params,
            Some(Span::new(10, 20)),
        )
        .expect_err("missing return should fail");
    assert!(
        err.message.contains("does not return a value"),
        "{}",
        err.message
    );
}

#[test]
fn evaluate_quote_literal_builds_span_and_hygiene() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let mut ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let literal = QuoteLiteral {
        expression: ExprNode::Assign {
            op: AssignOp::AddAssign,
            target: ExprNode::Identifier("lhs".into()).boxed(),
            value: ExprNode::Unary {
                op: UnOp::Neg,
                expr: ExprNode::Identifier("rhs".into()).boxed(),
                postfix: false,
            }
            .boxed(),
        }
        .boxed(),
        source: "lhs += -rhs".into(),
        sanitized: "lhs += -rhs".into(),
        content_span: Some(QuoteSourceSpan { start: 1, end: 5 }),
        interpolations: vec![QuoteInterpolation {
            placeholder: "id".into(),
            expression: ExprNode::Quote(Box::new(QuoteLiteral {
                expression: ExprNode::Identifier("inner".into()).boxed(),
                source: "inner".into(),
                sanitized: "inner".into(),
                content_span: None,
                interpolations: Vec::new(),
                hygiene_anchor: 0,
            })),
            expression_text: "quote(inner)".into(),
            span: Some(QuoteSourceSpan { start: 0, end: 5 }),
        }],
        hygiene_anchor: 2,
    };
    let outer_span = Span::new(10, 25);
    let mut env = EvalEnv {
        namespace: None,
        owner: None,
        span: Some(outer_span),
        params: None,
        locals: None,
    };
    let result = ctx
        .evaluate_quote_literal(&literal, &mut env)
        .expect("quote literal evaluation succeeds")
        .value;
    let ConstValue::Struct { fields, .. } = result else {
        panic!("quote literal should produce struct");
    };
    let span_value = fields
        .iter()
        .find(|(name, _)| name == "Span")
        .expect("span present")
        .1
        .clone();
    let ConstValue::Struct {
        fields: span_fields,
        ..
    } = span_value
    else {
        panic!("span should be struct");
    };
    let mut start = None;
    let mut end = None;
    for (name, value) in span_fields {
        match (name.as_str(), value) {
            ("Start", ConstValue::UInt(v)) => start = Some(v),
            ("End", ConstValue::UInt(v)) => end = Some(v),
            _ => {}
        }
    }
    assert_eq!(start, Some(11));
    assert_eq!(end, Some(15));

    let hygiene_value = fields
        .iter()
        .find(|(name, _)| name == "Hygiene")
        .expect("hygiene present")
        .1
        .clone();
    let ConstValue::Struct {
        fields: hygiene_fields,
        ..
    } = hygiene_value
    else {
        panic!("hygiene should be struct");
    };
    let anchor = 12u128;
    let expected_seed = {
        let mut hash = (anchor as u64) ^ (literal.sanitized.len() as u64).rotate_left(13);
        hash ^= (literal.interpolations.len() as u64).rotate_left(27);
        for byte in literal.sanitized.as_bytes() {
            hash = hash.wrapping_mul(1099511628211).wrapping_add(*byte as u64);
        }
        hash
    };
    let mut seen_anchor = None;
    let mut seed = None;
    for (name, value) in hygiene_fields {
        match (name.as_str(), value) {
            ("Anchor", ConstValue::UInt(v)) => seen_anchor = Some(v),
            ("Seed", ConstValue::UInt(v)) => seed = Some(v),
            _ => {}
        }
    }
    assert_eq!(seen_anchor, Some(anchor));
    assert_eq!(seed, Some(expected_seed as u128));

    // Ensure we don't recurse infinitely when the outer span is reused.
    let captures = ctx.build_quote_capture_list(&ExprNode::Identifier("lhs".into()));
    assert_eq!(captures, vec!["lhs".to_string()]);
}

#[test]
fn evaluate_quote_literal_rejects_non_quote_interpolation() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let config = ConstEvalConfig::default();
    let mut ctx = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let literal = QuoteLiteral {
        expression: ExprNode::Identifier("value".into()).boxed(),
        source: "value".into(),
        sanitized: "value".into(),
        content_span: None,
        interpolations: vec![QuoteInterpolation {
            placeholder: "p".into(),
            expression: ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(1))),
            expression_text: "1".into(),
            span: Some(QuoteSourceSpan { start: 0, end: 1 }),
        }],
        hygiene_anchor: 0,
    };
    let mut env = EvalEnv {
        namespace: None,
        owner: None,
        span: Some(Span::new(1, 4)),
        params: None,
        locals: None,
    };
    let err = ctx
        .evaluate_quote_literal(&literal, &mut env)
        .expect_err("interpolation must be a quote");
    assert!(
        err.message.contains("must evaluate to `Std.Meta.Quote`"),
        "{}",
        err.message
    );
}

#[test]
fn build_quote_node_value_maps_expression_kinds() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    let pattern = PatternAst {
        node: PatternNode::Wildcard,
        span: None,
        metadata: PatternMetadata::default(),
    };
    let lambda_expr = ExprNode::Lambda(LambdaExpr {
        params: vec![LambdaParam {
            modifier: Some(LambdaParamModifier::In),
            ty: Some("int".into()),
            name: "p".into(),
            span: None,
            default: None,
        }],
        captures: vec!["c".into()],
        body: LambdaBody::Expression(ExprNode::Identifier("p".into()).boxed()),
        is_async: false,
        span: None,
    });
    let lambda_block = ExprNode::Lambda(LambdaExpr {
        params: Vec::new(),
        captures: Vec::new(),
        body: LambdaBody::Block(LambdaBlock {
            text: "{ return 1; }".into(),
            span: None,
        }),
        is_async: false,
        span: None,
    });
    let mut named_arg = CallArgument::named(
        CallArgumentName {
            text: "named".into(),
            span: None,
        },
        ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(2))),
        None,
        None,
    )
    .with_modifier(CallArgumentModifier::Ref, None);
    named_arg.inline_binding = Some(InlineBinding {
        kind: InlineBindingKind::Var,
        name: "tmp".into(),
        keyword_span: None,
        name_span: None,
        initializer: Some(ExprNode::Literal(LiteralConst::without_numeric(
            ConstValue::Int(3),
        ))),
        initializer_span: None,
    });
    let new_object = ExprNode::New(NewExpr {
        type_name: "Demo.Point".into(),
        type_span: None,
        keyword_span: None,
        array_lengths: None,
        args: vec![CallArgument::positional(
            ExprNode::Identifier("x".into()),
            None,
            None,
        )],
        arguments_span: None,
        initializer: Some(NewInitializer::Object {
            fields: vec![ObjectInitializerField {
                name: "x".into(),
                name_span: None,
                value: ExprNode::Identifier("field".into()),
                value_span: None,
                span: None,
            }],
            span: None,
        }),
        span: None,
    });
    let new_collection = ExprNode::New(NewExpr {
        type_name: "Demo.List".into(),
        type_span: None,
        keyword_span: None,
        array_lengths: None,
        args: Vec::new(),
        arguments_span: None,
        initializer: Some(NewInitializer::Collection {
            elements: vec![ExprNode::Identifier("item".into())],
            span: None,
        }),
        span: None,
    });
    let interpolated = ExprNode::InterpolatedString(InterpolatedStringExpr {
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
    });
    let quote = ExprNode::Quote(Box::new(QuoteLiteral {
        expression: ExprNode::Identifier("q".into()).boxed(),
        source: "q".into(),
        sanitized: "q".into(),
        content_span: None,
        interpolations: Vec::new(),
        hygiene_anchor: 1,
    }));

    let cases: Vec<(ExprNode, &str)> = vec![
        (
            ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Bool(true))),
            "Literal",
        ),
        (ExprNode::Identifier("x".into()), "Identifier"),
        (
            ExprNode::Unary {
                op: UnOp::Not,
                expr: ExprNode::Identifier("x".into()).boxed(),
                postfix: false,
            },
            "Unary",
        ),
        (
            ExprNode::Binary {
                op: BinOp::Add,
                left: ExprNode::Identifier("a".into()).boxed(),
                right: ExprNode::Identifier("b".into()).boxed(),
            },
            "Binary",
        ),
        (
            ExprNode::Conditional {
                condition: ExprNode::Identifier("cond".into()).boxed(),
                then_branch: ExprNode::Identifier("lhs".into()).boxed(),
                else_branch: ExprNode::Identifier("rhs".into()).boxed(),
            },
            "Conditional",
        ),
        (
            ExprNode::Cast {
                target: "int".into(),
                expr: ExprNode::Identifier("number".into()).boxed(),
                syntax: CastSyntax::Paren,
            },
            "Cast",
        ),
        (
            ExprNode::IsPattern {
                value: ExprNode::Identifier("val".into()).boxed(),
                pattern: pattern.clone(),
                guards: Vec::new(),
            },
            "Pattern",
        ),
        (lambda_expr, "Lambda"),
        (lambda_block, "Lambda"),
        (
            ExprNode::Parenthesized(ExprNode::Identifier("wrap".into()).boxed()),
            "Tuple",
        ),
        (
            ExprNode::Tuple(vec![
                ExprNode::Identifier("first".into()),
                ExprNode::Identifier("second".into()),
            ]),
            "Tuple",
        ),
        (
            ExprNode::Assign {
                target: ExprNode::Identifier("lhs".into()).boxed(),
                op: AssignOp::NullCoalesceAssign,
                value: ExprNode::Identifier("rhs".into()).boxed(),
            },
            "Assign",
        ),
        (
            ExprNode::Member {
                base: ExprNode::Identifier("obj".into()).boxed(),
                member: "prop".into(),
                null_conditional: false,
            },
            "Member",
        ),
        (
            ExprNode::Call {
                callee: ExprNode::Member {
                    base: ExprNode::Identifier("service".into()).boxed(),
                    member: "run".into(),
                    null_conditional: false,
                }
                .boxed(),
                args: vec![named_arg.clone()],
                generics: Some(vec!["T".into()]),
            },
            "Call",
        ),
        (
            ExprNode::Ref {
                expr: ExprNode::Identifier("target".into()).boxed(),
                readonly: true,
            },
            "Ref",
        ),
        (new_object, "New"),
        (new_collection, "New"),
        (
            ExprNode::Index {
                base: ExprNode::Identifier("arr".into()).boxed(),
                indices: vec![ExprNode::Literal(LiteralConst::without_numeric(
                    ConstValue::Int(0),
                ))],
                null_conditional: false,
            },
            "Index",
        ),
        (
            ExprNode::Await {
                expr: ExprNode::Identifier("future".into()).boxed(),
            },
            "Await",
        ),
        (
            ExprNode::TryPropagate {
                expr: ExprNode::Identifier("fallible".into()).boxed(),
                question_span: Some(Span::new(2, 3)),
            },
            "TryPropagate",
        ),
        (
            ExprNode::Throw {
                expr: Some(ExprNode::Identifier("err".into()).boxed()),
            },
            "Throw",
        ),
        (ExprNode::Throw { expr: None }, "Throw"),
        (
            ExprNode::SizeOf(SizeOfOperand::Type("int".into())),
            "SizeOf",
        ),
        (
            ExprNode::SizeOf(SizeOfOperand::Value(
                ExprNode::Identifier("x".into()).boxed(),
            )),
            "SizeOf",
        ),
        (
            ExprNode::AlignOf(SizeOfOperand::Type("int".into())),
            "AlignOf",
        ),
        (
            ExprNode::AlignOf(SizeOfOperand::Value(
                ExprNode::Identifier("x".into()).boxed(),
            )),
            "AlignOf",
        ),
        (
            ExprNode::NameOf(NameOfOperand {
                segments: vec!["Demo".into(), "Foo".into()],
                text: "Demo.Foo".into(),
                span: None,
            }),
            "NameOf",
        ),
        (interpolated, "InterpolatedString"),
        (quote, "Quote"),
    ];

    for (expr, expected_kind) in cases {
        let node = ctx.build_quote_node_value(&expr);
        let (kind, _, children) = quote_node_summary(&node);
        assert_eq!(kind, expected_kind);
        if !matches!(
            expected_kind,
            "Literal" | "Identifier" | "SizeOf" | "AlignOf" | "NameOf" | "Throw"
        ) {
            assert!(
                !children.is_empty(),
                "expected children for kind {}",
                expected_kind
            );
        }
    }
}

#[test]
fn quote_helpers_cover_operator_symbols() {
    let (mut symbol_index, mut layouts) = default_ctx();
    let mut ctx = ConstEvalContext::with_config(
        &mut symbol_index,
        &mut layouts,
        None,
        ConstEvalConfig::default(),
    );
    for (op, symbol) in [
        (UnOp::Neg, "-"),
        (UnOp::UnaryPlus, "+"),
        (UnOp::Not, "!"),
        (UnOp::BitNot, "~"),
        (UnOp::Increment, "++"),
        (UnOp::Decrement, "--"),
        (UnOp::Deref, "*"),
        (UnOp::AddrOf, "&"),
        (UnOp::AddrOfMut, "&mut"),
    ] {
        let node = ctx.build_quote_node_value(&ExprNode::Unary {
            op,
            expr: ExprNode::Identifier("x".into()).boxed(),
            postfix: false,
        });
        let (_, value, _) = quote_node_summary(&node);
        assert_eq!(value, Some(symbol.to_string()));
    }
    for (op, symbol) in [
        (BinOp::Sub, "-"),
        (BinOp::Mul, "*"),
        (BinOp::Div, "/"),
        (BinOp::Rem, "%"),
        (BinOp::BitAnd, "&"),
        (BinOp::BitOr, "|"),
        (BinOp::BitXor, "^"),
        (BinOp::Shl, "<<"),
        (BinOp::Shr, ">>"),
        (BinOp::Eq, "=="),
        (BinOp::Ne, "!="),
        (BinOp::Lt, "<"),
        (BinOp::Le, "<="),
        (BinOp::Gt, ">"),
        (BinOp::Ge, ">="),
        (BinOp::And, "&&"),
        (BinOp::Or, "||"),
        (BinOp::NullCoalesce, "??"),
    ] {
        let node = ctx.build_quote_node_value(&ExprNode::Binary {
            op,
            left: ExprNode::Identifier("l".into()).boxed(),
            right: ExprNode::Identifier("r".into()).boxed(),
        });
        let (_, value, _) = quote_node_summary(&node);
        assert_eq!(value, Some(symbol.to_string()));
    }
    for (op, symbol) in [
        (AssignOp::Assign, "="),
        (AssignOp::AddAssign, "+="),
        (AssignOp::SubAssign, "-="),
        (AssignOp::MulAssign, "*="),
        (AssignOp::DivAssign, "/="),
        (AssignOp::RemAssign, "%="),
        (AssignOp::BitAndAssign, "&="),
        (AssignOp::BitOrAssign, "|="),
        (AssignOp::BitXorAssign, "^="),
        (AssignOp::ShlAssign, "<<="),
        (AssignOp::ShrAssign, ">>="),
        (AssignOp::NullCoalesceAssign, "??="),
    ] {
        let node = ctx.build_quote_node_value(&ExprNode::Assign {
            target: ExprNode::Identifier("t".into()).boxed(),
            op,
            value: ExprNode::Identifier("v".into()).boxed(),
        });
        let (_, value, _) = quote_node_summary(&node);
        assert_eq!(value, Some(symbol.to_string()));
    }
}
