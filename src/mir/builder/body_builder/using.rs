use super::*;

body_builder_impl! {

    #[expect(
        clippy::too_many_lines,
        reason = "Using lowering coordinates resource management and diagnostics in a single routine."
    )]
    pub(super) fn lower_using_statement(
        &mut self,
        statement: &AstStatement,
        using: &crate::frontend::ast::UsingStatement,
    ) {
        self.push_scope();

        let mut resources: Vec<(LocalId, Option<Span>)> = Vec::new();

        match &using.resource {
            crate::frontend::ast::UsingResource::Expression(expr) => {
                let Some(operand) = self.lower_expression_operand(expr) else {
                    self.push_pending(statement, PendingStatementKind::Using);
                    self.pop_scope();
                    return;
                };
                let operand_ty = self.operand_ty(&operand);

                let span = expr.span.or(statement.span);
                let id = self.next_using_id;
                self.next_using_id += 1;
                let name = format!("__using_resource_{id}");
                let resource_local = self.register_resource_decl(LocalDecl::new(
                    Some(name),
                    Ty::Unknown,
                    true,
                    span,
                                        LocalKind::Local,
                ));

                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(resource_local),
                        value: Rvalue::Use(operand.clone()),
                    },
                });
                if let Some(ty) = operand_ty {
                    self.hint_local_ty(resource_local, ty);
                }

                resources.push((resource_local, span));
            }
            crate::frontend::ast::UsingResource::Declaration(decl) => {
                if decl.declarators.is_empty() {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "using declaration requires at least one binding".into(),
                        span: statement.span,
                                            });
                    self.push_pending(statement, PendingStatementKind::Using);
                    self.pop_scope();
                    return;
                }

                let start = self.locals.len();
                self.lower_variable_declaration(statement.span, decl);
                for index in start..self.locals.len() {
                    let local = LocalId(index);
                    if matches!(self.locals[index].kind, LocalKind::Local) {
                        let span = self.locals[index].span.or(statement.span);
                        resources.push((local, span));
                    }
                }
                if resources.is_empty() {
                    self.pop_scope();
                    self.push_pending(statement, PendingStatementKind::Using);
                    return;
                }
            }
        }

        self.schedule_defer_drops(&resources);

        if let Some(body) = &using.body {
            self.lower_statement(body.as_ref());

            let current_block = self.current_block;
            if self.blocks[current_block.0].terminator.is_none() {
                self.emit_storage_dead_for_resources(&resources);
            }
        } else {
            self.emit_storage_dead_for_resources(&resources);
        }

        self.mark_resources_dead(&resources);

        self.pop_scope();
    }

}
