use super::diagnostics::format_span;
use super::operations::AssignmentKind;
use super::*;
use crate::mir::ConstValue;
use crate::mir::data::{
    BasicBlock, BorrowKind, LocalId, MatchArm, Operand, ParamMode, Place, Terminator, Ty,
};
use crate::mir::layout::AutoTraitStatus;

impl<'a> BorrowChecker<'a> {
    pub(super) fn process_terminator(
        &mut self,
        state: &mut BorrowState<'a>,
        terminator: &Terminator,
        location: Location,
        block: &BasicBlock,
    ) {
        let ctx = TerminatorContext::new(block, location);
        match terminator {
            Terminator::SwitchInt { discr, .. } => {
                self.handle_switch_int(state, discr, ctx);
            }
            Terminator::Match { value, arms, .. } => {
                self.handle_match(state, value, arms, ctx);
            }
            Terminator::Return => {
                self.handle_return(state, ctx);
            }
            Terminator::Call {
                func,
                args,
                arg_modes,
                destination,
                ..
            } => {
                let operands = CallOperands {
                    func,
                    args,
                    modes: arg_modes,
                    destination: destination.as_ref(),
                };
                self.handle_call(state, &operands, ctx);
            }
            Terminator::Yield { value, .. } => {
                self.handle_yield(state, value, ctx);
            }
            Terminator::Await {
                future,
                destination,
                ..
            } => {
                self.handle_await(state, future, destination.as_ref(), ctx);
            }
            Terminator::Throw { exception, .. } => {
                if let Some(value) = exception {
                    self.visit_operand(state, value, ctx.span(), ctx.location);
                }
            }
            Terminator::Goto { .. }
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_) => {}
        }
    }

    fn handle_switch_int(
        &mut self,
        state: &mut BorrowState<'a>,
        discr: &Operand,
        ctx: TerminatorContext<'_>,
    ) {
        self.visit_operand(state, discr, ctx.span(), ctx.location);
    }

    fn handle_match(
        &mut self,
        state: &mut BorrowState<'a>,
        value: &Place,
        arms: &[MatchArm],
        ctx: TerminatorContext<'_>,
    ) {
        self.ensure_initialized(
            state,
            value.local,
            StatementSite::new(ctx.span(), ctx.location),
            ErrorKeyKind::UseOfUninit(value.local),
        );
        if arms.iter().any(|arm| arm.guard.is_some()) {
            // Guards evaluate later during lowering; nothing to visit yet.
        }
    }

    fn handle_return(&mut self, state: &mut BorrowState<'a>, ctx: TerminatorContext<'_>) {
        let ret_local = LocalId(0);
        let ret_decl = self.function.body.local(ret_local);
        let ret_is_unit = ret_decl.map_or(false, |decl| matches!(decl.ty, Ty::Unit));
        if !ret_is_unit {
            self.ensure_initialized(
                state,
                ret_local,
                StatementSite::new(ctx.span(), ctx.location),
                ErrorKeyKind::UseOfUninit(ret_local),
            );
        }
        self.check_required_assignments(state, ctx.span(), ctx.location);
    }

    fn handle_call(
        &mut self,
        state: &mut BorrowState<'a>,
        operands: &CallOperands<'_>,
        ctx: TerminatorContext<'_>,
    ) {
        self.visit_operand(state, operands.func, ctx.span(), ctx.location);
        for (arg, mode) in operands.args.iter().zip(operands.modes.iter()) {
            if matches!(mode, ParamMode::Out) {
                let out_place = match arg {
                    Operand::Borrow(borrow) => Some(&borrow.place),
                    Operand::Copy(place) | Operand::Move(place) => Some(place),
                    _ => None,
                };
                if let Some(place) = out_place {
                    let site = StatementSite::new(ctx.span(), ctx.location);
                    let union_action = Self::union_assignment_for_destination(state, place);
                    self.record_assignment(
                        state,
                        place,
                        site,
                        union_action,
                        None,
                        AssignmentKind::Regular,
                    );
                    continue;
                }
            }

            self.visit_operand(state, arg, ctx.span(), ctx.location);
        }
        if let Some(dest) = operands.destination {
            let union_action = Self::union_assignment_for_destination(state, dest);
            let site = StatementSite::new(ctx.span(), ctx.location);
            let assignment_kind = self.stack_alloc_assignment_for_call(state, operands);
            self.record_assignment(state, dest, site, union_action, None, assignment_kind);
        }

        for arg in operands.args {
            if let Operand::Borrow(borrow) = arg {
                self.release_loans_for_place(state, &borrow.place, ctx.location);
            }
        }
        self.track_span_views_for_call(state, operands, ctx);
    }

    fn handle_yield(
        &mut self,
        state: &mut BorrowState<'a>,
        value: &Operand,
        ctx: TerminatorContext<'_>,
    ) {
        self.visit_operand(state, value, ctx.span(), ctx.location);
    }

    fn handle_await(
        &mut self,
        state: &mut BorrowState<'a>,
        future: &Place,
        destination: Option<&Place>,
        ctx: TerminatorContext<'_>,
    ) {
        let await_span = self
            .await_metadata(ctx.block.id)
            .and_then(|meta| meta.span)
            .or(ctx.span());
        self.visit_place(state, future, await_span, ctx.location);

        if let Some(dest) = destination {
            let union_action = Self::union_assignment_for_destination(state, dest);
            let site = StatementSite::new(await_span, ctx.location);
            self.record_assignment(
                state,
                dest,
                site,
                union_action,
                None,
                AssignmentKind::Regular,
            );
        }

        self.check_await_unique_borrows(state, await_span, ctx);
        self.check_await_auto_traits(state, await_span, ctx);
        self.check_await_pinned_locals(state, await_span, ctx);
        self.check_await_stack_allocs(state, await_span, ctx);
    }

    fn check_await_unique_borrows(
        &mut self,
        state: &BorrowState<'a>,
        await_span: Option<Span>,
        ctx: TerminatorContext<'_>,
    ) {
        for loan in state.active_loans.values() {
            if matches!(loan.info.kind, BorrowKind::Unique)
                && !Self::place_is_pinned(state, &loan.info.place)
            {
                self.report_error(
                    ctx.location,
                    await_span,
                    ErrorKeyKind::AwaitWithUniqueBorrow,
                    format!(
                        "cannot await while unique borrow of `{}` is active (borrowed at {})",
                        self.place_description(&loan.info.place),
                        format_span(loan.info.origin_span)
                    ),
                );
            }
        }
    }

    fn check_await_auto_traits(
        &mut self,
        state: &BorrowState<'a>,
        await_span: Option<Span>,
        ctx: TerminatorContext<'_>,
    ) {
        let site = StatementSite::new(await_span, ctx.location);
        for loan in state.active_loans.values() {
            let Some(decl) = self.function.body.local(loan.info.place.local) else {
                continue;
            };
            let ty_name = match &decl.ty {
                Ty::Named(name) => name.as_str().to_string(),
                Ty::String => "string".to_string(),
                Ty::Str => "str".to_string(),
                _ => continue,
            };
            let traits = self.type_layouts.resolve_auto_traits(&ty_name);
            let place_description = self.place_description(&loan.info.place);
            match loan.info.kind {
                BorrowKind::Shared => {
                    let report = TraitReport {
                        loan_local: loan.info.place.local,
                        place: &loan.info.place,
                        place_description: &place_description,
                        ty_name: ty_name.as_str(),
                        site,
                    };
                    self.ensure_shareable_trait(traits.shareable, report);
                }
                BorrowKind::Unique => {
                    let report = TraitReport {
                        loan_local: loan.info.place.local,
                        place: &loan.info.place,
                        place_description: &place_description,
                        ty_name: ty_name.as_str(),
                        site,
                    };
                    self.ensure_thread_safe_trait(state, traits.thread_safe, report);
                }
                BorrowKind::Raw => {}
            }
        }
    }

    fn check_await_pinned_locals(
        &mut self,
        state: &BorrowState<'a>,
        await_span: Option<Span>,
        ctx: TerminatorContext<'_>,
    ) {
        let Some(machine) = self.function.body.async_machine.as_ref() else {
            return;
        };
        for &local in &machine.cross_locals {
            let facts = state.local_facts(local);
            if facts.is_pinned() || !matches!(facts.init, InitState::Init | InitState::Maybe) {
                continue;
            }
            let Some(decl) = self.function.body.local(local) else {
                continue;
            };
            if !Self::requires_pin_for_async(&decl.ty) {
                continue;
            }
            self.report_error(
                ctx.location,
                await_span.or(decl.span),
                ErrorKeyKind::AwaitRequiresPin(local),
                format!(
                    "`{}` must be pinned to survive across `await`; mark the binding `@pinned` or use a pinned memory space",
                    self.local_name(local)
                ),
            );
        }
    }

    fn check_await_stack_allocs(
        &mut self,
        state: &BorrowState<'a>,
        await_span: Option<Span>,
        ctx: TerminatorContext<'_>,
    ) {
        for (index, facts) in state.locals.iter().enumerate() {
            if facts.stack_alloc.is_some() && facts.init == InitState::Init {
                let local = LocalId(index);
                self.report_error(
                    ctx.location,
                    await_span,
                    ErrorKeyKind::AwaitWithStackAlloc(local),
                    format!(
                        "cannot await while stack-allocated span `{}` is live; drop it or copy into heap-backed storage before awaiting",
                        self.local_name(local)
                    ),
                );
            }
        }
    }

    fn requires_pin_for_async(ty: &Ty) -> bool {
        match ty {
            Ty::Nullable(inner) => Self::requires_pin_for_async(inner),
            Ty::Pointer(ptr) => Self::requires_pin_for_async(&ptr.element),
            Ty::Ref(reference) => Self::requires_pin_for_async(&reference.element),
            _ => ty.is_accelerator_stream() || ty.is_accelerator_event(),
        }
    }

    fn ensure_shareable_trait(&mut self, status: AutoTraitStatus, report: TraitReport<'_>) {
        let place = report.place_description;
        let ty = report.ty_name;
        match status {
            AutoTraitStatus::Yes => {}
            AutoTraitStatus::No => {
                self.report_error(
                    report.site.location,
                    report.site.span,
                    ErrorKeyKind::AwaitRequiresShareable(report.loan_local),
                    format!(
                        "cannot await while shared borrow of `{place}` is active; type `{ty}` is not Shareable"
                    ),
                );
            }
            AutoTraitStatus::Unknown => {
                self.report_error(
                    report.site.location,
                    report.site.span,
                    ErrorKeyKind::AwaitRequiresShareable(report.loan_local),
                    format!(
                        "cannot await while shared borrow of `{place}` is active; compiler cannot prove type `{ty}` is Shareable"
                    ),
                );
            }
        }
    }

    fn ensure_thread_safe_trait(
        &mut self,
        state: &BorrowState<'a>,
        status: AutoTraitStatus,
        report: TraitReport<'_>,
    ) {
        if !Self::place_is_pinned(state, report.place) {
            return;
        }

        let place = report.place_description;
        let ty = report.ty_name;
        match status {
            AutoTraitStatus::Yes => {}
            AutoTraitStatus::No => {
                self.report_error(
                    report.site.location,
                    report.site.span,
                    ErrorKeyKind::AwaitRequiresThreadSafe(report.loan_local),
                    format!(
                        "cannot await while pinned unique borrow of `{place}` is active; type `{ty}` is not ThreadSafe"
                    ),
                );
            }
            AutoTraitStatus::Unknown => {
                self.report_error(
                    report.site.location,
                    report.site.span,
                    ErrorKeyKind::AwaitRequiresThreadSafe(report.loan_local),
                    format!(
                        "cannot await while pinned unique borrow of `{place}` is active; compiler cannot prove type `{ty}` is ThreadSafe"
                    ),
                );
            }
        }
    }
    fn track_span_views_for_call(
        &mut self,
        state: &mut BorrowState<'a>,
        operands: &CallOperands<'_>,
        ctx: TerminatorContext<'_>,
    ) {
        let dest = match operands.destination {
            Some(place) => place,
            None => return,
        };

        let dest_local = dest.local;
        let dest_decl = match self.function.body.local(dest_local) {
            Some(decl) => decl,
            None => return,
        };
        let view_kind = match dest_decl.ty {
            Ty::Span(_) => SpanViewKind::Mutable,
            Ty::ReadOnlySpan(_) => SpanViewKind::ReadOnly,
            _ => return,
        };

        if state
            .active_loans
            .values()
            .any(|loan| loan.associated_view == Some(dest_local))
        {
            return;
        }

        let func_name = self.call_symbol_name(operands.func);
        let Some(root_place) = self.span_root_from_call(state, operands, func_name, view_kind)
        else {
            return;
        };

        let key = SpanViewLoanKey {
            location: SyntheticLocationKey::from(ctx.location),
            view_local: dest_local,
            root: SyntheticPlaceKey::from(&root_place),
            kind: view_kind.borrow_kind(),
        };
        let (borrow_id, region) = if let Some(ids) = self.span_view_loan_cache.get(&key) {
            *ids
        } else {
            let ids = self.allocate_synthetic_ids();
            self.span_view_loan_cache.insert(key, ids);
            ids
        };
        let loan = LoanInfo {
            borrow_id,
            kind: view_kind.borrow_kind(),
            place: root_place,
            region,
            origin_span: ctx.span(),
        };
        state.record_borrow(loan);
        if let Some(entry) = state.active_loans.get_mut(&borrow_id) {
            entry.associated_view = Some(dest_local);
        }
        self.register_region(region, borrow_id, ctx.location);
    }

    fn call_symbol_name<'b>(&self, operand: &'b Operand) -> Option<&'b str> {
        match operand {
            Operand::Const(constant) => match &constant.value {
                ConstValue::Symbol(name) => Some(name.as_str()),
                _ => None,
            },
            _ => None,
        }
    }

    fn span_root_from_call(
        &self,
        state: &BorrowState<'a>,
        operands: &CallOperands<'_>,
        func_name: Option<&str>,
        kind: SpanViewKind,
    ) -> Option<Place> {
        if let Some(name) = func_name {
            if kind == SpanViewKind::Mutable && name.ends_with("chic_rt_span_slice_mut") {
                if let Some(arg) = operands.args.get(0) {
                    if let Some(place) = Self::extract_view_place(arg) {
                        return self.find_view_root(state, place.local);
                    }
                }
            } else if kind == SpanViewKind::ReadOnly
                && name.ends_with("chic_rt_span_slice_readonly")
            {
                if let Some(arg) = operands.args.get(0) {
                    if let Some(place) = Self::extract_view_place(arg) {
                        return self.find_view_root(state, place.local);
                    }
                }
            }
        }

        for arg in operands.args {
            if let Some(place) = Self::extract_array_like_place(arg) {
                let local = place.local;
                let decl = self.function.body.local(local)?;
                if self.place_matches_span_root(&decl.ty, kind) {
                    return Some(place.clone());
                }
                if matches!(decl.ty, Ty::ReadOnlySpan(_) | Ty::Span(_)) {
                    if let Some(root) = self.find_view_root(state, local) {
                        return Some(root);
                    }
                }
            }
        }
        None
    }

    fn stack_alloc_assignment_for_call(
        &self,
        state: &BorrowState<'a>,
        operands: &CallOperands<'_>,
    ) -> AssignmentKind {
        let dest = match operands.destination {
            Some(place) => place,
            None => return AssignmentKind::Regular,
        };
        let dest_decl = match self.function.body.local(dest.local) {
            Some(decl) => decl,
            None => return AssignmentKind::Regular,
        };
        let view_kind = match dest_decl.ty {
            Ty::Span(_) => Some(SpanViewKind::Mutable),
            Ty::ReadOnlySpan(_) => Some(SpanViewKind::ReadOnly),
            _ => None,
        };
        let Some(kind) = view_kind else {
            return AssignmentKind::Regular;
        };
        let func_name = self.call_symbol_name(operands.func);
        let Some(root_place) = self.span_root_from_call(state, operands, func_name, kind) else {
            return AssignmentKind::Regular;
        };
        if state.local_facts(root_place.local).stack_alloc.is_some() {
            AssignmentKind::StackAllocPropagated
        } else {
            AssignmentKind::Regular
        }
    }

    fn extract_array_like_place(operand: &Operand) -> Option<&Place> {
        match operand {
            Operand::Borrow(borrow) => Some(&borrow.place),
            Operand::Copy(place) | Operand::Move(place) => Some(place),
            _ => None,
        }
    }

    fn extract_view_place(operand: &Operand) -> Option<&Place> {
        if let Operand::Borrow(borrow) = operand {
            Some(&borrow.place)
        } else {
            None
        }
    }

    fn find_view_root(&self, state: &BorrowState<'a>, view_local: LocalId) -> Option<Place> {
        state
            .active_loans
            .values()
            .find(|loan| loan.associated_view == Some(view_local))
            .map(|loan| loan.info.place.clone())
    }

    fn place_matches_span_root(&self, ty: &Ty, kind: SpanViewKind) -> bool {
        match ty {
            Ty::Array(_) => true,
            Ty::Vec(_) => kind == SpanViewKind::Mutable,
            Ty::Span(_) | Ty::ReadOnlySpan(_) => false,
            Ty::String => kind == SpanViewKind::ReadOnly,
            Ty::Named(named) => {
                let canonical = named.canonical_path();
                match canonical.as_str() {
                    "Std::Collections::VecPtr" => true,
                    "Std::Collections::VecViewPtr" => kind == SpanViewKind::ReadOnly,
                    "Std::Collections::ArrayPtr" => true,
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpanViewKind {
    Mutable,
    ReadOnly,
}

impl SpanViewKind {
    const fn borrow_kind(self) -> BorrowKind {
        match self {
            SpanViewKind::Mutable => BorrowKind::Unique,
            SpanViewKind::ReadOnly => BorrowKind::Shared,
        }
    }
}
