use super::*;
use crate::mir::AsyncFrameField;
use crate::mir::async_types::{future_result_ty, task_result_ty};
use crate::typeck::AutoTraitConstraintOrigin;

body_builder_impl! {
    pub(super) fn lower_await_expr(
        &mut self,
        expr: ExprNode,
        span: Option<Span>,
            ) -> Option<Operand> {
        if !self.is_async {
            self.diagnostics.push(LoweringDiagnostic {
                message: "await is only allowed inside async functions or testcases".into(),
                span,
                            });
            return None;
        }
        if self.lock_depth > 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot await while a lock guard is active".into(),
                span,
                            });
            return None;
        }
        if self.unsafe_depth > 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: "await is not permitted inside unsafe blocks".into(),
                span,
                            });
            return None;
        }

        let awaited_operand = self.lower_expr_node(expr, span)?;
        let future_local = self.ensure_operand_local(awaited_operand, span);
        self.record_async_suspension();
        Some(self.emit_await_terminator(future_local, span))
    }

    pub(super) fn finalize_async_state(&mut self) {
        let suspend_points = std::mem::take(&mut self.suspend_points);
        if !self.is_async {
            return;
        }

        let pinned_locals = self
            .locals
            .iter()
            .enumerate()
            .filter_map(|(idx, decl)| decl.is_pinned.then_some(LocalId(idx)))
            .collect::<Vec<_>>();
        let cross_locals = self.async_cross_locals.iter().copied().collect::<Vec<_>>();
        let frame_fields = self
            .async_cross_locals
            .iter()
            .filter_map(|local| {
                let decl = self.locals.get(local.0)?;
                Some(AsyncFrameField {
                    local: *local,
                    name: decl.name.clone(),
                    ty: decl.ty.clone(),
                })
            })
            .collect::<Vec<_>>();
        self.body.async_machine = Some(AsyncStateMachine {
            suspend_points,
            pinned_locals,
            cross_locals,
            frame_fields,
            result_local: self.async_result_local,
            result_ty: self.async_result_ty.clone(),
            context_local: self.async_context_local,
            policy: self.async_policy.clone(),
        });
        if !self.async_cross_locals.is_empty() {
            self.emit_async_trait_constraints();
        }
    }

    pub(super) fn finalize_generator_state(&mut self) {
        let generator_points = std::mem::take(&mut self.generator_points);
        if self.is_generator || !generator_points.is_empty() {
            self.body.generator = Some(GeneratorStateMachine {
                yields: generator_points,
            });
        }
    }

    pub(super) fn lower_yield_return(&mut self, statement: &AstStatement, expression: &Expression) {
        self.is_generator = true;

        let Some(value_operand) = self.lower_expression_operand(expression) else {
            self.push_pending(statement, PendingStatementKind::YieldReturn);
            return;
        };

        let value_local = self.ensure_operand_local(value_operand, expression.span);
        let yield_block = self.current_block;
        let resume_block = self.new_block(statement.span);
        let drop_block = self.new_block(statement.span);

        self.set_terminator(
            expression.span,
            Terminator::Yield {
                value: Operand::Move(Place::new(value_local)),
                resume: resume_block,
                drop: drop_block,
            },
        );

        self.generator_points.push(GeneratorYieldPoint {
            id: self.next_yield_id,
            yield_block,
            resume_block,
            drop_block,
            value: Some(value_local),
            span: expression.span,
                    });
        self.next_yield_id += 1;

        if self.blocks[drop_block.0].terminator.is_none() {
            self.blocks[drop_block.0].terminator = Some(Terminator::Return);
        }
        self.switch_to_block(resume_block);
        self.mark_local_dead(value_local);
    }

    pub(super) fn lower_yield_break(&mut self, statement: &AstStatement) {
        self.is_generator = true;
        self.drop_to_scope_depth(0, statement.span);
        self.set_terminator(statement.span, Terminator::Return);
    }

    fn emit_await_terminator(&mut self, future_local: LocalId, span: Option<Span>) -> Operand {
        let result_local = self.create_temp(span);
        if let Some(future_decl) = self.locals.get(future_local.0) {
            let ty = future_result_ty(&future_decl.ty)
                .or_else(|| task_result_ty(&future_decl.ty))
                .unwrap_or(Ty::Unit);
            if let Some(dest_decl) = self.locals.get_mut(result_local.0) {
                dest_decl.ty = ty;
            }
        }
        let await_block = self.current_block;
        let resume_block = self.new_block(span);
        let drop_block = self.new_block(span);
        self.set_terminator(
            span,
                        Terminator::Await {
                future: Place::new(future_local),
                destination: Some(Place::new(result_local)),
                resume: resume_block,
                drop: drop_block,
            },
        );
        self.suspend_points.push(AsyncSuspendPoint {
            id: self.next_suspend_id,
            await_block,
            resume_block,
            drop_block,
            future: future_local,
            destination: Some(result_local),
            span,
        });
        self.next_suspend_id += 1;
        if self.blocks[drop_block.0].terminator.is_none() {
            self.blocks[drop_block.0].terminator = Some(Terminator::Return);
        }
        self.switch_to_block(resume_block);
        Operand::Copy(Place::new(result_local))
    }

    fn record_async_suspension(&mut self) {
        let mut candidates = Vec::new();
        for scope in &self.scopes {
            for entry in &scope.locals {
                if entry.live {
                    candidates.push(entry.local);
                }
            }
        }
        for (index, decl) in self.locals.iter().enumerate() {
            if matches!(decl.kind, LocalKind::Arg(_)) {
                candidates.push(LocalId(index));
            }
        }

        for local in candidates {
            if let Some(decl) = self.locals.get(local.0) {
                if decl.is_pinned {
                    continue;
                }
                match decl.kind {
                    LocalKind::Return | LocalKind::Temp => {}
                    _ => {
                        self.async_cross_locals.insert(local);
                    }
                }
            }
        }
    }

fn emit_async_trait_constraints(&mut self) {
        for local in &self.async_cross_locals {
            if local.0 == 0 {
                continue;
            }
            let Some(decl) = self.locals.get(local.0) else {
                continue;
            };
            if decl.is_pinned {
                continue;
            }
            if matches!(decl.kind, LocalKind::Temp | LocalKind::Return) {
                continue;
            }
            let Some(ty_name) = self
                .resolve_ty_name(&decl.ty)
                .or_else(|| Self::canonical_constraint_type_name(&decl.ty))
            else {
                continue;
            };
            if ty_name.is_empty() {
                continue;
            }
            let target = decl.name.clone().unwrap_or_else(|| format!("_{}", local.0));
            let trait_kind = Self::auto_trait_for_local(decl);
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::RequiresAutoTrait {
                    function: self.function_name.clone(),
                    target,
                    ty: ty_name.clone(),
                    trait_kind,
                    origin: AutoTraitConstraintOrigin::AsyncSuspend,
                },
                decl.span,
            ));
        }
    }

    fn auto_trait_for_local(decl: &LocalDecl) -> AutoTraitKind {
        if matches!(decl.param_mode, Some(ParamMode::In)) {
            return AutoTraitKind::Shareable;
        }
        if let Some(kind) = Self::borrow_trait_from_ty(&decl.ty) {
            return kind;
        }
        AutoTraitKind::ThreadSafe
    }

    fn borrow_trait_from_ty(ty: &Ty) -> Option<AutoTraitKind> {
        match ty {
            Ty::Ref(ref_ty) => Some(if ref_ty.readonly {
                AutoTraitKind::Shareable
            } else {
                AutoTraitKind::ThreadSafe
            }),
            Ty::Nullable(inner) => Self::borrow_trait_from_ty(inner),
            _ => None,
        }
    }

    fn canonical_constraint_type_name(ty: &Ty) -> Option<String> {
        match ty {
            Ty::Ref(reference) => Self::canonical_constraint_type_name(&reference.element),
            Ty::Nullable(inner) => Self::canonical_constraint_type_name(inner),
            _ => {
                let name = ty.canonical_name();
                if name.is_empty() || name == "<unknown>" {
                    None
                } else {
                    Some(name)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::RefTy;

    fn local_decl(name: &str, ty: Ty) -> LocalDecl {
        LocalDecl::new(Some(name.to_string()), ty, false, None, LocalKind::Local)
    }

    #[test]
    fn auto_trait_uses_param_mode_for_in_parameters() {
        let mut decl = local_decl("borrow", Ty::named("Demo"));
        decl.param_mode = Some(ParamMode::In);
        assert!(matches!(
            BodyBuilder::auto_trait_for_local(&decl),
            AutoTraitKind::Shareable
        ));
    }

    #[test]
    fn auto_trait_treats_ref_readonly_locals_as_shareable() {
        let ref_ty = RefTy {
            element: Ty::named("string"),
            readonly: true,
        };
        let decl = local_decl("alias", Ty::Ref(Box::new(ref_ty)));
        assert!(matches!(
            BodyBuilder::auto_trait_for_local(&decl),
            AutoTraitKind::Shareable
        ));
    }

    #[test]
    fn auto_trait_requires_threadsafe_for_mutable_refs() {
        let ref_ty = RefTy {
            element: Ty::named("string"),
            readonly: false,
        };
        let decl = local_decl("alias", Ty::Ref(Box::new(ref_ty)));
        assert!(matches!(
            BodyBuilder::auto_trait_for_local(&decl),
            AutoTraitKind::ThreadSafe
        ));
    }
}
