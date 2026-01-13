use super::*;

body_builder_impl! {
    pub(super) fn lower_region_statement(
        &mut self,
        statement: &AstStatement,
        name: &str,
        body: &Block,
    ) {
        self.push_scope();

        let region_ty = Ty::named("Std.Memory.RegionHandle");
        self.ensure_ty_layout_for_ty(&region_ty);

        let mut decl = LocalDecl::new(
            Some(name.to_string()),
            region_ty.clone(),
            false,
            statement.span,
            LocalKind::Local,
        );
        decl.is_nullable = false;
        let region_local = self.push_local(decl);
        self.push_statement(MirStatement {
            span: statement.span,
            kind: MirStatementKind::StorageLive(region_local),
        });
        self.bind_name(name, region_local);
        self.record_local(region_local, statement.span);

        let init_expr_text = format!("Std.Memory.Region.Enter(\"{name}\")");
        let init_expr = Expression::new(init_expr_text, statement.span);
        let Some(region_operand) = self.lower_expression_operand(&init_expr) else {
            self.push_pending(statement, PendingStatementKind::Region);
            self.emit_storage_dead_for_resources(&[(region_local, statement.span)]);
            self.mark_local_dead(region_local);
            self.pop_scope();
            return;
        };

        self.push_statement(MirStatement {
            span: statement.span,
            kind: MirStatementKind::Assign {
                place: Place::new(region_local),
                value: Rvalue::Use(region_operand),
            },
        });

        self.schedule_defer_drop(region_local, statement.span);
        self.lower_block(body);

        if self.blocks[self.current_block.0].terminator.is_none() {
            self.emit_storage_dead_for_resources(&[(region_local, statement.span)]);
        }
        self.mark_local_dead(region_local);
        self.pop_scope();
    }
}
