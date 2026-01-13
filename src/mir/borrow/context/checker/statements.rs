use super::diagnostics::format_span;
use super::operations::AssignmentKind;
use super::*;
use crate::mir::data::{
    BorrowKind, InlineAsmOperandKind, InterpolatedStringSegment, LocalId, Operand, ParamMode,
    Place, Rvalue, Statement, StatementKind, Ty,
};

impl<'a> BorrowChecker<'a> {
    pub(super) fn process_statement(
        &mut self,
        state: &mut BorrowState<'a>,
        statement: &Statement,
        location: Location,
    ) {
        let site = StatementSite::new(statement.span, location);
        match &statement.kind {
            StatementKind::Assign { place, value } => {
                self.handle_assignment(state, place, value, site);
            }
            StatementKind::StorageLive(local) => {
                Self::reset_local_state(state, *local);
            }
            StatementKind::StorageDead(local) => {
                self.handle_storage_dead(state, *local, site);
            }
            StatementKind::Deinit(place) => {
                self.handle_deinit(state, place, site);
            }
            StatementKind::DefaultInit { place } => {
                self.handle_zero_init(state, place, site);
            }
            StatementKind::ZeroInit { place } => {
                self.handle_zero_init(state, place, site);
            }
            StatementKind::ZeroInitRaw { pointer, length } => {
                self.visit_operand(state, pointer, site.span, site.location);
                self.visit_operand(state, length, site.span, site.location);
            }
            StatementKind::Drop { place, .. } => {
                self.handle_drop(state, place, site);
            }
            StatementKind::EnterUnsafe => {
                state.unsafe_depth = state.unsafe_depth.saturating_add(1);
            }
            StatementKind::ExitUnsafe => {
                state.unsafe_depth = state.unsafe_depth.saturating_sub(1);
            }
            StatementKind::Borrow {
                borrow_id,
                kind,
                place,
                region,
            } => {
                let request = BorrowRequest {
                    borrow_id: *borrow_id,
                    kind: *kind,
                    place,
                    region: *region,
                    site,
                };
                self.handle_borrow(state, request);
            }
            StatementKind::Assert { cond, .. } => {
                self.visit_operand(state, cond, site.span, site.location);
            }
            StatementKind::EnqueueKernel {
                stream,
                kernel,
                args,
                completion,
            } => {
                self.visit_place(state, stream, site.span, site.location);
                self.visit_operand(state, kernel, site.span, site.location);
                for arg in args {
                    self.visit_operand(state, arg, site.span, site.location);
                }
                let mut deps = vec![stream.clone()];
                deps.extend(args.iter().filter_map(Self::operand_place));
                self.track_stream_event_lifetimes(state, deps, completion.as_ref(), site);
                if let Some(event) = completion {
                    let union_action =
                        BorrowChecker::union_assignment_for_destination(state, event);
                    self.record_assignment(
                        state,
                        event,
                        site,
                        union_action,
                        None,
                        AssignmentKind::Regular,
                    );
                }
            }
            StatementKind::EnqueueCopy {
                stream,
                dst,
                src,
                bytes,
                completion,
                ..
            } => {
                self.visit_place(state, stream, site.span, site.location);
                self.visit_place(state, dst, site.span, site.location);
                self.visit_place(state, src, site.span, site.location);
                self.visit_operand(state, bytes, site.span, site.location);
                let deps = vec![stream.clone(), dst.clone(), src.clone()];
                self.track_stream_event_lifetimes(state, deps, completion.as_ref(), site);
                if let Some(event) = completion {
                    let union_action =
                        BorrowChecker::union_assignment_for_destination(state, event);
                    self.record_assignment(
                        state,
                        event,
                        site,
                        union_action,
                        None,
                        AssignmentKind::Regular,
                    );
                }
            }
            StatementKind::RecordEvent { stream, event } => {
                self.visit_place(state, stream, site.span, site.location);
                self.visit_place(state, event, site.span, site.location);
                self.track_stream_event_lifetimes(state, vec![stream.clone()], Some(event), site);
                let union_action = BorrowChecker::union_assignment_for_destination(state, event);
                self.record_assignment(
                    state,
                    event,
                    site,
                    union_action,
                    None,
                    AssignmentKind::Regular,
                );
            }
            StatementKind::WaitEvent { event, stream } => {
                if let Some(stream) = stream {
                    self.visit_place(state, stream, site.span, site.location);
                }
                self.visit_place(state, event, site.span, site.location);
                self.release_event_loans(state, event.local, site.location);
            }
            StatementKind::MmioStore { value, .. } | StatementKind::StaticStore { value, .. } => {
                self.visit_operand(state, value, site.span, site.location);
            }
            StatementKind::AtomicStore { value, .. } => {
                self.visit_operand(state, value, site.span, site.location);
            }
            StatementKind::AtomicFence { .. } => {}
            StatementKind::InlineAsm(asm) => {
                for operand in &asm.operands {
                    match &operand.kind {
                        InlineAsmOperandKind::In { value }
                        | InlineAsmOperandKind::Const { value } => {
                            self.visit_operand(state, value, site.span, site.location);
                        }
                        InlineAsmOperandKind::InOut { input, output, .. } => {
                            self.visit_operand(state, input, site.span, site.location);
                            let union_action =
                                BorrowChecker::union_assignment_for_destination(state, output);
                            self.record_assignment(
                                state,
                                output,
                                site,
                                union_action,
                                Some(NullState::Unknown),
                                AssignmentKind::Regular,
                            );
                        }
                        InlineAsmOperandKind::Out { place, .. } => {
                            let union_action =
                                BorrowChecker::union_assignment_for_destination(state, place);
                            self.record_assignment(
                                state,
                                place,
                                site,
                                union_action,
                                Some(NullState::Unknown),
                                AssignmentKind::Regular,
                            );
                        }
                        InlineAsmOperandKind::Sym { .. } => {}
                    }
                }
            }
            StatementKind::Retag { .. }
            | StatementKind::DeferDrop { .. }
            | StatementKind::Eval(_)
            | StatementKind::MarkFallibleHandled { .. }
            | StatementKind::Nop
            | StatementKind::Pending(_) => {}
        }
    }

    fn handle_assignment(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        value: &Rvalue,
        site: StatementSite,
    ) {
        let union_action = self.determine_union_assignment(state, place, value, site);
        self.visit_rvalue(state, value, site.span, site.location);
        let null_state = self.rvalue_null_state(state, value);
        let assignment_kind = self.classify_stack_alloc_assignment(state, value);
        self.record_assignment(
            state,
            place,
            site,
            union_action,
            null_state,
            assignment_kind,
        );
    }

    fn reset_local_state(state: &mut BorrowState<'a>, local: LocalId) {
        let facts = state.local_facts_mut(local);
        let requires_init = facts.requires_init;
        if facts.nullable {
            facts.init = InitState::Init;
            facts.null_state = NullState::Null;
        } else {
            facts.init = InitState::Uninit;
            facts.null_state = NullState::Unknown;
        }
        facts.requires_init = requires_init;
        facts.stack_alloc = None;
        facts.assignment_count = 0;
        facts.last_assignment = None;
        facts.last_move = None;
        state.clear_union_state(local);
    }

    fn handle_storage_dead(
        &mut self,
        state: &mut BorrowState<'a>,
        local: LocalId,
        site: StatementSite,
    ) {
        let place = Place::new(local);
        self.release_loans_for_place(state, &place, site.location);
        if let Some(decl) = self.function.body.local(local) {
            if matches!(decl.ty, Ty::ReadOnlySpan(_) | Ty::Span(_)) {
                self.release_view(state, local, site.location);
            }
        }
        Self::reset_local_state(state, local);
    }

    fn handle_deinit(&mut self, state: &mut BorrowState<'a>, place: &Place, site: StatementSite) {
        self.release_loans_for_place(state, place, site.location);
        if place.projection.is_empty() {
            if let Some(decl) = self.function.body.local(place.local) {
                if matches!(decl.ty, Ty::ReadOnlySpan(_) | Ty::Span(_)) {
                    self.release_view(state, place.local, site.location);
                }
            }
        }
        let facts = state.local_facts_mut(place.local);
        facts.init = InitState::Init;
        facts.requires_init = false;
        facts.null_state = if facts.nullable {
            NullState::Null
        } else {
            NullState::Unknown
        };
        facts.stack_alloc = None;
        facts.last_move = site.span;
        state.clear_union_state(place.local);
    }

    fn handle_drop(&mut self, state: &mut BorrowState<'a>, place: &Place, site: StatementSite) {
        if !self.ensure_initialized(
            state,
            place.local,
            site,
            ErrorKeyKind::UseOfUninit(place.local),
        ) {
            return;
        }
        self.release_loans_for_place(state, place, site.location);
        if place.projection.is_empty() {
            if let Some(decl) = self.function.body.local(place.local) {
                if matches!(decl.ty, Ty::ReadOnlySpan(_) | Ty::Span(_)) {
                    self.release_view(state, place.local, site.location);
                }
            }
        }
        {
            let facts = state.local_facts_mut(place.local);
            facts.stack_alloc = None;
        }
        state.clear_union_state(place.local);
    }

    fn handle_zero_init(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        site: StatementSite,
    ) {
        let union_action = Self::union_assignment_for_destination(state, place);
        self.record_assignment(
            state,
            place,
            site,
            union_action,
            None,
            AssignmentKind::Regular,
        );
    }

    fn classify_stack_alloc_assignment(
        &self,
        state: &BorrowState<'a>,
        value: &Rvalue,
    ) -> AssignmentKind {
        if matches!(value, Rvalue::SpanStackAlloc { .. }) {
            AssignmentKind::StackAllocOrigin
        } else if self.rvalue_uses_stack_alloc(state, value) {
            AssignmentKind::StackAllocPropagated
        } else {
            AssignmentKind::Regular
        }
    }

    fn rvalue_uses_stack_alloc(&self, state: &BorrowState<'a>, value: &Rvalue) -> bool {
        match value {
            Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => {
                self.operand_uses_stack_alloc(state, op)
            }
            Rvalue::Binary { lhs, rhs, .. } => {
                self.operand_uses_stack_alloc(state, lhs)
                    || self.operand_uses_stack_alloc(state, rhs)
            }
            Rvalue::Aggregate { fields, .. } => fields
                .iter()
                .any(|field| self.operand_uses_stack_alloc(state, field)),
            Rvalue::AddressOf { place, .. } | Rvalue::Len(place) => {
                self.place_uses_stack_alloc(state, place)
            }
            Rvalue::Cast { operand, .. } => self.operand_uses_stack_alloc(state, operand),
            Rvalue::StringInterpolate { segments } => segments.iter().any(|segment| {
                if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                    self.operand_uses_stack_alloc(state, operand)
                } else {
                    false
                }
            }),
            Rvalue::NumericIntrinsic(intrinsic) => {
                intrinsic
                    .operands
                    .iter()
                    .any(|op| self.operand_uses_stack_alloc(state, op))
                    || intrinsic
                        .out
                        .as_ref()
                        .map_or(false, |place| self.place_uses_stack_alloc(state, place))
            }
            Rvalue::DecimalIntrinsic(decimal) => {
                self.operand_uses_stack_alloc(state, &decimal.lhs)
                    || self.operand_uses_stack_alloc(state, &decimal.rhs)
                    || decimal
                        .addend
                        .as_ref()
                        .map_or(false, |addend| self.operand_uses_stack_alloc(state, addend))
                    || self.operand_uses_stack_alloc(state, &decimal.rounding)
                    || self.operand_uses_stack_alloc(state, &decimal.vectorize)
            }
            Rvalue::AtomicLoad { target, .. } => self.place_uses_stack_alloc(state, target),
            Rvalue::AtomicRmw { target, value, .. } => {
                self.place_uses_stack_alloc(state, target)
                    || self.operand_uses_stack_alloc(state, value)
            }
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                ..
            } => {
                self.place_uses_stack_alloc(state, target)
                    || self.operand_uses_stack_alloc(state, expected)
                    || self.operand_uses_stack_alloc(state, desired)
            }
            Rvalue::StaticLoad { .. } => false,
            Rvalue::StaticRef { .. } => false,
            Rvalue::Pending(_) => false,
            Rvalue::SpanStackAlloc { .. } => false,
        }
    }

    fn operand_uses_stack_alloc(&self, state: &BorrowState<'a>, operand: &Operand) -> bool {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.place_uses_stack_alloc(state, place)
            }
            Operand::Borrow(borrow) => self.place_uses_stack_alloc(state, &borrow.place),
            Operand::Const(_) | Operand::Mmio(_) | Operand::Pending(_) => false,
        }
    }

    fn place_uses_stack_alloc(&self, state: &BorrowState<'a>, place: &Place) -> bool {
        state.local_facts(place.local).stack_alloc.is_some()
    }

    fn operand_place(operand: &Operand) -> Option<Place> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => Some(place.clone()),
            Operand::Borrow(borrow) => Some(borrow.place.clone()),
            _ => None,
        }
    }

    fn track_stream_event_lifetimes(
        &mut self,
        state: &mut BorrowState<'a>,
        dependencies: Vec<Place>,
        completion: Option<&Place>,
        site: StatementSite,
    ) {
        let Some(event_place) = completion else {
            return;
        };
        let until_local = event_place.local;
        let location_key = SyntheticLocationKey::from(site.location);
        for place in dependencies {
            let key = StreamEventLoanKey {
                location: location_key,
                dependency: SyntheticPlaceKey::from(&place),
                until_local,
            };
            let (borrow_id, region) = if let Some(ids) = self.stream_event_loan_cache.get(&key) {
                *ids
            } else {
                let ids = self.allocate_synthetic_ids();
                self.stream_event_loan_cache.insert(key, ids);
                ids
            };
            let loan = LoanInfo {
                borrow_id,
                kind: BorrowKind::Shared,
                place: place.clone(),
                region,
                origin_span: site.span,
            };
            self.register_region(region, borrow_id, site.location);
            state.record_borrow_until(loan, Some(until_local));
        }
    }

    fn handle_borrow(&mut self, state: &mut BorrowState<'a>, request: BorrowRequest<'_>) {
        let facts = state.local_facts(request.place.local);
        let is_out_param = facts.param_mode == Some(ParamMode::Out);
        let skip_init_check = is_out_param
            || self.out_argument_regions.contains(&request.region)
            || matches!(request.kind, BorrowKind::Raw);
        if !skip_init_check
            && !self.ensure_initialized(
                state,
                request.place.local,
                request.site,
                ErrorKeyKind::UseOfUninit(request.place.local),
            )
        {
            return;
        }

        if matches!(request.kind, BorrowKind::Unique) {
            if !facts.mutable && facts.param_mode != Some(ParamMode::Out) {
                self.emit_immutable_binding_error(
                    request.site.location,
                    request.site.span,
                    request.place.local,
                    "take a mutable borrow of",
                );
            }
        }

        if let Some(conflict) = Self::find_conflicting_loan(state, request.place, request.kind) {
            self.report_error(
                request.site.location,
                request.site.span,
                ErrorKeyKind::BorrowConflict(request.place.local, request.kind),
                format!(
                    "conflicting borrow of `{}`: existing {:?} borrow from {}",
                    self.local_name(request.place.local),
                    conflict.info.kind,
                    format_span(conflict.info.origin_span)
                ),
            );
            return;
        }

        let loan = LoanInfo {
            borrow_id: request.borrow_id,
            kind: request.kind,
            place: request.place.clone(),
            region: request.region,
            origin_span: request.site.span,
        };
        self.register_region(request.region, request.borrow_id, request.site.location);
        state.record_borrow(loan);
    }
}
