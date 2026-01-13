use std::collections::HashMap;

use crate::frontend::ast::{BindingModifier, Block, FunctionDecl};
use crate::frontend::diagnostics::Span;
use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::ConstEvalResult;
use crate::mir::builder::const_eval::diagnostics::ConstEvalError;
use crate::mir::builder::const_eval::environment::FunctionFrame;
use crate::mir::builder::symbol_index::FunctionDeclSymbol;
use crate::mir::data::{ConstValue, Ty};

use super::super::super::environment::EvalEnv;

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn execute_const_function(
        &mut self,
        symbol: FunctionDeclSymbol,
        args: Vec<(Option<String>, ConstEvalResult)>,
        caller_env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        if self.fn_stack.iter().any(|entry| entry == &symbol.qualified) {
            return Err(ConstEvalError {
                message: format!(
                    "cycle detected while evaluating compile-time function `{}`",
                    symbol.qualified
                ),
                span: caller_env.span,
            });
        }

        let function = &symbol.function;
        if function.is_async {
            return Err(ConstEvalError {
                message: format!(
                    "compile-time function `{}` cannot be async",
                    symbol.qualified
                ),
                span: caller_env.span,
            });
        }
        if function.is_extern {
            return Err(ConstEvalError {
                message: format!(
                    "compile-time function `{}` cannot be extern",
                    symbol.qualified
                ),
                span: caller_env.span,
            });
        }
        if function
            .generics
            .as_ref()
            .is_some_and(|generics| !generics.params.is_empty())
        {
            return Err(ConstEvalError {
                message: format!(
                    "compile-time function `{}` cannot be generic",
                    symbol.qualified
                ),
                span: caller_env.span,
            });
        }

        let body = function.body.as_ref().ok_or_else(|| ConstEvalError {
            message: format!(
                "compile-time function `{}` requires a body",
                symbol.qualified
            ),
            span: caller_env.span,
        })?;

        if function.signature.parameters.len() != args.len() {
            return Err(ConstEvalError {
                message: format!(
                    "compile-time function `{}` expects {} arguments but received {}",
                    symbol.qualified,
                    function.signature.parameters.len(),
                    args.len()
                ),
                span: caller_env.span,
            });
        }

        let mut scope_params: HashMap<String, ConstValue> = HashMap::new();
        for (index, param) in function.signature.parameters.iter().enumerate() {
            if matches!(param.binding, BindingModifier::Ref | BindingModifier::Out) {
                return Err(ConstEvalError {
                    message: format!(
                        "compile-time function `{}` parameter `{}` cannot use `{}` binding",
                        symbol.qualified,
                        param.name,
                        match param.binding {
                            BindingModifier::Ref => "ref",
                            BindingModifier::Out => "out",
                            BindingModifier::In => "in",
                            BindingModifier::Value => "value",
                        }
                    ),
                    span: caller_env.span,
                });
            }

            let (arg_name, value) = args[index].clone();
            if let Some(name) = arg_name {
                if !name.eq_ignore_ascii_case(&param.name) {
                    return Err(ConstEvalError {
                        message: format!(
                            "compile-time function `{}` does not have parameter `{}`",
                            symbol.qualified, name
                        ),
                        span: caller_env.span,
                    });
                }
            }

            let ty = Ty::from_type_expr(&param.ty);
            self.ensure_ty_layout(&ty);
            let coerced = self
                .convert_value_to_type(value.value, value.literal.clone(), &ty, caller_env.span)
                .map_err(|err| err.with_span_if_missing(caller_env.span))?;
            scope_params.insert(param.name.clone(), coerced);
        }

        let fn_namespace = symbol.namespace.as_deref().or(caller_env.namespace);
        let fn_owner = symbol.owner.as_deref();

        self.fn_stack.push(symbol.qualified.clone());
        let result_value = self.evaluate_pure_function_body(
            function,
            body,
            fn_namespace,
            fn_owner,
            &scope_params,
            caller_env.span,
        );
        self.fn_stack.pop();
        result_value
    }

    pub(crate) fn evaluate_pure_function_body(
        &mut self,
        function: &FunctionDecl,
        body: &Block,
        namespace: Option<&str>,
        owner: Option<&str>,
        params: &HashMap<String, ConstValue>,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let mut frame = FunctionFrame::new();
        frame.push_scope();
        let result = self.evaluate_block_statements(
            &body.statements,
            function,
            namespace,
            owner,
            params,
            &mut frame,
        )?;
        frame.pop_scope();

        let value = result.ok_or_else(|| ConstEvalError {
            message: format!(
                "compile-time function `{}` does not return a value",
                function.name
            ),
            span,
        })?;

        let return_ty = Ty::from_type_expr(&function.signature.return_type);
        self.ensure_ty_layout(&return_ty);
        let converted = self
            .convert_value_to_type(value, None, &return_ty, span)
            .map_err(|err| err.with_span_if_missing(span))?;
        Ok(ConstEvalResult::new(converted))
    }
}
