use super::shared::{
    ForeachBindingInfo, ForeachBindingMode, ForeachLocals, defer_drop_for_place,
    ensure_enumerator_local, initialise_foreach_locals, parse_foreach_binding, plan_foreach_blocks,
    storage_dead_local, storage_live_local,
};
use super::*;
use crate::mir::layout::TypeLayout;
use crate::mir::{ConstOperand, ConstValue};

#[derive(Clone, Copy)]
enum IntrinsicForeachKind {
    Vec,
    Array,
    Span,
    ReadOnlySpan,
    String,
    Str,
}

body_builder_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Foreach lowering orchestrates binding, enumerator, and borrow semantics in one flow."
    )]
    pub(crate) fn lower_foreach_statement(
        &mut self,
        statement: &AstStatement,
        foreach: &crate::frontend::ast::ForeachStatement,
    ) {
        self.push_scope();

        let binding_span = foreach.binding_span.or(statement.span);
        let mut binding_info = match parse_foreach_binding(&foreach.binding, binding_span) {
            Ok(info) => info,
            Err(diag) => {
                self.diagnostics.push(diag);
                self.push_pending(statement, PendingStatementKind::Foreach);
                self.pop_scope();
                return;
            }
        };

        let mut locals = initialise_foreach_locals(
            self,
            &binding_info,
            binding_span,
            foreach.expression.span,
        );

        if let Some(parsed) = self.expression_node(&foreach.expression) {
            if let ExprNode::Range(range_expr) = parsed {
                let has_from_end = range_expr
                    .start
                    .as_ref()
                    .is_some_and(|endpoint| endpoint.from_end)
                    || range_expr
                        .end
                        .as_ref()
                        .is_some_and(|endpoint| endpoint.from_end);
                if has_from_end {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "foreach iteration over index-from-end ranges is unsupported without a container length"
                            .to_string(),
                        span: foreach.expression.span.or(statement.span),
                    });
                    self.push_pending(statement, PendingStatementKind::Foreach);
                    storage_dead_local(self, locals.sequence_local, foreach.expression.span);
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.bindings.remove(&locals.sequence_name);
                        scope.bindings.remove(&locals.enumerator_name);
                    }
                    self.pop_scope();
                    return;
                }
            }
        }

        let Some(sequence_operand) = self.lower_expression_operand(&foreach.expression) else {
            self.push_pending(statement, PendingStatementKind::Foreach);
            self.pop_scope();
            return;
        };

        let sequence_ty = self.operand_ty(&sequence_operand);
        self.push_statement(MirStatement {
            span: foreach.expression.span,
            kind: MirStatementKind::Assign {
                place: Place::new(locals.sequence_local),
                value: Rvalue::Use(sequence_operand),
            },
        });
        if let Some(seq_ty) = sequence_ty.as_ref() {
            self.update_binding_ty(&mut binding_info, seq_ty, locals.iter_local);
        }
        if let Some(seq_ty) = sequence_ty.as_ref() {
            if self.lower_foreach_range(statement, foreach, &mut binding_info, &mut locals, seq_ty) {
                return;
            }
        }
        if let Some(seq_ty) = sequence_ty.as_ref() {
            if let Some(local) = self.locals.get_mut(locals.sequence_local.0) {
                local.ty = seq_ty.clone();
                local.is_nullable = matches!(seq_ty, Ty::Nullable(_));
            }
            if let Some(kind) = self.intrinsic_foreach_kind(seq_ty) {
                if self.container_inline_allowed(seq_ty) {
                    self.lower_foreach_intrinsic(statement, foreach, &binding_info, &locals, kind);
                    self.pop_scope();
                    return;
                }
                if self.lower_foreach_intrinsic_enumerator(
                    statement,
                    foreach,
                    &binding_info,
                    &mut locals,
                ) {
                    self.pop_scope();
                    return;
                }
            }
        }

        let get_enumerator_text = format!("{}.GetEnumerator()", locals.sequence_name);
        let Some(enumerator_expr) =
            self.parse_synthetic_expression(&get_enumerator_text, foreach.expression.span)
        else {
            self.push_pending(statement, PendingStatementKind::Foreach);
            self.pop_scope();
            return;
        };

        let Some(enumerator_operand) =
            self.lower_expr_node(enumerator_expr, foreach.expression.span)
        else {
            self.push_pending(statement, PendingStatementKind::Foreach);
            self.pop_scope();
            return;
        };

        let enumerator_local =
            ensure_enumerator_local(self, &mut locals, foreach.expression.span);
        let enumerator_place = Place::new(enumerator_local);

        self.push_statement(MirStatement {
            span: foreach.expression.span,
            kind: MirStatementKind::Assign {
                place: enumerator_place.clone(),
                value: Rvalue::Use(enumerator_operand),
            },
        });
        storage_dead_local(self, locals.sequence_local, foreach.expression.span);
        defer_drop_for_place(self, enumerator_place.clone(), foreach.expression.span);

        let move_next_text = format!("{}.MoveNext()", locals.enumerator_name);
        let current_text = format!("{}.Current", locals.enumerator_name);

        let blocks = plan_foreach_blocks(
            self,
            foreach.expression.span,
            binding_span,
            foreach.body.span,
            statement.span,
        );

        self.ensure_goto(blocks.condition, statement.span);

        self.switch_to_block(blocks.condition);
        let Some(move_next_expr) =
            self.parse_synthetic_expression(&move_next_text, foreach.expression.span)
        else {
            self.push_pending(statement, PendingStatementKind::Foreach);
            self.pop_scope();
            return;
        };
        let Some(move_next_operand) =
            self.lower_expr_node(move_next_expr, foreach.expression.span)
        else {
            self.push_pending(statement, PendingStatementKind::Foreach);
            self.pop_scope();
            return;
        };
        self.set_terminator(
            foreach.expression.span,
            Terminator::SwitchInt {
                discr: move_next_operand,
                targets: vec![(1, blocks.prepare)],
                otherwise: blocks.exit,
            },
        );

        self.switch_to_block(blocks.prepare);
        storage_live_local(self, locals.iter_local, binding_span);

        match binding_info.mode {
            ForeachBindingMode::Value => {
                let Some(current_expr) =
                    self.parse_synthetic_expression(&current_text, binding_span)
                else {
                    self.push_pending(statement, PendingStatementKind::Foreach);
                    self.pop_scope();
                    return;
                };
                let Some(current_operand) = self.lower_expr_node(current_expr, binding_span) else {
                    self.push_pending(statement, PendingStatementKind::Foreach);
                    self.pop_scope();
                    return;
                };
                self.write_foreach_binding(
                    &binding_info,
                    &locals,
                    binding_span,
                    Some(current_operand),
                    None,
                );
            }
            ForeachBindingMode::In | ForeachBindingMode::RefReadonly | ForeachBindingMode::Ref => {
                let Some(current_expr) =
                    self.parse_synthetic_expression(&current_text, binding_span)
                else {
                    self.push_pending(statement, PendingStatementKind::Foreach);
                    self.pop_scope();
                    return;
                };
                let Some(place) = self.lower_place_expr(current_expr, binding_span) else {
                    self.push_pending(statement, PendingStatementKind::Foreach);
                    self.pop_scope();
                    return;
                };
                self.write_foreach_binding(&binding_info, &locals, binding_span, None, Some(place));
            }
        }

        self.ensure_goto(blocks.body, binding_span);

        self.switch_to_block(blocks.body);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(blocks.break_cleanup, blocks.cleanup, loop_scope_depth);
        self.lower_statement(foreach.body.as_ref());
        self.pop_loop();
        self.ensure_goto(blocks.cleanup, foreach.body.span);

        self.switch_to_block(blocks.cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.ensure_goto(blocks.condition, binding_span);

        self.switch_to_block(blocks.break_cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.ensure_goto(blocks.exit, binding_span);

        self.switch_to_block(blocks.exit);
        if let Some(enumerator_local) = locals.enumerator_local {
            storage_dead_local(self, enumerator_local, statement.span);
        }

        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.remove(&locals.sequence_name);
            scope.bindings.remove(&locals.enumerator_name);
        }

        self.pop_scope();
    }

    fn write_foreach_binding(
        &mut self,
        binding_info: &ForeachBindingInfo,
        locals: &ForeachLocals,
        binding_span: Option<Span>,
        value_operand: Option<Operand>,
        place: Option<Place>,
    ) {
        match binding_info.mode {
            ForeachBindingMode::Value => {
                let operand = value_operand.expect("value operand required for foreach binding");
                if let Some(ty) = self.operand_ty(&operand) {
                    self.hint_local_ty(locals.iter_local, ty);
                }
                self.push_statement(MirStatement {
                    span: binding_span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(locals.iter_local),
                        value: Rvalue::Use(operand),
                    },
                });
            }
            ForeachBindingMode::In
            | ForeachBindingMode::RefReadonly
            | ForeachBindingMode::Ref => {
                let place = place.expect("place required for reference foreach binding");
                let (borrow_id, region) = self.fresh_borrow();
                let borrow_kind = if matches!(binding_info.mode, ForeachBindingMode::Ref) {
                    BorrowKind::Unique
                } else {
                    BorrowKind::Shared
                };
                self.push_statement(MirStatement {
                    span: binding_span,
                    kind: MirStatementKind::Borrow {
                        borrow_id,
                        kind: borrow_kind,
                        place: place.clone(),
                        region,
                    },
                });
                let borrow_operand = Operand::Borrow(BorrowOperand {
                    kind: borrow_kind,
                    place,
                    region,
                    span: binding_span,
                });
                self.push_statement(MirStatement {
                    span: binding_span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(locals.iter_local),
                        value: Rvalue::Use(borrow_operand),
                    },
                });
            }
        }
    }

    fn update_binding_ty(
        &mut self,
        binding_info: &mut ForeachBindingInfo,
        sequence_ty: &Ty,
        iter_local: LocalId,
    ) {
        if !matches!(binding_info.ty, Ty::Unknown) {
            if let Some(local) = self.locals.get_mut(iter_local.0) {
                local.ty = binding_info.ty.clone();
                local.is_nullable = matches!(binding_info.ty, Ty::Nullable(_));
            }
            return;
        }
        if let Some(element_ty) = self.element_ty_from_sequence(sequence_ty) {
            binding_info.ty = element_ty.clone();
            if let Some(local) = self.locals.get_mut(iter_local.0) {
                local.ty = element_ty.clone();
                local.is_nullable = matches!(element_ty, Ty::Nullable(_));
            }
        }
    }

    fn element_ty_from_sequence(&self, ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Vec(vec_ty) => Some((*vec_ty.element).clone()),
            Ty::Array(array_ty) => Some((*array_ty.element).clone()),
            Ty::Span(span_ty) => Some((*span_ty.element).clone()),
            Ty::ReadOnlySpan(span_ty) => Some((*span_ty.element).clone()),
            Ty::String | Ty::Str => Some(Ty::named("char")),
            Ty::Nullable(inner) => self.element_ty_from_sequence(inner),
            Ty::Ref(reference) => self.element_ty_from_sequence(&reference.element),
            _ => None,
        }
    }

    fn intrinsic_foreach_kind(&self, ty: &Ty) -> Option<IntrinsicForeachKind> {
        match ty {
            Ty::Nullable(inner) => self.intrinsic_foreach_kind(inner),
            Ty::Ref(reference) => self.intrinsic_foreach_kind(&reference.element),
            Ty::Vec(_) => Some(IntrinsicForeachKind::Vec),
            Ty::Array(array_ty) => {
                if array_ty.rank == 1 {
                    Some(IntrinsicForeachKind::Array)
                } else {
                    None
                }
            }
            Ty::Span(_) => Some(IntrinsicForeachKind::Span),
            Ty::ReadOnlySpan(_) => Some(IntrinsicForeachKind::ReadOnlySpan),
            Ty::String => Some(IntrinsicForeachKind::String),
            Ty::Str => Some(IntrinsicForeachKind::Str),
            _ => None,
        }
    }

    fn container_inline_allowed(&mut self, ty: &Ty) -> bool {
        self.ensure_ty_layout_for_ty(ty);
        let canonical = ty.canonical_name();
        match self.type_layouts.layout_for_name(&canonical) {
            Some(TypeLayout::Struct(layout)) => {
                let allow_cross = crate::mir::layout::table::cross_inline_override(&canonical)
                    .unwrap_or(layout.allow_cross_inline);
                layout.is_intrinsic && self.intrinsic_inline_allowed(&canonical, allow_cross)
            }
            Some(TypeLayout::Class(layout)) => {
                let allow_cross = crate::mir::layout::table::cross_inline_override(&canonical)
                    .unwrap_or(layout.allow_cross_inline);
                layout.is_intrinsic && self.intrinsic_inline_allowed(&canonical, allow_cross)
            }
            _ => false,
        }
    }

    fn intrinsic_inline_allowed(&self, ty_name: &str, allow_cross_inline: bool) -> bool {
        if self.type_is_local(ty_name) {
            true
        } else {
            allow_cross_inline
        }
    }

    fn type_is_local(&self, ty_name: &str) -> bool {
        if self.type_visibilities.contains_key(ty_name) {
            return true;
        }
        if let Some(base) = ty_name.split('<').next() {
            if self.type_visibilities.contains_key(base) {
                return true;
            }
        }
        false
    }

    fn lower_foreach_intrinsic(
        &mut self,
        statement: &AstStatement,
        foreach: &crate::frontend::ast::ForeachStatement,
        binding_info: &ForeachBindingInfo,
        locals: &ForeachLocals,
        _kind: IntrinsicForeachKind,
    ) {
        let binding_span = foreach.binding_span.or(statement.span);
        let idx_name = format!("__foreach_idx_local_{}", locals.iter_local.0);
        let len_name = format!("__foreach_len_local_{}", locals.iter_local.0);
        let idx_local = self.push_local(LocalDecl::new(
            Some(idx_name),
            Ty::named("usize"),
            true,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, idx_local, binding_span);
        let len_local = self.push_local(LocalDecl::new(
            Some(len_name),
            Ty::named("usize"),
            false,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, len_local, binding_span);

        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::UInt(0)))),
            },
        });

        let mut sequence_place = Place::new(locals.sequence_local);
        self.normalise_place(&mut sequence_place);
        self.push_statement(MirStatement {
            span: foreach.expression.span,
            kind: MirStatementKind::Assign {
                place: Place::new(len_local),
                value: Rvalue::Len(sequence_place.clone()),
            },
        });

        let blocks = plan_foreach_blocks(
            self,
            foreach.expression.span,
            binding_span,
            foreach.body.span,
            statement.span,
        );

        self.ensure_goto(blocks.condition, statement.span);

        self.switch_to_block(blocks.condition);
        let cond_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_cond_local_{}", locals.iter_local.0)),
            Ty::named("bool"),
            false,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, cond_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(cond_local),
                value: Rvalue::Binary {
                    op: BinOp::Lt,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Copy(Place::new(len_local)),
                    rounding: None,
                },
            },
        });
        self.set_terminator(
            foreach.expression.span,
            Terminator::SwitchInt {
                discr: Operand::Copy(Place::new(cond_local)),
                targets: vec![(1, blocks.prepare)],
                otherwise: blocks.exit,
            },
        );

        self.switch_to_block(blocks.prepare);
        storage_live_local(self, locals.iter_local, binding_span);
        defer_drop_for_place(self, Place::new(locals.iter_local), binding_span);
        let mut element_place = Place::new(locals.sequence_local);
        element_place.projection.push(ProjectionElem::Index(idx_local));
        self.normalise_place(&mut element_place);
        self.write_foreach_binding(
            binding_info,
            locals,
            binding_span,
            Some(Operand::Copy(element_place.clone())),
            Some(element_place),
        );
        self.ensure_goto(blocks.body, binding_span);

        self.switch_to_block(blocks.body);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(blocks.break_cleanup, blocks.cleanup, loop_scope_depth);
        self.lower_statement(foreach.body.as_ref());
        self.pop_loop();
        self.ensure_goto(blocks.cleanup, foreach.body.span);

        self.switch_to_block(blocks.cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::UInt(1))),
                    rounding: None,
                },
            },
        });
        self.ensure_goto(blocks.condition, foreach.body.span);

        self.switch_to_block(blocks.break_cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.ensure_goto(blocks.exit, binding_span);

        self.switch_to_block(blocks.exit);
        storage_dead_local(self, idx_local, binding_span);
        storage_dead_local(self, len_local, binding_span);
        storage_dead_local(self, cond_local, binding_span);
        storage_dead_local(self, locals.sequence_local, statement.span);
        if let Some(enumerator_local) = locals.enumerator_local {
            storage_dead_local(self, enumerator_local, statement.span);
        }
    }

    fn lower_foreach_range(
        &mut self,
        statement: &AstStatement,
        foreach: &crate::frontend::ast::ForeachStatement,
        binding_info: &mut ForeachBindingInfo,
        locals: &mut ForeachLocals,
        sequence_ty: &Ty,
    ) -> bool {
        let Some(name) = self
            .resolve_ty_name(sequence_ty)
            .or_else(|| Some(sequence_ty.canonical_name()))
        else {
            return false;
        };
        let inclusive = match name.as_str() {
            "Std::Range::Range" => false,
            "Std::Range::RangeInclusive" => true,
            "Std::Range::RangeFrom" | "Std::Range::RangeTo" | "Std::Range::RangeFull" => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "foreach iteration over open-ended {name} requires a concrete end bound"
                    ),
                    span: foreach.expression.span.or(statement.span),
                });
                self.push_pending(statement, PendingStatementKind::Foreach);
                storage_dead_local(self, locals.sequence_local, statement.span);
                if let Some(scope) = self.scopes.last_mut() {
                    scope.bindings.remove(&locals.sequence_name);
                    scope.bindings.remove(&locals.enumerator_name);
                }
                self.pop_scope();
                return true;
            }
            _ => return false,
        };

        binding_info.ty = Ty::named("usize");
        if let Some(local) = self.locals.get_mut(locals.iter_local.0) {
            local.ty = Ty::named("usize");
            local.is_nullable = false;
        }

        let binding_span = foreach.binding_span.or(statement.span);
        let mut sequence_place = Place::new(locals.sequence_local);
        self.normalise_place(&mut sequence_place);

        let mut start_place = sequence_place.clone();
        start_place
            .projection
            .push(ProjectionElem::FieldNamed("Start".into()));
        self.normalise_place(&mut start_place);

        let mut end_place = sequence_place.clone();
        end_place
            .projection
            .push(ProjectionElem::FieldNamed("End".into()));
        self.normalise_place(&mut end_place);

        let mut start_value = start_place.clone();
        start_value
            .projection
            .push(ProjectionElem::FieldNamed("Value".into()));
        self.normalise_place(&mut start_value);
        let start_local =
            self.ensure_operand_local(Operand::Copy(start_value.clone()), foreach.expression.span);
        self.hint_local_ty(start_local, Ty::named("usize"));

        let mut end_value = end_place.clone();
        end_value
            .projection
            .push(ProjectionElem::FieldNamed("Value".into()));
        self.normalise_place(&mut end_value);
        let end_local =
            self.ensure_operand_local(Operand::Copy(end_value.clone()), foreach.expression.span);
        self.hint_local_ty(end_local, Ty::named("usize"));

        let idx_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_idx_local_{}", locals.iter_local.0)),
            Ty::named("usize"),
            true,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, idx_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Use(Operand::Copy(Place::new(start_local))),
            },
        });

        let limit_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_limit_local_{}", locals.iter_local.0)),
            Ty::named("usize"),
            true,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, limit_local, binding_span);
        let limit_value = if inclusive {
            let temp = self.create_temp(foreach.expression.span);
            if let Some(local) = self.locals.get_mut(temp.0) {
                local.ty = Ty::named("usize");
                local.is_nullable = false;
            }
            self.push_statement(MirStatement {
                span: foreach.expression.span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Binary {
                        op: BinOp::Add,
                        lhs: Operand::Copy(Place::new(end_local)),
                        rhs: Operand::Const(ConstOperand::new(ConstValue::UInt(1))),
                        rounding: None,
                    },
                },
            });
            Operand::Copy(Place::new(temp))
        } else {
            Operand::Copy(Place::new(end_local))
        };
        self.push_statement(MirStatement {
            span: foreach.expression.span,
            kind: MirStatementKind::Assign {
                place: Place::new(limit_local),
                value: Rvalue::Use(limit_value),
            },
        });

        let blocks = plan_foreach_blocks(
            self,
            foreach.expression.span,
            binding_span,
            foreach.body.span,
            statement.span,
        );

        self.ensure_goto(blocks.condition, statement.span);
        self.switch_to_block(blocks.condition);
        let cond_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_cond_local_{}", locals.iter_local.0)),
            Ty::named("bool"),
            false,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, cond_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(cond_local),
                value: Rvalue::Binary {
                    op: BinOp::Lt,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Copy(Place::new(limit_local)),
                    rounding: None,
                },
            },
        });
        self.set_terminator(
            foreach.expression.span,
            Terminator::SwitchInt {
                discr: Operand::Copy(Place::new(cond_local)),
                targets: vec![(1, blocks.prepare)],
                otherwise: blocks.exit,
            },
        );

        self.switch_to_block(blocks.prepare);
        storage_live_local(self, locals.iter_local, binding_span);
        defer_drop_for_place(self, Place::new(locals.iter_local), binding_span);
        match binding_info.mode {
            ForeachBindingMode::Value => {
                self.write_foreach_binding(
                    binding_info,
                    locals,
                    binding_span,
                    Some(Operand::Copy(Place::new(idx_local))),
                    None,
                );
            }
            _ => {
                self.write_foreach_binding(
                    binding_info,
                    locals,
                    binding_span,
                    None,
                    Some(Place::new(idx_local)),
                );
            }
        }
        self.ensure_goto(blocks.body, binding_span);

        self.switch_to_block(blocks.body);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(blocks.break_cleanup, blocks.cleanup, loop_scope_depth);
        self.lower_statement(foreach.body.as_ref());
        self.pop_loop();
        self.ensure_goto(blocks.cleanup, foreach.body.span);

        self.switch_to_block(blocks.cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::UInt(1))),
                    rounding: None,
                },
            },
        });
        self.ensure_goto(blocks.condition, foreach.body.span);

        self.switch_to_block(blocks.break_cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.ensure_goto(blocks.exit, binding_span);

        self.switch_to_block(blocks.exit);
        storage_dead_local(self, idx_local, binding_span);
        storage_dead_local(self, limit_local, binding_span);
        storage_dead_local(self, cond_local, binding_span);
        storage_dead_local(self, locals.sequence_local, statement.span);

        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.remove(&locals.sequence_name);
            scope.bindings.remove(&locals.enumerator_name);
        }
        self.pop_scope();
        true
    }

    fn lower_foreach_intrinsic_enumerator(
        &mut self,
        statement: &AstStatement,
        foreach: &crate::frontend::ast::ForeachStatement,
        binding_info: &ForeachBindingInfo,
        locals: &mut ForeachLocals,
    ) -> bool {
        let binding_span = foreach.binding_span.or(statement.span);
        let enumerator_local =
            ensure_enumerator_local(self, locals, foreach.expression.span);

        let idx_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_enum_idx_{}", locals.iter_local.0)),
            Ty::named("usize"),
            true,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, idx_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::UInt(0)))),
            },
        });

        let len_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_enum_len_{}", locals.iter_local.0)),
            Ty::named("usize"),
            false,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, len_local, binding_span);

        let mut sequence_place = Place::new(locals.sequence_local);
        self.normalise_place(&mut sequence_place);
        self.push_statement(MirStatement {
            span: foreach.expression.span,
            kind: MirStatementKind::Assign {
                place: Place::new(len_local),
                value: Rvalue::Len(sequence_place.clone()),
            },
        });

        let cond_local = self.push_local(LocalDecl::new(
            Some(format!("__foreach_enum_cond_{}", locals.iter_local.0)),
            Ty::named("bool"),
            false,
            binding_span,
            LocalKind::Local,
        ));
        storage_live_local(self, cond_local, binding_span);

        let blocks = plan_foreach_blocks(
            self,
            foreach.expression.span,
            binding_span,
            foreach.body.span,
            statement.span,
        );

        self.ensure_goto(blocks.condition, statement.span);

        self.switch_to_block(blocks.condition);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(cond_local),
                value: Rvalue::Binary {
                    op: BinOp::Lt,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Copy(Place::new(len_local)),
                    rounding: None,
                },
            },
        });
        self.set_terminator(
            foreach.expression.span,
            Terminator::SwitchInt {
                discr: Operand::Copy(Place::new(cond_local)),
                targets: vec![(1, blocks.prepare)],
                otherwise: blocks.exit,
            },
        );

        self.switch_to_block(blocks.prepare);
        storage_live_local(self, locals.iter_local, binding_span);
        defer_drop_for_place(self, Place::new(locals.iter_local), binding_span);
        let mut element_place = Place::new(locals.sequence_local);
        element_place.projection.push(ProjectionElem::Index(idx_local));
        self.normalise_place(&mut element_place);
        self.write_foreach_binding(
            binding_info,
            locals,
            binding_span,
            Some(Operand::Copy(element_place.clone())),
            Some(element_place),
        );
        self.ensure_goto(blocks.body, binding_span);

        self.switch_to_block(blocks.body);
        let loop_scope_depth = self.scope_depth();
        self.push_loop(blocks.break_cleanup, blocks.cleanup, loop_scope_depth);
        self.lower_statement(foreach.body.as_ref());
        self.pop_loop();
        self.ensure_goto(blocks.cleanup, foreach.body.span);

        self.switch_to_block(blocks.cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.push_statement(MirStatement {
            span: binding_span,
            kind: MirStatementKind::Assign {
                place: Place::new(idx_local),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(Place::new(idx_local)),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::UInt(1))),
                    rounding: None,
                },
            },
        });
        self.ensure_goto(blocks.condition, foreach.body.span);

        self.switch_to_block(blocks.break_cleanup);
        storage_dead_local(self, locals.iter_local, binding_span);
        self.ensure_goto(blocks.exit, binding_span);

        self.switch_to_block(blocks.exit);
        storage_dead_local(self, idx_local, binding_span);
        storage_dead_local(self, len_local, binding_span);
        storage_dead_local(self, cond_local, binding_span);
        storage_dead_local(self, locals.sequence_local, statement.span);
        storage_dead_local(self, enumerator_local, statement.span);
        true
    }
}
