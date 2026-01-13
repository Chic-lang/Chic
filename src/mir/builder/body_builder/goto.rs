use super::*;
use crate::frontend::ast::{CasePattern, PatternGuard};

body_builder_impl! {
    pub(super) fn lower_goto_statement(&mut self, statement: &AstStatement, goto: &GotoStatement) {
        match &goto.target {
            GotoTarget::Case { pattern, guards } => {
                let target = self.resolve_goto_case(pattern, guards, statement.span);
                self.apply_switch_goto(statement, target, "`goto case`");
            }
            GotoTarget::Default => {
                let target = self.resolve_goto_default(statement.span);
                self.apply_switch_goto(statement, target, "`goto default`");
            }
            GotoTarget::Label(label) => self.lower_goto_label(statement, label),
        }
    }

    fn apply_switch_goto(
        &mut self,
        statement: &AstStatement,
        target: Option<SwitchTarget>,
        kind: &str,
    ) {
        if let Some(target) = target {
            let source_depth = self.scope_depth();
            if target.scope_depth > source_depth {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("{kind} cannot jump into a nested scope"),
                    span: statement.span,
                                    });
            }
            let snapshot = self.capture_scope_snapshot();
            let drops =
                Self::compute_drops_from_snapshot(&snapshot, target.scope_depth, statement.span);
            self.emit_goto_drop_statements(self.current_block, drops);
            self.set_terminator(
                statement.span,
                Terminator::Goto {
                    target: target.block,
                },
            );
            let next_block = self.new_block(statement.span);
            self.switch_to_block(next_block);
        } else {
            self.push_pending(statement, PendingStatementKind::Goto);
        }
    }

    fn lower_goto_label(&mut self, statement: &AstStatement, label: &str) {
        let source_depth = self.scope_depth();
        let snapshot = self.capture_scope_snapshot();

        let (target_block, label_defined, label_depth) =
            if let Some(state) = self.labels.get_mut(label) {
                state.span = state.span.or(statement.span);
                (state.block, state.defined, state.scope_depth)
            } else {
                let placeholder = self.new_block(statement.span);
                self.labels.insert(
                    label.to_string(),
                    LabelState {
                        block: placeholder,
                        scope_depth: 0,
                        defined: false,
                        span: statement.span,
                                            },
                );
                (placeholder, false, 0)
            };

        if label_defined {
            if label_depth > source_depth {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("`goto {label}` cannot jump into a nested scope"),
                    span: statement.span,
                                    });
            }
            let drops = Self::compute_drops_from_snapshot(&snapshot, label_depth, statement.span);
            self.emit_goto_drop_statements(self.current_block, drops);
        } else {
            self.pending_gotos
                .entry(label.to_string())
                .or_default()
                .push(PendingGoto {
                    block: self.current_block,
                    span: statement.span,
                                        source_depth,
                    scope_snapshot: snapshot,
                });
        }

        self.set_terminator(
            statement.span,
            Terminator::Goto {
                target: target_block,
            },
        );
        let next_block = self.new_block(statement.span);
        self.switch_to_block(next_block);
    }

    fn resolve_pending_goto(
        &mut self,
        label: &str,
        target_block: BlockId,
        label_depth: usize,
        pending: &PendingGoto,
    ) {
        if label_depth > pending.source_depth {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`goto {label}` cannot jump into a nested scope"),
                span: pending.span,
                            });
        }

        let drops =
            Self::compute_drops_from_snapshot(&pending.scope_snapshot, label_depth, pending.span);
        self.emit_goto_drop_statements(pending.block, drops);
        if let Some(block_ref) = self.blocks.get_mut(pending.block.0) {
            block_ref.terminator = Some(Terminator::Goto {
                target: target_block,
            });
        }
    }

    pub(super) fn lower_labeled_statement(&mut self, span: Option<Span>, label: &str, inner: &AstStatement) {
        let scope_depth = self.scope_depth();
        let block_id;
        if let Some(state) = self.labels.get_mut(label) {
            if state.defined {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("duplicate label `{label}`"),
                    span,
                                    });
            }
            state.defined = true;
            state.scope_depth = scope_depth;
            state.span = state.span.or(span);
            block_id = state.block;
        } else {
            let block = if self.blocks[self.current_block.0].terminator.is_none()
                && self.blocks[self.current_block.0].statements.is_empty()
            {
                self.current_block
            } else {
                let new_block = self.new_block(span);
                self.switch_to_block(new_block);
                new_block
            };
            self.labels.insert(
                label.to_string(),
                LabelState {
                    block,
                    scope_depth,
                    defined: true,
                    span,
                                    },
            );
            block_id = block;
        }

        if self.current_block != block_id {
            self.switch_to_block(block_id);
        }

        if let Some(pendings) = self.pending_gotos.remove(label) {
            for pending in pendings {
                self.resolve_pending_goto(label, block_id, scope_depth, &pending);
            }
        }

        self.lower_statement(inner);
    }

    fn resolve_goto_case(
        &mut self,
        pattern: &CasePattern,
        guards: &[PatternGuard],
        span: Option<Span>,
    ) -> Option<SwitchTarget> {
        let Some(binding_name) = self
            .current_switch_context()
            .map(|ctx| ctx.binding_name.clone())
        else {
            self.report_switch_only(
                "`goto case` is only valid inside a switch statement",
                &pattern.raw,
                span,
                            );
            return None;
        };

        for guard_expr in guards {
            let expr = &guard_expr.expression;
            if !expr.text.trim().is_empty() {
                self.report_switch_only(
                    "`goto case` cannot include a `when` guard",
                    expr,
                    expr.span.or(span),
                );
                return None;
            }
        }

        let parsed = match self.parse_case_pattern(pattern, &binding_name) {
            Ok(pattern) => pattern,
            Err(diag) => {
                self.diagnostics.push(diag);
                return None;
            }
        };

        if !parsed.pre_guards.is_empty() || !parsed.post_guards.is_empty() {
            self.report_switch_only(
                "`goto case` does not support list patterns",
                &pattern.raw,
                span,
                            );
            return None;
        }

        let Some(key) = parsed.key else {
            self.report_switch_only(
                "`goto case` requires a resolvable pattern target",
                &pattern.raw,
                span,
                            );
            return None;
        };

        let text = pattern.raw.text.trim();
        self.lookup_case_target(&key, &pattern.raw, span, text)
    }

    fn resolve_goto_default(&mut self, span: Option<Span>) -> Option<SwitchTarget> {
        let Some(ctx) = self.current_switch_context() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`goto default` is only valid inside a switch statement".into(),
                span,
                            });
            return None;
        };

        if let Some(target) = ctx.default_target {
            Some(target)
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`goto default` requires a `default` label in the switch".into(),
                span,
                            });
            None
        }
    }

    fn lookup_case_target(
        &mut self,
        key: &str,
        expr: &Expression,
        span: Option<Span>,
                text: &str,
    ) -> Option<SwitchTarget> {
        let Some(ctx) = self.current_switch_context() else {
            self.report_switch_only(
                "`goto case` is only valid inside a switch statement",
                expr,
                span,
                            );
            return None;
        };
        if let Some(target) = ctx.label_map.get(key) {
            if target.allows_goto {
                return Some(*target);
            }
            self.report_switch_only(
                "`goto case` cannot target a pattern with a `when` guard",
                expr,
                span,
                            );
            None
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("no `case` label for pattern `{text}`"),
                span: expr.span.or(span),
                            });
            None
        }
    }

    fn report_switch_only(&mut self, message: &str, expr: &Expression, span: Option<Span>) {
        self.diagnostics.push(LoweringDiagnostic {
            message: message.into(),
            span: expr.span.or(span),
                    });
    }

    fn capture_scope_snapshot(&self) -> Vec<ScopeSnapshot> {
        self.scopes
            .iter()
            .enumerate()
            .map(|(index, frame)| ScopeSnapshot {
                depth: index + 1,
                locals: frame
                    .locals
                    .iter()
                    .filter(|entry| entry.live)
                    .map(|entry| ScopeLocalSnapshot {
                        local: entry.local,
                        span: entry.span,
                                            })
                    .collect(),
            })
            .collect()
    }

    pub(super) fn mark_local_dead(&mut self, local: LocalId) {
        for frame in self.scopes.iter_mut().rev() {
            if let Some(entry) = frame
                .locals
                .iter_mut()
                .rev()
                .find(|entry| entry.local == local)
            {
                if entry.live {
                    entry.live = false;
                }
                break;
            }
        }
    }

    fn compute_drops_from_snapshot(
        snapshot: &[ScopeSnapshot],
        target_depth: usize,
        span: Option<Span>,
            ) -> Vec<(LocalId, Option<Span>)> {
        let mut drops = Vec::new();
        for scope in snapshot.iter().rev() {
            if scope.depth > target_depth {
                for local in scope.locals.iter().rev() {
                    let drop_span = local.span.or(span);
                    drops.push((local.local, drop_span));
                }
            }
        }
        drops
    }

    fn emit_goto_drop_statements(&mut self, block: BlockId, drops: Vec<(LocalId, Option<Span>)>) {
        if drops.is_empty() {
            return;
        }

        for (local, _) in &drops {
            self.mark_local_dead(*local);
        }

        if let Some(block_ref) = self.blocks.get_mut(block.0) {
            for (local, drop_span) in drops {
                block_ref.statements.push(MirStatement {
                    span: drop_span,
                                        kind: MirStatementKind::StorageDead(local),
                });
            }
        }
    }
}
