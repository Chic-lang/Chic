use super::*;
use crate::mir::CastKind;

body_builder_impl! {

    #[expect(
        clippy::too_many_lines,
        reason = "Try lowering coordinates catch/finally/cleanup logic in a single pass for readability."
    )]
    pub(super) fn lower_try_statement(&mut self, statement: &AstStatement, try_stmt: &TryStatement) {
        let try_span = statement.span;
        let try_entry = self.ensure_active_block();
        let has_catches = !try_stmt.catches.is_empty();
        let has_finally = try_stmt.finally.is_some();

        let exception_name = format!("__exception{}", self.next_exception_id);
        self.next_exception_id += 1;
        let exception_local = self.push_local(LocalDecl::new(
            Some(exception_name),
            Ty::Nullable(Box::new(Ty::named("Exception"))),
            true,
            try_span,
            LocalKind::Temp,
        ));
        self.push_statement(MirStatement {
            span: try_span,
            kind: MirStatementKind::StorageLive(exception_local),
        });
        self.push_statement(MirStatement {
            span: try_span,
            kind: MirStatementKind::Assign {
                place: Place::new(exception_local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Null))),
            },
        });
        self.record_local(exception_local, try_span);

        let exception_flag = if has_finally {
            let flag_name = format!("__pending_exception{}", self.next_exception_id);
            self.next_exception_id += 1;
            let flag_local = self.push_local(LocalDecl::new(
                Some(flag_name),
                Ty::named("bool"),
                true,
                try_span,
                LocalKind::Temp,
            ));
            self.push_statement(MirStatement {
                span: try_span,
                kind: MirStatementKind::StorageLive(flag_local),
            });
            self.assign_bool(flag_local, false, try_span);
            self.record_local(flag_local, try_span);
            Some(flag_local)
        } else {
            None
        };
        // note: exception_flag is initialised to false above.

        let after_block = self.new_block(try_span);
        let dispatch_block = if has_catches {
            Some(self.new_block(try_span))
        } else {
            None
        };
        let unhandled_block = Some(self.new_block(try_span));
        let unwind_capture = Some(self.new_block(try_span));
        let try_scope_depth = self.scope_depth();

        let (finally_entry, finally_exit) = if let Some(fin) = &try_stmt.finally {
            (
                Some(self.new_block(fin.span)),
                Some(self.new_block(fin.span)),
            )
        } else {
            (None, None)
        };

        let catch_blocks = self.allocate_catch_blocks(&try_stmt.catches);

        let context = TryContext {
            exception_local,
            exception_flag,
            dispatch_block,
            finally_entry,
            after_block,
            unhandled_block,
            unwind_capture,
            scope_depth: try_scope_depth,
        };
        self.try_stack.push(context);

        self.lower_block(&try_stmt.body);

        let try_exit = self.current_block;
        self.ensure_try_success_path(try_span, &context);

        let catches_metadata = self.lower_catch_regions(
            try_span,
            &try_stmt.catches,
            &catch_blocks,
            &context,
            exception_local,
            exception_flag,
            finally_entry,
            after_block,
            dispatch_block,
            unhandled_block,
        );

        let finally_meta = self.lower_finally_region(
            finally_entry,
            finally_exit,
            try_stmt.finally.as_ref(),
            exception_flag,
            after_block,
            dispatch_block,
            unhandled_block,
        );

        if let Some(unwind_capture) = self.try_stack.last().and_then(|ctx| ctx.unwind_capture) {
            self.switch_to_block(unwind_capture);
            let payload_local = self.create_temp(try_span);
            self.hint_local_ty(payload_local, Ty::named("nint"));
            let type_local = self.create_temp(try_span);
            self.hint_local_ty(type_local, Ty::named("nuint"));

            let payload_borrow = self.borrow_argument_place(
                Place::new(payload_local),
                BorrowKind::Raw,
                try_span,
            );
            let type_borrow =
                self.borrow_argument_place(Place::new(type_local), BorrowKind::Raw, try_span);
            let captured = {
                let temp = self.create_temp(try_span);
                self.hint_local_ty(temp, Ty::named("i32"));
                let destination = Place::new(temp);
                let continue_block = self.new_block(try_span);
                self.set_terminator(
                    try_span,
                    Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "chic_rt.take_pending_exception".to_string(),
                        ))),
                        args: vec![payload_borrow, type_borrow],
                        arg_modes: vec![ParamMode::Out, ParamMode::Out],
                        destination: Some(destination.clone()),
                        target: continue_block,
                        unwind: None,
                        dispatch: None,
                    },
                );
                self.switch_to_block(continue_block);
                Operand::Copy(destination)
            };

            let capture_dispatch = self.new_block(try_span);
            self.set_terminator(
                try_span,
                Terminator::SwitchInt {
                    discr: captured.clone(),
                    targets: vec![(1, capture_dispatch)],
                    otherwise: after_block,
                },
            );

            self.switch_to_block(capture_dispatch);
            let exception_ty = self
                .local_decl(exception_local)
                .map(|decl| decl.ty.clone())
                .unwrap_or_else(|| Ty::named("Exception"));
            self.push_statement(MirStatement {
                span: try_span,
                kind: MirStatementKind::Assign {
                    place: Place::new(exception_local),
                    value: Rvalue::Cast {
                        kind: CastKind::IntToPointer,
                        operand: Operand::Copy(Place::new(payload_local)),
                        source: Ty::named("nint"),
                        target: exception_ty,
                        rounding: None,
                    },
                },
            });
            if let Some(flag) = exception_flag {
                self.assign_bool(flag, true, try_span);
            }

            if let Some(dispatch) = dispatch_block {
                self.set_terminator(try_span, Terminator::Goto { target: dispatch });
            } else if let Some(unhandled) = unhandled_block {
                self.set_terminator(try_span, Terminator::Goto { target: unhandled });
            } else {
                self.set_terminator(try_span, Terminator::Goto { target: after_block });
            }
        }

        self.cleanup_try_region(try_span, exception_local, exception_flag, after_block);

        self.try_stack.pop();

        let region_id = self.exception_regions.len();
        self.exception_regions.push(ExceptionRegion {
            id: region_id,
            span: try_span,
                        try_entry,
            try_exit,
            after_block,
            dispatch: dispatch_block,
            catches: catches_metadata,
            finally: finally_meta,
        });
    }

}
