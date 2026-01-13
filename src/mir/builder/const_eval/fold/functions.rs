use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};
use crate::syntax::expr::{CallArgument, ExprNode};

use super::super::ConstEvalResult;
use super::super::environment::EvalEnv;

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn evaluate_call(
        &mut self,
        callee: &ExprNode,
        generics: Option<&[String]>,
        args: &[CallArgument],
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let segments =
            diagnostics::expr_path_segments(callee).map_err(|message| ConstEvalError {
                message,
                span: env.span,
            })?;
        let mut evaluated_args = Vec::with_capacity(args.len());
        for arg in args {
            let result = self.evaluate_node(&arg.value, env)?;
            let name = arg.name.as_ref().map(|name| name.text.clone());
            evaluated_args.push((name, result));
        }
        if let Some(value) =
            self.try_evaluate_reflect_intrinsic(&segments, generics, &evaluated_args, env)?
        {
            return Ok(ConstEvalResult::new(value));
        }
        if let Some(value) =
            self.try_evaluate_decimal_intrinsic(&segments, &evaluated_args, env.span)?
        {
            return Ok(ConstEvalResult::new(value));
        }
        let symbol = self.resolve_const_function(
            env.namespace,
            env.owner,
            &segments,
            &evaluated_args,
            env.span,
        )?;
        let cache_key = self.const_fn_cache_key(&symbol, &evaluated_args);
        if let Some(result) = self.const_fn_cache_lookup(&cache_key).cloned() {
            self.record_fn_cache_hit();
            return Ok(result);
        }
        self.record_fn_cache_miss();
        let result = self.execute_const_function(symbol, evaluated_args, env)?;
        self.const_fn_cache_store(cache_key, result.clone());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::const_eval_config::ConstEvalConfig;
    use crate::frontend::parser::parse_module;
    use crate::mir::TypeLayoutTable;
    use crate::mir::builder::symbol_index::SymbolIndex;
    use crate::mir::data::ConstValue;
    use crate::syntax::expr::builders::{CallArgument, ExprNode, LiteralConst};

    #[test]
    fn evaluate_call_hits_function_cache() {
        let parsed = parse_module(
            r#"
namespace Demo;

public const fn Twice(int value) -> int { return value * 2; }
"#,
        )
        .expect("module parses");
        assert!(
            parsed.diagnostics.is_empty(),
            "parse diagnostics: {:?}",
            parsed.diagnostics
        );
        let mut symbol_index = SymbolIndex::build(&parsed.module);
        let mut layouts = TypeLayoutTable::default();
        let mut ctx = ConstEvalContext::with_config(
            &mut symbol_index,
            &mut layouts,
            None,
            ConstEvalConfig {
                fuel_limit: ConstEvalConfig::default().fuel_limit,
                enable_expression_memo: false,
            },
        );
        let callee = ExprNode::Identifier("Twice".into());
        let args = vec![CallArgument::positional(
            ExprNode::Literal(LiteralConst::without_numeric(ConstValue::Int(2))),
            None,
            None,
        )];
        let mut env = EvalEnv {
            namespace: Some("Demo"),
            owner: None,
            span: None,
            params: None,
            locals: None,
        };
        let first = ctx
            .evaluate_call(&callee, None, &args, &mut env)
            .expect("first call succeeds");
        assert!(
            matches!(first.value, ConstValue::Int(v) if v == 4),
            "unexpected call result: {:?}",
            first.value
        );
        let second = ctx
            .evaluate_call(&callee, None, &args, &mut env)
            .expect("cached call succeeds");
        assert!(
            matches!(second.value, ConstValue::Int(v) if v == 4),
            "unexpected cached result: {:?}",
            second.value
        );
        let summary = ctx.evaluate_all();
        assert_eq!(summary.metrics.fn_cache_hits, 1);
        assert_eq!(summary.metrics.fn_cache_misses, 1);
    }
}
