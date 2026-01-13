use super::*;

body_builder_impl! {

    #[expect(
        clippy::too_many_lines,
        reason = "Fixed statement lowering manages pinning, locals, and diagnostics cohesively."
    )]
    pub(super) fn lower_fixed_statement(
        &mut self,
        statement: &AstStatement,
        fixed: &crate::frontend::ast::FixedStatement,
    ) {
        self.push_scope();

        if fixed.declaration.declarators.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "fixed statement requires at least one binding".into(),
                span: statement.span,
                            });
            self.push_pending(statement, PendingStatementKind::Fixed);
            self.pop_scope();
            return;
        }

        let base_ty = fixed
            .declaration
            .type_annotation
            .as_ref()
            .map_or(Ty::Unknown, |expr| {
                let ty = Ty::from_type_expr(expr);
                self.ensure_ty_layout_for_ty(&ty);
                ty
            });
        let mutable = !matches!(
            fixed.declaration.modifier,
            VariableModifier::Let
        );

        let fixed_id = self.next_fixed_id;
        self.next_fixed_id += 1;

        let mut pointer_locals: Vec<(LocalId, Option<Span>)> = Vec::new();
        let mut guard_locals: Vec<(LocalId, Option<Span>)> = Vec::new();

        for (index, declarator) in fixed.declaration.declarators.iter().enumerate() {
            let Some(initializer) = &declarator.initializer else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "fixed binding `{}` requires an initializer",
                        declarator.name
                    ),
                    span: statement.span,
                                    });
                self.push_pending(statement, PendingStatementKind::Fixed);
                self.pop_scope();
                return;
            };

            let pointer_span = initializer.span.or(statement.span);
            let pointer_local = self.register_resource_decl(LocalDecl::new(
                Some(declarator.name.clone()),
                base_ty.clone(),
                mutable,
                pointer_span,
                LocalKind::Local,
            ));
            self.bind_name(&declarator.name, pointer_local);
            pointer_locals.push((pointer_local, pointer_span));

            let guard_name = format!("__fixed_guard_{fixed_id}_{index}");
            let mut guard_decl = LocalDecl::new(
                Some(guard_name),
                Ty::Unknown,
                true,
                initializer.span,
                LocalKind::Local,
            );
            guard_decl.is_pinned = true;
            let guard_local = self.register_resource_decl(guard_decl);
            guard_locals.push((guard_local, initializer.span));

            let Some(parsed) = self.expression_node(initializer) else {
                self.push_pending(statement, PendingStatementKind::Fixed);
                self.pop_scope();
                return;
            };

            let place = match parsed {
                ExprNode::Cast { expr, .. } => self.lower_place_expr(*expr, initializer.span),
                other => self.lower_place_expr(other, initializer.span),
            };

            let Some(place) = place else {
                self.push_pending(statement, PendingStatementKind::Fixed);
                self.pop_scope();
                return;
            };

            let (borrow_id, region) = self.fresh_borrow();
            self.push_statement(MirStatement {
                span: initializer.span,
                                kind: MirStatementKind::Borrow {
                    borrow_id,
                    kind: BorrowKind::Unique,
                    place: place.clone(),
                    region,
                },
            });

            let borrow_operand = Operand::Borrow(BorrowOperand {
                kind: BorrowKind::Unique,
                place: place.clone(),
                region,
                span: initializer.span,
                            });
            self.push_statement(MirStatement {
                span: initializer.span,
                                kind: MirStatementKind::Assign {
                    place: Place::new(guard_local),
                    value: Rvalue::Use(borrow_operand),
                },
            });

            self.schedule_defer_drop(guard_local, initializer.span);

            self.push_statement(MirStatement {
                span: initializer.span,
                                kind: MirStatementKind::Assign {
                    place: Place::new(pointer_local),
                    value: Rvalue::AddressOf {
                        mutability: Mutability::Mutable,
                        place,
                    },
                },
            });
        }

        self.lower_statement(fixed.body.as_ref());

        let block_has_no_term = self.blocks[self.current_block.0].terminator.is_none();

        if block_has_no_term {
            self.emit_storage_dead_for_resources(&pointer_locals);
            self.mark_resources_dead(&pointer_locals);
            self.emit_storage_dead_for_resources(&guard_locals);
            self.mark_resources_dead(&guard_locals);
        } else {
            self.mark_resources_dead(&pointer_locals);
            self.mark_resources_dead(&guard_locals);
        }

        self.pop_scope();
    }

}
