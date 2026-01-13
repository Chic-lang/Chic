use super::*;
use crate::mir::AggregateKind;
use crate::mir::data::ReadOnlySpanTy;
use std::collections::HashSet;

body_builder_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Switch-as-match lowering must manage guards, bindings, and fallbacks coherently."
    )]
    pub(crate) fn lower_switch_as_match(
        &mut self,
        discr_local: LocalId,
        cases: &[SwitchCase],
        fallback_block: BlockId,
        switch_span: Option<Span>,
    ) {
        if cases.is_empty() {
            self.ensure_goto(fallback_block, switch_span);
            return;
        }

        let mut continuations = vec![fallback_block; cases.len() + 1];
        continuations[cases.len()] = fallback_block;

        for (index, case) in cases.iter().enumerate().rev() {
            let match_block = self.new_block(case.span);
            continuations[index] = match_block;
            let rest_block = continuations[index + 1];

            let binding_block = self.new_block(case.span.or(switch_span));
            let post_guard_entry =
                self.lower_guard_chain(&case.guards, case.body_block, rest_block, case.span);
            self.emit_binding_assignments(binding_block, case, discr_local, post_guard_entry);
            let pre_guard_entry =
                self.lower_guard_chain(&case.pre_guards, binding_block, rest_block, case.span);

            let pattern = match &case.pattern {
                CasePatternKind::Wildcard => Pattern::Wildcard,
                CasePatternKind::Literal(value) => Pattern::Literal(value.clone()),
                CasePatternKind::Complex(pattern) => pattern.clone(),
            };

            let arm = MatchArm {
                pattern,
                guard: Self::aggregate_guard_metadata(
                    &case
                        .pre_guards
                        .iter()
                        .chain(case.guards.iter())
                        .cloned()
                        .collect::<Vec<_>>(),
                ),
                bindings: case.bindings.clone(),
                target: pre_guard_entry,
            };

            let prev = self.current_block;
            self.current_block = match_block;
            self.set_terminator(
                case.span.or(switch_span),
                Terminator::Match {
                    value: Place::new(discr_local),
                    arms: vec![arm],
                    otherwise: rest_block,
                },
            );
            self.current_block = prev;
        }

        self.set_terminator(
            switch_span,
            Terminator::Goto {
                target: continuations[0],
            },
        );

        self.switch_to_block(fallback_block);
    }

    fn emit_binding_assignments(
        &mut self,
        block: BlockId,
        case: &SwitchCase,
        discr_local: LocalId,
        next_block: BlockId,
    ) {
        let prev = self.current_block;
        self.current_block = block;

        if let Some(plan) = &case.list_plan {
            let len_span = plan.span.or(case.span);
            let len_place = Place::new(plan.length_local);
            if let Some(local) = self.locals.get_mut(plan.length_local.0) {
                local.ty = Ty::named("usize");
                local.is_nullable = false;
            }
            let len_assign = MirStatement {
                span: len_span,
                kind: MirStatementKind::Assign {
                    place: len_place.clone(),
                    value: Rvalue::Len(Place::new(discr_local)),
                },
            };
            self.push_statement(len_assign);

            for index in &plan.indices {
                let value = if index.from_end {
                    Rvalue::Binary {
                        op: BinOp::Sub,
                        lhs: Operand::Copy(len_place.clone()),
                        rhs: Operand::Const(ConstOperand::new(ConstValue::Int(
                            i128::try_from(index.offset).unwrap_or(0),
                        ))),
                        rounding: None,
                    }
                } else {
                    Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(
                        i128::try_from(index.offset).unwrap_or(0),
                    ))))
                };
                let assign = MirStatement {
                    span: len_span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(index.local),
                        value,
                    },
                };
                self.push_statement(assign);
                if let Some(local) = self.locals.get_mut(index.local.0) {
                    local.ty = Ty::named("usize");
                    local.is_nullable = false;
                }
            }
        }

        let mut initialized = HashSet::new();
        for binding in &case.bindings {
            if initialized.insert(binding.local) {
                let live = MirStatement {
                    span: binding.span.or(case.span),
                                        kind: MirStatementKind::StorageLive(binding.local),
                };
                self.push_statement(live);
                self.record_local(binding.local, binding.span.or(case.span));
            }
            if self.lower_subslice_binding(binding, discr_local, case) {
                continue;
            }

            let source = self.build_binding_place(discr_local, binding);
            let span = binding.span.or(case.span);
            let operand = match binding.mode {
                PatternBindingMode::Value => Operand::Copy(source.clone()),
                PatternBindingMode::Move => Operand::Move(source.clone()),
                PatternBindingMode::In | PatternBindingMode::RefReadonly => {
                    self.borrow_argument_place(source.clone(), BorrowKind::Shared, span)
                }
                PatternBindingMode::Ref => {
                    self.borrow_argument_place(source.clone(), BorrowKind::Unique, span)
                }
            };
            let assign = MirStatement {
                span,
                                kind: MirStatementKind::Assign {
                    place: Place::new(binding.local),
                    value: Rvalue::Use(operand),
                },
            };
            self.push_statement(assign);
        }

        self.set_terminator(case.span, Terminator::Goto { target: next_block });

        self.current_block = prev;
    }

    fn lower_subslice_binding(
        &mut self,
        binding: &PatternBinding,
        discr_local: LocalId,
        case: &SwitchCase,
    ) -> bool {
        let (position, (from, to)) = match binding
            .projection
            .iter()
            .enumerate()
            .find_map(|(idx, proj)| match proj {
                PatternProjectionElem::Subslice { from, to } => Some((idx, (*from, *to))),
                _ => None,
            }) {
            Some(data) => data,
            None => return false,
        };

        let Some(plan) = case.list_plan.as_ref() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "internal error: missing list destructure plan for slice binding".into(),
                span: binding.span.or(case.span),
            });
            return false;
        };

        let mut prefix_binding = binding.clone();
        prefix_binding.projection.truncate(position);
        let base_place = self.build_binding_place(discr_local, &prefix_binding);
        let base_ty = match self.place_ty(&base_place) {
            Some(ty) => ty,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "unable to resolve type for list slice binding".into(),
                    span: binding.span.or(case.span),
                });
                return false;
            }
        };
        let sequence_ty = Self::strip_nullable(&base_ty);
        let Some(element_ty) = self.sequence_element_ty(&sequence_ty) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "list slice binding is only supported on sequences".into(),
                span: binding.span.or(case.span),
            });
            return false;
        };

        let span_ty = Ty::ReadOnlySpan(ReadOnlySpanTy {
            element: Box::new(element_ty.clone()),
        });
        if let Some(local) = self.locals.get_mut(binding.local.0) {
            local.ty = span_ty.clone();
            local.is_nullable = false;
        }

        let span_name = span_ty.canonical_name();
        let span = binding.span.or(case.span);

        let len_place = Place::new(plan.length_local);
        let slice_len_local = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(slice_len_local.0) {
            local.ty = Ty::named("usize");
            local.is_nullable = false;
        }
        let total_trim = from.saturating_add(to);
        let slice_len = MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(slice_len_local),
                value: Rvalue::Binary {
                    op: BinOp::Sub,
                    lhs: Operand::Copy(len_place.clone()),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::Int(
                        i128::try_from(total_trim).unwrap_or(0),
                    ))),
                    rounding: None,
                },
            },
        };
        self.push_statement(slice_len);

        let mut ptr_place = base_place.clone();
        ptr_place
            .projection
            .push(ProjectionElem::FieldNamed("ptr".into()));
        let ptr_operand = Operand::Copy(ptr_place.clone());

        let elem_size = self
            .type_layouts
            .size_and_align_for_ty(&element_ty)
            .map(|(size, _)| size)
            .unwrap_or(0);
        let size_const = ConstValue::Int(i128::try_from(elem_size).unwrap_or(0));
        let elem_align = self
            .type_layouts
            .size_and_align_for_ty(&element_ty)
            .map(|(_, align)| align.max(1))
            .unwrap_or(1);
        let align_const = ConstValue::Int(i128::try_from(elem_align).unwrap_or(1));
        let offset_local = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(offset_local.0) {
            local.ty = Ty::named("usize");
            local.is_nullable = false;
        }
        let offset_stmt = MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(offset_local),
                value: Rvalue::Binary {
                    op: BinOp::Mul,
                    lhs: Operand::Const(ConstOperand::new(size_const.clone())),
                    rhs: Operand::Const(ConstOperand::new(ConstValue::Int(
                        i128::try_from(from).unwrap_or(0),
                    ))),
                    rounding: None,
                },
            },
        };
        self.push_statement(offset_stmt);

        let start_ty = self.place_ty(&ptr_place).unwrap_or(Ty::named("usize"));
        let start_ptr_local = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(start_ptr_local.0) {
            local.ty = start_ty;
        }
        let start_ptr = MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(start_ptr_local),
                value: Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: ptr_operand.clone(),
                    rhs: Operand::Copy(Place::new(offset_local)),
                    rounding: None,
                },
            },
        };
        self.push_statement(start_ptr);

        let aggregate = Rvalue::Aggregate {
            kind: AggregateKind::Adt {
                name: span_name,
                variant: None,
            },
            fields: vec![
                Operand::Copy(Place::new(start_ptr_local)),
                Operand::Copy(Place::new(slice_len_local)),
                Operand::Const(ConstOperand::new(size_const)),
                Operand::Const(ConstOperand::new(align_const)),
            ],
        };
        let assign = MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(binding.local),
                value: aggregate,
            },
        };
        self.push_statement(assign);
        true
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Binding projection must trace through struct/union/enum layouts precisely."
    )]
    fn build_binding_place(&self, discr_local: LocalId, binding: &PatternBinding) -> Place {
        let mut place = Place::new(discr_local);

        let mut current_type = self
            .locals
            .get(discr_local.0)
            .and_then(|decl| self.resolve_ty_name(&decl.ty));

        let mut current_struct = current_type
            .as_deref()
            .and_then(|name| self.lookup_struct_layout_by_name(name));
        let mut current_union = current_type
            .as_deref()
            .and_then(|name| self.lookup_union_layout(name));
        let mut current_variant: Option<&EnumVariantLayout> = None;

        for proj in &binding.projection {
            match proj {
                PatternProjectionElem::Variant { path, variant } => {
                    let enum_name = if path.is_empty() {
                        current_type.clone()
                    } else {
                        self.qualify_pattern_path(path)
                    };

                    if let Some(enum_name) = enum_name.clone().or_else(|| current_type.clone())
                        && let Some(enum_layout) = self.lookup_enum_layout(&enum_name)
                        && let Some(index) =
                            enum_layout.variants.iter().position(|v| v.name == *variant)
                    {
                        let Some(variant_index) = u32::try_from(index).ok() else {
                            continue;
                        };
                        place.projection.push(ProjectionElem::Downcast {
                            variant: variant_index,
                        });
                        current_variant = enum_layout.variants.get(index);
                        current_struct = None;
                        current_union = None;
                        current_type = None;
                        continue;
                    }

                    place
                        .projection
                        .push(ProjectionElem::FieldNamed(format!("variant::{variant}")));
                    current_variant = None;
                    current_struct = None;
                    current_union = None;
                    current_type = None;
                }
                PatternProjectionElem::FieldNamed(name) => {
                    if let Some(variant_layout) = current_variant
                        && let Some(field) = variant_layout.fields.iter().find(|f| f.name == *name)
                    {
                        place.projection.push(ProjectionElem::Field(field.index));
                        current_type = self.resolve_ty_name(&field.ty);
                        current_struct = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_layout_by_name(ty));
                        current_union = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_union_layout(ty));
                        current_variant = None;
                        continue;
                    }

                    if let Some(struct_layout) = current_struct
                        && let Some(field) = struct_layout.fields.iter().find(|f| f.name == *name)
                    {
                        place.projection.push(ProjectionElem::Field(field.index));
                        current_type = self.resolve_ty_name(&field.ty);
                        current_struct = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_layout_by_name(ty));
                        current_union = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_union_layout(ty));
                        current_variant = None;
                        continue;
                    }

                    if let Some(union_layout) = current_union
                        && let Some(field) = union_layout.views.iter().find(|f| f.name == *name)
                    {
                        place.projection.push(ProjectionElem::UnionField {
                            index: field.index,
                            name: field.name.clone(),
                        });
                        current_type = self.resolve_ty_name(&field.ty);
                        current_struct = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_layout_by_name(ty));
                        current_union = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_union_layout(ty));
                        current_variant = None;
                        continue;
                    }

                    place
                        .projection
                        .push(ProjectionElem::FieldNamed(name.clone()));
                    current_variant = None;
                    current_struct = None;
                    current_union = None;
                    current_type = None;
                }
                PatternProjectionElem::FieldIndex(index) => {
                    if let Some(variant_layout) = current_variant
                        && let Some(field) =
                            variant_layout.fields.iter().find(|f| f.index == *index)
                    {
                        place.projection.push(ProjectionElem::Field(*index));
                        current_type = self.resolve_ty_name(&field.ty);
                        current_struct = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_layout_by_name(ty));
                        current_union = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_union_layout(ty));
                        current_variant = None;
                        continue;
                    }

                    if let Some(struct_layout) = current_struct
                        && let Some(field) = struct_layout.fields.iter().find(|f| f.index == *index)
                    {
                        place.projection.push(ProjectionElem::Field(*index));
                        current_type = self.resolve_ty_name(&field.ty);
                        current_struct = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_struct_layout_by_name(ty));
                        current_union = current_type
                            .as_deref()
                            .and_then(|ty| self.lookup_union_layout(ty));
                        current_variant = None;
                        continue;
                    }

                    place.projection.push(ProjectionElem::Field(*index));
                    current_variant = None;
                    current_struct = None;
                    current_union = None;
                    current_type = None;
                }
                PatternProjectionElem::Index(local) => {
                    place.projection.push(ProjectionElem::Index(*local));
                    current_variant = None;
                    current_struct = None;
                    current_union = None;
                    current_type = None;
                }
                PatternProjectionElem::Subslice { from, to } => {
                    place.projection.push(ProjectionElem::Subslice {
                        from: *from,
                        to: *to,
                    });
                    current_variant = None;
                    current_struct = None;
                    current_union = None;
                    current_type = None;
                }
            }
        }
        place
    }
}

impl BodyBuilder<'_> {
    pub(crate) fn aggregate_guard_metadata(guards: &[GuardMetadata]) -> Option<MatchGuard> {
        if guards.is_empty() {
            return None;
        }
        let text = guards
            .iter()
            .map(|guard| guard.expr.text.trim().to_string())
            .collect::<Vec<_>>()
            .join(" && ");
        Some(MatchGuard {
            expr: text,
            span: guards.first().and_then(|guard| guard.expr.span),
            parsed: guards.iter().all(|guard| guard.node.is_some()),
        })
    }

    pub(crate) fn lower_guard_chain(
        &mut self,
        guards: &[GuardMetadata],
        success_block: BlockId,
        failure_block: BlockId,
        span: Option<Span>,
    ) -> BlockId {
        if guards.is_empty() {
            return success_block;
        }

        let mut current_target = success_block;
        for guard in guards.iter().rev() {
            let guard_block = self.new_block(guard.expr.span.or(span));
            let prev = self.current_block;
            self.current_block = guard_block;

            let prev_guard_flag = self.in_guard_expression;
            self.in_guard_expression = true;
            let condition = if let Some(node) = guard.node.clone() {
                self.lower_expr_node(node, guard.expr.span)
            } else {
                self.lower_expression_operand(&guard.expr)
            };
            self.in_guard_expression = prev_guard_flag;

            if let Some(cond_operand) = condition {
                self.set_terminator(
                    guard.expr.span,
                    Terminator::SwitchInt {
                        discr: cond_operand,
                        targets: vec![(1, current_target)],
                        otherwise: failure_block,
                    },
                );
            } else {
                self.set_terminator(
                    guard.expr.span,
                    Terminator::Goto {
                        target: failure_block,
                    },
                );
            }

            self.current_block = prev;
            current_target = guard_block;
        }

        current_target
    }
}
