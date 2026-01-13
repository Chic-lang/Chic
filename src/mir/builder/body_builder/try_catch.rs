use super::*;
use crate::frontend::ast::{Block, CatchClause};

#[derive(Clone, Copy)]
pub(super) struct CatchBlockInfo {
    pub clause_index: usize,
    pub entry: BlockId,
    pub body: BlockId,
    pub cleanup: BlockId,
}

body_builder_impl! {
    pub(super) fn allocate_catch_blocks(&mut self, catches: &[CatchClause]) -> Vec<CatchBlockInfo> {
        catches
            .iter()
            .enumerate()
            .map(|(index, clause)| CatchBlockInfo {
                clause_index: index,
                entry: self.new_block(clause.body.span),
                body: self.new_block(clause.body.span),
                cleanup: self.new_block(clause.body.span),
            })
            .collect()
    }

    pub(super) fn lower_catch_regions(
        &mut self,
        try_span: Option<Span>,
        catches: &[CatchClause],
        blocks: &[CatchBlockInfo],
        context: &TryContext,
        exception_local: LocalId,
        exception_flag: Option<LocalId>,
        finally_entry: Option<BlockId>,
        after_block: BlockId,
        dispatch_block: Option<BlockId>,
        unhandled_block: Option<BlockId>,
    ) -> Vec<CatchRegion> {
        let mut metadata = Vec::new();
        let mut iter = blocks.iter().peekable();

        while let Some(info) = iter.next() {
            let clause = &catches[info.clause_index];
            let catch_ty = self.resolve_catch_type(clause);
            let next_entry = iter
                .peek()
                .map(|next| next.entry)
                .or(unhandled_block)
                .unwrap_or(context.after_block);

            self.switch_to_block(info.entry);
            self.push_scope();

            // Any path that enters a catch clause conceptually handles the pending exception,
            // even if the catch body returns early. Mark it handled at catch entry so the
            // fallible-drop pass does not treat it as escaping via early exits.
            self.mark_fallible_handled(exception_local, clause.body.span);

            let binding_local = self.bind_catch_variable(clause, exception_local, catch_ty.as_ref());
            if let Some(local) = binding_local {
                self.mark_fallible_handled(local, clause.body.span);
            }
            let filter_meta = self.lower_catch_filter(clause, info.body, next_entry);
            if filter_meta.is_none() {
                self.set_terminator(clause.body.span, Terminator::Goto { target: info.body });
            }

            self.switch_to_block(info.body);
            self.lower_block(&clause.body);
            self.ensure_goto(info.cleanup, clause.body.span);

            self.switch_to_block(info.cleanup);
            self.finalize_catch_cleanup(
                clause,
                binding_local,
                exception_flag,
                finally_entry,
                after_block,
            );

            self.pop_scope();

            metadata.push(CatchRegion {
                span: clause.body.span,
                                entry: info.entry,
                body: info.body,
                cleanup: info.cleanup,
                ty: catch_ty.clone(),
                binding: binding_local,
                filter: filter_meta,
            });
        }

        if let Some(dispatch) = dispatch_block {
            self.switch_to_block(dispatch);
            if let Some(first) = blocks.first() {
                self.set_terminator(try_span, Terminator::Goto { target: first.entry });
            } else if let Some(unhandled) = unhandled_block {
                self.set_terminator(try_span, Terminator::Goto { target: unhandled });
            }
        }

        if let Some(unhandled) = unhandled_block {
            self.switch_to_block(unhandled);
            let exception_ty = self
                .local_decl(exception_local)
                .map(|decl| decl.ty.clone())
                .or_else(|| Some(Ty::named("Exception")));
            self.set_terminator(
                try_span,
                Terminator::Throw {
                    exception: Some(Operand::Copy(Place::new(exception_local))),
                    ty: exception_ty,
                },
            );
        }

        metadata
    }

    fn bind_catch_variable(
        &mut self,
        clause: &CatchClause,
        exception_local: LocalId,
        resolved_ty: Option<&Ty>,
    ) -> Option<LocalId> {
        let Some(name) = &clause.identifier else {
            return None;
        };

        let ty = resolved_ty
            .cloned()
            .unwrap_or_else(|| Ty::named("Exception"));
        let mut decl = LocalDecl::new(
            Some(name.clone()),
            ty,
            true,
            clause.body.span,
            LocalKind::Local,
        );
        if clause
            .type_annotation
            .as_ref()
            .is_some_and(|ty| ty.is_nullable())
        {
            decl.is_nullable = true;
        }
        let local = self.push_local(decl);
        self.bind_name(name, local);
        self.push_statement(MirStatement {
            span: clause.body.span,
                        kind: MirStatementKind::StorageLive(local),
        });
        self.record_local(local, clause.body.span);
        self.push_statement(MirStatement {
            span: clause.body.span,
                        kind: MirStatementKind::Assign {
                place: Place::new(local),
                value: Rvalue::Use(Operand::Copy(Place::new(exception_local))),
            },
        });
        Some(local)
    }

    fn resolve_catch_type(&mut self, clause: &CatchClause) -> Option<Ty> {
        let annotation = clause.type_annotation.as_ref();
        let ty = annotation
            .map_or_else(|| Ty::named("Exception"), |expr| {
                let ty = Ty::from_type_expr(expr);
                self.ensure_ty_layout_for_ty(&ty);
                ty
            });
        if let Some(type_expr) = annotation {
            if type_expr.is_nullable() {
                let mut display = type_expr.name.clone();
                if !display.ends_with('?') {
                    display.push('?');
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "catch clause type `{display}` may be `null`; catch handlers must bind non-null exception values",
                    ),
                    span: clause.body.span,
                                    });
            }
        }
        if self.ty_is_exception(&ty) {
            Some(ty)
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "catch clause type `{}` does not derive from `Exception`",
                    ty.canonical_name()
                ),
                span: clause.body.span,
                            });
            None
        }
    }

    fn lower_catch_filter(
        &mut self,
        clause: &CatchClause,
        body_block: BlockId,
        next_target: BlockId,
    ) -> Option<CatchFilter> {
        let filter_expr = clause.filter.as_ref()?;
        let operand = self.lower_expression_operand(filter_expr);
        let filter_block = self.current_block;
        let filter_meta = CatchFilter {
            expr: filter_expr.text.clone(),
            span: filter_expr.span,
                        parsed: operand.is_some(),
            block: filter_block,
        };

        if let Some(discr) = operand {
            self.set_terminator(
                filter_expr.span,
                Terminator::SwitchInt {
                    discr,
                    targets: vec![(1, body_block)],
                    otherwise: next_target,
                },
            );
        } else {
            self.set_terminator(
                filter_expr.span,
                Terminator::Goto { target: next_target },
            );
        }

        Some(filter_meta)
    }

    fn finalize_catch_cleanup(
        &mut self,
        clause: &CatchClause,
        binding_local: Option<LocalId>,
        exception_flag: Option<LocalId>,
        finally_entry: Option<BlockId>,
        after_block: BlockId,
    ) {
        if let Some(local) = binding_local {
            self.push_statement(MirStatement {
                span: clause.body.span,
                                kind: MirStatementKind::StorageDead(local),
            });
        }
        if let Some(flag) = exception_flag {
            self.assign_bool(flag, false, clause.body.span);
        }
        if let Some(entry) = finally_entry {
            self.ensure_goto(entry, clause.body.span);
        } else {
            self.ensure_goto(after_block, clause.body.span);
        }
    }

    pub(super) fn lower_finally_region(
        &mut self,
        finally_entry: Option<BlockId>,
        finally_exit: Option<BlockId>,
        finally_block: Option<&Block>,
        exception_flag: Option<LocalId>,
        after_block: BlockId,
        dispatch_block: Option<BlockId>,
        unhandled_block: Option<BlockId>,
    ) -> Option<FinallyRegion> {
        let (entry, exit, block) = match (finally_entry, finally_exit, finally_block) {
            (Some(entry), Some(exit), Some(block)) => (entry, exit, block),
            _ => return None,
        };

        self.switch_to_block(entry);
        self.push_scope();
        self.lower_block(block);
        self.ensure_goto(exit, block.span);
        self.pop_scope();

        self.switch_to_block(exit);
        if let Some(flag) = exception_flag {
            if let Some(target) = dispatch_block.or(unhandled_block) {
                let discr = Operand::Copy(Place::new(flag));
                self.set_terminator(
                    block.span,
                    Terminator::SwitchInt {
                        discr,
                        targets: vec![(1, target)],
                        otherwise: after_block,
                    },
                );
            } else {
                self.set_terminator(block.span, Terminator::Goto { target: after_block });
            }
        } else {
            self.set_terminator(block.span, Terminator::Goto { target: after_block });
        }

        Some(FinallyRegion {
            span: block.span,
                        entry,
            exit,
        })
    }

    pub(super) fn ensure_try_success_path(&mut self, span: Option<Span>, ctx: &TryContext) {
        let current = self.current_block;
        if self.blocks[current.0].terminator.is_some() {
            return;
        }

        if let Some(finally_entry) = ctx.finally_entry {
            if let Some(flag) = ctx.exception_flag {
                self.assign_bool(flag, false, span);
            }
            self.ensure_goto(finally_entry, span);
        } else {
            self.ensure_goto(ctx.after_block, span);
        }
    }

    pub(super) fn cleanup_try_region(
        &mut self,
        span: Option<Span>,
                exception_local: LocalId,
        exception_flag: Option<LocalId>,
        after_block: BlockId,
    ) {
        self.switch_to_block(after_block);
        if let Some(flag) = exception_flag {
            self.push_statement(MirStatement {
                span,
                                kind: MirStatementKind::StorageDead(flag),
            });
        }
        self.mark_fallible_handled(exception_local, span);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::StorageDead(exception_local),
        });
    }
}
