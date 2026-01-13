use super::base::is_trace_enabled;
use super::*;
use crate::mir::data::{
    BlockId, BorrowId, BorrowKind, FunctionKind, LocalDecl, LocalId, LocalKind, Operand, ParamMode,
    Place, RegionVar, Terminator,
};
use crate::mir::state::AsyncSuspendPoint;
use crate::mir::{BorrowCheckResult, MirFunction, TypeLayoutTable};

const SYNTHETIC_ID_BASE: usize = 1_000_000;

impl<'a> BorrowChecker<'a> {
    pub(in crate::mir::borrow) fn new(
        function: &'a MirFunction,
        layouts: &'a TypeLayoutTable,
    ) -> Self {
        Self {
            function,
            diagnostics: Vec::new(),
            reported: HashSet::new(),
            regions: HashMap::new(),
            out_argument_regions: HashSet::new(),
            type_layouts: layouts,
            next_synthetic_borrow: SYNTHETIC_ID_BASE,
            next_synthetic_region: SYNTHETIC_ID_BASE,
            stream_event_loan_cache: HashMap::new(),
            span_view_loan_cache: HashMap::new(),
        }
    }

    pub(super) fn await_metadata(&self, block: BlockId) -> Option<&AsyncSuspendPoint> {
        self.function
            .body
            .async_machine
            .as_ref()
            .and_then(|machine| {
                machine
                    .suspend_points
                    .iter()
                    .find(|point| point.await_block == block)
            })
    }

    pub(in crate::mir::borrow) fn run(&mut self) -> BorrowCheckResult {
        let body = &self.function.body;
        if body.blocks.is_empty() {
            return BorrowCheckResult::default();
        }

        self.out_argument_regions = collect_out_argument_regions(body);

        let trace_enabled = is_trace_enabled();
        let run_start = Instant::now();
        if trace_enabled {
            eprintln!(
                "[borrow::trace] run start for {} ({} blocks)",
                self.function.name,
                body.blocks.len()
            );
        }

        let mut worklist = VecDeque::new();
        let mut entry_state = BorrowState::new(body, self.type_layouts);
        self.seed_parameters(&mut entry_state);

        let mut in_states: Vec<Option<BorrowState<'a>>> = vec![None; body.blocks.len()];
        in_states[0] = Some(entry_state.clone());
        worklist.push_back((body.entry(), entry_state));

        let mut iterations: usize = 0;
        while let Some((block_id, mut state)) = worklist.pop_front() {
            iterations = iterations.saturating_add(1);
            if trace_enabled && iterations % 16 == 0 {
                eprintln!(
                    "[borrow::trace] iter {iterations}: visiting block {} (queue={})",
                    block_id.0,
                    worklist.len()
                );
            }
            let block = &body.blocks[block_id.0];
            for (index, statement) in block.statements.iter().enumerate() {
                let location = Location::Statement {
                    block: block_id,
                    index,
                };
                self.process_statement(&mut state, statement, location);
            }

            if let Some(terminator) = block.terminator.as_ref() {
                let location = Location::Terminator { block: block_id };
                self.process_terminator(&mut state, terminator, location, block);
                for successor in successors(terminator) {
                    let successor_state = in_states[successor.0].as_mut();
                    if let Some(existing) = successor_state {
                        if existing.merge_from(&state) {
                            worklist.push_back((successor, existing.clone()));
                        }
                    } else {
                        in_states[successor.0] = Some(state.clone());
                        worklist.push_back((successor, state.clone()));
                    }
                }
            }
        }

        let _ = &self.regions;

        if trace_enabled {
            eprintln!(
                "[borrow::trace] run complete for {} in {:?} (iterations={iterations})",
                self.function.name,
                run_start.elapsed()
            );
        }

        BorrowCheckResult {
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    fn seed_parameters(&mut self, state: &mut BorrowState<'a>) {
        for (index, decl) in self.function.body.locals.iter().enumerate() {
            let local_id = LocalId(index);
            self.seed_local(state, decl, local_id, index);
        }
    }

    fn seed_local(
        &mut self,
        state: &mut BorrowState<'a>,
        decl: &LocalDecl,
        local_id: LocalId,
        index: usize,
    ) {
        match decl.kind {
            LocalKind::Return => {
                let facts = state.local_facts_mut(local_id);
                Self::initialise_return_local(facts);
            }
            LocalKind::Arg(_) => {
                let mode = decl.param_mode.unwrap_or(ParamMode::Value);
                let borrow_kind = {
                    let facts = state.local_facts_mut(local_id);
                    let kind = Self::apply_param_mode(facts, mode);
                    if mode == ParamMode::Out
                        && self.function.kind == FunctionKind::Constructor
                        && decl
                            .name
                            .as_deref()
                            .map(|name| {
                                name.eq_ignore_ascii_case("self")
                                    || name.eq_ignore_ascii_case("this")
                            })
                            .unwrap_or(false)
                    {
                        facts.init = InitState::Init;
                        facts.requires_init = false;
                        facts.assignment_count = facts.assignment_count.max(1);
                        facts.last_assignment = facts.last_assignment.or(decl.span);
                    }
                    kind
                };
                if let Some(kind) = borrow_kind {
                    let request = ParamBorrow {
                        local: local_id,
                        index,
                        kind,
                        origin_span: decl.span,
                    };
                    self.register_param_borrow(state, request);
                }
            }
            LocalKind::Local | LocalKind::Temp => {
                let facts = state.local_facts_mut(local_id);
                Self::initialise_temporary(facts);
            }
        }

        Self::seed_union_state(state, decl, local_id);
    }

    fn initialise_return_local(facts: &mut LocalFacts) {
        facts.init = InitState::Uninit;
        facts.requires_init = true;
    }

    fn initialise_temporary(facts: &mut LocalFacts) {
        if facts.nullable {
            facts.init = InitState::Init;
        } else {
            facts.init = InitState::Uninit;
        }
    }

    fn apply_param_mode(facts: &mut LocalFacts, mode: ParamMode) -> Option<BorrowKind> {
        match mode {
            ParamMode::Value => {
                facts.init = InitState::Init;
                facts.requires_init = false;
                None
            }
            ParamMode::In => {
                facts.init = InitState::Init;
                facts.requires_init = false;
                None
            }
            ParamMode::Ref => {
                facts.init = InitState::Init;
                facts.requires_init = false;
                None
            }
            ParamMode::Out => {
                facts.init = InitState::Uninit;
                facts.requires_init = true;
                None
            }
        }
    }

    pub(super) fn allocate_synthetic_ids(&mut self) -> (BorrowId, RegionVar) {
        let borrow = BorrowId(self.next_synthetic_borrow);
        self.next_synthetic_borrow = self.next_synthetic_borrow.saturating_add(1);
        let region = RegionVar(self.next_synthetic_region);
        self.next_synthetic_region = self.next_synthetic_region.saturating_add(1);
        (borrow, region)
    }

    fn register_param_borrow(&mut self, state: &mut BorrowState<'a>, borrow: ParamBorrow) {
        let loan = LoanInfo {
            borrow_id: BorrowId(borrow.index),
            kind: borrow.kind,
            place: Place::new(borrow.local),
            region: RegionVar(borrow.index),
            origin_span: borrow.origin_span,
        };
        self.register_region(
            loan.region,
            loan.borrow_id,
            Location::Terminator {
                block: self.function.body.entry(),
            },
        );
        state.record_borrow(loan);
    }

    fn seed_union_state(state: &mut BorrowState<'a>, decl: &LocalDecl, local_id: LocalId) {
        if let Some(info) = state.union_info_mut(local_id) {
            match decl.kind {
                LocalKind::Arg(_) => match decl.param_mode.unwrap_or(ParamMode::Value) {
                    ParamMode::Out => info.active = None,
                    _ => info.active = Some(UnionActiveState::Unknown),
                },
                _ => info.active = None,
            }
        }
    }
}

fn collect_out_argument_regions(body: &crate::mir::MirBody) -> HashSet<RegionVar> {
    let mut out = HashSet::new();
    for block in &body.blocks {
        let Some(Terminator::Call {
            args, arg_modes, ..
        }) = block.terminator.as_ref()
        else {
            continue;
        };
        for (arg, mode) in args.iter().zip(arg_modes.iter()) {
            if !matches!(mode, ParamMode::Out) {
                continue;
            }
            if let Operand::Borrow(borrow) = arg {
                out.insert(borrow.region);
            }
        }
    }
    out
}

fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Goto { target } => vec![*target],
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => targets
            .iter()
            .map(|(_, block)| *block)
            .chain(std::iter::once(*otherwise))
            .collect(),
        Terminator::Match {
            arms, otherwise, ..
        } => arms
            .iter()
            .map(|arm| arm.target)
            .chain(std::iter::once(*otherwise))
            .collect(),
        Terminator::Call { target, unwind, .. } => {
            let mut out = vec![*target];
            if let Some(unwind) = unwind {
                out.push(*unwind);
            }
            out
        }
        Terminator::Yield { resume, drop, .. } | Terminator::Await { resume, drop, .. } => {
            vec![*resume, *drop]
        }
        Terminator::Return
        | Terminator::Throw { .. }
        | Terminator::Panic
        | Terminator::Unreachable
        | Terminator::Pending(_) => Vec::new(),
    }
}
