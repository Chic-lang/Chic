use super::*;
use crate::mir::{AtomicFenceScope, AtomicOrdering};

body_builder_impl! {
    pub(super) fn lower_checked_statement(&mut self, statement: &AstStatement) {
        let AstStatementKind::Checked { body } = &statement.kind else {
            unreachable!("lower_checked_statement called with non-checked statement");
        };
        let previous_unchecked_depth = self.unchecked_depth;
        self.unchecked_depth = 0;
        self.lower_block(body);
        self.unchecked_depth = previous_unchecked_depth;
    }

    pub(super) fn lower_atomic_statement(&mut self, statement: &AstStatement) {
        let AstStatementKind::Atomic { ordering, body } = &statement.kind else {
            unreachable!("lower_atomic_statement called with non-atomic statement");
        };
        let order = ordering
            .as_ref()
            .and_then(|expr| self.atomic_order_from_expression(expr, "atomic block ordering"))
            .unwrap_or(AtomicOrdering::SeqCst);
        let prologue_span = ordering
            .as_ref()
            .and_then(|expr| expr.span)
            .or(statement.span);

        self.push_statement(MirStatement {
            span: prologue_span,
            kind: MirStatementKind::AtomicFence {
                order,
                scope: AtomicFenceScope::BlockEnter,
            },
        });

        self.atomic_depth += 1;
        self.lower_block(body);
        self.atomic_depth -= 1;

        self.push_statement(MirStatement {
            span: statement.span,
            kind: MirStatementKind::AtomicFence {
                order,
                scope: AtomicFenceScope::BlockExit,
            },
        });
    }

    pub(super) fn lower_unchecked_statement(&mut self, statement: &AstStatement) {
        let AstStatementKind::Unchecked { body } = &statement.kind else {
            unreachable!("lower_unchecked_statement called with non-unchecked statement");
        };
        self.unchecked_depth += 1;
        self.lower_block(body);
        self.unchecked_depth -= 1;
    }
}
