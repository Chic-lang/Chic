use super::*;

body_builder_impl! {
    pub(crate) fn lower_tuple_expr(&mut self, elements: Vec<ExprNode>, span: Option<Span>) -> Option<Operand> {
        let mut operands = Vec::with_capacity(elements.len());
        let mut element_types = Vec::with_capacity(elements.len());

        for element in elements {
            let operand = self.lower_expr_node(element, span)?;
            let element_ty = self.operand_ty(&operand).unwrap_or(Ty::Unknown);
            element_types.push(element_ty);
            operands.push(operand);
        }

        let tuple_ty = TupleTy::new(element_types);
        self.ensure_ty_layout_for_ty(&Ty::Tuple(tuple_ty.clone()));

        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = Ty::Tuple(tuple_ty);
            local.is_nullable = false;
        }

        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Aggregate {
                    kind: AggregateKind::Tuple,
                    fields: operands,
                },
            },
        });

        Some(Operand::Copy(Place::new(temp)))
    }
    pub(crate) fn lower_interpolated_string(
        &mut self,
        interpolated: InterpolatedStringExpr,
        span: Option<Span>,
            ) -> Option<Operand> {
        let mut segments = Vec::with_capacity(interpolated.segments.len());
        for segment in interpolated.segments {
            match segment {
                ExprInterpolatedStringSegment::Text(text) => {
                    if text.is_empty() {
                        continue;
                    }
                    let id = self
                        .string_interner
                        .intern(&text, StrLifetime::Static, span);
                    segments.push(MirInterpolatedStringSegment::Text { id });
                }
                ExprInterpolatedStringSegment::Expr(InterpolatedExprSegment {
                    expr,
                    expr_text,
                    alignment,
                    format,
                    span: expr_span,
                                    }) => {
                    let operand = self.lower_expr_node(expr, expr_span.or(span))?;
                    let format_id = format.map(|fmt| {
                        self.string_interner
                            .intern(&fmt, StrLifetime::Static, expr_span.or(span))
                    });
                    segments.push(MirInterpolatedStringSegment::Expr {
                        operand,
                        alignment,
                        format: format_id,
                        expr_text,
                        span: expr_span.or(span),
                                            });
                }
            }
        }

        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = Ty::String;
        }
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::StringInterpolate { segments },
            },
        });
        Some(Operand::Copy(Place::new(temp)))
    }
}
