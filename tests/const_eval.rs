use chic::const_eval_config::ConstEvalConfig;
use chic::frontend::ast::Expression;
use chic::frontend::parser::parse_module;
use chic::mir::{BinOp, ConstEvalContext, ConstValue, SymbolIndex, Ty, TypeLayoutTable};
use chic::syntax::expr::builders::{CallArgument, ExprNode, LiteralConst};

fn binary_expr(lhs: i128, op: BinOp, rhs: i128, text: &str) -> Expression {
    Expression::with_node(
        text,
        None,
        ExprNode::Binary {
            op,
            left: ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(lhs))).boxed(),
            right: ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(rhs))).boxed(),
        },
    )
}

#[test]
fn memoization_metrics_record_hits_and_misses() {
    let mut symbol_index = SymbolIndex::default();
    let mut layouts = TypeLayoutTable::default();
    let config = ConstEvalConfig::default();
    let mut context = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);

    let expr = binary_expr(1, BinOp::Add, 2, "1 + 2");
    let ty = Ty::named("int");

    let value = context
        .evaluate_expression(&expr, None, None, None, None, &ty, None)
        .expect("first evaluation succeeds")
        .value;
    match value {
        ConstValue::Int(result) => assert_eq!(result, 3),
        _ => panic!("unexpected const value variant"),
    }

    let value = context
        .evaluate_expression(&expr, None, None, None, None, &ty, None)
        .expect("memoized evaluation succeeds")
        .value;
    match value {
        ConstValue::Int(result) => assert_eq!(result, 3),
        _ => panic!("unexpected const value variant"),
    }

    let summary = context.evaluate_all();
    assert!(
        summary.errors.is_empty(),
        "expected no const-eval diagnostics, found {:?}",
        summary.errors
    );

    let metrics = summary.metrics;
    assert_eq!(metrics.expressions_requested, 2);
    assert_eq!(metrics.expressions_evaluated, 1);
    assert_eq!(metrics.memo_hits, 1);
    assert_eq!(metrics.memo_misses, 1);
    assert_eq!(metrics.fuel_consumed, 3);
    assert_eq!(metrics.fuel_exhaustions, 0);
    assert_eq!(metrics.fuel_limit, ConstEvalConfig::default().fuel_limit);
    assert_eq!(metrics.cache_entries, 1);
}

#[test]
fn fuel_limit_exhaustion_reports_diagnostics() {
    let mut symbol_index = SymbolIndex::default();
    let mut layouts = TypeLayoutTable::default();
    let config = ConstEvalConfig::default().with_fuel_limit(Some(2));
    let mut context = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);

    let expr = binary_expr(1, BinOp::Add, 2, "1 + 2");
    let ty = Ty::named("int");

    let error = context
        .evaluate_expression(&expr, None, None, None, None, &ty, None)
        .expect_err("evaluation should exhaust fuel");
    assert!(
        error
            .message
            .contains("constant evaluation aborted: fuel limit of 2 exhausted"),
        "unexpected error message: {}",
        error.message
    );

    let summary = context.evaluate_all();
    assert!(
        summary.errors.is_empty(),
        "expected no queued const-eval diagnostics, found {:?}",
        summary.errors
    );
    let metrics = summary.metrics;
    assert_eq!(metrics.expressions_requested, 1);
    assert_eq!(metrics.expressions_evaluated, 0);
    assert_eq!(metrics.memo_hits, 0);
    assert_eq!(metrics.memo_misses, 0);
    assert_eq!(metrics.fuel_consumed, 2);
    assert_eq!(metrics.fuel_exhaustions, 1);
    assert_eq!(metrics.fuel_limit, Some(2));
    assert_eq!(metrics.cache_entries, 0);
}

#[test]
fn const_fn_calls_are_cached() {
    let parsed = parse_module(
        r#"
namespace Demo;

public const fn Twice(int value) -> int { return value * 2; }
"#,
    )
    .expect("parse module");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );

    let mut symbol_index = SymbolIndex::build(&parsed.module);
    let mut layouts = TypeLayoutTable::default();
    let config = ConstEvalConfig {
        fuel_limit: ConstEvalConfig::default().fuel_limit,
        enable_expression_memo: false,
    };
    let mut context = ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
    let expr = Expression::with_node(
        "Twice(2)".to_string(),
        None,
        ExprNode::Call {
            callee: ExprNode::Identifier("Twice".into()).boxed(),
            args: vec![CallArgument::positional(
                ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(2))),
                None,
                None,
            )],
            generics: None,
        },
    );
    let ty = Ty::named("int");
    let namespace = Some("Demo");
    context
        .evaluate_expression(&expr, namespace, None, None, None, &ty, None)
        .expect("first evaluation succeeds");
    context
        .evaluate_expression(&expr, namespace, None, None, None, &ty, None)
        .expect("second evaluation cached");
    let summary = context.evaluate_all();
    assert!(
        summary.errors.is_empty(),
        "expected no const-eval errors, got {:?}",
        summary.errors
    );
    assert_eq!(summary.metrics.fn_cache_hits, 1);
    assert_eq!(summary.metrics.fn_cache_misses, 1);
}
