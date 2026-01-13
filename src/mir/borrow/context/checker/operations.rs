use super::diagnostics::format_span;
use super::*;
use crate::mir::ConstValue;
use crate::mir::ProjectionElem;
use crate::mir::data::{
    BorrowKind, BorrowOperand, InterpolatedStringSegment, LocalId, LocalKind, Operand, ParamMode,
    Place, Rvalue, Ty,
};
use crate::mir::layout::{FieldLayout, StructLayout, TypeLayout, UnionFieldMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AssignmentKind {
    Regular,
    StackAllocOrigin,
    StackAllocPropagated,
}

impl<'a> BorrowChecker<'a> {
    pub(super) fn union_assignment_for_destination(
        state: &BorrowState<'a>,
        dest: &Place,
    ) -> Option<UnionActiveState> {
        state
            .union_info(dest.local)
            .map(|_| UnionActiveState::Unknown)
    }

    pub(super) fn record_assignment(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        site: StatementSite,
        union_action: Option<UnionActiveState>,
        null_state: Option<NullState>,
        assignment_kind: AssignmentKind,
    ) {
        let local = place.local;
        let is_root_assignment = place.projection.is_empty();
        let (param_mode, mutable, prior_assignments) = {
            let facts = state.local_facts(local);
            (facts.param_mode, facts.mutable, facts.assignment_count)
        };

        if matches!(param_mode, Some(ParamMode::In)) {
            self.report_error(
                site.location,
                site.span,
                ErrorKeyKind::ImmutableAssignment(local),
                format!(
                    "cannot assign to `in` parameter `{}`",
                    self.local_name(local)
                ),
            );
        }

        if is_root_assignment
            && !mutable
            && prior_assignments > 0
            && param_mode != Some(ParamMode::Out)
        {
            self.emit_immutable_binding_error(site.location, site.span, local, "assign to");
        }

        self.release_loans_for_place(state, place, site.location);
        if is_root_assignment {
            if let Some(decl) = self.function.body.local(local) {
                if matches!(decl.ty, Ty::ReadOnlySpan(_) | Ty::Span(_)) {
                    self.release_view(state, local, site.location);
                }
            }
        }

        {
            let facts = state.local_facts_mut(local);
            facts.init = InitState::Init;
            if is_root_assignment {
                facts.assignment_count = prior_assignments.saturating_add(1).min(2);
                facts.last_assignment = site.span;
            }
            facts.last_move = None;

            if is_root_assignment {
                if let Some(state_hint) = null_state {
                    let allow_internal_return_null = self
                        .function
                        .body
                        .local(local)
                        .is_some_and(|decl| decl.kind == LocalKind::Return)
                        && prior_assignments == 0
                        && (site.span.is_none() || site.span == facts.decl_span);
                    if !facts.nullable
                        && matches!(state_hint, NullState::Null)
                        && !allow_internal_return_null
                    {
                        self.report_error(
                            site.location,
                            site.span.or(facts.decl_span),
                            ErrorKeyKind::NullAssignment(local),
                            format!(
                                "cannot assign `null` to non-nullable binding `{}`",
                                self.local_name(local)
                            ),
                        );
                    }
                    if !facts.nullable && matches!(state_hint, NullState::Unknown) {
                        self.report_error(
                            site.location,
                            site.span.or(facts.decl_span),
                            ErrorKeyKind::MaybeNullAssignment(local),
                            format!(
                                "value assigned to `{}` may be `null`; add a guard or use `??` to provide a non-null default",
                                self.local_name(local)
                            ),
                        );
                    }
                    facts.null_state = match state_hint {
                        NullState::Null => {
                            if facts.nullable {
                                NullState::Null
                            } else {
                                NullState::Unknown
                            }
                        }
                        NullState::NonNull => NullState::NonNull,
                        NullState::Unknown => NullState::Unknown,
                    };
                } else if facts.nullable {
                    facts.null_state = NullState::Unknown;
                } else {
                    facts.null_state = NullState::NonNull;
                }
            }

            if is_root_assignment {
                match assignment_kind {
                    AssignmentKind::Regular => {
                        facts.stack_alloc = None;
                    }
                    AssignmentKind::StackAllocOrigin => {
                        facts.stack_alloc = Some(StackAllocState::origin(site.span));
                    }
                    AssignmentKind::StackAllocPropagated => {
                        facts.stack_alloc = Some(StackAllocState::propagated(site.span));
                    }
                }
            }
        }

        if let Some(state_info) = union_action {
            state.set_union_active(local, Some(state_info));
        } else if state.union_info(local).is_some() {
            state.set_union_active(local, Some(UnionActiveState::Unknown));
        }
    }

    pub(super) fn rvalue_null_state(
        &self,
        state: &BorrowState<'a>,
        value: &Rvalue,
    ) -> Option<NullState> {
        match value {
            Rvalue::Use(op)
            | Rvalue::Unary { operand: op, .. }
            | Rvalue::Cast { operand: op, .. } => self.operand_null_state(state, op),
            _ => None,
        }
    }

    fn operand_null_state(&self, state: &BorrowState<'a>, operand: &Operand) -> Option<NullState> {
        match operand {
            Operand::Const(constant) => {
                if matches!(constant.value(), ConstValue::Null) {
                    Some(NullState::Null)
                } else {
                    Some(NullState::NonNull)
                }
            }
            Operand::Copy(place) | Operand::Move(place) => {
                Some(self.place_null_state(state, place))
            }
            Operand::Borrow(borrow) => Some(self.place_null_state(state, &borrow.place)),
            Operand::Mmio(_) | Operand::Pending(_) => None,
        }
    }

    fn place_null_state(&self, state: &BorrowState<'a>, place: &Place) -> NullState {
        let facts = state.local_facts(place.local);
        if facts.nullable {
            facts.null_state
        } else {
            NullState::NonNull
        }
    }

    pub(super) fn visit_rvalue(
        &mut self,
        state: &mut BorrowState<'a>,
        value: &Rvalue,
        span: Option<Span>,
        location: Location,
    ) {
        match value {
            Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => {
                self.visit_operand(state, op, span, location);
            }
            Rvalue::Binary { lhs, rhs, .. } => {
                self.visit_operand(state, lhs, span, location);
                self.visit_operand(state, rhs, span, location);
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    self.visit_operand(state, field, span, location);
                }
            }
            Rvalue::AddressOf { place, .. } | Rvalue::Len(place) => {
                self.visit_place(state, place, span, location);
            }
            Rvalue::Cast { operand, .. } => self.visit_operand(state, operand, span, location),
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        self.visit_operand(state, operand, span, location);
                    }
                }
            }
            Rvalue::NumericIntrinsic(intrinsic) => {
                for operand in &intrinsic.operands {
                    self.visit_operand(state, operand, span, location);
                }
                if let Some(out) = &intrinsic.out {
                    self.visit_place(state, out, span, location);
                }
            }
            Rvalue::DecimalIntrinsic(decimal) => {
                self.visit_operand(state, &decimal.lhs, span, location);
                self.visit_operand(state, &decimal.rhs, span, location);
                if let Some(addend) = &decimal.addend {
                    self.visit_operand(state, addend, span, location);
                }
                self.visit_operand(state, &decimal.rounding, span, location);
                self.visit_operand(state, &decimal.vectorize, span, location);
            }
            Rvalue::AtomicLoad { target, .. } => {
                self.visit_place(state, target, span, location);
            }
            Rvalue::AtomicRmw { target, value, .. } => {
                self.visit_place(state, target, span, location);
                self.visit_operand(state, value, span, location);
            }
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                ..
            } => {
                self.visit_place(state, target, span, location);
                self.visit_operand(state, expected, span, location);
                self.visit_operand(state, desired, span, location);
            }
            Rvalue::StaticLoad { .. } => {}
            Rvalue::StaticRef { .. } => {}
            Rvalue::Pending(_) => {}
            Rvalue::SpanStackAlloc { .. } => {}
        }
    }

    pub(super) fn visit_operand(
        &mut self,
        state: &mut BorrowState<'a>,
        operand: &Operand,
        span: Option<Span>,
        location: Location,
    ) {
        match operand {
            Operand::Copy(place) => self.handle_operand_copy(state, place, span, location),
            Operand::Move(place) => self.handle_operand_move(state, place, span, location),
            Operand::Borrow(borrow) => {
                self.handle_operand_borrow(state, borrow, span, location);
            }
            Operand::Mmio(_) | Operand::Const(_) | Operand::Pending(_) => {}
        }
    }

    fn handle_operand_copy(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        span: Option<Span>,
        location: Location,
    ) {
        let site = StatementSite::new(span, location);
        self.ensure_initialized(
            state,
            place.local,
            site,
            ErrorKeyKind::UseOfUninit(place.local),
        );
        self.check_union_read(state, place, span, location);
        self.ensure_place_non_null(state, place, span, location);
    }

    fn handle_operand_move(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        span: Option<Span>,
        location: Location,
    ) {
        let site = StatementSite::new(span, location);
        if !self.ensure_initialized(
            state,
            place.local,
            site,
            ErrorKeyKind::UseOfUninit(place.local),
        ) {
            return;
        }

        if matches!(
            state.local_facts(place.local).param_mode,
            Some(ParamMode::In | ParamMode::Ref)
        ) {
            self.report_error(
                location,
                span,
                ErrorKeyKind::MoveOfParam(place.local),
                format!(
                    "cannot move `{}` because it is a `{}` parameter",
                    self.local_name(place.local),
                    match state.local_facts(place.local).param_mode {
                        Some(ParamMode::In) => "in",
                        Some(ParamMode::Ref) => "ref",
                        _ => "by-reference",
                    }
                ),
            );
            return;
        }

        self.check_union_read(state, place, span, location);
        self.ensure_place_non_null(state, place, span, location);

        if let Some((owner, dependents)) = self.view_dependents_for_place(place) {
            let dependent_list = dependents.join(", ");
            self.report_error(
                location,
                span,
                ErrorKeyKind::MoveBreaksViewDependency(place.local),
                format!(
                    "cannot move `{owner}` because dependent view field(s) [{dependent_list}] must be dropped or reassigned first"
                ),
            );
            return;
        }

        if state.local_facts(place.local).is_pinned() {
            self.report_error(
                location,
                span,
                                ErrorKeyKind::MoveOfPinned(place.local),
                format!(
                    "cannot move pinned binding `{}`; values marked with `@pin` or `Pin<T>` must remain in place",
                    self.local_name(place.local)
                ),
            );
            return;
        }

        if let Some(conflict) = Self::find_active_loan(state, place.local) {
            self.report_error(
                location,
                span,
                ErrorKeyKind::MoveWhileBorrowed(place.local),
                format!(
                    "cannot move `{}` while {:?} borrow is active (borrowed at {})",
                    self.local_name(place.local),
                    conflict.info.kind,
                    format_span(conflict.info.origin_span)
                ),
            );
        }

        let facts = state.local_facts_mut(place.local);
        facts.init = InitState::Uninit;
        facts.last_move = span;
        state.clear_union_state(place.local);
    }

    fn handle_operand_borrow(
        &mut self,
        state: &mut BorrowState<'a>,
        borrow: &BorrowOperand,
        span: Option<Span>,
        location: Location,
    ) {
        let site = StatementSite::new(span, location);
        if !self.out_argument_regions.contains(&borrow.region)
            && !matches!(borrow.kind, BorrowKind::Raw)
        {
            self.ensure_initialized(
                state,
                borrow.place.local,
                site,
                ErrorKeyKind::UseOfUninit(borrow.place.local),
            );
        }
        self.check_union_borrow(state, borrow, span, location);
        self.ensure_place_non_null(state, &borrow.place, span, location);
        if matches!(borrow.kind, BorrowKind::Unique) {
            let facts = state.local_facts(borrow.place.local);
            if !facts.mutable && facts.param_mode != Some(ParamMode::Out) {
                self.emit_immutable_binding_error(
                    location,
                    span.or(borrow.span),
                    borrow.place.local,
                    "take a mutable borrow of",
                );
            }
        }
        if (matches!(borrow.kind, BorrowKind::Unique) || union_field_index(&borrow.place).is_some())
            && let Some(conflict) = Self::find_conflicting_loan(state, &borrow.place, borrow.kind)
        {
            if conflict.info.region == borrow.region {
                self.release_loans_for_place(state, &borrow.place, location);
            } else {
                self.report_error(
                    location,
                    span,
                    ErrorKeyKind::BorrowConflict(borrow.place.local, borrow.kind),
                    format!(
                        "conflicting borrow of `{}`: existing {:?} borrow from {}",
                        self.local_name(borrow.place.local),
                        conflict.info.kind,
                        format_span(conflict.info.origin_span)
                    ),
                );
            }
        }
    }

    pub(super) fn visit_place(
        &mut self,
        state: &BorrowState<'a>,
        place: &Place,
        span: Option<Span>,
        location: Location,
    ) {
        let site = StatementSite::new(span, location);
        self.ensure_initialized(
            state,
            place.local,
            site,
            ErrorKeyKind::UseOfUninit(place.local),
        );
        if !place.projection.is_empty() {
            self.ensure_place_non_null(state, place, span, location);
        }
        self.check_union_read(state, place, span, location);
    }

    fn ensure_place_non_null(
        &mut self,
        state: &BorrowState<'a>,
        place: &Place,
        span: Option<Span>,
        location: Location,
    ) {
        let facts = state.local_facts(place.local);
        if !facts.nullable {
            return;
        }
        match facts.null_state {
            NullState::NonNull => {}
            NullState::Null => {
                self.report_error(
                    location,
                    span.or(facts.decl_span),
                    ErrorKeyKind::NullUse(place.local),
                    format!(
                        "dereferencing `{}` after it has been set to `null`",
                        self.local_name(place.local)
                    ),
                );
            }
            NullState::Unknown => {
                self.report_warning(
                    location,
                    span.or(facts.decl_span),
                    ErrorKeyKind::MaybeNullUse(place.local),
                    format!(
                        "value `{}` may be `null` here; guard the access or provide a default",
                        self.local_name(place.local)
                    ),
                );
            }
        }
    }

    pub(super) fn ensure_initialized(
        &mut self,
        state: &BorrowState<'a>,
        local: LocalId,
        site: StatementSite,
        key_kind: ErrorKeyKind,
    ) -> bool {
        let facts = state.local_facts(local);
        match facts.init {
            InitState::Init => true,
            InitState::Maybe => {
                if facts.requires_init {
                    self.report_error(
                        site.location,
                        site.span.or(facts.decl_span),
                        key_kind,
                        format!(
                            "use of `{}` may occur before it is assigned",
                            self.local_name(local)
                        ),
                    );
                }
                false
            }
            InitState::Uninit => {
                if facts.requires_init {
                    self.report_error(
                        site.location,
                        site.span.or(facts.decl_span),
                        key_kind,
                        format!("use of `{}` before it is assigned", self.local_name(local)),
                    );
                }
                false
            }
        }
    }

    pub(super) fn check_required_assignments(
        &mut self,
        state: &BorrowState<'a>,
        span: Option<Span>,
        location: Location,
    ) {
        for (index, facts) in state.locals.iter().enumerate() {
            if matches!(facts.param_mode, Some(ParamMode::Out)) && facts.assignment_count == 0 {
                self.report_error(
                    location,
                    span,
                    ErrorKeyKind::OutNotAssigned(LocalId(index)),
                    format!(
                        "`out` parameter `{}` was not assigned",
                        self.local_name(LocalId(index))
                    ),
                );
            }
        }
    }

    pub(super) fn find_conflicting_loan<'state>(
        state: &'state BorrowState<'a>,
        place: &Place,
        new_kind: BorrowKind,
    ) -> Option<&'state ActiveLoan> {
        let local = place.local;
        let new_index = union_field_index(place);
        for loan in state.loans_on_local(local) {
            if matches!(loan.info.kind, BorrowKind::Raw) || matches!(new_kind, BorrowKind::Raw) {
                continue;
            }

            let existing_index = union_field_index(&loan.info.place);
            match (new_index, existing_index) {
                (Some(a), Some(b)) => {
                    if a != b {
                        return Some(loan);
                    }
                    if matches!(loan.info.kind, BorrowKind::Shared)
                        && matches!(new_kind, BorrowKind::Shared)
                    {
                        continue;
                    }
                    return Some(loan);
                }
                (Some(_), None) | (None, Some(_)) => {
                    return Some(loan);
                }
                (None, None) => {
                    if matches!(loan.info.kind, BorrowKind::Shared)
                        && matches!(new_kind, BorrowKind::Shared)
                    {
                        continue;
                    }
                    return Some(loan);
                }
            }
        }
        None
    }

    pub(super) fn find_active_loan<'state>(
        state: &'state BorrowState<'a>,
        local: LocalId,
    ) -> Option<&'state ActiveLoan> {
        state.loans_on_local(local).next()
    }

    pub(super) fn union_field_mode(
        state: &BorrowState<'a>,
        local: LocalId,
        index: u32,
    ) -> Option<UnionFieldMode> {
        state.union_field(local, index).map(|field| field.mode)
    }

    pub(super) fn union_field_name(
        &self,
        state: &BorrowState<'a>,
        local: LocalId,
        index: u32,
    ) -> String {
        state.union_field(local, index).map_or_else(
            || format!("{}::field#{index}", self.local_name(local)),
            |field| field.name.clone(),
        )
    }

    fn view_dependents_for_place(&self, place: &Place) -> Option<(String, Vec<String>)> {
        if place.projection.is_empty() {
            return None;
        }

        let mut ty = self.function.body.locals.get(place.local.0)?.ty.clone();
        let mut owner_name: Option<String> = None;
        let mut owner_fields: Option<&[FieldLayout]> = None;
        let mut path = self.local_name(place.local);

        for elem in &place.projection {
            match elem {
                ProjectionElem::Field(index) => {
                    let layout = Self::struct_layout_for_ty(ty.clone(), self.type_layouts)?;
                    let field = layout.fields.iter().find(|field| field.index == *index)?;
                    path = format!("{path}.{}", field.name);
                    ty = field.ty.clone();
                    owner_name = Some(field.name.clone());
                    owner_fields = Some(&layout.fields);
                }
                ProjectionElem::Deref => {
                    ty = Self::deref_ty(&ty)?;
                    path.push_str(".*");
                    owner_name = None;
                }
                ProjectionElem::Index(_) | ProjectionElem::ConstantIndex { .. } => return None,
                ProjectionElem::UnionField { .. }
                | ProjectionElem::FieldNamed(_)
                | ProjectionElem::Downcast { .. }
                | ProjectionElem::Subslice { .. } => return None,
            }
        }

        let owner = owner_name?;
        let fields = owner_fields?;
        let dependents: Vec<String> = fields
            .iter()
            .filter_map(|field| {
                if field.view_of.as_deref() == Some(&owner) {
                    Some(field.name.clone())
                } else {
                    None
                }
            })
            .collect();
        if dependents.is_empty() {
            None
        } else {
            Some((path, dependents))
        }
    }

    fn struct_layout_for_ty<'b>(ty: Ty, layouts: &'b TypeLayoutTable) -> Option<&'b StructLayout> {
        match ty {
            Ty::Named(named) => match layouts.types.get(&named.name) {
                Some(TypeLayout::Struct(layout)) | Some(TypeLayout::Class(layout)) => Some(layout),
                _ => None,
            },
            Ty::Nullable(inner) => Self::struct_layout_for_ty(*inner, layouts),
            Ty::Pointer(ptr) => Self::struct_layout_for_ty(ptr.element.clone(), layouts),
            _ => None,
        }
    }

    fn deref_ty(ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Pointer(ptr) => Some(ptr.element.clone()),
            Ty::Nullable(inner) => Self::deref_ty(inner),
            _ => None,
        }
    }

    fn check_union_read(
        &mut self,
        state: &BorrowState<'a>,
        place: &Place,
        span: Option<Span>,
        location: Location,
    ) {
        let Some(info) = state.union_info(place.local) else {
            return;
        };
        let Some(field_index) = union_field_index(place) else {
            return;
        };
        let Some(field) = info.layout.views.iter().find(|f| f.index == field_index) else {
            return;
        };

        match info.active.as_ref() {
            Some(UnionActiveState::Field { index }) if *index == field_index => {}
            Some(UnionActiveState::Field { index }) => {
                let active_name = self.union_field_name(state, place.local, *index);
                let message = format!(
                    "cannot read union view `{}` while `{}` is active",
                    field.name, active_name
                );
                self.report_error(
                    location,
                    span.or(field.span),
                    ErrorKeyKind::UnionViewMismatch(place.local),
                    message,
                );
            }
            Some(UnionActiveState::Unknown) | None => {
                let message = format!("union view `{}` is not active", field.name);
                self.report_error(
                    location,
                    span.or(field.span),
                    ErrorKeyKind::UnionInactive(place.local),
                    message,
                );
            }
        }
    }

    fn check_union_borrow(
        &mut self,
        state: &BorrowState<'a>,
        borrow: &BorrowOperand,
        span: Option<Span>,
        location: Location,
    ) {
        self.check_union_read(state, &borrow.place, span, location);
        let Some(field_index) = union_field_index(&borrow.place) else {
            return;
        };
        let Some(mode) = Self::union_field_mode(state, borrow.place.local, field_index) else {
            return;
        };
        if matches!(mode, UnionFieldMode::Readonly) && matches!(borrow.kind, BorrowKind::Unique) {
            let message = format!(
                "cannot take mutable borrow of readonly union view `{}`",
                self.union_field_name(state, borrow.place.local, field_index)
            );
            self.report_error(
                location,
                span,
                ErrorKeyKind::UnionReadonly(borrow.place.local),
                message,
            );
        }
    }

    pub(super) fn determine_union_assignment(
        &mut self,
        state: &mut BorrowState<'a>,
        place: &Place,
        value: &Rvalue,
        site: StatementSite,
    ) -> Option<UnionActiveState> {
        let _ = state.union_info(place.local)?;

        if let Some(field_index) = union_field_index(place) {
            if matches!(
                Self::union_field_mode(state, place.local, field_index),
                Some(UnionFieldMode::Readonly)
            ) {
                let message = format!(
                    "cannot assign to readonly union view `{}`",
                    self.union_field_name(state, place.local, field_index)
                );
                self.report_error(
                    site.location,
                    site.span,
                    ErrorKeyKind::UnionReadonly(place.local),
                    message,
                );
            }
            return Some(UnionActiveState::Field { index: field_index });
        }

        let mut result: Option<UnionActiveState> = None;
        if let Rvalue::Use(operand) = value {
            match operand {
                Operand::Copy(src_place) | Operand::Move(src_place) => {
                    if let Some(index) = union_field_index(src_place) {
                        result = Some(UnionActiveState::Field { index });
                    } else if let Some(active) = state.union_active(src_place.local) {
                        result = Some(active.clone());
                    }
                }
                Operand::Borrow(borrow) => {
                    if let Some(index) = union_field_index(&borrow.place) {
                        result = Some(UnionActiveState::Field { index });
                    } else if let Some(active) = state.union_active(borrow.place.local) {
                        result = Some(active.clone());
                    }
                }
                _ => {}
            }
        }

        Some(result.unwrap_or(UnionActiveState::Unknown))
    }
}
