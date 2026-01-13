use super::*;

body_builder_impl! {

    pub(super) fn lower_lock_statement(
        &mut self,
        statement: &AstStatement,
        expression: &Expression,
        body: &AstStatement,
    ) {
        self.push_scope();

        let lock_span = expression.span.or(statement.span);
        let mut resources: Vec<(LocalId, Option<Span>)> = Vec::new();

        let Some(lock_operand) = self.lower_expression_operand(expression) else {
            self.push_pending(statement, PendingStatementKind::Lock);
            self.pop_scope();
            return;
        };

        let lock_id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock_name = format!("__lock_target_{lock_id}");
        let lock_local = self.register_resource_decl(LocalDecl::new(
            Some(lock_name.clone()),
            Ty::Unknown,
            true,
            lock_span,
            LocalKind::Local,
        ));
        self.bind_name(&lock_name, lock_local);

        self.push_statement(MirStatement {
            span: lock_span,
            kind: MirStatementKind::Assign {
                place: Place::new(lock_local),
                value: Rvalue::Use(lock_operand.clone()),
            },
        });
        if let Some(ty) = self.operand_ty(&lock_operand) {
            self.hint_local_ty(lock_local, ty);
        }
        resources.push((lock_local, lock_span));

        let lock_ty = self.operand_ty(&Operand::Copy(Place::new(lock_local)));
        if let Some(ty) = lock_ty.as_ref().filter(|ty| !matches!(ty, Ty::Unknown)) {
            let lockable = match ty {
                Ty::Named(named) => {
                    let canonical = named.canonical_path();
                    let stripped = canonical.split('<').next().unwrap_or(&canonical);
                    stripped.eq_ignore_ascii_case("Std::Sync::Lock")
                        || stripped.eq_ignore_ascii_case("std::sync::Lock")
                }
                Ty::Nullable(inner) => matches!(**inner, Ty::Unknown) || {
                    let name = inner.canonical_name();
                    let stripped = name.split('<').next().unwrap_or(&name);
                    stripped.eq_ignore_ascii_case("Std::Sync::Lock")
                        || stripped.eq_ignore_ascii_case("std::sync::Lock")
                },
                Ty::Ref(reference) => {
                    let name = reference.element.canonical_name();
                    let stripped = name.split('<').next().unwrap_or(&name);
                    stripped.eq_ignore_ascii_case("Std::Sync::Lock")
                        || stripped.eq_ignore_ascii_case("std::sync::Lock")
                }
                _ => false,
            };
            if !lockable {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "[LOCK0001] cannot apply `lock` to expression of type `{}`; expected Std.Sync.Lock or compatible type",
                        ty.canonical_name()
                    ),
                    span: lock_span,
                });
                self.emit_storage_dead_for_resources(&resources);
                self.mark_resources_dead(&resources);
                self.pop_scope();
                return;
            }
        }

        let callee = ExprNode::Member {
            base: Box::new(ExprNode::Identifier(lock_name)),
            member: "Enter".to_string(),
            null_conditional: false,
        };
        let Some(guard_operand) =
            self.lower_call(callee, Vec::new(), None, expression.span, true)
        else {
            self.push_pending(statement, PendingStatementKind::Lock);
            self.emit_storage_dead_for_resources(&resources);
            self.mark_resources_dead(&resources);
            self.pop_scope();
            return;
        };

        let guard_local = self.ensure_operand_local(guard_operand, expression.span);
        self.schedule_defer_drop(guard_local, expression.span);
        resources.push((guard_local, expression.span));

        self.lock_depth += 1;
        self.lower_statement(body);
        self.lock_depth -= 1;

        if self.blocks[self.current_block.0].terminator.is_none() {
            self.emit_storage_dead_for_resources(&resources);
        }
        self.mark_resources_dead(&resources);

        self.pop_scope();
    }

}
