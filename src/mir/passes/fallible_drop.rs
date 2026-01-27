use std::collections::{HashSet, VecDeque};

use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::mir::data::{
    BasicBlock, BlockId, ConstValue, LocalDecl, LocalId, LocalKind, Operand, Place, Rvalue,
    Statement, StatementKind, Terminator,
};
use crate::mir::{Abi, FunctionKind, MirFunction, MirModule};

#[derive(Clone, PartialEq)]
struct FallibleOrigin {
    span: Option<Span>,
}

pub fn check_fallible_values(module: &MirModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for function in &module.functions {
        if matches!(function.signature.abi, Abi::Extern(_)) {
            continue;
        }
        if function.body.blocks.is_empty() {
            continue;
        }
        let mut analyzer = FunctionAnalyzer::new(module, function);
        analyzer.run(&mut diagnostics);
    }
    diagnostics
}

struct FunctionAnalyzer<'a> {
    function: &'a MirFunction,
    fallible_locals: Vec<bool>,
    ignored_locals: HashSet<usize>,
}

impl<'a> FunctionAnalyzer<'a> {
    fn new(module: &'a MirModule, function: &'a MirFunction) -> Self {
        let mut fallible_locals = Vec::new();
        let mut ignored_locals = HashSet::new();
        for (index, local) in function.body.locals.iter().enumerate() {
            let mut fallible = module.type_layouts.ty_is_fallible(&local.ty);
            if matches!(function.kind, FunctionKind::Constructor)
                && matches!(local.kind, LocalKind::Arg(0))
            {
                fallible = false;
            }
            if matches!(function.kind, FunctionKind::Method)
                && matches!(local.kind, LocalKind::Arg(0))
            {
                fallible = false;
                ignored_locals.insert(index);
            }
            if matches!(local.kind, LocalKind::Return) {
                fallible = false;
                ignored_locals.insert(index);
            }
            fallible_locals.push(fallible);
        }
        Self {
            function,
            fallible_locals,
            ignored_locals,
        }
    }

    fn run(&mut self, diagnostics: &mut Vec<Diagnostic>) {
        if !self.fallible_locals.iter().any(|flag| *flag) {
            return;
        }
        let block_count = self.function.body.blocks.len();
        let local_count = self.function.body.locals.len();
        let mut in_states = vec![vec![None; local_count]; block_count];
        let mut out_states = vec![vec![None; local_count]; block_count];
        self.seed_entry_state(&mut in_states[0]);
        let mut worklist = VecDeque::new();
        let mut queued = vec![false; block_count];
        worklist.push_back(0);
        queued[0] = true;

        while let Some(block_index) = worklist.pop_front() {
            queued[block_index] = false;
            let mut state = in_states[block_index].clone();
            self.apply_block(block_index, &mut state, diagnostics);
            if state != out_states[block_index] {
                out_states[block_index] = state.clone();
            }
            for succ in
                terminator_successors(self.function.body.blocks[block_index].terminator.as_ref())
            {
                let succ_index = succ.0;
                if merge_state(&mut in_states[succ_index], &state) && !queued[succ_index] {
                    worklist.push_back(succ_index);
                    queued[succ_index] = true;
                }
            }
        }
    }

    fn seed_entry_state(&self, state: &mut [Option<FallibleOrigin>]) {
        for (index, local) in self.function.body.locals.iter().enumerate() {
            if matches!(local.kind, LocalKind::Arg(_)) && self.is_tracked_local(index) {
                state[index] = Some(FallibleOrigin { span: local.span });
            }
        }
    }

    fn apply_block(
        &self,
        block_index: usize,
        state: &mut [Option<FallibleOrigin>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block = &self.function.body.blocks[block_index];
        for statement in &block.statements {
            self.handle_statement(statement, state, diagnostics);
        }
        if let Some(term) = &block.terminator {
            self.handle_terminator(block, term, state, diagnostics);
        } else {
            // Unterminated block still counts as scope exit.
            self.report_unhandled(state, block.span, diagnostics);
        }
    }

    fn handle_statement(
        &self,
        statement: &Statement,
        state: &mut [Option<FallibleOrigin>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match &statement.kind {
            StatementKind::Assign { place, value } => {
                if let Rvalue::Use(operand) = value {
                    match operand {
                        Operand::Move(operand_place) => {
                            if let Some(local) = self.place_local(operand_place) {
                                self.clear_local(local, state);
                            }
                        }
                        Operand::Copy(operand_place) => {
                            if let Some(local) = self.place_local(operand_place)
                                && matches!(self.local_decl(local).kind, LocalKind::Temp)
                            {
                                self.clear_local(local, state);
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(local) = self.place_local(place)
                    && matches!(value, Rvalue::Use(Operand::Const(constant)) if matches!(constant.value, ConstValue::Null))
                {
                    // Assigning `null` to a fallible local clears it; `null` does not represent an
                    // unhandled fallible value (e.g. try/catch exception slots start as `null`).
                    self.clear_local(local, state);
                } else {
                    self.record_assignment(place, statement.span, state);
                }
            }
            StatementKind::StorageDead(local) => {
                self.handle_storage_dead(*local, statement.span, state, diagnostics);
            }
            StatementKind::Drop { place, .. } => {
                if let Some(local) = self.place_local(place) {
                    self.handle_storage_dead(local, statement.span, state, diagnostics);
                }
            }
            StatementKind::MarkFallibleHandled { local } => {
                self.clear_local(*local, state);
            }
            _ => {}
        }
    }

    fn handle_terminator(
        &self,
        block: &BasicBlock,
        terminator: &Terminator,
        state: &mut [Option<FallibleOrigin>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match terminator {
            Terminator::Call { destination, .. } | Terminator::Await { destination, .. } => {
                if let Some(place) = destination {
                    self.record_assignment(place, block.span, state);
                }
            }
            Terminator::Match { value, .. } => {
                if let Some(local) = self.place_local(value) {
                    self.clear_local(local, state);
                }
            }
            Terminator::Return | Terminator::Panic | Terminator::Unreachable => {
                self.report_unhandled(state, block.span, diagnostics);
            }
            Terminator::Throw { exception, .. } => {
                if let Some(exception) = exception {
                    if let Some(local) = self.operand_local(exception) {
                        self.clear_local(local, state);
                    }
                }
                self.report_unhandled(state, block.span, diagnostics);
            }
            _ => {}
        }
    }

    fn record_assignment(
        &self,
        place: &Place,
        span: Option<Span>,
        state: &mut [Option<FallibleOrigin>],
    ) {
        if let Some(local) = self.place_local(place) {
            if self.is_tracked_local(local.0) {
                state[local.0] = Some(FallibleOrigin { span });
            } else {
                state[local.0] = None;
            }
        }
    }

    fn handle_storage_dead(
        &self,
        local: LocalId,
        span: Option<Span>,
        state: &mut [Option<FallibleOrigin>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if !self.is_tracked_local(local.0) {
            state[local.0] = None;
            return;
        }
        if let Some(origin) = state[local.0].take() {
            if matches!(self.local_decl(local).kind, LocalKind::Temp) {
                self.emit_drop_warning(local, span, origin.span, diagnostics);
            }
        }
    }

    fn emit_drop_warning(
        &self,
        local: LocalId,
        drop_span: Option<Span>,
        origin_span: Option<Span>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let message = format!(
            "[EH0001] fallible value `{}` is dropped without handling",
            self.describe_local(local),
        );
        diagnostics.push(Diagnostic::warning(message, drop_span));
        if let Some(origin_span) = origin_span {
            diagnostics.push(Diagnostic::note("value produced here", Some(origin_span)));
        }
    }

    fn report_unhandled(
        &self,
        state: &[Option<FallibleOrigin>],
        span: Option<Span>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for (index, origin) in state.iter().enumerate() {
            if origin.is_none() || !self.is_tracked_local(index) {
                continue;
            }
            let local = LocalId(index);
            let message = format!(
                "[EH0002] fallible value `{}` may exit this scope without being handled",
                self.describe_local(local),
            );
            diagnostics.push(Diagnostic::error(message, span));
            if let Some(origin_span) = origin.as_ref().and_then(|o| o.span) {
                diagnostics.push(Diagnostic::note("value produced here", Some(origin_span)));
            }
        }
    }

    fn place_local(&self, place: &Place) -> Option<LocalId> {
        if place.projection.is_empty() {
            Some(place.local)
        } else {
            None
        }
    }

    fn operand_local(&self, operand: &Operand) -> Option<LocalId> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place_local(place),
            _ => None,
        }
    }

    fn clear_local(&self, local: LocalId, state: &mut [Option<FallibleOrigin>]) {
        if local.0 < state.len() {
            state[local.0] = None;
        }
    }

    fn describe_local(&self, local: LocalId) -> String {
        self.local_decl(local)
            .name
            .clone()
            .unwrap_or_else(|| format!("_{}", local.0))
    }

    fn local_decl(&self, local: LocalId) -> &LocalDecl {
        &self.function.body.locals[local.0]
    }

    fn is_tracked_local(&self, index: usize) -> bool {
        self.fallible_locals.get(index).copied().unwrap_or(false)
            && !self.ignored_locals.contains(&index)
    }
}

fn merge_state(dest: &mut [Option<FallibleOrigin>], src: &[Option<FallibleOrigin>]) -> bool {
    let mut changed = false;
    for (dst, src) in dest.iter_mut().zip(src) {
        if dst.is_none() && src.is_some() {
            *dst = src.clone();
            changed = true;
        }
    }
    changed
}

fn terminator_successors(terminator: Option<&Terminator>) -> Vec<BlockId> {
    let mut succ = Vec::new();
    let Some(term) = terminator else {
        return succ;
    };
    match term {
        Terminator::Goto { target } => succ.push(*target),
        Terminator::SwitchInt {
            targets, otherwise, ..
        } => {
            succ.extend(targets.iter().map(|(_, block)| *block));
            succ.push(*otherwise);
        }
        Terminator::Match {
            arms, otherwise, ..
        } => {
            succ.extend(arms.iter().map(|arm| arm.target));
            succ.push(*otherwise);
        }
        Terminator::Call { target, unwind, .. } => {
            succ.push(*target);
            if let Some(cleanup) = unwind {
                succ.push(*cleanup);
            }
        }
        Terminator::Yield { resume, drop, .. } => {
            succ.push(*resume);
            succ.push(*drop);
        }
        Terminator::Await { resume, drop, .. } => {
            succ.push(*resume);
            succ.push(*drop);
        }
        Terminator::Throw { .. }
        | Terminator::Panic
        | Terminator::Return
        | Terminator::Unreachable
        | Terminator::Pending(_) => {}
    }
    succ
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::Span;
    use crate::mir::data::{
        BasicBlock, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, MirBody, Operand,
        Place, Rvalue, Statement,
    };
    use crate::mir::{MirFunction, MirModule, Terminator, Ty};

    fn fallible_ty() -> Ty {
        Ty::named("Std::Result")
    }

    fn default_sig() -> FnSig {
        FnSig {
            params: Vec::new(),
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        }
    }

    fn new_block(id: usize) -> BasicBlock {
        BasicBlock::new(BlockId(id), None)
    }

    fn assign_stmt(local: LocalId, span: Option<Span>) -> Statement {
        Statement {
            span,
            kind: StatementKind::Assign {
                place: Place::new(local),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
            },
        }
    }

    fn make_function(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> MirFunction {
        MirFunction {
            name: "Test::Function".into(),
            kind: FunctionKind::Function,
            signature: default_sig(),
            body: MirBody {
                arg_count: locals
                    .iter()
                    .filter(|local| matches!(local.kind, LocalKind::Arg(_)))
                    .count(),
                locals,
                blocks,
                span: None,
                async_machine: None,
                generator: None,
                exception_regions: Vec::new(),
                vectorize_decimal: false,
                effects: Vec::new(),
                stream_metadata: Vec::new(),
                debug_notes: Vec::new(),
            },
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        }
    }

    fn build_module(function: MirFunction) -> MirModule {
        let mut module = MirModule::default();
        module.functions.push(function);
        module
    }

    #[test]
    fn warns_on_temporaries_dropped_immediately() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("_tmp".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Temp,
        ));
        let mut block = new_block(0);
        block.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageLive(LocalId(1)),
        });
        block
            .statements
            .push(assign_stmt(LocalId(1), Some(Span::new(2, 3))));
        block.statements.push(Statement {
            span: Some(Span::new(3, 4)),
            kind: StatementKind::StorageDead(LocalId(1)),
        });
        block.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![block]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("EH0001")),
            "expected EH0001 warning, got {diagnostics:#?}"
        );
    }

    #[test]
    fn explicit_discard_clears_state() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("_".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Temp,
        ));
        let mut block = new_block(0);
        block
            .statements
            .push(assign_stmt(LocalId(1), Some(Span::new(2, 3))));
        block.statements.push(Statement {
            span: Some(Span::new(2, 3)),
            kind: StatementKind::MarkFallibleHandled { local: LocalId(1) },
        });
        block.statements.push(Statement {
            span: Some(Span::new(3, 4)),
            kind: StatementKind::StorageDead(LocalId(1)),
        });
        block.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![block]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        assert!(
            diagnostics.is_empty(),
            "discard should clear fallible state, got {diagnostics:#?}"
        );
    }

    #[test]
    fn reports_unhandled_exit() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("value".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Local,
        ));
        let mut block = new_block(0);
        block
            .statements
            .push(assign_stmt(LocalId(1), Some(Span::new(2, 3))));
        block.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![block]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("EH0002")),
            "expected EH0002 error, got {diagnostics:#?}"
        );
    }

    #[test]
    fn pattern_match_consumes_value() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("result".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Local,
        ));
        let mut entry = new_block(0);
        entry
            .statements
            .push(assign_stmt(LocalId(1), Some(Span::new(2, 3))));
        entry.terminator = Some(Terminator::Match {
            value: Place::new(LocalId(1)),
            arms: vec![
                crate::mir::data::MatchArm {
                    pattern: crate::mir::data::Pattern::Wildcard,
                    guard: None,
                    bindings: Vec::new(),
                    target: BlockId(1),
                },
                crate::mir::data::MatchArm {
                    pattern: crate::mir::data::Pattern::Wildcard,
                    guard: None,
                    bindings: Vec::new(),
                    target: BlockId(2),
                },
            ],
            otherwise: BlockId(1),
        });
        let mut then_block = new_block(1);
        then_block.terminator = Some(Terminator::Return);
        let mut else_block = new_block(2);
        else_block.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![entry, then_block, else_block]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        assert!(
            diagnostics.is_empty(),
            "match terminator should consume fallible value, got {diagnostics:#?}"
        );
    }

    #[test]
    fn await_results_are_tracked() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("value".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Local,
        ));
        let mut entry = new_block(0);
        entry.terminator = Some(Terminator::Await {
            future: Place::new(LocalId(1)),
            destination: Some(Place::new(LocalId(1))),
            resume: BlockId(1),
            drop: BlockId(2),
        });
        let mut resume_block = new_block(1);
        resume_block.terminator = Some(Terminator::Return);
        let mut drop_block = new_block(2);
        drop_block.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![entry, resume_block, drop_block]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("EH0002")),
            "awaited result should be tracked until handled"
        );
    }

    #[test]
    fn multi_branch_exit_reports_each_path() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("value".into()),
            fallible_ty(),
            true,
            Some(Span::new(1, 2)),
            LocalKind::Local,
        ));
        let mut entry = new_block(0);
        entry
            .statements
            .push(assign_stmt(LocalId(1), Some(Span::new(2, 3))));
        entry.terminator = Some(Terminator::SwitchInt {
            discr: Operand::Const(ConstOperand::new(ConstValue::Int(0))),
            targets: vec![(0, BlockId(1))],
            otherwise: BlockId(2),
        });
        let mut first = new_block(1);
        first.terminator = Some(Terminator::Return);
        let mut second = new_block(2);
        second.terminator = Some(Terminator::Return);
        let function = make_function(locals, vec![entry, first, second]);
        let module = build_module(function);
        let diagnostics = check_fallible_values(&module);
        let error_count = diagnostics
            .iter()
            .filter(|diag| diag.message.contains("EH0002"))
            .count();
        assert_eq!(
            error_count, 2,
            "expected one EH0002 per exit path, got {diagnostics:#?}"
        );
    }
}
