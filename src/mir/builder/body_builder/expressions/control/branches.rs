use super::*;
use crate::mir::layout::{EnumLayout, EnumVariantLayout};

// Branch-oriented control expressions: ternary lowering and `?` propagation.

body_builder_impl! {
    pub(crate) fn lower_boolean_branch(
        &mut self,
        expr: ExprNode,
        true_block: BlockId,
        false_block: BlockId,
        span: Option<Span>,
    ) -> bool {
        match expr {
            ExprNode::Parenthesized(inner) => {
                self.lower_boolean_branch(*inner, true_block, false_block, span)
            }
            ExprNode::Unary {
                op: UnOp::Not,
                expr,
                postfix: false,
            } => self.lower_boolean_branch(*expr, false_block, true_block, span),
            ExprNode::Binary {
                op: BinOp::And,
                left,
                right,
            } => {
                let rhs_block = self.new_block(span);
                if !self.lower_boolean_branch(*left, rhs_block, false_block, span) {
                    return false;
                }
                self.switch_to_block(rhs_block);
                self.lower_boolean_branch(*right, true_block, false_block, span)
            }
            ExprNode::Binary {
                op: BinOp::Or,
                left,
                right,
            } => {
                let rhs_block = self.new_block(span);
                if !self.lower_boolean_branch(*left, true_block, rhs_block, span) {
                    return false;
                }
                self.switch_to_block(rhs_block);
                self.lower_boolean_branch(*right, true_block, false_block, span)
            }
            other => {
                let Some(discr) = self.lower_expr_node(other, span) else {
                    return false;
                };
                if matches!(discr, Operand::Pending(_)) {
                    return false;
                }
                self.set_terminator(
                    span,
                    Terminator::SwitchInt {
                        discr,
                        targets: vec![(1, true_block)],
                        otherwise: false_block,
                    },
                );
                true
            }
        }
    }

    pub(crate) fn report_assignment_expr(&mut self, span: Option<Span>) -> Option<Operand> {
        self.diagnostics.push(LoweringDiagnostic {
            message: "assignment expressions are not supported in this context".into(),
            span,
        });
        None
    }

    pub(crate) fn lower_conditional_expr(
        &mut self,
        condition: ExprNode,
        then_branch: ExprNode,
        else_branch: ExprNode,
        span: Option<Span>,
    ) -> Option<Operand> {
        let then_block = self.new_block(span);
        let else_block = self.new_block(span);
        let join_block = self.new_block(span);

        let result_local = self.create_temp_untracked(span);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::StorageLive(result_local),
        });
        if !self.lower_boolean_branch(condition, then_block, else_block, span) {
            return None;
        }

        self.switch_to_block(then_block);
        let mut then_operand = self.lower_expr_node(then_branch, span)?;
        if let Some(ty) = self.operand_ty(&then_operand) {
            if let Some(local) = self.locals.get_mut(result_local.0) {
                if matches!(local.ty, Ty::Unknown) {
                    local.ty = ty.clone();
                    local.is_nullable = matches!(ty, Ty::Nullable(_));
                }
            }
        }
        if let Some(target_ty) = self
            .locals
            .get(result_local.0)
            .and_then(|decl| (!matches!(decl.ty, Ty::Unknown)).then(|| decl.ty.clone()))
        {
            then_operand = self.coerce_operand_to_ty(then_operand, &target_ty, false, span);
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(result_local),
                value: Rvalue::Use(then_operand),
            },
        });
        self.ensure_goto(join_block, span);

        self.switch_to_block(else_block);
        let mut else_operand = self.lower_expr_node(else_branch, span)?;
        if let Some(target_ty) = self
            .locals
            .get(result_local.0)
            .and_then(|decl| (!matches!(decl.ty, Ty::Unknown)).then(|| decl.ty.clone()))
        {
            else_operand = self.coerce_operand_to_ty(else_operand, &target_ty, false, span);
        } else if let Some(ty) = self.operand_ty(&else_operand) {
            if let Some(local) = self.locals.get_mut(result_local.0) {
                if matches!(local.ty, Ty::Unknown) {
                    local.ty = ty.clone();
                    local.is_nullable = matches!(ty, Ty::Nullable(_));
                }
            }
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(result_local),
                value: Rvalue::Use(else_operand),
            },
        });
        self.ensure_goto(join_block, span);

        self.switch_to_block(join_block);
        Some(Operand::Copy(Place::new(result_local)))
    }

    pub(crate) fn lower_try_propagate_expr(
        &mut self,
        value: ExprNode,
        question_span: Option<Span>,
        span: Option<Span>,
    ) -> Option<Operand> {
        let diagnostic_span = question_span.or(span);
        let operand = self.lower_expr_node(value, span)?;
        let operand_ty = self.operand_ty(&operand);
        let result_local = self.ensure_operand_local(operand, span);

        if let (Some(ty), Some(decl)) = (operand_ty.clone(), self.locals.get_mut(result_local.0)) {
            if matches!(decl.ty, Ty::Unknown) {
                decl.ty = ty;
            }
        }

        let operand_ty = match operand_ty {
            Some(ty) => ty,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "cannot use `?` with an operand of unknown type".into(),
                    span: diagnostic_span,
                });
                return None;
            }
        };

        let operand_type_name = self
            .resolve_ty_name(&operand_ty)
            .unwrap_or_else(|| operand_ty.canonical_name());
        let operand_layout = match self.find_enum_layout(&operand_type_name) {
            Some(layout) => layout,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`?` requires an enum `Result<T, E>` operand; `{operand_type_name}` is not an enum"
                    ),
                    span: diagnostic_span,
                });
                return None;
            }
        };

        let operand_ok_variant =
            match Self::find_result_variant(&operand_layout, &["Ok", "Success"]) {
                Some(variant) => variant,
                None => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`{}` does not expose an `Ok`/`Success` variant required for `?`",
                            operand_layout.name
                        ),
                        span: diagnostic_span,
                    });
                    return None;
                }
            };
        let operand_err_variant =
            match Self::find_result_variant(&operand_layout, &["Err", "Error", "Failure"]) {
                Some(variant) => variant,
                None => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`{}` does not expose an `Err`/`Error` variant required for `?`",
                            operand_layout.name
                        ),
                        span: diagnostic_span,
                    });
                    return None;
                }
            };

        if operand_ok_variant.fields.len() > 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`?` currently supports `Ok` variants with at most one payload field"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }
        if operand_err_variant.fields.len() > 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`?` currently supports `Err` variants with at most one payload field"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }

        let ok_payload_ty = operand_ok_variant
            .fields
            .get(0)
            .map(|field| field.ty.clone())
            .unwrap_or(Ty::Unit);
        let err_source_ty = operand_err_variant
            .fields
            .get(0)
            .map(|field| field.ty.clone())
            .unwrap_or(Ty::Unit);

        let result_value_local = self.create_temp(span);
        if !self.declared_effects.is_empty() {
            return self.lower_question_mark_effectful(
                operand_layout.clone(),
                operand_ok_variant.clone(),
                operand_err_variant.clone(),
                ok_payload_ty,
                result_local,
                result_value_local,
                span,
                diagnostic_span,
            );
        }
        let return_type = self.return_type.clone();
        let return_type_name = self
            .resolve_ty_name(&return_type)
            .unwrap_or_else(|| return_type.canonical_name());
        let return_layout = match self.find_enum_layout(&return_type_name) {
            Some(layout) => layout,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`?` requires the enclosing function to return `Result<_, _>`; found `{return_type_name}`"
                    ),
                    span: diagnostic_span,
                });
                return None;
            }
        };

        let return_ok_variant =
            match Self::find_result_variant(&return_layout, &["Ok", "Success"]) {
                Some(variant) => variant,
                None => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`{}` does not expose an `Ok`/`Success` variant required when returning from `?`",
                            return_layout.name
                        ),
                        span: diagnostic_span,
                    });
                    return None;
                }
            };
        let return_err_variant =
            match Self::find_result_variant(&return_layout, &["Err", "Error", "Failure"]) {
                Some(variant) => variant,
                None => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`{}` does not expose an `Err`/`Error` variant required when propagating from `?`",
                            return_layout.name
                        ),
                        span: diagnostic_span,
                    });
                    return None;
                }
            };

        if return_ok_variant.fields.len() > 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`?` currently supports `Ok` variants with at most one payload field"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }
        if return_err_variant.fields.len() > 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`?` currently supports `Err` variants with at most one payload field"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }
        if operand_err_variant.fields.len() != return_err_variant.fields.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "mismatched error payload arity between operand and return types when using `?`"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }
        if operand_ok_variant.fields.len() != return_ok_variant.fields.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "mismatched success payload arity between operand and return types when using `?`"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }

        let ok_payload_ty = return_ok_variant
            .fields
            .get(0)
            .map(|field| field.ty.clone())
            .unwrap_or(Ty::Unit);
        let err_target_ty = return_err_variant
            .fields
            .get(0)
            .map(|field| field.ty.clone())
            .unwrap_or(Ty::Unit);

        if let Some(local) = self.locals.get_mut(result_value_local.0) {
            local.ty = ok_payload_ty.clone();
            local.is_nullable = matches!(ok_payload_ty, Ty::Nullable(_));
        }

        let ok_block = self.new_block(span);
        let err_block = self.new_block(span);
        let join_block = self.new_block(span);

        let ok_pattern = Self::result_variant_pattern(&operand_layout, &operand_ok_variant);
        let err_pattern = Self::result_variant_pattern(&operand_layout, &operand_err_variant);

        self.set_terminator(
            span,
            Terminator::Match {
                value: Place::new(result_local),
                arms: vec![
                    MatchArm {
                        pattern: ok_pattern,
                        guard: None,
                        bindings: Vec::new(),
                        target: ok_block,
                    },
                    MatchArm {
                        pattern: err_pattern,
                        guard: None,
                        bindings: Vec::new(),
                        target: err_block,
                    },
                ],
                otherwise: err_block,
            },
        );

        // Ok branch
        self.switch_to_block(ok_block);
        let ok_operand = if operand_ok_variant.fields.is_empty() {
            Operand::Const(ConstOperand::new(ConstValue::Unit))
        } else {
            let mut place = Place::new(result_local);
            place.projection.push(ProjectionElem::Downcast {
                variant: operand_ok_variant.index,
            });
            let field = &operand_ok_variant.fields[0];
            place.projection.push(ProjectionElem::Field(field.index));
            Operand::Move(place)
        };
        let coerced_ok = if operand_ok_variant.fields.is_empty() {
            ok_operand
        } else {
            self.coerce_operand_to_ty(ok_operand, &ok_payload_ty, false, span)
        };
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(result_value_local),
                value: Rvalue::Use(coerced_ok),
            },
        });
        self.mark_fallible_handled(result_local, span);
        self.emit_storage_dead(result_local, span);
        self.ensure_goto(join_block, span);

        // Err branch
        self.switch_to_block(err_block);
        if operand_err_variant.fields.is_empty() {
            // No payload, so just construct the Err variant and return early.
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(LocalId(0)),
                    value: Rvalue::Aggregate {
                        kind: AggregateKind::Adt {
                            name: return_layout.name.clone(),
                            variant: Some(return_err_variant.name.clone()),
                        },
                        fields: Vec::new(),
                    },
                },
            });
            self.mark_fallible_handled(result_local, span);
            self.emit_storage_dead(result_local, span);
            self.set_terminator(diagnostic_span, Terminator::Return);
        } else {
            let mut err_place = Place::new(result_local);
            err_place.projection.push(ProjectionElem::Downcast {
                variant: operand_err_variant.index,
            });
            err_place.projection.push(ProjectionElem::Field(
                operand_err_variant.fields[0].index,
            ));
            let fallback_place = err_place.clone();
            let error_operand = Operand::Move(err_place);
            let (converted_operand, temp_local) = match self.convert_error_operand(
                error_operand,
                &err_source_ty,
                &err_target_ty,
                diagnostic_span,
            ) {
                Some(result) => result,
                None => (Operand::Move(fallback_place), None),
            };

            let aggregate_operand = match converted_operand {
                Operand::Copy(place) => Operand::Move(place),
                other => other,
            };

            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(LocalId(0)),
                    value: Rvalue::Aggregate {
                        kind: AggregateKind::Adt {
                            name: return_layout.name.clone(),
                            variant: Some(return_err_variant.name.clone()),
                        },
                        fields: vec![aggregate_operand],
                    },
                },
            });

            if let Some(temp) = temp_local {
                self.emit_storage_dead(temp, span);
            }
            self.mark_fallible_handled(result_local, span);
            self.emit_storage_dead(result_local, span);
            self.set_terminator(diagnostic_span, Terminator::Return);
        }

        self.switch_to_block(join_block);
        Some(Operand::Copy(Place::new(result_value_local)))
    }

    fn lower_question_mark_effectful(
        &mut self,
        operand_layout: EnumLayout,
        operand_ok_variant: EnumVariantLayout,
        operand_err_variant: EnumVariantLayout,
        ok_payload_ty: Ty,
        result_local: LocalId,
        result_value_local: LocalId,
        span: Option<Span>,
        diagnostic_span: Option<Span>,
    ) -> Option<Operand> {
        if let Some(local) = self.locals.get_mut(result_value_local.0) {
            local.ty = ok_payload_ty.clone();
            local.is_nullable = matches!(ok_payload_ty, Ty::Nullable(_));
        }

        let ok_block = self.new_block(span);
        let err_block = self.new_block(span);
        let join_block = self.new_block(span);

        let ok_pattern = Self::result_variant_pattern(&operand_layout, &operand_ok_variant);
        let err_pattern = Self::result_variant_pattern(&operand_layout, &operand_err_variant);

        self.set_terminator(
            span,
            Terminator::Match {
                value: Place::new(result_local),
                arms: vec![
                    MatchArm {
                        pattern: ok_pattern,
                        guard: None,
                        bindings: Vec::new(),
                        target: ok_block,
                    },
                    MatchArm {
                        pattern: err_pattern,
                        guard: None,
                        bindings: Vec::new(),
                        target: err_block,
                    },
                ],
                otherwise: err_block,
            },
        );

        self.switch_to_block(ok_block);
        let ok_operand = if operand_ok_variant.fields.is_empty() {
            Operand::Const(ConstOperand::new(ConstValue::Unit))
        } else {
            let mut place = Place::new(result_local);
            place.projection.push(ProjectionElem::Downcast {
                variant: operand_ok_variant.index,
            });
            let field = &operand_ok_variant.fields[0];
            place.projection.push(ProjectionElem::Field(field.index));
            Operand::Move(place)
        };
        let coerced_ok = if operand_ok_variant.fields.is_empty() {
            ok_operand
        } else {
            self.coerce_operand_to_ty(ok_operand, &ok_payload_ty, false, span)
        };
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(result_value_local),
                value: Rvalue::Use(coerced_ok),
            },
        });
        self.emit_storage_dead(result_local, span);
        self.ensure_goto(join_block, span);

        self.switch_to_block(err_block);
        if operand_err_variant.fields.len() != 1 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`?` used with a `throws` clause requires the `Err` variant to carry exactly one exception payload"
                    .into(),
                span: diagnostic_span,
            });
            return None;
        }
        let mut err_place = Place::new(result_local);
        err_place.projection.push(ProjectionElem::Downcast {
            variant: operand_err_variant.index,
        });
        let field = &operand_err_variant.fields[0];
        err_place
            .projection
            .push(ProjectionElem::Field(field.index));
        let err_ty = field.ty.clone();
        if !self.ty_is_exception(&err_ty) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "`?` requires the error payload `{}` to derive from `Exception` when used with `throws`",
                    err_ty.canonical_name()
                ),
                span: diagnostic_span,
            });
            return None;
        }
        let throw_operand = Operand::Move(err_place);
        self.emit_storage_dead(result_local, span);
        if !self.emit_throw(diagnostic_span, Some(throw_operand)) {
            return None;
        }

        self.switch_to_block(join_block);
        Some(Operand::Copy(Place::new(result_value_local)))
    }
}
