use super::*;

body_builder_impl! {
    pub(super) fn push_loop(
        &mut self,
        break_target: BlockId,
        continue_target: BlockId,
        scope_depth: usize,
    ) {
        self.loop_stack.push(LoopContext {
            break_target,
            continue_target,
            scope_depth,
        });
    }

    pub(super) fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    pub(super) fn current_loop(&self) -> Option<LoopContext> {
        self.loop_stack.last().copied()
    }

    pub(crate) fn lower_break_statement(&mut self, statement: &AstStatement) {
        if let Some((target, scope_depth)) = self.current_switch_break_target() {
            self.drop_to_scope_depth(scope_depth, statement.span);
            self.set_terminator(statement.span, Terminator::Goto { target });
        } else if let Some(ctx) = self.current_loop() {
            self.drop_to_scope_depth(ctx.scope_depth, statement.span);
            self.set_terminator(
                statement.span,
                Terminator::Goto {
                    target: ctx.break_target,
                },
            );
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "break statement is only allowed inside loops".into(),
                span: statement.span,
                            });
            self.push_pending(statement, PendingStatementKind::Break);
        }
    }

    pub(crate) fn lower_continue_statement(&mut self, statement: &AstStatement) {
        if let Some(ctx) = self.current_loop() {
            self.drop_to_scope_depth(ctx.scope_depth, statement.span);
            self.set_terminator(
                statement.span,
                Terminator::Goto {
                    target: ctx.continue_target,
                },
            );
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "continue statement is only allowed inside loops".into(),
                span: statement.span,
                            });
            self.push_pending(statement, PendingStatementKind::Continue);
        }
    }
}
