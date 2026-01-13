use super::*;
use crate::mir::RefTy;
use std::collections::{HashSet, VecDeque};

body_builder_impl! {
    pub(super) fn operand_ty(&self, operand: &Operand) -> Option<Ty> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_ty(place),
            Operand::Const(constant) => {
                if let ConstValue::Symbol(name) = &constant.value {
                    if let Some(signature) = self.symbol_index.function_signature(name) {
                        return Some(Ty::Fn(signature.clone()));
                    }
                }
                let name = self.const_operand_type_from_const_operand(constant)?;
                Some(Ty::named(name))
            }
            Operand::Borrow(borrow) => {
                let base = self.place_ty(&borrow.place)?;
                let readonly = matches!(borrow.kind, BorrowKind::Shared);
                Some(Ty::Ref(Box::new(RefTy::new(base, readonly))))
            }
            Operand::Mmio(op) => Some(op.ty.clone()),
            Operand::Pending(_) => None,
        }
    }

    pub(super) fn ty_is_exception(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Named(_) => {
                let canonical = strip_generics(&ty.canonical_name());
                self.type_inherits_exception(&canonical)
            }
            Ty::Nullable(inner) => self.ty_is_exception(inner),
            Ty::Ref(reference) => self.ty_is_exception(&reference.element),
            _ => false,
        }
    }

    fn type_inherits_exception(&self, canonical: &str) -> bool {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back(canonical.to_string());
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let stripped = current.trim_end_matches('?');
            let normalized = stripped.replace('.', "::");
            if matches!(
                normalized.as_str(),
                "Exception" | "Std::Exception" | "System::Exception"
            ) || normalized.ends_with("Exception")
            {
                return true;
            }
            if let Some(info) = self
                .type_layouts
                .class_layout_info(normalized.as_str())
            {
                if info.kind == ClassLayoutKind::Error {
                    return true;
                }
                for base in &info.bases {
                    queue.push_back(base.replace('.', "::"));
                }
            }

            if let Some(bases) = self.class_bases.get(normalized.as_str()) {
                for base in bases {
                    queue.push_back(base.replace('.', "::"));
                }
                continue;
            }

            if let Some(descriptor) = self
                .symbol_index
                .reflection_descriptor(stripped)
                .or_else(|| self.symbol_index.reflection_descriptor(normalized.as_str()))
                .or_else(|| {
                    normalized
                        .rsplit("::")
                        .next()
                        .and_then(|short| self.symbol_index.reflection_descriptor(short))
                })
            {
                for base in &descriptor.bases {
                    queue.push_back(base.name.replace('.', "::"));
                }
            }
        }
        false
    }

    fn validate_throw_operand(&mut self, span: Option<Span>, operand: &Operand) -> Option<Ty> {
        let ty = self.operand_ty(operand)?;
        if matches!(ty, Ty::Unknown) {
            return None;
        }
        if !self.ty_is_exception(&ty) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "throw operand of type `{}` does not derive from `Exception`",
                    ty.canonical_name()
                ),
                span,
                            });
            return None;
        }
        if self
            .operand_is_nullable(operand)
            .is_some_and(|nullable| nullable)
        {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "throw operand of type `{}` may be `null`; guard the value or provide a non-null default before throwing",
                    ty.canonical_name()
                ),
                span,
                            });
            return None;
        }
        Some(ty)
    }

    pub(super) fn lower_throw_statement(
        &mut self,
        statement: &AstStatement,
        expression: Option<&Expression>,
    ) {
        let operand = if let Some(expr) = expression {
            match self.lower_expression_operand(expr) {
                Some(op) => Some(op),
                None => {
                    self.push_pending(statement, PendingStatementKind::Throw);
                    return;
                }
            }
        } else {
            None
        };

        if !self.emit_throw(statement.span, operand) {
            self.push_pending(statement, PendingStatementKind::Throw);
        }
    }

    pub(super) fn emit_throw(
        &mut self,
        span: Option<Span>,
        mut operand: Option<Operand>,
    ) -> bool {
        if let Some(Operand::Copy(place)) = operand.as_ref() {
            if place.projection.is_empty() {
                operand = Some(Operand::Move(place.clone()));
            }
        }

        let operand_ty = operand
            .as_ref()
            .and_then(|op| self.validate_throw_operand(span, op));
        if operand.is_some() && operand_ty.is_none() {
            return false;
        }

        if let Some(Operand::Move(place)) = operand.as_ref() {
            if place.projection.is_empty() {
                self.mark_fallible_handled(place.local, span);
            }
        }

        if let Some(context) = self.try_stack.last().copied() {
            if let Some(value) = operand {
                self.push_statement(MirStatement {
                    span,
                                        kind: MirStatementKind::Assign {
                        place: Place::new(context.exception_local),
                        value: Rvalue::Use(value),
                    },
                });
            }

            if let Some(flag) = context.exception_flag {
                self.assign_bool(flag, true, span);
            }

            self.drop_to_scope_depth(context.scope_depth, span);

            let target = context
                .finally_entry
                .or(context.dispatch_block)
                .or(context.unhandled_block);
            if let Some(target) = target {
                self.set_terminator(span, Terminator::Goto { target });
                true
            } else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "throw requires a catch or finally handler".into(),
                    span,
                                    });
                false
            }
        } else {
            if operand.is_none() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "cannot rethrow outside a catch block".into(),
                    span,
                                    });
                return false;
            }
            let mut final_operand = operand.expect("throw operand is present");
            if let Operand::Move(place) = &final_operand {
                if place.projection.is_empty() {
                    let moved_local = place.local;
                    let temp = self.create_temp_untracked(span);
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::StorageLive(temp),
                    });
                    if let Some(ty) = operand_ty.as_ref() {
                        self.hint_local_ty(temp, ty.clone());
                    }
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: Place::new(temp),
                            value: Rvalue::Use(final_operand),
                        },
                    });
                    self.mark_local_dead(moved_local);
                    final_operand = Operand::Move(Place::new(temp));
                }
            }

            self.drop_to_scope_depth(0, span);
            self.set_terminator(
                span,
                Terminator::Throw {
                    exception: Some(final_operand),
                    ty: operand_ty,
                },
            );
            true
        }
    }
}

fn strip_generics(name: &str) -> String {
    let mut depth = 0i32;
    let mut result = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '<' => depth += 1,
            '>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ => {
                if depth == 0 {
                    result.push(ch);
                }
            }
        }
    }
    result.trim().to_string()
}
