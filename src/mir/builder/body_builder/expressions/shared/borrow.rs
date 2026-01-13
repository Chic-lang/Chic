use super::*;

body_builder_impl! {
    pub(crate) fn ensure_operand_local(&mut self, operand: Operand, span: Option<Span>) -> LocalId {
        let operand_ty = self.operand_ty(&operand);
        let local = match operand {
            Operand::Copy(place) if place.projection.is_empty() => place.local,
            Operand::Move(place) if place.projection.is_empty() => place.local,
            Operand::Mmio(spec) => {
                let temp = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                                        kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(Operand::Mmio(spec)),
                    },
                });
                temp
            }
            Operand::Borrow(borrow) => {
                let temp = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(Operand::Borrow(borrow)),
                    },
                });
                temp
            }
            other => {
                let temp = self.create_temp(span);
                self.push_statement(MirStatement {
                    span,
                                        kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(other),
                    },
                });
                temp
            }
        };

        if let Some(ty) = operand_ty {
            if let Some(decl) = self.locals.get_mut(local.0) {
                if matches!(decl.ty, Ty::Unknown) {
                    decl.ty = ty;
                }
            }
        }

        local
    }
    pub(crate) fn assign_bool(&mut self, local: LocalId, value: bool, span: Option<Span>) {
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Bool(value)))),
            },
        });
    }
    pub(crate) fn parse_synthetic_expression(
        &mut self,
        text: &str,
        span: Option<Span>,
            ) -> Option<ExprNode> {
        match parse_expression(text) {
            Ok(expr) => Some(expr),
            Err(err) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "failed to analyse generated foreach expression `{text}`: {}",
                        err.message
                    ),
                    span: err.span.or(span),
                                    });
                None
            }
        }
    }
    pub(crate) fn borrow_argument_place(
        &mut self,
        place: Place,
        kind: BorrowKind,
        span: Option<Span>,
            ) -> Operand {
        let (borrow_id, region) = self.fresh_borrow();
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Borrow {
                borrow_id,
                kind,
                place: place.clone(),
                region,
            },
        });
        Operand::Borrow(BorrowOperand {
            kind,
            place,
            region,
            span,
                    })
    }
    pub(crate) fn fresh_borrow(&mut self) -> (BorrowId, RegionVar) {
        let borrow = BorrowId(self.next_borrow_id);
        self.next_borrow_id += 1;
        let region = RegionVar(self.next_region_id);
        self.next_region_id += 1;
        (borrow, region)
    }
}
