use super::shared::LoopBlockPlan;
use super::*;

body_builder_impl! {
    pub(crate) fn lower_do_while_statement(
        &mut self,
        statement: &AstStatement,
        body: &AstStatement,
        condition: &crate::frontend::ast::Expression,
    ) {
        let plan = LoopBlockPlan::new(
            self,
            condition.span,
            body.span,
            statement.span,
            None,
        );

        self.ensure_goto(plan.body, statement.span);

        self.switch_to_block(plan.body);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(plan.exit, plan.condition, loop_scope_depth);
        self.lower_statement(body);
        self.pop_loop();
        self.ensure_goto(plan.condition, body.span);

        self.switch_to_block(plan.condition);
        self.validate_required_initializer(condition);
        let Some(cond_node) = self.expression_node(condition) else {
            self.push_pending(statement, PendingStatementKind::DoWhile);
            self.switch_to_block(plan.exit);
            return;
        };
        if !self.lower_boolean_branch(cond_node, plan.body, plan.exit, condition.span) {
            self.push_pending(statement, PendingStatementKind::DoWhile);
            self.switch_to_block(plan.exit);
            return;
        }

        self.switch_to_block(plan.exit);
    }
}
