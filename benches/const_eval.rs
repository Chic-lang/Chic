use chic::const_eval_config::ConstEvalConfig;
use chic::frontend::ast::Expression;
use chic::frontend::parser::parse_module;
use chic::mir::{ConstEvalContext, ConstValue, SymbolIndex, Ty, TypeLayoutTable};
use chic::syntax::expr::builders::{CallArgument, ExprNode, LiteralConst};
use criterion::{Criterion, criterion_group, criterion_main};

fn const_fn_source() -> &'static str {
    r#"
namespace Bench;

public const fn Twice(int value) -> int { return value * 2; }
public const fn AddOne(int value) -> int { return value + 1; }
"#
}

fn build_twice_call(target: i32) -> ExprNode {
    ExprNode::Call {
        callee: ExprNode::Identifier("Twice".into()).boxed(),
        args: vec![CallArgument::positional(
            ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(
                target as i128,
            ))),
            None,
            None,
        )],
        generics: None,
    }
}

fn bench_const_fn_ctfe(c: &mut Criterion) {
    let parsed = parse_module(const_fn_source()).expect("parse module");
    assert!(
        parsed.diagnostics.is_empty(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );

    c.bench_function("const_fn_cached_eval", |b| {
        b.iter(|| {
            let mut symbol_index = SymbolIndex::build(&parsed.module);
            let mut layouts = TypeLayoutTable::default();
            let config = ConstEvalConfig {
                fuel_limit: ConstEvalConfig::default().fuel_limit,
                enable_expression_memo: false,
            };
            let mut ctx =
                ConstEvalContext::with_config(&mut symbol_index, &mut layouts, None, config);
            let expr = build_twice_call(32);
            let ty = Ty::named("int");
            let namespace = Some("Bench");
            let expression = Expression::with_node("Twice(32)", None, expr);
            let _ = ctx
                .evaluate_expression(&expression, namespace, None, None, None, &ty, None)
                .expect("first eval");
            let _ = ctx
                .evaluate_expression(&expression, namespace, None, None, None, &ty, None)
                .expect("cached eval");
            let summary = ctx.evaluate_all();
            assert!(
                summary.errors.is_empty(),
                "ctfe errors: {:?}",
                summary.errors
            );
        });
    });
}

criterion_group!(const_eval, bench_const_fn_ctfe);
criterion_main!(const_eval);
