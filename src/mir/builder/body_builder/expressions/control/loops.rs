use super::*;

// Lowering paths for assignment-style control expressions (direct, property, and MMIO).

#[derive(Clone)]
struct NullConditionalSegment {
    kind: NullConditionalKind,
    null_conditional: bool,
}

#[derive(Clone)]
enum NullConditionalKind {
    Member(String),
    Index(Vec<ExprNode>),
}

body_builder_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Assignment lowering handles all compound operators in one place."
    )]
    pub(crate) fn lower_assignment_statement(
        &mut self,
        target: ExprNode,
        op: AssignOp,
        value: ExprNode,
        span: Option<Span>,
    ) -> bool {
        if self.target_contains_null_conditional(&target) {
            return self.lower_null_conditional_assignment(target, op, value, span);
        }

        if matches!(op, AssignOp::NullCoalesceAssign) {
            return self.lower_null_coalesce_assignment(target, value, span);
        }

        if self
            .target_looks_like_static_member(&target)
            .unwrap_or(false)
        {
            if let Some(result) = self.try_static_assignment(&target, op, value.clone(), span) {
                return result;
            }
        }
        if let ExprNode::Identifier(name) = &target {
            if let Some(result) =
                self.try_same_type_static_assignment(name, op, value.clone(), span)
            {
                return result;
            }
            if let Some(result) =
                self.try_namespace_static_assignment(name, op, value.clone(), span)
            {
                return result;
            }
        }

        if let Some(result) = self.try_property_assignment(&target, op, &value, span) {
            return result;
        }

        let Some(place) = self.lower_place_expr(target, span) else {
            return false;
        };
        let Some(rhs_operand) = self.lower_expr_node(value, span) else {
            return false;
        };

        if let Some(mmio_target) = self.mmio_operand_for_place(&place) {
            return self.lower_mmio_assignment(mmio_target, op, rhs_operand, span);
        }

        let mut rhs_operand = rhs_operand;
        if matches!(op, AssignOp::Assign) {
            rhs_operand = self.coerce_operand_to_place(rhs_operand, &place, false, span);
            rhs_operand = self.maybe_move_operand_for_value_assignment(rhs_operand);
        }

        if let Some(bin_op) = Self::bin_op_for_assign(op) {
            let lhs_view = Operand::Copy(place.clone());
            match self.resolve_overloaded_binary(bin_op, &lhs_view, &rhs_operand, span) {
                OperatorResolution::Handled(overload) => {
                    let args = vec![Operand::Copy(place.clone()), rhs_operand];
                    let Some(result_operand) = self.emit_operator_call(overload, args, span) else {
                        return false;
                    };
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place,
                            value: Rvalue::Use(result_operand),
                        },
                    });
                    return true;
                }
                OperatorResolution::Error => return false,
                OperatorResolution::Skip => {}
            }
        }

        let rvalue = match op {
            AssignOp::Assign => Rvalue::Use(rhs_operand),
            AssignOp::AddAssign => {
                Rvalue::Binary {
                    op: BinOp::Add,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::SubAssign => {
                Rvalue::Binary {
                    op: BinOp::Sub,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::MulAssign => {
                Rvalue::Binary {
                    op: BinOp::Mul,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::DivAssign => {
                Rvalue::Binary {
                    op: BinOp::Div,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::RemAssign => {
                Rvalue::Binary {
                    op: BinOp::Rem,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::BitAndAssign => {
                Rvalue::Binary {
                    op: BinOp::BitAnd,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::BitOrAssign => {
                Rvalue::Binary {
                    op: BinOp::BitOr,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::BitXorAssign => {
                Rvalue::Binary {
                    op: BinOp::BitXor,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::ShlAssign => {
                Rvalue::Binary {
                    op: BinOp::Shl,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::ShrAssign => {
                Rvalue::Binary {
                    op: BinOp::Shr,
                    lhs: Operand::Copy(place.clone()),
                    rhs: rhs_operand,
                    rounding: None,
                }
            }
            AssignOp::NullCoalesceAssign => unreachable!(
                "null-coalescing assignment handled before compound assignment lowering"
            ),
        };

        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place,
                value: rvalue,
            },
        });
        true
    }

    pub(crate) fn try_property_assignment(
        &mut self,
        target: &ExprNode,
        op: AssignOp,
        value: &ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        if let ExprNode::Member { base, member, .. } = target {
            let base_expr = *base.clone();
            return self.lower_property_assignment(
                base_expr,
                member.clone(),
                op,
                value.clone(),
                span,
            );
        }
        if let ExprNode::Identifier(name) = target {
            if let Some(owner) = self.current_self_type_name()
                && self.symbol_index.property(&owner, name).is_some()
            {
                let base = ExprNode::Identifier("self".to_string());
                return self.lower_property_assignment(base, name.clone(), op, value.clone(), span);
            }
        }
        None
    }

    pub(crate) fn lower_property_assignment(
        &mut self,
        base_expr: ExprNode,
        member: String,
        op: AssignOp,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        if self.member_chain_unresolved(&base_expr) {
            if let Some(owner) = self.resolve_static_owner_expr(&base_expr) {
                if let Some(symbol) = self.symbol_index.property(&owner, &member) {
                    if symbol.is_static {
                        return self.lower_static_property_assignment(
                            &owner,
                            &member,
                            symbol,
                            op,
                            value_expr,
                            span,
                        );
                    }
                }
            }
        }

        let base_operand = match self.lower_expr_node(base_expr, span) {
            Some(operand) => operand,
            None => return Some(false),
        };

        let Some((type_name, symbol_ref)) =
            self.property_symbol_from_operand(&base_operand, &member)
        else {
            return None;
        };
        let symbol = symbol_ref.clone();

        if op != AssignOp::Assign {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "compound assignment on property `{type_name}.{member}` is not supported"
                ),
                span,
            });
            return Some(false);
        }

        let Some((accessor_metadata, accessor_kind)) =
            self.property_setter_metadata(&symbol, &type_name, &member, span)
        else {
            return Some(false);
        };

        if symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static property `{type_name}.{member}` must be accessed using the type name"
                ),
                span: symbol.span.or(span),
            });
            return Some(false);
        }

        if !self.validate_property_setter_context(
            accessor_kind,
            &type_name,
            &member,
            &base_operand,
            symbol.span,
            span,
        ) {
            return Some(false);
        }

        let rhs_operand = match self.lower_expr_node(value_expr, span) {
            Some(operand) => operand,
            None => return Some(false),
        };

        let mut args = Vec::new();
        args.push(base_operand.clone());
        args.push(rhs_operand);

        if self
            .emit_property_call(&accessor_metadata.function, args, None, span)
            .is_none()
        {
            return Some(false);
        }

        Some(true)
    }

    pub(crate) fn lower_mmio_assignment(
        &mut self,
        target: MmioOperand,
        op: AssignOp,
        rhs_operand: Operand,
        span: Option<Span>,
    ) -> bool {
        match op {
            AssignOp::Assign => {
                if !self.validate_mmio_access(&target, MmioIntent::Write, span) {
                    return false;
                }

                let value_local = self.ensure_operand_local(rhs_operand, span);
                let value_operand = Operand::Copy(Place::new(value_local));
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::MmioStore {
                        target,
                        value: value_operand,
                    },
                });
                true
            }
            _ => {
                let Some(bin_op) = Self::bin_op_for_assign(op) else {
                    return false;
                };
                if !self.validate_mmio_access(&target, MmioIntent::ReadWrite, span) {
                    return false;
                }

                let lhs_local = self.ensure_operand_local(Operand::Mmio(target.clone()), span);
                let lhs_operand = Operand::Copy(Place::new(lhs_local));
                let rhs_local = self.ensure_operand_local(rhs_operand, span);
                let rhs_value = Operand::Copy(Place::new(rhs_local));

                let result_local = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(result_local),
                        value: Rvalue::Binary {
                            op: bin_op,
                            lhs: lhs_operand,
                            rhs: rhs_value,
                            rounding: None,
                        },
                    },
                });

                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::MmioStore {
                        target,
                        value: Operand::Copy(Place::new(result_local)),
                    },
                });
                true
            }
        }
    }

    pub(crate) fn target_contains_null_conditional(&self, expr: &ExprNode) -> bool {
        match expr {
            ExprNode::Member {
                base,
                null_conditional,
                ..
            } => *null_conditional || self.target_contains_null_conditional(base),
            ExprNode::Index {
                base,
                null_conditional,
                ..
            } => *null_conditional || self.target_contains_null_conditional(base),
            ExprNode::Parenthesized(inner) => self.target_contains_null_conditional(inner),
            _ => false,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn lower_null_conditional_assignment(
        &mut self,
        target: ExprNode,
        op: AssignOp,
        value: ExprNode,
        span: Option<Span>,
    ) -> bool {
        let repr = Self::expr_to_string(&target);
        let (base_expr, segments) = match self.decompose_null_conditional_chain(target) {
            Some(result) => result,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "null-conditional assignment requires a member or index target"
                        .into(),
                    span,
                });
                return false;
            }
        };

        if segments.is_empty() {
            return self.lower_assignment_statement(base_expr, op, value, span);
        }

        let mut base_operand = match self.lower_expr_node(base_expr, span) {
            Some(op) => op,
            None => return false,
        };
        let mut base_local = self.ensure_operand_local(base_operand.clone(), span);
        self.push_scope();
        let mut current_name = self.bind_temp_local(base_local);

        let mut skip_block: Option<BlockId> = None;
        let mut join_block: Option<BlockId> = None;

        for (index, segment) in segments.iter().enumerate() {
            let is_final = index == segments.len() - 1;

            if segment.null_conditional {
                if skip_block.is_none() {
                    skip_block = Some(self.new_block(span));
                    join_block = Some(self.new_block(span));
                }
                let skip = skip_block.expect("skip block initialised");
                let place = Place::new(base_local);
                let Some((next_block, payload_place)) =
                    self.branch_on_null_conditional(&place, &repr, skip, span)
                else {
                    self.pop_scope();
                    return false;
                };
                self.switch_to_block(next_block);
                let payload_ty = self.place_ty(&payload_place);
                let payload_local = self.create_temp(span);
                if let Some(decl) = self.locals.get_mut(payload_local.0) {
                    if let Some(ty) = payload_ty {
                        decl.ty = ty.clone();
                        decl.is_nullable = matches!(ty, Ty::Nullable(_));
                    }
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(payload_local),
                        value: Rvalue::Use(Operand::Copy(payload_place.clone())),
                    },
                });
                base_local = payload_local;
                current_name = self.bind_temp_local(base_local);
            }

            if is_final {
                let target_expr = self.build_segment_expr(
                    ExprNode::Identifier(current_name.clone()),
                    segment,
                    false,
                );
                let success = self.lower_assignment_statement(target_expr, op, value, span);
                if let (Some(skip), Some(join)) = (skip_block, join_block) {
                    self.ensure_goto(join, span);
                    self.switch_to_block(skip);
                    self.ensure_goto(join, span);
                    self.switch_to_block(join);
                }
                self.pop_scope();
                return success;
            }

            let step_expr =
                self.build_segment_expr(ExprNode::Identifier(current_name.clone()), segment, false);
            base_operand = match self.lower_expr_node(step_expr, span) {
                Some(op) => op,
                None => {
                    self.pop_scope();
                    return false;
                }
            };
            base_local = self.ensure_operand_local(base_operand.clone(), span);
            current_name = self.bind_temp_local(base_local);
        }

        if let (Some(skip), Some(join)) = (skip_block, join_block) {
            self.switch_to_block(skip);
            self.ensure_goto(join, span);
            self.switch_to_block(join);
        }

        self.pop_scope();
        true
    }

    fn build_segment_expr(
        &self,
        base: ExprNode,
        segment: &NullConditionalSegment,
        null_conditional: bool,
    ) -> ExprNode {
        match &segment.kind {
            NullConditionalKind::Member(name) => ExprNode::Member {
                base: Box::new(base),
                member: name.clone(),
                null_conditional,
            },
            NullConditionalKind::Index(indices) => ExprNode::Index {
                base: Box::new(base),
                indices: indices.clone(),
                null_conditional,
            },
        }
    }

    fn decompose_null_conditional_chain(
        &self,
        mut expr: ExprNode,
    ) -> Option<(ExprNode, Vec<NullConditionalSegment>)> {
        let mut segments = Vec::new();
        loop {
            match expr {
                ExprNode::Member {
                    base,
                    member,
                    null_conditional,
                } => {
                    segments.push(NullConditionalSegment {
                        kind: NullConditionalKind::Member(member),
                        null_conditional,
                    });
                    expr = *base;
                }
                ExprNode::Index {
                    base,
                    indices,
                    null_conditional,
                } => {
                    segments.push(NullConditionalSegment {
                        kind: NullConditionalKind::Index(indices),
                        null_conditional,
                    });
                    expr = *base;
                }
                ExprNode::Parenthesized(inner) => {
                    expr = *inner;
                }
                _ => break,
            }
        }
        if segments.is_empty() {
            return None;
        }
        segments.reverse();
        Some((expr, segments))
    }

    fn bind_temp_local(&mut self, local: LocalId) -> String {
        let name = format!("__nullc{}", self.temp_counter);
        self.temp_counter += 1;
        self.bind_name(&name, local);
        name
    }

    fn branch_on_null_conditional(
        &mut self,
        place: &Place,
        repr: &str,
        skip_block: BlockId,
        span: Option<Span>,
    ) -> Option<(BlockId, Place)> {
        let Some(info) = self.nullable_place_info(place, repr, "?.", span) else {
            return None;
        };
        self.ensure_nullable_layout(&info);

        let mut flag_place = place.clone();
        flag_place
            .projection
            .push(ProjectionElem::FieldNamed("HasValue".into()));
        self.normalise_place(&mut flag_place);
        let mut value_place = place.clone();
        value_place
            .projection
            .push(ProjectionElem::FieldNamed("Value".into()));
        self.normalise_place(&mut value_place);

        let non_null = self.new_block(span);
        self.set_terminator(
            span,
            Terminator::SwitchInt {
                discr: Operand::Copy(flag_place),
                targets: vec![(0, skip_block)],
                otherwise: non_null,
            },
        );
        Some((non_null, value_place))
    }
}
