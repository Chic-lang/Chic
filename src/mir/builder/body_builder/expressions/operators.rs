use super::*;
use crate::mir::casts::{ScalarKind, classify_scalar};

body_builder_impl! {
    fn promoted_numeric_result_ty(&self, lhs: Option<&Ty>, rhs: Option<&Ty>) -> Option<Ty> {
        let pointer_size = crate::mir::builder::pointer_size() as u32;
        let lhs = lhs?;
        let rhs = rhs?;
        let (Ty::Named(lhs_named), Ty::Named(rhs_named)) = (lhs, rhs) else {
            return None;
        };
        let lhs_kind = classify_scalar(self.primitive_registry, lhs_named.as_str(), pointer_size)?;
        let rhs_kind = classify_scalar(self.primitive_registry, rhs_named.as_str(), pointer_size)?;

        let mut float_bits: Option<u16> = None;
        for kind in [lhs_kind, rhs_kind] {
            if let ScalarKind::Float(info) = kind {
                float_bits = Some(float_bits.map_or(info.bits, |existing| existing.max(info.bits)));
            }
        }
        let Some(bits) = float_bits else {
            return None;
        };
        let name = match bits {
            16 => "float16",
            32 => "float",
            64 => "double",
            128 => "float128",
            _ => return None,
        };
        Some(Ty::named(name))
    }

    pub(crate) fn normalise_const(&mut self, value: ConstValue, span: Option<Span>) -> ConstValue {
        match value {
            ConstValue::RawStr(text) => {
                let id = self
                    .string_interner
                    .intern(&text, StrLifetime::Static, span);
                ConstValue::Str { id, value: text }
            }
            other => other,
        }
    }
    pub(crate) fn lower_unary_expr(
        &mut self,
        op: UnOp,
        expr: ExprNode,
        postfix: bool,
        span: Option<Span>,
            ) -> Option<Operand> {
        match op {
            UnOp::AddrOf | UnOp::AddrOfMut => {
                let mutable = matches!(op, UnOp::AddrOfMut);
                if let Some(operand) = match &expr {
                    ExprNode::Identifier(name) => {
                        self.lower_namespace_static_address(name, mutable, span)
                    }
                    ExprNode::Member { base, member, .. } => {
                        self.lower_namespace_static_address_path(base, member, mutable, span)
                    }
                    _ => None,
                } {
                    return Some(operand);
                }
                let Some(place) = self.lower_place_expr(expr, span) else {
                    return None;
                };
                let place_ty = self.place_ty(&place);
                let place_for_rvalue = place.clone();
                let temp = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                                        kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::AddressOf {
                            mutability: if matches!(op, UnOp::AddrOfMut) {
                                Mutability::Mutable
                            } else {
                                Mutability::Immutable
                            },
                            place: place_for_rvalue,
                        },
                    },
                });
                if let Some(element_ty) = place_ty {
                    let pointer =
                        crate::mir::PointerTy::new(element_ty, matches!(op, UnOp::AddrOfMut));
                    self.hint_local_ty(temp, Ty::Pointer(Box::new(pointer)));
                }
                return Some(Operand::Copy(Place::new(temp)));
            }
            UnOp::Deref => {
                if self.unsafe_depth == 0 {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "dereferencing a pointer requires an `unsafe` block".into(),
                        span,
                    });
                }
                let operand = self.lower_expr_node(expr, span)?;
                let operand_ty = self.operand_ty(&operand);
                let temp = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Unary {
                            op: UnOp::Deref,
                            operand,
                            rounding: None,
                        },
                    },
                });
                if let Some(Ty::Pointer(pointer_ty)) = operand_ty {
                    if let Some(local) = self.locals.get_mut(temp.0) {
                        local.ty = pointer_ty.element.clone();
                    }
                }
                return Some(Operand::Copy(Place::new(temp)));
            }
            _ => {}
        }
        if matches!(op, UnOp::Increment | UnOp::Decrement) {
            if self.target_contains_null_conditional(&expr) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "null-conditional member access cannot be used with ++ or --; use an explicit null-check and assignment instead".into(),
                    span,
                });
                return None;
            }
            let Some(place) = self.lower_place_expr(expr, span) else {
                return None;
            };
            let place_ty = self.place_ty(&place);
            let saved_original = if postfix {
                let temp = self.create_temp(span);
                if let Some(ty) = place_ty.clone() {
                    if let Some(local) = self.locals.get_mut(temp.0) {
                        local.ty = ty;
                    }
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(Operand::Copy(place.clone())),
                    },
                });
                Some(Operand::Copy(Place::new(temp)))
            } else {
                None
            };

            let operand = Operand::Copy(place.clone());
            let result_operand = match self.resolve_overloaded_unary(op, &operand, span) {
                OperatorResolution::Handled(overload) => {
                    self.emit_operator_call(overload, vec![operand.clone()], span)?
                }
                OperatorResolution::Error => return None,
                OperatorResolution::Skip => {
                    let temp = self.create_temp(span);
                    if let Some(ty) = place_ty.clone() {
                        if let Some(local) = self.locals.get_mut(temp.0) {
                            local.ty = ty;
                        }
                    }
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: Place::new(temp),
                            value: Rvalue::Unary {
                                op,
                                operand,
                                rounding: None,
                            },
                        },
                    });
                    Operand::Copy(Place::new(temp))
                }
            };

            let assigned = self.coerce_operand_to_place(result_operand.clone(), &place, false, span);
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: place.clone(),
                    value: Rvalue::Use(assigned.clone()),
                },
            });

            let result = if postfix {
                saved_original.unwrap_or_else(|| Operand::Copy(place))
            } else {
                Operand::Copy(place)
            };
            return Some(result);
        }

        let operand = self.lower_expr_node(expr, span)?;
        match self.resolve_overloaded_unary(op, &operand, span) {
            OperatorResolution::Handled(overload) => {
                return self.emit_operator_call(overload, vec![operand], span);
            }
            OperatorResolution::Error => return None,
            OperatorResolution::Skip => {}
        }
        let operand_ty = self.operand_ty(&operand);
        let temp = self.create_temp(span);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Unary {
                    op,
                    operand,
                    rounding: None,
                },
            },
        });
        if let Some(op_ty) = operand_ty {
            let inferred = match op {
                UnOp::Not => {
                    if op_ty == Ty::named("bool") {
                        Some(Ty::named("bool"))
                    } else {
                        None
                    }
                }
                UnOp::BitNot | UnOp::Neg | UnOp::UnaryPlus => Some(op_ty),
                _ => None,
            };
            if let Some(hint) = inferred {
                if let Some(local) = self.locals.get_mut(temp.0) {
                    local.ty = hint;
                }
            }
        }
        Some(Operand::Copy(Place::new(temp)))
    }
    pub(crate) fn lower_binary_expr(
        &mut self,
            op: BinOp,
            left: ExprNode,
            right: ExprNode,
            span: Option<Span>,
            ) -> Option<Operand> {
        if matches!(op, BinOp::And | BinOp::Or) {
            return self.lower_logical_expr(op, left, right, span);
        }
        let lhs = self.lower_expr_node(left, span)?;
        let rhs = self.lower_expr_node(right, span)?;
        if let Operand::Pending(pending) = &lhs {
            let temp = self.create_temp(span);
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Pending(PendingRvalue {
                        repr: pending.repr.clone(),
                        span: pending.span.or(span),
                    }),
                },
            });
            return Some(Operand::Copy(Place::new(temp)));
        }
        if let Operand::Pending(pending) = &rhs {
            let temp = self.create_temp(span);
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Pending(PendingRvalue {
                        repr: pending.repr.clone(),
                        span: pending.span.or(span),
                    }),
                },
            });
            return Some(Operand::Copy(Place::new(temp)));
        }
        match self.resolve_overloaded_binary(op, &lhs, &rhs, span) {
            OperatorResolution::Handled(overload) => {
                return self.emit_operator_call(overload, vec![lhs, rhs], span);
            }
            OperatorResolution::Error => return None,
            OperatorResolution::Skip => {}
        }
        let lhs_ty = self.operand_ty(&lhs);
        let rhs_ty = self.operand_ty(&rhs);
        let promoted_numeric = self.promoted_numeric_result_ty(lhs_ty.as_ref(), rhs_ty.as_ref());

        let result_ty = match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                Some(Ty::named("bool"))
            }
            BinOp::And | BinOp::Or => Some(Ty::named("bool")),
            BinOp::NullCoalesce => lhs_ty.clone().or(rhs_ty.clone()),
            BinOp::Add | BinOp::Sub => {
                let lhs_is_stringy =
                    matches!(lhs_ty, Some(Ty::String)) || matches!(lhs_ty, Some(Ty::Str));
                let rhs_is_stringy =
                    matches!(rhs_ty, Some(Ty::String)) || matches!(rhs_ty, Some(Ty::Str));
                let is_string_op = matches!(op, BinOp::Add) && (lhs_is_stringy || rhs_is_stringy);
                if is_string_op {
                    Some(Ty::String)
                } else {
                    match (lhs_ty.clone(), rhs_ty.clone()) {
                        (Some(Ty::Pointer(_)), Some(Ty::Pointer(_))) if matches!(op, BinOp::Sub) => {
                            Some(Ty::named("isize"))
                        }
                        (Some(Ty::Pointer(_)), Some(_)) => lhs_ty.clone(),
                        (Some(_), Some(Ty::Pointer(_))) if matches!(op, BinOp::Add) => {
                            rhs_ty.clone()
                        }
                        _ => promoted_numeric.clone().or_else(|| lhs_ty.clone().or(rhs_ty.clone())),
                    }
                }
            }
            _ => promoted_numeric.or_else(|| lhs_ty.clone().or(rhs_ty.clone())),
        };

        let temp = self.create_temp(span);
        if let Some(result_ty) = result_ty {
            if let Some(decl) = self.locals.get_mut(temp.0) {
                decl.ty = result_ty;
                decl.is_nullable = matches!(decl.ty, Ty::Nullable(_));
            }
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Binary {
                    op,
                    lhs,
                    rhs,
                    rounding: None,
                },
            },
        });
        Some(Operand::Copy(Place::new(temp)))
    }

    fn lower_logical_expr(
        &mut self,
        op: BinOp,
        left: ExprNode,
        right: ExprNode,
        span: Option<Span>,
    ) -> Option<Operand> {
        let mut lhs = self.lower_expr_node(left, span)?;
        if let Operand::Pending(pending) = &lhs {
            let temp = self.create_temp(span);
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Pending(PendingRvalue {
                        repr: pending.repr.clone(),
                        span: pending.span.or(span),
                    }),
                },
            });
            lhs = Operand::Copy(Place::new(temp));
        }

        let then_block = self.new_block(span);
        let else_block = self.new_block(span);
        let join_block = self.new_block(span);

        let result_local = self.create_temp(span);
        self.hint_local_ty(result_local, Ty::named("bool"));
        self.assign_bool(result_local, false, span);

        self.set_terminator(
            span,
            Terminator::SwitchInt {
                discr: lhs,
                targets: vec![(1, then_block)],
                otherwise: else_block,
            },
        );

        match op {
            BinOp::Or => {
                self.switch_to_block(then_block);
                self.assign_bool(result_local, true, span);
                self.ensure_goto(join_block, span);

                self.switch_to_block(else_block);
                let mut rhs = self.lower_expr_node(right, span)?;
                if let Operand::Pending(pending) = &rhs {
                    let temp = self.create_temp(span);
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: Place::new(temp),
                            value: Rvalue::Pending(PendingRvalue {
                                repr: pending.repr.clone(),
                                span: pending.span.or(span),
                            }),
                        },
                    });
                    rhs = Operand::Copy(Place::new(temp));
                }
                rhs = self.coerce_operand_to_ty(rhs, &Ty::named("bool"), false, span);
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(result_local),
                        value: Rvalue::Use(rhs),
                    },
                });
                self.ensure_goto(join_block, span);
            }
            BinOp::And => {
                self.switch_to_block(then_block);
                let mut rhs = self.lower_expr_node(right, span)?;
                if let Operand::Pending(pending) = &rhs {
                    let temp = self.create_temp(span);
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: Place::new(temp),
                            value: Rvalue::Pending(PendingRvalue {
                                repr: pending.repr.clone(),
                                span: pending.span.or(span),
                            }),
                        },
                    });
                    rhs = Operand::Copy(Place::new(temp));
                }
                rhs = self.coerce_operand_to_ty(rhs, &Ty::named("bool"), false, span);
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(result_local),
                        value: Rvalue::Use(rhs),
                    },
                });
                self.ensure_goto(join_block, span);

                self.switch_to_block(else_block);
                self.assign_bool(result_local, false, span);
                self.ensure_goto(join_block, span);
            }
            _ => unreachable!("lower_logical_expr expects &&/|| operators"),
        }

        self.switch_to_block(join_block);
        Some(Operand::Copy(Place::new(result_local)))
    }
}
