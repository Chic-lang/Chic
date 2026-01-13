use super::super::*;

body_builder_impl! {
    pub(crate) fn push_local(
        &mut self,
        decl: LocalDecl,
    ) -> LocalId {
        let id = LocalId(self.locals.len());
        self.locals.push(decl);
        id
    }

    pub(crate) fn local_decl(
        &self,
        id: LocalId,
    ) -> Option<&LocalDecl> {
        self.locals.get(id.0)
    }

    pub(crate) fn record_local(
        &mut self,
        local: LocalId,
        span: Option<Span>,
    ) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.locals.push(ScopeLocal {
                local,
                span,
                live: true,
            });
        }
    }

    pub(crate) fn emit_storage_dead(
        &mut self,
        local: LocalId,
        span: Option<Span>,
    ) {
        let stmt = MirStatement {
            span,
            kind: MirStatementKind::StorageDead(local),
        };
        self.push_statement(stmt);
    }

    #[allow(dead_code)]
    pub(crate) fn drop_to_scope_depth(
        &mut self,
        target_depth: usize,
        span: Option<Span>,
    ) {
        let current_depth = self.scope_depth();
        if target_depth >= current_depth {
            return;
        }

        let mut drops: Vec<(LocalId, Option<Span>)> = Vec::new();
        let mut depth = current_depth;
        while depth > target_depth {
            let frame_index = depth - 1;
            if let Some(frame) = self.scopes.get_mut(frame_index) {
                for entry in frame.locals.iter_mut().rev() {
                    if entry.live {
                        entry.live = false;
                        let drop_span = entry.span.or(span);
                        drops.push((entry.local, drop_span));
                    }
                }
            }
            depth -= 1;
        }

        for (local, drop_span) in drops {
            self.emit_storage_dead(local, drop_span);
        }
    }

    pub(crate) fn create_temp(
        &mut self,
        span: Option<Span>,
    ) -> LocalId {
        let name = Some(format!("$t{}", self.temp_counter));
        self.temp_counter += 1;
        let decl = LocalDecl::new(name, Ty::Unknown, true, span, LocalKind::Temp);
        let id = self.push_local(decl);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::StorageLive(id),
        });
        self.record_local(id, span);
        id
    }

    pub(crate) fn create_temp_untracked(
        &mut self,
        span: Option<Span>,
    ) -> LocalId {
        let name = Some(format!("$t{}", self.temp_counter));
        self.temp_counter += 1;
        let decl = LocalDecl::new(name, Ty::Unknown, true, span, LocalKind::Temp);
        self.push_local(decl)
    }

    pub(crate) fn hint_local_ty(&mut self, local: LocalId, ty: Ty) {
        if let Some(decl) = self.locals.get_mut(local.0) {
            if matches!(decl.ty, Ty::Unknown) {
                decl.is_nullable = matches!(ty, Ty::Nullable(_));
                decl.ty = ty;
            }
        }
    }
}
