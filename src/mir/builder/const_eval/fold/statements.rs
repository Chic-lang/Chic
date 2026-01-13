use std::collections::HashMap;

use crate::frontend::ast::{
    ConstStatement, Expression, FunctionDecl, Statement, StatementKind, VariableDeclaration,
    VariableModifier,
};
use crate::frontend::diagnostics::Span;
use crate::mir::data::{ConstValue, Ty};

use super::super::ConstEvalResult;
use super::super::environment::{EvalEnv, FunctionFrame};
use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn evaluate_block_statements(
        &mut self,
        statements: &[Statement],
        function: &FunctionDecl,
        namespace: Option<&str>,
        owner: Option<&str>,
        params: &HashMap<String, ConstValue>,
        frame: &mut FunctionFrame,
    ) -> Result<Option<ConstValue>, ConstEvalError> {
        for statement in statements {
            if let Some(result) =
                self.evaluate_statement(statement, function, namespace, owner, params, frame)?
            {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub(crate) fn evaluate_statement(
        &mut self,
        statement: &Statement,
        function: &FunctionDecl,
        namespace: Option<&str>,
        owner: Option<&str>,
        params: &HashMap<String, ConstValue>,
        frame: &mut FunctionFrame,
    ) -> Result<Option<ConstValue>, ConstEvalError> {
        match &statement.kind {
            StatementKind::Block(block) => {
                frame.push_scope();
                let result = self.evaluate_block_statements(
                    &block.statements,
                    function,
                    namespace,
                    owner,
                    params,
                    frame,
                )?;
                frame.pop_scope();
                Ok(result)
            }
            StatementKind::Empty => Ok(None),
            StatementKind::ConstDeclaration(const_stmt) => {
                self.evaluate_const_declaration(
                    const_stmt,
                    namespace,
                    owner,
                    params,
                    frame,
                    statement.span,
                )?;
                Ok(None)
            }
            StatementKind::VariableDeclaration(decl) => {
                self.evaluate_variable_declaration(
                    decl,
                    namespace,
                    owner,
                    params,
                    frame,
                    statement.span,
                )?;
                Ok(None)
            }
            StatementKind::Expression(expr) => {
                let _ = self.evaluate_expression_in_frame(
                    expr,
                    frame,
                    params,
                    namespace,
                    owner,
                    expr.span.or(statement.span),
                )?;
                Ok(None)
            }
            StatementKind::Return { expression } => {
                let value = if let Some(expr) = expression {
                    self.evaluate_expression_in_frame(
                        expr,
                        frame,
                        params,
                        namespace,
                        owner,
                        expr.span.or(statement.span),
                    )?
                    .value
                } else {
                    ConstValue::Unit
                };
                Ok(Some(value))
            }
            StatementKind::If(if_stmt) => {
                let condition = self
                    .evaluate_expression_in_frame(
                        &if_stmt.condition,
                        frame,
                        params,
                        namespace,
                        owner,
                        if_stmt.condition.span.or(statement.span),
                    )?
                    .value;
                let flag = match condition {
                    ConstValue::Bool(value) => value,
                    other => {
                        return Err(ConstEvalError {
                            message: format!(
                                "compile-time function condition expected `bool`, found {other:?}"
                            ),
                            span: if_stmt.condition.span.or(statement.span),
                        });
                    }
                };
                if flag {
                    frame.push_scope();
                    let result = self.evaluate_statement(
                        &if_stmt.then_branch,
                        function,
                        namespace,
                        owner,
                        params,
                        frame,
                    )?;
                    frame.pop_scope();
                    Ok(result)
                } else if let Some(else_branch) = &if_stmt.else_branch {
                    frame.push_scope();
                    let result = self.evaluate_statement(
                        else_branch,
                        function,
                        namespace,
                        owner,
                        params,
                        frame,
                    )?;
                    frame.pop_scope();
                    Ok(result)
                } else {
                    Ok(None)
                }
            }
            other => Err(ConstEvalError {
                message: format!(
                    "`{}` statements are not supported in compile-time function evaluation",
                    diagnostics::statement_kind_name(other)
                ),
                span: statement.span,
            }),
        }
    }

    pub(crate) fn evaluate_const_declaration(
        &mut self,
        const_stmt: &ConstStatement,
        namespace: Option<&str>,
        owner: Option<&str>,
        params: &HashMap<String, ConstValue>,
        frame: &mut FunctionFrame,
        fallback_span: Option<Span>,
    ) -> Result<(), ConstEvalError> {
        let ty = Ty::from_type_expr(&const_stmt.declaration.ty);
        self.ensure_ty_layout(&ty);
        for declarator in &const_stmt.declaration.declarators {
            let span = declarator
                .initializer
                .span
                .or(declarator.span)
                .or(fallback_span);
            let result = self.evaluate_expression_in_frame(
                &declarator.initializer,
                frame,
                params,
                namespace,
                owner,
                span,
            )?;
            let converted = self
                .convert_value_to_type(result.value, result.literal, &ty, span)
                .map_err(|err| err.with_span_if_missing(span))?;
            frame.declare(&declarator.name, converted, false, span)?;
        }
        Ok(())
    }

    pub(crate) fn evaluate_variable_declaration(
        &mut self,
        decl: &VariableDeclaration,
        namespace: Option<&str>,
        owner: Option<&str>,
        params: &HashMap<String, ConstValue>,
        frame: &mut FunctionFrame,
        fallback_span: Option<Span>,
    ) -> Result<(), ConstEvalError> {
        let mutable = matches!(decl.modifier, VariableModifier::Var);
        let explicit_ty = decl.type_annotation.as_ref().map(|ty| {
            let ty = Ty::from_type_expr(ty);
            self.ensure_ty_layout(&ty);
            ty
        });

        for declarator in &decl.declarators {
            let initializer = declarator
                .initializer
                .as_ref()
                .ok_or_else(|| ConstEvalError {
                    message: format!(
                        "variable `{}` in compile-time function requires an initializer",
                        declarator.name
                    ),
                    span: fallback_span,
                })?;
            let span = initializer.span.or(fallback_span);
            let mut result = self.evaluate_expression_in_frame(
                initializer,
                frame,
                params,
                namespace,
                owner,
                span,
            )?;
            if let Some(ty) = &explicit_ty {
                let converted = self
                    .convert_value_to_type(result.value, result.literal.clone(), ty, span)
                    .map_err(|err| err.with_span_if_missing(span))?;
                result = ConstEvalResult::with_literal(converted, None);
            }
            frame.declare(&declarator.name, result.value, mutable, span)?;
        }

        Ok(())
    }

    pub(crate) fn evaluate_expression_in_frame(
        &mut self,
        expr: &Expression,
        frame: &mut FunctionFrame,
        params: &HashMap<String, ConstValue>,
        namespace: Option<&str>,
        owner: Option<&str>,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let node = expr.node.as_ref().ok_or_else(|| ConstEvalError {
            message: "expression is not a valid compile-time value".into(),
            span: expr.span.or(span),
        })?;
        let mut env = EvalEnv {
            namespace,
            owner,
            span: expr.span.or(span),
            params: Some(params),
            locals: Some(frame as &mut dyn super::super::environment::LocalResolver),
        };
        self.evaluate_node(node, &mut env)
    }
}
