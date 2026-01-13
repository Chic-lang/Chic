use super::*;
use crate::mir::ConstEvalContext;

body_builder_impl! {
    pub(crate) fn lower_block(&mut self, block: &Block) {
        self.push_scope();
        self.predeclare_block_local_functions(block);
        for statement in &block.statements {
            self.lower_statement(statement);
        }
        self.pop_scope();
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Statement lowering must consider all Chic control-flow forms in one place."
    )]
    pub(crate) fn lower_statement(&mut self, statement: &AstStatement) {
        if self.try_lower_resource_statement(statement) {
            return;
        }

        match &statement.kind {
            AstStatementKind::Empty => {}
            AstStatementKind::Block(block) => self.lower_block(block),
            AstStatementKind::VariableDeclaration(decl) => {
                self.lower_variable_declaration(statement.span, decl);
            }
            AstStatementKind::ConstDeclaration(const_stmt) => {
                self.lower_const_statement(statement, const_stmt);
            }
            AstStatementKind::Expression(expr) => {
                if !self.lower_expression_statement(expr) {
                    let pending = PendingRvalue {
                        repr: expr.text.clone(),
                        span: expr.span,
                                            };
                    let stmt = MirStatement {
                        span: statement.span,
                                                kind: MirStatementKind::Eval(pending),
                    };
                    self.push_statement(stmt);
                }
            }
            AstStatementKind::Return { expression } => {
                self.lower_return_statement(statement, expression.as_ref());
            }
            AstStatementKind::Break => self.lower_break_statement(statement),
            AstStatementKind::Continue => self.lower_continue_statement(statement),
            AstStatementKind::Goto(goto) => self.lower_goto_statement(statement, goto),
            AstStatementKind::Throw { expression } => {
                self.lower_throw_statement(statement, expression.as_ref());
            }
            AstStatementKind::If(if_stmt) => self.lower_if_statement(statement, if_stmt),
            AstStatementKind::While { condition, body } => {
                self.lower_while_statement(statement, condition, body.as_ref());
            }
            AstStatementKind::DoWhile { body, condition } => {
                self.lower_do_while_statement(statement, body.as_ref(), condition);
            }
            AstStatementKind::For(for_stmt) => {
                self.lower_for_statement(statement, for_stmt);
            }
            AstStatementKind::Foreach(foreach) => {
                self.lower_foreach_statement(statement, foreach);
            }
            AstStatementKind::Switch(switch_stmt) => {
                self.lower_switch_statement(statement, switch_stmt);
            }
            AstStatementKind::Try(try_stmt) => {
                self.lower_try_statement(statement, try_stmt);
            }
            AstStatementKind::Region { name, body } => {
                self.lower_region_statement(statement, name, body);
            }
            AstStatementKind::Checked { .. } => self.lower_checked_statement(statement),
            AstStatementKind::Atomic { .. } => self.lower_atomic_statement(statement),
            AstStatementKind::Unchecked { .. } => self.lower_unchecked_statement(statement),
            AstStatementKind::YieldReturn { expression } => {
                self.lower_yield_return(statement, expression);
            }
            AstStatementKind::YieldBreak => self.lower_yield_break(statement),
            AstStatementKind::Using(_)
            | AstStatementKind::Lock { .. }
            | AstStatementKind::Fixed(_) => unreachable!(
                "resource statements handled by try_lower_resource_statement"
            ),
            AstStatementKind::Unsafe { body } => {
                self.lower_unsafe_statement(statement, body.as_ref());
            }
            AstStatementKind::Labeled {
                label,
                statement: inner,
            } => self.lower_labeled_statement(statement.span, label, inner.as_ref()),
            AstStatementKind::LocalFunction(_) => self.lower_local_function_statement(statement),
        }
    }

    pub(super) fn lower_const_statement(
        &mut self,
        statement: &AstStatement,
        const_stmt: &crate::frontend::ast::ConstStatement,
    ) {
        let mut env_consts = self.const_environment();
        let ty = Ty::from_type_expr(&const_stmt.declaration.ty);

        for declarator in &const_stmt.declaration.declarators {
            let expr_span = declarator.initializer.span.or(declarator.span).or(statement.span);
            let eval_result = {
                let mut eval_index = self.symbol_index.clone();
                let mut eval_ctx =
                    ConstEvalContext::new(&mut eval_index, self.type_layouts, Some(self.import_resolver));
                eval_ctx.evaluate_expression(
                    &declarator.initializer,
                    self.namespace.as_deref(),
                    self.namespace.as_deref(),
                    Some(&env_consts),
                    None,
                    &ty,
                    expr_span,
                )
            };
            match eval_result {
                Ok(result) => {
                    env_consts.insert(declarator.name.clone(), result.value.clone());
                    self.bind_const(&declarator.name, result.value);
                }
                Err(err) => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: err.message,
                        span: err.span.or(expr_span),
                                            });
                }
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Variable declaration lowering needs to capture all Chic binding forms cohesively."
    )]
    pub(super) fn lower_variable_declaration(
        &mut self,
        span: Option<Span>,
                decl: &VariableDeclaration,
    ) {
        let mutable = matches!(decl.modifier, VariableModifier::Var);
        let ty = decl
            .type_annotation
            .as_ref()
            .map_or(Ty::Unknown, |expr| {
                let ty = Ty::from_type_expr(expr);
                self.ensure_ty_layout_for_ty(&ty);
                ty
            });
        let pinned_by_type = decl
            .type_annotation
            .as_ref()
            .is_some_and(|ty| is_pin_type_name(&ty.name));

        for declarator in &decl.declarators {
            let mut local_decl = LocalDecl::new(
                Some(declarator.name.clone()),
                ty.clone(),
                mutable,
                span,
                                LocalKind::Local,
            );
            if decl
                .type_annotation
                .as_ref()
                .is_some_and(|ty| ty.is_nullable())
            {
                local_decl.is_nullable = true;
            }
            if decl.is_pinned || pinned_by_type {
                local_decl.is_pinned = true;
            }
            let local_id = self.push_local(local_decl);
            self.bind_name(&declarator.name, local_id);

            let live_stmt = MirStatement {
                span,
                                kind: MirStatementKind::StorageLive(local_id),
            };
            self.push_statement(live_stmt);
            self.record_local(local_id, span);

            if let Some(initializer) = &declarator.initializer {
                let assign_span = initializer.span.or(span);
                if let Some(mut operand) = self.lower_expression_operand(initializer) {
                    let mut handled_source: Option<LocalId> = None;
                    if declarator.name == "_" {
                        if let Operand::Copy(place) | Operand::Move(place) = &operand {
                            if place.projection.is_empty() {
                                handled_source = Some(place.local);
                            }
                        }
                    }
                    let mut inferred_ty: Option<Ty> = None;
                    let mut declared_ty = self.locals[local_id.0].ty.clone();
                    let inferred_nullable = self.operand_is_nullable(&operand);
                    if matches!(declared_ty, Ty::Unknown) {
                        if let Some(op_ty) = self.operand_ty(&operand) {
                            if !matches!(op_ty, Ty::Unknown) {
                                inferred_ty = Some(op_ty.clone());
                                if let Some(local) = self.locals.get_mut(local_id.0) {
                                    local.is_nullable =
                                        inferred_nullable.unwrap_or(matches!(op_ty, Ty::Nullable(_)));
                                    local.ty = op_ty.clone();
                                }
                                declared_ty = op_ty;
                            }
                        }
                    }
                    if matches!(declared_ty, Ty::Unknown) {
                        if let Some(fn_ty) = self.operand_fn_ty(&operand) {
                            declared_ty = Ty::Fn(fn_ty.clone());
                            if let Some(local) = self.locals.get_mut(local_id.0) {
                                local.ty = Ty::Fn(fn_ty);
                            }
                        } else if let Some(type_name) = self.operand_type_name(&operand) {
                            if self.closure_registry.contains_key(&type_name) {
                                declared_ty = Ty::named(type_name.clone());
                                if let Some(local) = self.locals.get_mut(local_id.0) {
                                    local.ty = Ty::named(type_name);
                                }
                            }
                        }
                    }
                    if !matches!(declared_ty, Ty::Unknown) {
                        operand =
                            self.coerce_operand_to_ty(operand, &declared_ty, false, assign_span);
                    }
                    operand = self.maybe_move_operand_for_value_assignment(operand);
                    if std::env::var("CHIC_DEBUG_PLACE_TYPES").is_ok()
                        && self.function_name.contains("DateTimeFormatting")
                    {
                        let current_ty = self.locals[local_id.0].ty.canonical_name();
                        let inferred = inferred_ty
                            .as_ref()
                            .map(|ty| ty.canonical_name())
                            .unwrap_or_else(|| "<none>".to_string());
                        eprintln!(
                            "[chic-debug] decl `{}` in {} -> ty={} (inferred={}) init={}",
                            declarator.name,
                            self.function_name,
                            current_ty,
                            inferred,
                            initializer.text
                        );
                    }
                    let assign = MirStatement {
                        span: assign_span,
                                                kind: MirStatementKind::Assign {
                            place: Place::new(local_id),
                            value: Rvalue::Use(operand),
                        },
                    };
                    self.push_statement(assign);
                    if declarator.name == "_" {
                        self.mark_fallible_handled(local_id, assign_span);
                        if let Some(source_local) = handled_source {
                            self.mark_fallible_handled(source_local, assign_span);
                        }
                    }
                } else {
                    let pending = PendingRvalue {
                        repr: initializer.text.clone(),
                        span: initializer.span,
                                            };
                    let assign = MirStatement {
                        span: assign_span,
                                                kind: MirStatementKind::Assign {
                            place: Place::new(local_id),
                            value: Rvalue::Pending(pending),
                        },
                    };
                    self.push_statement(assign);
                    if declarator.name == "_" {
                        self.mark_fallible_handled(local_id, assign_span);
                    }
                }
            }
        }
    }

    pub(super) fn lower_if_statement(
        &mut self,
        statement: &AstStatement,
        if_stmt: &crate::frontend::ast::IfStatement,
    ) {
        self.validate_required_initializer(&if_stmt.condition);
        let Some(cond_node) = self.expression_node(&if_stmt.condition) else {
            let pending = PendingStatement {
                kind: PendingStatementKind::If,
                detail: Some(format!(
                    "If pending lowering (condition=`{}`)",
                    if_stmt.condition.text
                )),
            };
            self.push_statement(MirStatement {
                span: statement.span,
                kind: MirStatementKind::Pending(pending),
            });
            return;
        };
        let then_block = self.new_block(if_stmt.then_branch.span);
        let else_block = if_stmt
            .else_branch
            .as_ref()
            .map(|else_branch| self.new_block(else_branch.span));
        let join_block = self.new_block(statement.span);
        let else_target = else_block.unwrap_or(join_block);

        if !self.lower_boolean_branch(cond_node, then_block, else_target, if_stmt.condition.span) {
            let pending = PendingStatement {
                kind: PendingStatementKind::If,
                detail: Some(format!(
                    "If pending lowering (condition=`{}`)",
                    if_stmt.condition.text
                )),
            };
            self.push_statement(MirStatement {
                span: statement.span,
                kind: MirStatementKind::Pending(pending),
            });
            return;
        }

        self.switch_to_block(then_block);
        self.lower_statement(if_stmt.then_branch.as_ref());
        self.ensure_goto(join_block, statement.span);

        if let Some((else_branch, else_block_id)) = if_stmt.else_branch.as_ref().zip(else_block) {
            self.switch_to_block(else_block_id);
            self.lower_statement(else_branch.as_ref());
            self.ensure_goto(join_block, statement.span);
        }

        self.switch_to_block(join_block);
    }

    pub(super) fn lower_unsafe_statement(&mut self, statement: &AstStatement, body: &AstStatement) {
        self.push_statement(MirStatement {
            span: statement.span,
                        kind: MirStatementKind::EnterUnsafe,
        });

        self.unsafe_depth += 1;
        self.lower_statement(body);
        self.unsafe_depth -= 1;

        let exit_block = self.new_block(statement.span);
        let current = self.current_block;
        let terminated = self.blocks[current.0].terminator.is_some();
        if !terminated {
            self.blocks[current.0].terminator = Some(Terminator::Goto { target: exit_block });
            if self.blocks[current.0].span.is_none() {
                self.blocks[current.0].span = statement.span;
            }
        }
        self.switch_to_block(exit_block);
        if !terminated {
            self.push_statement(MirStatement {
                span: statement.span,
                kind: MirStatementKind::ExitUnsafe,
            });
        }
    }

}
