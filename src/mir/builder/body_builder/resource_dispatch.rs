use super::*;

body_builder_impl! {
    pub(super) fn try_lower_resource_statement(&mut self, statement: &AstStatement) -> bool {
        match &statement.kind {
            AstStatementKind::Using(using_stmt) => {
                self.lower_using_statement(statement, using_stmt);
                true
            }
            AstStatementKind::Lock { expression, body } => {
                self.lower_lock_statement(statement, expression, body.as_ref());
                true
            }
            AstStatementKind::Fixed(fixed) => {
                self.lower_fixed_statement(statement, fixed);
                true
            }
            _ => false,
        }
    }

    pub(super) fn schedule_defer_drop(&mut self, local: LocalId, span: Option<Span>) {
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::DeferDrop {
                place: Place::new(local),
            },
        });
    }

    pub(super) fn schedule_defer_drops(&mut self, resources: &[(LocalId, Option<Span>)]) {
        for (local, span) in resources.iter().rev() {
            self.schedule_defer_drop(*local, *span);
        }
    }

    pub(super) fn register_resource_decl(&mut self, decl: LocalDecl) -> LocalId {
        let span = decl.span;
        let local = self.push_local(decl);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::StorageLive(local),
        });
        self.record_local(local, span);
        local
    }

    pub(super) fn emit_storage_dead_for_resources(
        &mut self,
        resources: &[(LocalId, Option<Span>)],
    ) {
        for (local, span) in resources.iter().rev() {
            self.push_statement(MirStatement {
                span: *span,
                                kind: MirStatementKind::StorageDead(*local),
            });
        }
    }

    pub(super) fn mark_resources_dead(&mut self, resources: &[(LocalId, Option<Span>)]) {
        for (local, _) in resources.iter().rev() {
            self.mark_local_dead(*local);
        }
    }
}
