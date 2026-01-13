use super::shared::LoopBlockPlan;
use super::*;

body_builder_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "For-loop lowering coordinates initializer, condition, iterator, and cleanup."
    )]
    pub(crate) fn lower_for_statement(
        &mut self,
        statement: &AstStatement,
        for_stmt: &crate::frontend::ast::ForStatement,
    ) {
        self.push_scope();

        if let Some(initializer) = &for_stmt.initializer {
            match initializer {
                crate::frontend::ast::ForInitializer::Declaration(decl) => {
                    self.lower_variable_declaration(statement.span, decl);
                }
                crate::frontend::ast::ForInitializer::Const(const_stmt) => {
                    self.lower_const_statement(statement, const_stmt);
                }
                crate::frontend::ast::ForInitializer::Expressions(exprs) => {
                    for expr in exprs {
                        if !self.lower_expression_statement(expr) {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "failed to lower for-loop initializer expression `{}`",
                                    expr.text
                                ),
                                span: expr.span,
                                                            });
                        }
                    }
                }
            }
        }

        let iterator_span = if for_stmt.iterator.is_empty() {
            None
        } else {
            for_stmt.body.span
        };
        let plan = LoopBlockPlan::new(
            self,
            for_stmt.condition.as_ref().and_then(|c| c.span),
            for_stmt.body.span,
            statement.span,
            iterator_span,
        );

        self.ensure_goto(plan.condition, statement.span);

        self.switch_to_block(plan.condition);
        if let Some(cond_expr) = &for_stmt.condition {
            self.validate_required_initializer(cond_expr);
            let Some(cond_node) = self.expression_node(cond_expr) else {
                self.push_pending(statement, PendingStatementKind::For);
                self.switch_to_block(plan.exit);
                self.pop_scope();
                return;
            };
            if !self.lower_boolean_branch(cond_node, plan.body, plan.exit, cond_expr.span) {
                self.push_pending(statement, PendingStatementKind::For);
                self.switch_to_block(plan.exit);
                self.pop_scope();
                return;
            }
        } else {
            self.set_terminator(statement.span, Terminator::Goto { target: plan.body });
        }

        self.switch_to_block(plan.body);
        let continue_target = plan.iterator.unwrap_or(plan.condition);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(plan.exit, continue_target, loop_scope_depth);
        self.lower_statement(for_stmt.body.as_ref());
        self.pop_loop();

        match plan.iterator {
            Some(iter_block) => {
                self.ensure_goto(iter_block, for_stmt.body.span);
                self.switch_to_block(iter_block);
                for expr in &for_stmt.iterator {
                    if !self.lower_expression_statement(expr) {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "failed to lower for-loop iterator expression `{}`",
                                expr.text
                            ),
                            span: expr.span,
                                                    });
                    }
                }
                self.ensure_goto(plan.condition, statement.span);
            }
            None => {
                self.ensure_goto(plan.condition, statement.span);
            }
        }

        self.switch_to_block(plan.exit);
        self.pop_scope();
    }
}
