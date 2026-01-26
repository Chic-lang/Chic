use std::collections::VecDeque;

use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode, FileId, Label, Span};
use crate::mir::data::{
    BinOp, BlockId, BorrowKind, ConstValue, LocalId, MirFunction, MirModule, Operand, ParamMode,
    Rvalue, StatementKind, Terminator, UnOp,
};

const UNREACHABLE_CODE: &str = "E0400";
const CATEGORY: &str = "reachability";

/// Perform reachability analysis over MIR bodies and emit unreachable-code diagnostics.
pub fn check_unreachable_code(module: &MirModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for function in &module.functions {
        if function.body.blocks.is_empty() {
            continue;
        }
        let mut analyzer = ReachabilityAnalyzer::new(function);
        analyzer.run(&mut diagnostics);
    }
    diagnostics
}

struct ReachabilityAnalyzer<'a> {
    function: &'a MirFunction,
    reachable: Vec<bool>,
    reasons: Vec<Option<UnreachableCause>>,
    predecessors: Vec<Vec<BlockId>>,
    function_consts: ConstEnv,
}

#[derive(Clone)]
struct UnreachableCause {
    span: Option<Span>,
    label: Option<String>,
    note: String,
}

impl<'a> ReachabilityAnalyzer<'a> {
    fn new(function: &'a MirFunction) -> Self {
        let block_count = function.body.blocks.len();
        let mut predecessors = vec![Vec::new(); block_count];
        for block in &function.body.blocks {
            for succ in all_successors(block.terminator.as_ref()) {
                if let Some(preds) = predecessors.get_mut(succ.0) {
                    preds.push(block.id);
                }
            }
        }
        Self {
            function,
            reachable: vec![false; block_count],
            reasons: vec![None; block_count],
            predecessors,
            function_consts: compute_function_consts(function),
        }
    }

    fn run(&mut self, diagnostics: &mut Vec<Diagnostic>) {
        self.propagate();
        let mut reported_spans = std::collections::HashSet::new();
        for (index, block) in self.function.body.blocks.iter().enumerate() {
            if self.reachable.get(index).copied().unwrap_or(false) {
                continue;
            }
            let body_span = self.function.body.span.or(self.function.span);
            let reason = self.reasons.get(index).cloned().flatten();
            let has_predecessors = self
                .predecessors
                .get(index)
                .map(|preds| !preds.is_empty())
                .unwrap_or(false);
            let has_reachable_pred = self
                .predecessors
                .get(index)
                .map(|preds| {
                    preds
                        .iter()
                        .any(|pred| self.reachable.get(pred.0).copied().unwrap_or(false))
                })
                .unwrap_or(false);
            let Some((mut span, mut from_statement)) = block_primary_span(block, has_predecessors)
            else {
                continue;
            };
            let mut span_is_fallback = false;
            if span.file_id == FileId::UNKNOWN {
                let fallback = body_span
                    .filter(|span| span.file_id != FileId::UNKNOWN)
                    .or_else(|| {
                        self.function.body.blocks.iter().find_map(|block| {
                            block
                                .statements
                                .iter()
                                .find_map(|stmt| stmt.span)
                                .or(block.span)
                                .filter(|span| span.file_id != FileId::UNKNOWN)
                        })
                    });
                if let Some(fallback) = fallback {
                    span = fallback;
                    from_statement = true;
                    span_is_fallback = true;
                }
            }
            let only_storage = block.statements.iter().all(|stmt| {
                matches!(
                    stmt.kind,
                    StatementKind::Nop
                        | StatementKind::StorageLive(_)
                        | StatementKind::StorageDead(_)
                        | StatementKind::MarkFallibleHandled { .. }
                )
            });
            if only_storage
                && matches!(block.terminator, Some(Terminator::Return))
                && body_span.is_some_and(|body_span| block.span == Some(body_span))
            {
                continue;
            }
            if !has_reachable_pred && reason.is_none() && only_storage {
                let trivial = match block.terminator {
                    None
                    | Some(Terminator::Return | Terminator::Panic | Terminator::Unreachable) => {
                        true
                    }
                    Some(Terminator::Goto { target }) => self
                        .function
                        .body
                        .blocks
                        .get(target.0)
                        .is_some_and(|target_block| {
                            matches!(target_block.terminator, Some(Terminator::Return))
                                && body_span
                                    .is_some_and(|body_span| target_block.span == Some(body_span))
                        }),
                    _ => false,
                };
                if trivial {
                    continue;
                }
            }
            if !has_reachable_pred
                && block.statements.is_empty()
                && matches!(block.terminator, Some(Terminator::Return))
                && body_span.is_some_and(|body_span| block.span == Some(body_span))
            {
                continue;
            }
            let meaningful_stmt = block.statements.iter().any(|stmt| {
                !matches!(
                    stmt.kind,
                    StatementKind::Nop
                        | StatementKind::StorageLive(_)
                        | StatementKind::StorageDead(_)
                        | StatementKind::MarkFallibleHandled { .. }
                )
            });
            let meaningful_term = matches!(
                block.terminator,
                Some(
                    Terminator::Return
                        | Terminator::Throw { .. }
                        | Terminator::Panic
                        | Terminator::Unreachable
                )
            ) && block
                .span
                .zip(body_span)
                .map(|(block_span, body_span)| block_span != body_span)
                .unwrap_or(true);
            if !has_reachable_pred && reason.is_none() && !(meaningful_stmt || meaningful_term) {
                continue;
            }
            if !from_statement && body_span.is_some_and(|body_span| body_span == span) {
                continue;
            }
            let span_key = (span.file_id, span.start, span.end);
            if !reported_spans.insert(span_key) {
                continue;
            }
            let mut diag = Diagnostic::error("unreachable code", Some(span));
            diag.code = Some(DiagnosticCode::new(
                UNREACHABLE_CODE.to_string(),
                Some(CATEGORY.into()),
            ));
            diag = diag.with_primary_label("unreachable code");

            if let Some(reason) = reason {
                if let Some(reason_span) = reason.span {
                    if let Some(label) = reason.label {
                        diag = diag.with_secondary(Label::secondary(reason_span, label));
                    }
                }
                diag.notes.push(reason.note);
            } else if let Some((note, exit_span)) = self.terminator_exit_note(BlockId(index)) {
                if let Some(span) = exit_span {
                    diag = diag
                        .with_secondary(Label::secondary(span, "control flow always exits here"));
                }
                diag.notes.push(note);
            } else {
                diag.notes
                    .push("control flow cannot reach this statement".into());
            }
            if span_is_fallback {
                diag.notes.push(format!(
                    "internal: reachability span missing; reporting at function `{}`",
                    self.function.name
                ));
            }

            diagnostics.push(diag);
        }
    }

    fn propagate(&mut self) {
        if self.function.body.blocks.is_empty() {
            return;
        }
        let mut worklist = VecDeque::new();
        self.mark_reachable(self.function.body.entry());
        worklist.push_back(self.function.body.entry());

        while let Some(block_id) = worklist.pop_front() {
            let Some(block) = self.function.body.blocks.get(block_id.0) else {
                continue;
            };
            let Some(term) = block.terminator.as_ref() else {
                continue;
            };
            let const_env = block_const_env(block, &self.function_consts);
            match term {
                Terminator::SwitchInt {
                    discr,
                    targets,
                    otherwise,
                } => {
                    let cond_span = block
                        .statements
                        .iter()
                        .find_map(|stmt| stmt.span)
                        .or(block.span);
                    let function_consts = self.function_consts.clone();
                    if let Some((taken, skipped, reason)) = self.evaluate_switch(
                        cond_span,
                        discr,
                        targets,
                        *otherwise,
                        &const_env,
                        &function_consts,
                    ) {
                        self.enqueue_target(taken, &mut worklist);
                        for skipped_block in skipped {
                            if let Some(reason) = reason.clone() {
                                self.record_reason(skipped_block, reason);
                            }
                        }
                        continue;
                    }
                    for &(_, target) in targets {
                        self.enqueue_target(target, &mut worklist);
                    }
                    self.enqueue_target(*otherwise, &mut worklist);
                }
                Terminator::Match {
                    arms, otherwise, ..
                } => {
                    for arm in arms {
                        self.enqueue_target(arm.target, &mut worklist);
                    }
                    self.enqueue_target(*otherwise, &mut worklist);
                }
                Terminator::Call { target, unwind, .. } => {
                    self.enqueue_target(*target, &mut worklist);
                    if let Some(cleanup) = unwind {
                        self.enqueue_target(*cleanup, &mut worklist);
                    }
                }
                Terminator::Yield { resume, drop, .. } | Terminator::Await { resume, drop, .. } => {
                    self.enqueue_target(*resume, &mut worklist);
                    self.enqueue_target(*drop, &mut worklist);
                }
                Terminator::Goto { target } => {
                    self.enqueue_target(*target, &mut worklist);
                }
                Terminator::Return
                | Terminator::Throw { .. }
                | Terminator::Panic
                | Terminator::Unreachable
                | Terminator::Pending(_) => {}
            }
        }
    }

    fn enqueue_target(&mut self, block: BlockId, worklist: &mut VecDeque<BlockId>) {
        if self.mark_reachable(block) {
            worklist.push_back(block);
        }
    }

    fn mark_reachable(&mut self, block: BlockId) -> bool {
        if let Some(flag) = self.reachable.get_mut(block.0) {
            if !*flag {
                *flag = true;
                return true;
            }
        }
        false
    }

    fn evaluate_switch(
        &mut self,
        span: Option<Span>,
        discr: &Operand,
        targets: &[(i128, BlockId)],
        otherwise: BlockId,
        env: &ConstEnv,
        globals: &ConstEnv,
    ) -> Option<(BlockId, Vec<BlockId>, Option<UnreachableCause>)> {
        if !matches!(discr, Operand::Const(_)) {
            return None;
        }
        let value = const_int_value(discr, env, globals)?;
        let is_bool_condition =
            targets.len() == 1 && targets[0].0 == 1 && (value == 0 || value == 1);
        let mut skipped = Vec::new();
        let mut taken = None;
        for (case, target) in targets {
            if *case == value {
                taken = Some(*target);
            } else {
                skipped.push(*target);
            }
        }
        let taken = taken.unwrap_or(otherwise);
        if otherwise != taken {
            skipped.push(otherwise);
        }
        let note = if is_bool_condition {
            UnreachableCause {
                span,
                label: Some("condition is constant".into()),
                note: if value == 0 {
                    "the condition is always false at compile time".into()
                } else {
                    "the condition is always true at compile time".into()
                },
            }
        } else {
            UnreachableCause {
                span,
                label: Some("condition is constant".into()),
                note: format!(
                    "condition is always true at compile time for `{value}`; condition is always false at compile time for all other values"
                ),
            }
        };
        Some((taken, skipped, Some(note)))
    }

    fn record_reason(&mut self, block: BlockId, reason: UnreachableCause) {
        if self.reachable.get(block.0).copied().unwrap_or(false) {
            return;
        }
        if let Some(slot) = self.reasons.get_mut(block.0) {
            if slot.is_none() {
                *slot = Some(reason);
            }
        }
    }

    fn terminator_exit_note(&self, block: BlockId) -> Option<(String, Option<Span>)> {
        if let Some(preds) = self.predecessors.get(block.0)
            && preds.is_empty()
            && block.0 > 0
        {
            if let Some(prev) = self.function.body.blocks.get(block.0 - 1)
                && matches!(
                    prev.terminator,
                    Some(
                        Terminator::Return
                            | Terminator::Throw { .. }
                            | Terminator::Panic
                            | Terminator::Unreachable
                    )
                )
            {
                let exit_span = prev
                    .statements
                    .iter()
                    .rev()
                    .find_map(|stmt| match stmt.kind {
                        StatementKind::Nop
                        | StatementKind::StorageLive(_)
                        | StatementKind::StorageDead(_)
                        | StatementKind::MarkFallibleHandled { .. } => None,
                        _ => stmt.span,
                    })
                    .or(prev.span);
                return Some((
                    "this code is unreachable because control flow always exits before it".into(),
                    exit_span,
                ));
            }
        }
        None
    }
}

fn block_primary_span(
    block: &crate::mir::BasicBlock,
    has_predecessors: bool,
) -> Option<(Span, bool)> {
    if let Some(span) = block.statements.iter().find_map(|stmt| match &stmt.kind {
        StatementKind::Nop => None,
        StatementKind::StorageLive(_)
        | StatementKind::StorageDead(_)
        | StatementKind::MarkFallibleHandled { .. } => None,
        _ => stmt.span,
    }) {
        return Some((span, true));
    }
    match &block.terminator {
        Some(
            Terminator::Return
            | Terminator::Throw { .. }
            | Terminator::Panic
            | Terminator::Unreachable
            | Terminator::Pending(_),
        ) if has_predecessors => block.span.map(|span| (span, false)),
        _ => None,
    }
}

fn all_successors(terminator: Option<&Terminator>) -> Vec<BlockId> {
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

fn const_int_value(operand: &Operand, env: &ConstEnv, globals: &ConstEnv) -> Option<i128> {
    let value = operand_const_value(operand, env, globals)?;
    const_value_to_int(&value)
}

type ConstEnv = std::collections::HashMap<LocalId, ConstValue>;

fn compute_function_consts(function: &MirFunction) -> ConstEnv {
    let mut consts = ConstEnv::new();
    let mut invalidated = std::collections::HashSet::new();
    for block in &function.body.blocks {
        invalidate_const_locals_for_terminator(&block.terminator, &mut consts, &mut invalidated);
        for stmt in &block.statements {
            match &stmt.kind {
                StatementKind::Assign { place, value } => {
                    if !place.projection.is_empty() {
                        invalidate_const_local(place.local, &mut consts, &mut invalidated);
                        continue;
                    }
                    let Some(const_val) = (match value {
                        Rvalue::Use(Operand::Const(c)) => Some(c.value.clone()),
                        _ => None,
                    }) else {
                        invalidate_const_local(place.local, &mut consts, &mut invalidated);
                        continue;
                    };
                    if invalidated.contains(&place.local) {
                        continue;
                    }
                    match consts.get(&place.local) {
                        None => {
                            consts.insert(place.local, const_val);
                        }
                        Some(_) => {
                            invalidate_const_local(place.local, &mut consts, &mut invalidated);
                        }
                    }
                }
                StatementKind::Borrow { kind, place, .. }
                    if matches!(kind, BorrowKind::Unique | BorrowKind::Raw) =>
                {
                    invalidate_const_local(place.local, &mut consts, &mut invalidated);
                }
                StatementKind::Deinit(place)
                | StatementKind::DeferDrop { place }
                | StatementKind::DefaultInit { place }
                | StatementKind::ZeroInit { place }
                | StatementKind::Drop { place, .. }
                | StatementKind::AtomicStore { target: place, .. } => {
                    invalidate_const_local(place.local, &mut consts, &mut invalidated);
                }
                StatementKind::EnqueueKernel { stream, .. } => {
                    invalidate_const_local(stream.local, &mut consts, &mut invalidated);
                }
                StatementKind::EnqueueCopy {
                    stream, dst, src, ..
                } => {
                    invalidate_const_local(stream.local, &mut consts, &mut invalidated);
                    invalidate_const_local(dst.local, &mut consts, &mut invalidated);
                    invalidate_const_local(src.local, &mut consts, &mut invalidated);
                }
                StatementKind::RecordEvent { stream, event } => {
                    invalidate_const_local(stream.local, &mut consts, &mut invalidated);
                    invalidate_const_local(event.local, &mut consts, &mut invalidated);
                }
                StatementKind::WaitEvent { event, stream } => {
                    invalidate_const_local(event.local, &mut consts, &mut invalidated);
                    if let Some(stream) = stream {
                        invalidate_const_local(stream.local, &mut consts, &mut invalidated);
                    }
                }
                _ => {}
            }
        }
    }
    consts
}

fn invalidate_const_locals_for_terminator(
    terminator: &Option<Terminator>,
    consts: &mut ConstEnv,
    invalidated: &mut std::collections::HashSet<LocalId>,
) {
    let Some(terminator) = terminator.as_ref() else {
        return;
    };
    match terminator {
        Terminator::Call {
            args,
            arg_modes,
            destination,
            ..
        } => {
            for (arg, mode) in args.iter().zip(arg_modes.iter()) {
                if !matches!(mode, ParamMode::Ref | ParamMode::Out) {
                    continue;
                }
                if let Some(local) = mutated_local_from_operand(arg) {
                    invalidate_const_local(local, consts, invalidated);
                }
            }
            if let Some(place) = destination {
                invalidate_const_local(place.local, consts, invalidated);
            }
        }
        Terminator::Await {
            future,
            destination,
            ..
        } => {
            invalidate_const_local(future.local, consts, invalidated);
            if let Some(place) = destination {
                invalidate_const_local(place.local, consts, invalidated);
            }
        }
        _ => {}
    }
}

fn mutated_local_from_operand(operand: &Operand) -> Option<LocalId> {
    match operand {
        Operand::Borrow(borrow) => Some(borrow.place.local),
        Operand::Copy(place) | Operand::Move(place) => Some(place.local),
        _ => None,
    }
}

fn invalidate_const_local(
    local: LocalId,
    consts: &mut ConstEnv,
    invalidated: &mut std::collections::HashSet<LocalId>,
) {
    consts.remove(&local);
    invalidated.insert(local);
}

fn block_const_env(block: &crate::mir::BasicBlock, globals: &ConstEnv) -> ConstEnv {
    let mut env = ConstEnv::new();
    for stmt in &block.statements {
        if let StatementKind::Assign { place, value } = &stmt.kind {
            if !place.projection.is_empty() {
                env.remove(&place.local);
                continue;
            }
            if let Some(value) = eval_rvalue_const(value, &env, globals) {
                env.insert(place.local, value);
            } else {
                env.remove(&place.local);
            }
        }
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::diagnostics::{FileId, Span};
    use crate::mir::data::{
        BasicBlock, BorrowOperand, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl,
        MirBody, Operand, Place, RegionVar, Rvalue, Statement,
    };
    use crate::mir::{Abi, MirFunction, MirModule, Terminator, Ty};

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

    fn block(id: usize, span: Option<Span>) -> BasicBlock {
        BasicBlock::new(BlockId(id), span)
    }

    fn stmt_assign_int(place: Place, value: i128, span: Option<Span>) -> Statement {
        Statement {
            span,
            kind: StatementKind::Assign {
                place,
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(value)))),
            },
        }
    }

    fn make_function(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> MirFunction {
        MirFunction {
            name: "Reachability::ConstInvalidation".into(),
            kind: FunctionKind::Function,
            signature: default_sig(),
            body: MirBody {
                arg_count: locals
                    .iter()
                    .filter(|local| matches!(local.kind, crate::mir::data::LocalKind::Arg(_)))
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
    fn out_argument_call_invalidates_local_constants() {
        let mut locals = Vec::new();
        locals.push(LocalDecl::new(
            None,
            Ty::Unit,
            false,
            None,
            crate::mir::data::LocalKind::Return,
        ));
        locals.push(LocalDecl::new(
            Some("flag".into()),
            Ty::named("int"),
            true,
            None,
            crate::mir::data::LocalKind::Local,
        ));

        let flag = LocalId(1);
        let span0 = Some(Span {
            file_id: FileId::UNKNOWN,
            start: 1,
            end: 2,
        });

        let mut entry = block(0, span0);
        entry
            .statements
            .push(stmt_assign_int(Place::new(flag), 0, span0));
        entry.terminator = Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Test::SetFlag".into(),
            ))),
            args: vec![Operand::Borrow(BorrowOperand {
                kind: BorrowKind::Unique,
                place: Place::new(flag),
                region: RegionVar(0),
                span: span0,
            })],
            arg_modes: vec![ParamMode::Out],
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        });

        let mut test_block = block(
            1,
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 3,
                end: 4,
            }),
        );
        test_block.terminator = Some(Terminator::SwitchInt {
            discr: Operand::Copy(Place::new(flag)),
            targets: vec![(1, BlockId(2))],
            otherwise: BlockId(3),
        });

        let mut true_block = block(
            2,
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 5,
                end: 6,
            }),
        );
        true_block.statements.push(stmt_assign_int(
            Place::new(LocalId(0)),
            0,
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 6,
                end: 7,
            }),
        ));
        true_block.terminator = Some(Terminator::Return);

        let mut false_block = block(
            3,
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 7,
                end: 8,
            }),
        );
        false_block.statements.push(stmt_assign_int(
            Place::new(LocalId(0)),
            0,
            Some(Span {
                file_id: FileId::UNKNOWN,
                start: 8,
                end: 9,
            }),
        ));
        false_block.terminator = Some(Terminator::Return);

        let function = make_function(locals, vec![entry, test_block, true_block, false_block]);
        let module = build_module(function);

        let diagnostics = check_unreachable_code(&module);
        assert!(
            diagnostics.is_empty(),
            "expected no unreachable diagnostics when out arg may mutate, got {diagnostics:#?}"
        );
    }
}

fn eval_rvalue_const(value: &Rvalue, env: &ConstEnv, globals: &ConstEnv) -> Option<ConstValue> {
    match value {
        Rvalue::Use(operand) => operand_const_value(operand, env, globals),
        Rvalue::Unary { op, operand, .. } => {
            let value = operand_const_value(operand, env, globals)?;
            eval_unop(*op, &value)
        }
        Rvalue::Binary { op, lhs, rhs, .. } => {
            let lhs_val = operand_const_value(lhs, env, globals)?;
            let rhs_val = operand_const_value(rhs, env, globals)?;
            eval_binop(*op, &lhs_val, &rhs_val)
        }
        _ => None,
    }
}

fn operand_const_value(
    operand: &Operand,
    env: &ConstEnv,
    globals: &ConstEnv,
) -> Option<ConstValue> {
    match operand {
        Operand::Const(constant) => Some(constant.value.clone()),
        Operand::Copy(place) | Operand::Move(place) => env
            .get(&place.local)
            .cloned()
            .or_else(|| globals.get(&place.local).cloned()),
        Operand::Borrow(_) | Operand::Mmio(_) | Operand::Pending(_) => None,
    }
}

fn const_value_to_int(value: &ConstValue) -> Option<i128> {
    match value {
        ConstValue::Bool(true) => Some(1),
        ConstValue::Bool(false) => Some(0),
        ConstValue::Int(v) | ConstValue::Int32(v) => Some(*v),
        ConstValue::UInt(v) => i128::try_from(*v).ok(),
        ConstValue::Enum { discriminant, .. } => Some(*discriminant),
        _ => None,
    }
}

fn const_value_to_bool(value: &ConstValue) -> Option<bool> {
    match value {
        ConstValue::Bool(v) => Some(*v),
        _ => None,
    }
}

fn eval_unop(op: UnOp, value: &ConstValue) -> Option<ConstValue> {
    match op {
        UnOp::Not => const_value_to_bool(value).map(|v| ConstValue::Bool(!v)),
        UnOp::BitNot => const_value_to_int(value).map(|v| ConstValue::Int(!v)),
        UnOp::Neg => const_value_to_int(value).map(|v| ConstValue::Int(-v)),
        _ => None,
    }
}

fn eval_binop(op: BinOp, lhs: &ConstValue, rhs: &ConstValue) -> Option<ConstValue> {
    match op {
        BinOp::Eq => match (const_value_to_int(lhs), const_value_to_int(rhs)) {
            (Some(a), Some(b)) => Some(ConstValue::Bool(a == b)),
            _ => const_value_to_bool(lhs)
                .zip(const_value_to_bool(rhs))
                .map(|(a, b)| ConstValue::Bool(a == b)),
        },
        BinOp::Ne => match (const_value_to_int(lhs), const_value_to_int(rhs)) {
            (Some(a), Some(b)) => Some(ConstValue::Bool(a != b)),
            _ => const_value_to_bool(lhs)
                .zip(const_value_to_bool(rhs))
                .map(|(a, b)| ConstValue::Bool(a != b)),
        },
        BinOp::Lt => const_value_to_int(lhs)
            .zip(const_value_to_int(rhs))
            .map(|(a, b)| ConstValue::Bool(a < b)),
        BinOp::Le => const_value_to_int(lhs)
            .zip(const_value_to_int(rhs))
            .map(|(a, b)| ConstValue::Bool(a <= b)),
        BinOp::Gt => const_value_to_int(lhs)
            .zip(const_value_to_int(rhs))
            .map(|(a, b)| ConstValue::Bool(a > b)),
        BinOp::Ge => const_value_to_int(lhs)
            .zip(const_value_to_int(rhs))
            .map(|(a, b)| ConstValue::Bool(a >= b)),
        BinOp::And => const_value_to_bool(lhs)
            .zip(const_value_to_bool(rhs))
            .map(|(a, b)| ConstValue::Bool(a && b)),
        BinOp::Or => const_value_to_bool(lhs)
            .zip(const_value_to_bool(rhs))
            .map(|(a, b)| ConstValue::Bool(a || b)),
        _ => None,
    }
}
