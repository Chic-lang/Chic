use super::*;
use crate::frontend::ast::CasePattern;
use crate::mir::builder::body_builder::switch::SwitchBindingLocal;
use crate::syntax::expr::SwitchExpr;
use std::collections::HashMap;

body_builder_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Switch expression lowering coordinates guards, bindings, and result assignment."
    )]
    pub(crate) fn lower_switch_expr(
        &mut self,
        switch_expr: SwitchExpr,
        span: Option<Span>,
    ) -> Option<Operand> {
        self.push_scope();
        let discr_operand = self.lower_expr_node(*switch_expr.value, switch_expr.span.or(span))?;
        if matches!(self.operand_ty(&discr_operand), Some(Ty::Pointer(_))) {
            self.diagnostics.push(LoweringDiagnostic {
                message: "patterns on pointer operands are not supported".into(),
                span: switch_expr.span.or(span),
            });
            self.pop_scope();
            return None;
        }

        let discr_local = self.ensure_operand_local(discr_operand, switch_expr.span.or(span));
        let match_binding = self.register_match_binding(discr_local);
        let mut binding_locals: HashMap<String, SwitchBindingLocal> = HashMap::new();

        let result_local = self.create_temp(switch_expr.span.or(span));
        let join_block = self.new_block(switch_expr.span.or(span));

        let mut cases = Vec::new();
        for arm in &switch_expr.arms {
            let pattern_text = Self::pattern_ast_to_string(&arm.pattern);
            let mut pattern_expr = Expression::new(pattern_text, arm.pattern.span.or(arm.span));
            if pattern_expr.span.is_none() {
                pattern_expr.span = arm.span.or(switch_expr.span).or(span);
            }
            let case_pattern = CasePattern::new(pattern_expr, Some(arm.pattern.clone()));

            let parsed = match self.parse_case_pattern(&case_pattern, &match_binding) {
                Ok(parsed) => parsed,
                Err(diag) => {
                    self.diagnostics.push(diag);
                    continue;
                }
            };

            let ParsedCasePattern {
                kind,
                key: _,
                pre_guards,
                post_guards,
                bindings: parsed_bindings,
                list_plan,
            } = parsed;

            let mut pre_guard_meta = Vec::new();
            for guard_expression in pre_guards {
                let node = guard_expression
                    .node
                    .clone()
                    .or_else(|| self.expression_node(&guard_expression));
                pre_guard_meta.push(GuardMetadata {
                    expr: guard_expression,
                    node,
                });
            }

            let mut guard_meta = Vec::new();
            for guard_expression in post_guards {
                let node = guard_expression
                    .node
                    .clone()
                    .or_else(|| self.expression_node(&guard_expression));
                guard_meta.push(GuardMetadata {
                    expr: guard_expression,
                    node,
                });
            }
            for guard in &arm.guards {
                let expr_text = Self::expr_to_string(&guard.expr);
                let guard_expr = Expression::with_node(
                    expr_text,
                    guard.span.or(arm.span).or(switch_expr.span).or(span),
                    guard.expr.clone(),
                );
                guard_meta.push(GuardMetadata {
                    expr: guard_expr,
                    node: Some(guard.expr.clone()),
                });
            }

            let mut case_bindings = parsed_bindings;
            if case_bindings.is_empty() {
                if let CasePatternKind::Complex(pattern) = &kind {
                    case_bindings.extend(Self::extract_pattern_bindings(
                        pattern,
                        case_pattern
                            .raw
                            .span
                            .or(arm.span)
                            .or(switch_expr.span)
                            .or(span),
                    ));
                }
            }

            let mut resolved_bindings = Vec::new();
            for spec in case_bindings {
                let BindingSpec {
                    name,
                    projection,
                    span: binding_span,
                    mutability,
                    mode,
                } = spec;
                let span = binding_span
                    .or(case_pattern.raw.span)
                    .or(arm.span)
                    .or(switch_expr.span)
                    .or(span);
                let mutable_flag = matches!(mutability, PatternBindingMutability::Mutable);
                let local_id = match binding_locals.entry(name.clone()) {
                    std::collections::hash_map::Entry::Occupied(entry) => {
                        let existing = entry.get();
                        if existing.mode != mode {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "pattern binding `{}` must use the same borrow mode across arms",
                                    name
                                ),
                                span,
                            });
                        }
                        if existing.mutability != mutability {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "pattern binding `{}` cannot switch between `let` and `var` across arms",
                                    name
                                ),
                                span,
                            });
                        }
                        existing.local
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        let decl = LocalDecl::new(
                            Some(name.clone()),
                            Ty::Unknown,
                            mutable_flag,
                            span,
                            LocalKind::Local,
                        );
                        let id = self.push_local(decl);
                        entry.insert(SwitchBindingLocal {
                            local: id,
                            mutability,
                            mode,
                        });
                        self.bind_name(&name, id);
                        id
                    }
                };

                resolved_bindings.push(PatternBinding {
                    name,
                    local: local_id,
                    projection,
                    span,
                    mutability,
                    mode,
                });
            }

            let arm_block = self.new_block(arm.span.or(switch_expr.span).or(span));
            cases.push(SwitchCase {
                pattern: kind.clone(),
                pre_guards: pre_guard_meta,
                guards: guard_meta,
                body_block: arm_block,
                span: arm.span.or(switch_expr.span).or(span),
                pattern_span: case_pattern.raw.span,
                bindings: resolved_bindings,
                list_plan,
            });
        }

        if cases.is_empty() {
            self.pop_scope();
            return None;
        }

        let exhaustive = cases
            .iter()
            .any(|case| matches!(case.pattern, CasePatternKind::Wildcard));
        let fallback_block = if exhaustive {
            join_block
        } else {
            self.new_block(switch_expr.span.or(span))
        };

        self.lower_switch_as_match(
            discr_local,
            &cases,
            fallback_block,
            switch_expr.span.or(span),
        );

        for (case, arm) in cases.iter().zip(switch_expr.arms.iter()) {
            self.switch_to_block(case.body_block);
            self.push_scope();
            let mut value_operand =
                match self.lower_expr_node(arm.expression.clone(), arm.span.or(span)) {
                    Some(op) => op,
                    None => {
                        self.pop_scope();
                        continue;
                    }
                };
            if let Some(target_ty) = self
                .locals
                .get(result_local.0)
                .and_then(|decl| (!matches!(decl.ty, Ty::Unknown)).then(|| decl.ty.clone()))
            {
                value_operand = self.coerce_operand_to_ty(
                    value_operand,
                    &target_ty,
                    false,
                    arm.span.or(span),
                );
            } else if let Some(ty) = self.operand_ty(&value_operand) {
                if let Some(local) = self.locals.get_mut(result_local.0) {
                    if matches!(local.ty, Ty::Unknown) {
                        local.ty = ty.clone();
                        local.is_nullable = matches!(ty, Ty::Nullable(_));
                    }
                }
            }
            self.push_statement(MirStatement {
                span: arm.span.or(span),
                kind: MirStatementKind::Assign {
                    place: Place::new(result_local),
                    value: Rvalue::Use(value_operand),
                },
            });
            self.ensure_goto(join_block, arm.span.or(span));
            self.pop_scope();
        }

        self.switch_to_block(fallback_block);
        if !exhaustive {
            self.diagnostics.push(LoweringDiagnostic {
                message: "non-exhaustive switch expression requires a `_`/`default` arm".into(),
                span: switch_expr.span.or(span),
            });
            self.set_terminator(span, Terminator::Unreachable);
        }
        self.switch_to_block(join_block);
        self.pop_scope();
        Some(Operand::Copy(Place::new(result_local)))
    }
}
