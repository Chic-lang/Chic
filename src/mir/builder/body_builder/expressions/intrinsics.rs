use super::*;

body_builder_impl! {
    pub(crate) fn lower_throw_expr(
        &mut self,
        expr: Option<Box<ExprNode>>,
        span: Option<Span>,
            ) -> Option<Operand> {
        let operand = if let Some(inner) = expr {
            Some(self.lower_expr_node(*inner, span)?)
        } else {
            None
        };

        if !self.emit_throw(span, operand) {
            return None;
        }

        Some(Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: "throw".into(),
            span,
                        info: None,
        }))
    }
    pub(crate) fn lower_sizeof_expr(
        &mut self,
        operand: SizeOfOperand,
        span: Option<Span>,
            ) -> Option<Operand> {
        match operand {
            SizeOfOperand::Type(text) => self.sizeof_type_operand(&text, span),
            SizeOfOperand::Value(expr) => self.sizeof_value_operand(*expr, span),
        }
    }
    pub(crate) fn lower_alignof_expr(
        &mut self,
        operand: SizeOfOperand,
        span: Option<Span>,
            ) -> Option<Operand> {
        match operand {
            SizeOfOperand::Type(text) => self.alignof_type_operand(&text, span),
            SizeOfOperand::Value(expr) => self.alignof_value_operand(*expr, span),
        }
    }
    pub(crate) fn sizeof_type_operand(&mut self, text: &str, span: Option<Span>) -> Option<Operand> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "expected type name inside `sizeof`".into(),
                span,
                            });
            return None;
        }

        let Some(type_expr) = parse_type_expression_text(trimmed) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{trimmed}` is not a valid type for `sizeof`"),
                span,
                            });
            return None;
        };

        let ty = Ty::from_type_expr(&type_expr);
        self.ensure_ty_layout_for_ty(&ty);
        self.size_for_ty(&ty, span)
    }
    pub(crate) fn sizeof_value_operand(&mut self, expr: ExprNode, span: Option<Span>) -> Option<Operand> {
        match expr {
            ExprNode::Identifier(name) => {
                let Some(local_id) = self.lookup_name(&name) else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("unknown identifier `{name}` in `sizeof` expression"),
                        span,
                                            });
                    return None;
                };
                let Some(local) = self.locals.get(local_id.0) else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`{name}` is not available in this scope"),
                        span,
                                            });
                    return None;
                };
                let ty = local.ty.clone();
                self.size_for_ty(&ty, span)
            }
            ExprNode::Parenthesized(inner) => self.sizeof_value_operand(*inner, span),
            other => {
                let repr = Self::expr_to_string(&other);
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`sizeof` expects a type or local variable, found `{repr}`"
                    ),
                    span,
                                    });
                None
            }
        }
    }
    pub(crate) fn size_for_ty(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        let current_type = self.current_self_type_name();
        match type_size_and_align_for_ty(
            ty,
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
        ) {
            Some((size, _align)) => Some(Operand::Const(ConstOperand::new(ConstValue::UInt(
                size as u128,
            )))),
            None => {
                if self.type_param_name_for_ty(ty).is_some() {
                    return self.runtime_type_size_operand(ty, span);
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot determine size for type `{}`",
                        ty.canonical_name()
                    ),
                    span,
                                    });
                None
            }
        }
    }
    pub(crate) fn alignof_type_operand(&mut self, text: &str, span: Option<Span>) -> Option<Operand> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "expected type name inside `alignof`".into(),
                span,
                            });
            return None;
        }

        let Some(type_expr) = parse_type_expression_text(trimmed) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{trimmed}` is not a valid type for `alignof`"),
                span,
                            });
            return None;
        };

        let ty = Ty::from_type_expr(&type_expr);
        self.ensure_ty_layout_for_ty(&ty);
        self.align_for_ty(&ty, span)
    }
    pub(crate) fn alignof_value_operand(&mut self, expr: ExprNode, span: Option<Span>) -> Option<Operand> {
        match expr {
            ExprNode::Identifier(name) => {
                let Some(local_id) = self.lookup_name(&name) else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("unknown identifier `{name}` in `alignof` expression"),
                        span,
                                            });
                    return None;
                };
                let Some(local) = self.locals.get(local_id.0) else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`{name}` is not available in this scope"),
                        span,
                                            });
                    return None;
                };
                let ty = local.ty.clone();
                self.align_for_ty(&ty, span)
            }
            ExprNode::Parenthesized(inner) => self.alignof_value_operand(*inner, span),
            other => {
                let repr = Self::expr_to_string(&other);
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`alignof` expects a type or local variable, found `{repr}`"
                    ),
                    span,
                                    });
                None
            }
        }
    }
    pub(crate) fn align_for_ty(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        let current_type = self.current_self_type_name();
        match type_size_and_align_for_ty(
            ty,
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
        ) {
            Some((_size, align)) => Some(Operand::Const(ConstOperand::new(ConstValue::UInt(
                align as u128,
            )))),
            None => {
                if self.type_param_name_for_ty(ty).is_some() {
                    return self.runtime_type_align_operand(ty, span);
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot determine alignment for type `{}`",
                        ty.canonical_name()
                    ),
                    span,
                                    });
                None
            }
        }
    }
}
