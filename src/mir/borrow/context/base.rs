use crate::frontend::diagnostics::Span;
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, LocalDecl, LocalId, LocalKind, MirBody, Operand,
    ParamMode, Place, ProjectionElem, RegionVar, Ty,
};
use crate::mir::layout::{TypeLayout, TypeLayoutTable, UnionFieldLayout, UnionLayout};
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
static TRACE_OVERRIDE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ErrorKeyKind {
    UseOfUninit(LocalId),
    MoveOfParam(LocalId),
    MoveWhileBorrowed(LocalId),
    MoveOfPinned(LocalId),
    MoveBreaksViewDependency(LocalId),
    BorrowConflict(LocalId, BorrowKind),
    AwaitWithUniqueBorrow,
    AwaitWithStackAlloc(LocalId),
    AwaitRequiresPin(LocalId),
    AwaitRequiresThreadSafe(LocalId),
    AwaitRequiresShareable(LocalId),
    OutNotAssigned(LocalId),
    ImmutableAssignment(LocalId),
    UnionInactive(LocalId),
    UnionReadonly(LocalId),
    UnionViewMismatch(LocalId),
    NullAssignment(LocalId),
    MaybeNullAssignment(LocalId),
    NullUse(LocalId),
    MaybeNullUse(LocalId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ErrorKey {
    pub(super) block: usize,
    pub(super) statement_index: Option<usize>,
    pub(super) kind: ErrorKeyKind,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Location {
    Statement { block: BlockId, index: usize },
    Terminator { block: BlockId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InitState {
    Uninit,
    Maybe,
    Init,
}

impl InitState {
    fn join(self, other: InitState) -> InitState {
        match (self, other) {
            (InitState::Init, InitState::Init) => InitState::Init,
            (InitState::Uninit, InitState::Uninit) => InitState::Uninit,
            _ => InitState::Maybe,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NullState {
    Unknown,
    Null,
    NonNull,
}

impl NullState {
    fn join(self, other: NullState) -> NullState {
        match (self, other) {
            (NullState::NonNull, NullState::NonNull) => NullState::NonNull,
            (NullState::Null, NullState::Null) => NullState::Null,
            (NullState::Unknown, _) | (_, NullState::Unknown) => NullState::Unknown,
            _ => NullState::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum StackAllocState {
    Origin { span: Option<Span> },
    Propagated { span: Option<Span> },
}

impl StackAllocState {
    pub(super) const fn origin(span: Option<Span>) -> Self {
        Self::Origin { span }
    }

    pub(super) const fn propagated(span: Option<Span>) -> Self {
        Self::Propagated { span }
    }
}

#[derive(Clone)]
pub(super) struct LocalFacts {
    pub(super) init: InitState,
    pub(super) requires_init: bool,
    pub(super) assignment_count: u32,
    pub(super) last_assignment: Option<Span>,
    pub(super) last_move: Option<Span>,
    pub(super) param_mode: Option<ParamMode>,
    pub(super) decl_span: Option<Span>,
    pub(super) mutable: bool,
    pub(super) pinned: bool,
    pub(super) nullable: bool,
    pub(super) null_state: NullState,
    pub(super) stack_alloc: Option<StackAllocState>,
}

impl LocalFacts {
    pub(super) fn from_decl(decl: &LocalDecl) -> Self {
        let requires_init = matches!(decl.kind, LocalKind::Return | LocalKind::Local);
        let nullable = decl.is_nullable;
        let null_state = match decl.kind {
            LocalKind::Arg(_) => NullState::Unknown,
            _ if nullable => NullState::Null,
            _ => NullState::Unknown,
        };
        Self {
            init: InitState::Uninit,
            requires_init,
            assignment_count: 0,
            last_assignment: None,
            last_move: None,
            param_mode: decl.param_mode,
            decl_span: decl.span,
            mutable: decl.mutable,
            pinned: decl.is_pinned,
            nullable,
            null_state,
            stack_alloc: None,
        }
    }

    pub(super) fn merge_from(&mut self, other: &LocalFacts) -> bool {
        let mut changed = false;
        let joined = self.init.join(other.init);
        if joined != self.init {
            self.init = joined;
            changed = true;
        }

        if other.assignment_count > self.assignment_count {
            self.assignment_count = other.assignment_count;
            self.last_assignment = other.last_assignment;
            changed = true;
        }

        if self.last_move.is_none() && other.last_move.is_some() {
            self.last_move = other.last_move;
            changed = true;
        }

        let requires_init = self.requires_init || other.requires_init;
        if requires_init != self.requires_init {
            self.requires_init = requires_init;
            changed = true;
        }

        let joined_null = self.null_state.join(other.null_state);
        if joined_null != self.null_state {
            self.null_state = joined_null;
            changed = true;
        }

        let merged_stack_alloc = match (&self.stack_alloc, &other.stack_alloc) {
            (Some(lhs), Some(_)) => Some(lhs.clone()),
            _ => None,
        };
        if self.stack_alloc != merged_stack_alloc {
            self.stack_alloc = merged_stack_alloc;
            changed = true;
        }

        changed
    }

    pub(super) fn is_pinned(&self) -> bool {
        self.pinned
    }
}

#[derive(Clone)]
pub(super) struct LoanInfo {
    pub(super) borrow_id: BorrowId,
    pub(super) kind: BorrowKind,
    pub(super) place: Place,
    pub(super) region: RegionVar,
    pub(super) origin_span: Option<Span>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Presence {
    Present,
    Maybe,
}

#[derive(Clone, Copy)]
pub(super) struct StatementSite {
    pub(super) span: Option<Span>,
    pub(super) location: Location,
}

impl StatementSite {
    pub(super) fn new(span: Option<Span>, location: Location) -> Self {
        Self { span, location }
    }
}

#[derive(Clone, Copy)]
pub(super) struct BorrowRequest<'p> {
    pub(super) borrow_id: BorrowId,
    pub(super) kind: BorrowKind,
    pub(super) place: &'p Place,
    pub(super) region: RegionVar,
    pub(super) site: StatementSite,
}

pub(super) struct CallOperands<'o> {
    pub(super) func: &'o Operand,
    pub(super) args: &'o [Operand],
    pub(super) modes: &'o [ParamMode],
    pub(super) destination: Option<&'o Place>,
}

#[derive(Clone, Copy)]
pub(super) struct TraitReport<'a> {
    pub(super) loan_local: LocalId,
    pub(super) place: &'a Place,
    pub(super) place_description: &'a str,
    pub(super) ty_name: &'a str,
    pub(super) site: StatementSite,
}

#[derive(Clone, Copy)]
pub(super) struct ParamBorrow {
    pub(super) local: LocalId,
    pub(super) index: usize,
    pub(super) kind: BorrowKind,
    pub(super) origin_span: Option<Span>,
}

#[derive(Clone, Copy)]
pub(super) struct TerminatorContext<'b> {
    pub(super) block: &'b BasicBlock,
    pub(super) location: Location,
}

impl<'b> TerminatorContext<'b> {
    pub(super) fn new(block: &'b BasicBlock, location: Location) -> Self {
        Self { block, location }
    }

    pub(super) fn span(self) -> Option<Span> {
        self.block.span
    }
}

pub(super) fn resolve_union_layout<'a>(
    layouts: &'a TypeLayoutTable,
    ty: &Ty,
) -> Option<&'a UnionLayout> {
    let Ty::Named(name) = ty else {
        return None;
    };
    if let Some(TypeLayout::Union(layout)) = layouts.types.get(name.as_str()) {
        return Some(layout);
    }

    if name.contains("::") {
        return None;
    }

    let mut candidate: Option<&UnionLayout> = None;
    for (key, layout) in &layouts.types {
        let TypeLayout::Union(union_layout) = layout else {
            continue;
        };
        if key.rsplit("::").next() != Some(name.as_str()) {
            continue;
        }
        if candidate.is_some() {
            return None;
        }
        candidate = Some(union_layout);
    }
    candidate
}

pub(super) fn union_field_index(place: &Place) -> Option<u32> {
    place.projection.iter().find_map(|elem| match elem {
        ProjectionElem::UnionField { index, .. } => Some(*index),
        _ => None,
    })
}

pub(super) fn is_trace_enabled() -> bool {
    #[cfg(test)]
    {
        if TRACE_OVERRIDE.load(Ordering::Relaxed) {
            return true;
        }
    }
    std::env::var_os("CHIC_WASM_TRACE").is_some()
}

#[cfg(test)]
pub(super) fn enable_trace_for_tests() -> TraceGuard {
    TRACE_OVERRIDE.store(true, Ordering::Relaxed);
    TraceGuard
}

#[cfg(test)]
pub(super) struct TraceGuard;

#[cfg(test)]
impl Drop for TraceGuard {
    fn drop(&mut self) {
        TRACE_OVERRIDE.store(false, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub(super) struct ActiveLoan {
    pub(super) info: LoanInfo,
    pub(super) presence: Presence,
    pub(super) associated_view: Option<LocalId>,
    pub(super) until_event: Option<LocalId>,
}

#[derive(Clone)]
pub(super) struct UnionLocalInfo<'a> {
    pub(super) layout: &'a UnionLayout,
    pub(super) active: Option<UnionActiveState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum UnionActiveState {
    Field { index: u32 },
    Unknown,
}

#[derive(Clone)]
pub(super) struct BorrowState<'a> {
    pub(super) locals: Vec<LocalFacts>,
    pub(super) active_loans: HashMap<BorrowId, ActiveLoan>,
    pub(super) union_locals: Vec<Option<UnionLocalInfo<'a>>>,
    pub(super) unsafe_depth: usize,
}

impl<'a> BorrowState<'a> {
    pub(super) fn new(body: &MirBody, layouts: &'a TypeLayoutTable) -> Self {
        let locals = body
            .locals
            .iter()
            .map(LocalFacts::from_decl)
            .collect::<Vec<_>>();
        let union_locals = body
            .locals
            .iter()
            .map(|decl| {
                resolve_union_layout(layouts, &decl.ty).map(|layout| UnionLocalInfo {
                    layout,
                    active: None,
                })
            })
            .collect::<Vec<_>>();
        Self {
            locals,
            active_loans: HashMap::new(),
            union_locals,
            unsafe_depth: 0,
        }
    }

    pub(super) fn merge_from(&mut self, other: &BorrowState<'a>) -> bool {
        let mut changed = false;
        if self.merge_locals(other) {
            changed = true;
        }
        if self.merge_union_locals(other) {
            changed = true;
        }
        if self.merge_active_loans(other) {
            changed = true;
        }
        if self.merge_unsafe_depth(other) {
            changed = true;
        }
        changed
    }

    pub(super) fn merge_locals(&mut self, other: &BorrowState<'a>) -> bool {
        let mut changed = false;
        for (lhs, rhs) in self.locals.iter_mut().zip(&other.locals) {
            if lhs.merge_from(rhs) {
                changed = true;
            }
        }
        changed
    }

    pub(super) fn merge_union_locals(&mut self, other: &BorrowState<'a>) -> bool {
        let mut changed = false;
        for (lhs, rhs) in self.union_locals.iter_mut().zip(&other.union_locals) {
            match (lhs.as_mut(), rhs.as_ref()) {
                (Some(lhs_info), Some(rhs_info)) => {
                    if Self::merge_union_active_state(lhs_info, rhs_info) {
                        changed = true;
                    }
                }
                (None, Some(rhs_info)) => {
                    *lhs = Some(rhs_info.clone());
                    changed = true;
                }
                _ => {}
            }
        }
        changed
    }

    pub(super) fn merge_union_active_state(
        lhs_info: &mut UnionLocalInfo<'a>,
        rhs_info: &UnionLocalInfo<'a>,
    ) -> bool {
        let joined = match (&lhs_info.active, &rhs_info.active) {
            (None, None) => None,
            (Some(UnionActiveState::Unknown), _) | (_, Some(UnionActiveState::Unknown)) => {
                Some(UnionActiveState::Unknown)
            }
            (
                Some(UnionActiveState::Field { index: a }),
                Some(UnionActiveState::Field { index: b }),
            ) if a == b => Some(UnionActiveState::Field { index: *a }),
            _ => Some(UnionActiveState::Unknown),
        };

        if lhs_info.active == joined {
            return false;
        }
        lhs_info.active = joined;
        true
    }

    pub(super) fn merge_active_loans(&mut self, other: &BorrowState<'a>) -> bool {
        let mut changed = false;
        for (id, loan) in &mut self.active_loans {
            if let Some(other_loan) = other.active_loans.get(id) {
                let presence = match (loan.presence, other_loan.presence) {
                    (Presence::Present, Presence::Present) => Presence::Present,
                    _ => Presence::Maybe,
                };
                if loan.presence != presence {
                    loan.presence = presence;
                    changed = true;
                }
                if loan.associated_view != other_loan.associated_view {
                    loan.associated_view = None;
                    changed = true;
                }
                if loan.until_event != other_loan.until_event {
                    loan.until_event = match (loan.until_event, other_loan.until_event) {
                        (Some(a), Some(b)) if a == b => Some(a),
                        _ => None,
                    };
                    changed = true;
                }
            } else if loan.presence != Presence::Maybe {
                loan.presence = Presence::Maybe;
                changed = true;
            }
        }

        for (id, loan) in &other.active_loans {
            if !self.active_loans.contains_key(id) {
                let mut cloned = loan.clone();
                if cloned.presence == Presence::Present {
                    cloned.presence = Presence::Maybe;
                }
                self.active_loans.insert(*id, cloned);
                changed = true;
            }
        }
        changed
    }

    pub(super) fn merge_unsafe_depth(&mut self, other: &BorrowState<'a>) -> bool {
        if self.unsafe_depth == other.unsafe_depth {
            return false;
        }
        let merged = self.unsafe_depth.min(other.unsafe_depth);
        if merged != self.unsafe_depth {
            self.unsafe_depth = merged;
            return true;
        }
        false
    }

    pub(super) fn loans_on_local(&self, local: LocalId) -> impl Iterator<Item = &ActiveLoan> {
        self.active_loans
            .values()
            .filter(move |loan| loan.info.place.local == local)
    }

    pub(super) fn remove_loans_for_place(&mut self, place: &Place) -> Vec<ActiveLoan> {
        let mut removed = Vec::new();
        let keys: Vec<BorrowId> = self
            .active_loans
            .iter()
            .filter_map(|(id, loan)| {
                if loan.info.place.local == place.local {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in keys {
            if let Some(loan) = self.active_loans.remove(&id) {
                removed.push(loan);
            }
        }
        removed
    }

    pub(super) fn remove_loans_for_view(&mut self, view: LocalId) -> Vec<ActiveLoan> {
        let mut removed = Vec::new();
        let keys: Vec<BorrowId> = self
            .active_loans
            .iter()
            .filter_map(|(id, loan)| {
                if loan.associated_view == Some(view) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in keys {
            if let Some(loan) = self.active_loans.remove(&id) {
                removed.push(loan);
            }
        }
        removed
    }

    pub(super) fn remove_loans_for_event(&mut self, event: LocalId) -> Vec<ActiveLoan> {
        let mut removed = Vec::new();
        let keys: Vec<BorrowId> = self
            .active_loans
            .iter()
            .filter_map(|(id, loan)| {
                if loan.until_event == Some(event) {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for id in keys {
            if let Some(loan) = self.active_loans.remove(&id) {
                removed.push(loan);
            }
        }
        removed
    }

    pub(super) fn record_borrow(&mut self, loan: LoanInfo) {
        self.record_borrow_until(loan, None);
    }

    pub(super) fn record_borrow_until(&mut self, loan: LoanInfo, until_event: Option<LocalId>) {
        self.active_loans.insert(
            loan.borrow_id,
            ActiveLoan {
                info: loan,
                presence: Presence::Present,
                associated_view: None,
                until_event,
            },
        );
    }

    pub(super) fn local_facts(&self, local: LocalId) -> &LocalFacts {
        &self.locals[local.0]
    }

    pub(super) fn local_facts_mut(&mut self, local: LocalId) -> &mut LocalFacts {
        &mut self.locals[local.0]
    }

    pub(super) fn union_info(&self, local: LocalId) -> Option<&UnionLocalInfo<'a>> {
        self.union_locals
            .get(local.0)
            .and_then(|info| info.as_ref())
    }

    pub(super) fn union_info_mut(&mut self, local: LocalId) -> Option<&mut UnionLocalInfo<'a>> {
        self.union_locals
            .get_mut(local.0)
            .and_then(|info| info.as_mut())
    }

    pub(super) fn clear_union_state(&mut self, local: LocalId) {
        if let Some(info) = self.union_info_mut(local) {
            info.active = None;
        }
    }

    pub(super) fn set_union_active(&mut self, local: LocalId, state: Option<UnionActiveState>) {
        if let Some(info) = self.union_info_mut(local) {
            info.active = state;
        }
    }

    pub(super) fn union_active(&self, local: LocalId) -> Option<&UnionActiveState> {
        self.union_info(local).and_then(|info| info.active.as_ref())
    }

    pub(super) fn union_field(&self, local: LocalId, index: u32) -> Option<&UnionFieldLayout> {
        self.union_info(local)
            .and_then(|info| info.layout.views.iter().find(|field| field.index == index))
    }
}

#[allow(dead_code)]
pub(super) struct RegionInfo {
    pub(super) start: Location,
    pub(super) end: Option<Location>,
    pub(super) loans: Vec<BorrowId>,
}
