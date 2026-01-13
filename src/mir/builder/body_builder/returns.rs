use super::*;
use crate::mir::{
    ArcTy, ArrayTy, FnTy, GenericArg, RcTy, ReadOnlySpanTy, RefTy, SpanTy, TupleTy, VecTy,
};

body_builder_impl! {
    pub(crate) fn lower_return_statement(
        &mut self,
        statement: &AstStatement,
        expression: Option<&Expression>,
    ) {
        if expression.is_none() && self.opaque_return.is_some() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "opaque return type requires a concrete value".into(),
                span: statement.span,
            });
        }
        self.ensure_active_block();
        if expression.is_none() && matches!(self.function_kind, FunctionKind::Testcase) {
            let target_place = if self.is_async {
                self.ensure_async_result_local(statement.span).map(Place::new)
            } else {
                Some(Place::new(LocalId(0)))
            };
            if let Some(place) = target_place {
                let bool_true = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
                self.push_statement(MirStatement {
                    span: statement.span,
                    kind: MirStatementKind::Assign {
                        place,
                        value: Rvalue::Use(bool_true),
                    },
                });
            }
        }
        if let Some(expr) = expression {
            let target_place = if self.is_async {
                self.ensure_async_result_local(expr.span)
                    .map(Place::new)
                    .or_else(|| Some(Place::new(LocalId(0))))
            } else {
                Some(Place::new(LocalId(0)))
            };
            if let Some(place) = target_place {
                if let Some(mut operand) = self.lower_expression_operand(expr) {
                    if matches!(
                        &operand,
                        Operand::Pending(PendingOperand { repr, .. }) if repr == "throw"
                    ) {
                        return;
                    }
                    self.validate_lending_return_source(&operand, expr.span);
                    self.track_opaque_return_from_operand(&operand, expr.span);
                    let target_ty = if self.is_async {
                        self.async_result_ty()
                            .cloned()
                            .unwrap_or_else(|| self.return_type.clone())
                    } else {
                        self.return_type.clone()
                    };
                    operand = self.coerce_operand_to_ty(operand, &target_ty, false, expr.span);
                    self.push_statement(MirStatement {
                        span: expr.span,
                        kind: MirStatementKind::Assign {
                            place,
                            value: Rvalue::Use(operand),
                        },
                    });
                } else {
                    let pending = PendingRvalue {
                        repr: expr.text.clone(),
                        span: expr.span,
                    };
                    self.push_statement(MirStatement {
                        span: expr.span,
                        kind: MirStatementKind::Assign {
                            place,
                            value: Rvalue::Pending(pending),
                        },
                    });
                }
            }
        }

        self.drop_to_scope_depth(0, statement.span);
        self.set_terminator(statement.span, Terminator::Return);
    }

    fn track_opaque_return_from_operand(&mut self, operand: &Operand, span: Option<Span>) {
        if self.opaque_return.is_none() {
            return;
        }
        let Some(name) = self.operand_type_name(operand) else {
            if let Some(info) = self.opaque_return.as_mut() {
                info.unknown_spans.push(span);
            }
            return;
        };
        let concrete_ty = self.parse_ty_from_name(&name);
        let concrete_name = concrete_ty.canonical_name();
        let Some(info) = self.opaque_return.as_mut() else {
            return;
        };
        if let Some(existing) = &info.inferred {
            if existing != &concrete_name {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "opaque return type inferred as `{existing}` but return expression has `{concrete_name}`"
                    ),
                    span,
                });
            }
            return;
        }
        info.inferred = Some(concrete_name.clone());
        self.apply_opaque_return_ty(&concrete_ty);
        self.record_impl_trait_bounds(&concrete_name, span);
    }

    fn validate_lending_return_source(&mut self, operand: &Operand, span: Option<Span>) {
        if self.lending_return_params.is_empty() {
            return;
        }
        let Some(local) = self.return_operand_root_local(operand) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "return value must originate from {} due to `lends(...)` clause",
                    self.describe_lending_params()
                ),
                span,
            });
            return;
        };
        if self.lending_return_params.contains(&local) {
            return;
        }
        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "return value must originate from {} due to `lends(...)` clause",
                self.describe_lending_params()
            ),
            span,
        });
    }

    fn return_operand_root_local(&self, operand: &Operand) -> Option<LocalId> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => Some(place.local),
            Operand::Borrow(borrow) => Some(borrow.place.local),
            Operand::Mmio(_) | Operand::Const(_) | Operand::Pending(_) => None,
        }
    }

    fn describe_lending_params(&self) -> String {
        let names: Vec<String> = self
            .lending_return_params
            .iter()
            .filter_map(|local| {
                self.locals
                    .get(local.0)
                    .and_then(|decl| decl.name.clone())
                    .or_else(|| Some(format!("_{}", local.0)))
            })
            .collect();
        if names.is_empty() {
            "`lends(...)` sources".to_string()
        } else {
            names
                .into_iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    fn apply_opaque_return_ty(&mut self, concrete: &Ty) {
        let substituted = substitute_opaque_ty(&self.return_type, concrete);
        self.ensure_ty_layout_for_ty(&substituted);
        self.return_type = substituted.clone();
        if let Some(ret) = self.locals.first_mut() {
            ret.ty = substituted.clone();
        }
        if self.is_async {
            self.async_result_ty = task_result_ty(&self.return_type);
            if self.async_result_ty.is_none() && matches!(self.function_kind, FunctionKind::Testcase)
            {
                self.async_result_ty = Some(Ty::named("bool"));
            }
        }
    }

    fn parse_ty_from_name(&self, name: &str) -> Ty {
        parse_type_expression_text(name)
            .map(|expr| Ty::from_type_expr(&expr))
            .unwrap_or_else(|| Ty::named(name.to_string()))
    }

    fn record_impl_trait_bounds(&mut self, concrete_name: &str, span: Option<Span>) {
        let Some(info) = self.opaque_return.as_ref() else {
            return;
        };
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for bound in &info.bounds {
            if !seen.insert(bound.clone()) {
                continue;
            }
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::ImplTraitBound {
                    function: self.function_name.clone(),
                    opaque_ty: concrete_name.to_string(),
                    bound: bound.clone(),
                },
                span.or(info.declared_span).or(self.body.span),
            ));
        }
    }
}

fn substitute_opaque_ty(target: &Ty, concrete: &Ty) -> Ty {
    match target {
        Ty::TraitObject(obj) if obj.opaque_impl => concrete.clone(),
        Ty::Nullable(inner) => Ty::Nullable(Box::new(substitute_opaque_ty(inner, concrete))),
        Ty::Ref(reference) => Ty::Ref(Box::new(RefTy::new(
            substitute_opaque_ty(&reference.element, concrete),
            reference.readonly,
        ))),
        Ty::Array(array) => Ty::Array(ArrayTy::new(
            Box::new(substitute_opaque_ty(&array.element, concrete)),
            array.rank,
        )),
        Ty::Vec(vec) => Ty::Vec(VecTy::new(Box::new(substitute_opaque_ty(
            &vec.element,
            concrete,
        )))),
        Ty::Span(span) => Ty::Span(SpanTy::new(Box::new(substitute_opaque_ty(
            &span.element,
            concrete,
        )))),
        Ty::ReadOnlySpan(span) => Ty::ReadOnlySpan(ReadOnlySpanTy::new(Box::new(
            substitute_opaque_ty(&span.element, concrete),
        ))),
        Ty::Rc(rc) => Ty::Rc(RcTy::new(Box::new(substitute_opaque_ty(
            &rc.element,
            concrete,
        )))),
        Ty::Arc(arc) => Ty::Arc(ArcTy::new(Box::new(substitute_opaque_ty(
            &arc.element,
            concrete,
        )))),
        Ty::Tuple(tuple) => {
            let elements = tuple
                .elements
                .iter()
                .map(|elem| substitute_opaque_ty(elem, concrete))
                .collect();
            Ty::Tuple(TupleTy::with_names(elements, tuple.element_names.clone()))
        }
        Ty::Fn(fn_ty) => {
            let params = fn_ty
                .params
                .iter()
                .map(|param| substitute_opaque_ty(param, concrete))
                .collect();
            let ret = substitute_opaque_ty(fn_ty.ret.as_ref(), concrete);
            Ty::Fn(FnTy::with_modes(
                params,
                fn_ty.param_modes.clone(),
                ret,
                fn_ty.abi.clone(),
                fn_ty.variadic,
            ))
        }
        Ty::Named(named) => {
            if named.args().is_empty() {
                Ty::Named(named.clone())
            } else {
                let args = named
                    .args()
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(ty) => {
                            GenericArg::Type(substitute_opaque_ty(ty, concrete))
                        }
                        GenericArg::Const(value) => GenericArg::Const(value.clone()),
                    })
                    .collect();
                Ty::named_generic(named.name.clone(), args)
            }
        }
        _ => target.clone(),
    }
}
