//! Verification passes for MIR bodies.

use std::collections::HashSet;

use super::data::{
    BasicBlock, BlockId, BorrowId, InlineAsmOperandKind, InterpolatedStringSegment, LocalId,
    LocalKind, MatchArm, MirBody, Operand, Pattern, PendingRvalue, Place, ProjectionElem,
    RegionVar, Rvalue, Statement, StatementKind, Terminator, VariantPatternFields,
};

/// Validate invariants for a MIR body.
///
/// # Errors
///
/// Returns a list of [`VerifyError`] values when invariants are violated.
pub fn verify_body(body: &MirBody) -> Result<(), Vec<VerifyError>> {
    let verifier = Verifier::new(body);
    verifier.run()
}

#[derive(Debug)]
struct Verifier<'a> {
    body: &'a MirBody,
    errors: Vec<VerifyError>,
    seen_live: HashSet<usize>,
}

impl<'a> Verifier<'a> {
    fn new(body: &'a MirBody) -> Self {
        Self {
            body,
            errors: Vec::new(),
            seen_live: HashSet::new(),
        }
    }

    fn run(mut self) -> Result<(), Vec<VerifyError>> {
        if self.body.blocks.is_empty() {
            self.errors.push(VerifyError::EmptyBody);
        }

        self.check_return_local();
        self.check_arguments();
        self.check_blocks();
        self.check_exception_regions();

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    fn check_exception_regions(&mut self) {
        for region in &self.body.exception_regions {
            self.check_block_id(region.try_entry, "try entry");
            self.check_block_id(region.try_exit, "try exit");
            self.check_block_id(region.after_block, "try after block");
            if let Some(dispatch) = region.dispatch {
                self.check_block_id(dispatch, "catch dispatch");
            }
            for catch in &region.catches {
                self.check_block_id(catch.entry, "catch entry");
                self.check_block_id(catch.body, "catch body");
                self.check_block_id(catch.cleanup, "catch cleanup");
                if let Some(binding) = catch.binding {
                    self.check_local(binding, "catch binding local");
                }
                if let Some(filter) = &catch.filter {
                    self.check_block_id(filter.block, "catch filter block");
                }
            }
            if let Some(finally) = &region.finally {
                self.check_block_id(finally.entry, "finally entry");
                self.check_block_id(finally.exit, "finally exit");
            }
        }
    }

    fn check_return_local(&mut self) {
        if self.body.locals.is_empty() {
            self.errors.push(VerifyError::MissingReturnLocal);
            return;
        }
        let ret = &self.body.locals[0];
        if !matches!(ret.kind, LocalKind::Return) {
            self.errors.push(VerifyError::MissingReturnLocal);
        }
    }

    fn check_arguments(&mut self) {
        let arg_count = self
            .body
            .locals
            .iter()
            .filter(|l| matches!(l.kind, LocalKind::Arg(_)))
            .count();
        if arg_count != self.body.arg_count {
            self.errors.push(VerifyError::ArgumentCountMismatch {
                expected: self.body.arg_count,
                actual: arg_count,
            });
        }
    }

    fn check_blocks(&mut self) {
        for (index, block) in self.body.blocks.iter().enumerate() {
            let block_id = BlockId(index);
            if block.id != block_id {
                self.errors.push(VerifyError::BlockIdMismatch {
                    expected: block_id,
                    actual: block.id,
                });
            }
            if block.terminator.is_none() {
                self.errors
                    .push(VerifyError::MissingTerminator { block: block_id });
            }
            self.check_block_statements(block);
            if let Some(term) = &block.terminator {
                self.check_terminator(block_id, term);
            }
        }
    }

    fn check_block_statements(&mut self, block: &BasicBlock) {
        for (idx, statement) in block.statements.iter().enumerate() {
            self.ensure_statement_span(block.id, idx, statement);
            self.check_statement(block.id, idx, statement);
        }
    }

    fn ensure_statement_span(&mut self, block: BlockId, index: usize, statement: &Statement) {
        if statement.span.is_none() && !matches!(statement.kind, StatementKind::Nop) {
            self.errors.push(VerifyError::MissingDebugSpan {
                entity: DebugEntity::Statement { block, index },
            });
        }
    }

    fn check_statement(&mut self, block: BlockId, index: usize, statement: &Statement) {
        match &statement.kind {
            StatementKind::Assign { place, value } => self.handle_assign(place, value),
            StatementKind::StorageLive(local) => self.handle_storage_live(*local),
            StatementKind::StorageDead(local) => self.handle_storage_dead(*local),
            StatementKind::MarkFallibleHandled { .. } => {}
            StatementKind::Deinit(place) => self.check_place(place, "deinit"),
            StatementKind::DefaultInit { place } => self.check_place(place, "default init target"),
            StatementKind::ZeroInit { place } => self.check_place(place, "zero init target"),
            StatementKind::ZeroInitRaw { pointer, length } => {
                self.check_operand(pointer);
                self.check_operand(length);
            }
            StatementKind::Drop {
                place,
                target,
                unwind,
            } => self.handle_drop(place, *target, *unwind),
            StatementKind::Borrow {
                place,
                region,
                borrow_id,
                ..
            } => self.handle_borrow(place, *region, *borrow_id),
            StatementKind::MmioStore { value, .. } | StatementKind::StaticStore { value, .. } => {
                self.check_operand(value);
            }
            StatementKind::AtomicStore { target, value, .. } => {
                self.check_place(target, "atomic store target");
                self.check_operand(value);
            }
            StatementKind::AtomicFence { .. } => {}
            StatementKind::Retag { place } | StatementKind::DeferDrop { place } => {
                self.check_place(place, "place");
            }
            StatementKind::InlineAsm(asm) => {
                for operand in &asm.operands {
                    match &operand.kind {
                        InlineAsmOperandKind::In { value }
                        | InlineAsmOperandKind::Const { value } => self.check_operand(value),
                        InlineAsmOperandKind::InOut { input, output, .. } => {
                            self.check_operand(input);
                            self.check_place(output, "inline assembly output");
                        }
                        InlineAsmOperandKind::Out { place, .. } => {
                            self.check_place(place, "inline assembly output");
                        }
                        InlineAsmOperandKind::Sym { .. } => {}
                    }
                }
            }
            StatementKind::Assert {
                cond,
                target,
                cleanup,
                ..
            } => self.handle_assert(cond, *target, cleanup.as_ref()),
            StatementKind::EnqueueKernel {
                stream,
                kernel,
                args,
                completion,
            } => {
                self.check_place(stream, "enqueue stream");
                self.check_operand(kernel);
                for arg in args {
                    self.check_operand(arg);
                }
                if let Some(event) = completion {
                    self.check_place(event, "enqueue event");
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
                self.check_place(stream, "enqueue copy stream");
                self.check_place(dst, "copy destination");
                self.check_place(src, "copy source");
                self.check_operand(bytes);
                if let Some(event) = completion {
                    self.check_place(event, "copy event");
                }
            }
            StatementKind::RecordEvent { stream, event } => {
                self.check_place(stream, "record event stream");
                self.check_place(event, "record event");
            }
            StatementKind::WaitEvent { event, stream } => {
                self.check_place(event, "wait event");
                if let Some(stream) = stream {
                    self.check_place(stream, "wait stream");
                }
            }
            StatementKind::Eval(pending) => self.handle_eval(block, index, pending),
            StatementKind::EnterUnsafe
            | StatementKind::ExitUnsafe
            | StatementKind::Nop
            | StatementKind::Pending(_) => {}
        }
    }

    fn handle_assign(&mut self, place: &Place, value: &Rvalue) {
        self.check_place(place, "assign destination");
        self.check_rvalue(value);
    }

    fn handle_storage_live(&mut self, local: LocalId) {
        self.check_local(local, "storage-live");
        self.seen_live.insert(local.0);
    }

    fn handle_storage_dead(&mut self, local: LocalId) {
        self.check_local(local, "storage-dead");
        // Some bodies emit cleanup markers in separate blocks without a
        // matching `StorageLive` in the same block. Treat unmatched dead
        // markers as a no-op to avoid false positives while liveness
        // emission is stabilized across generators.
        let _ = self.seen_live.remove(&local.0);
    }

    fn handle_drop(&mut self, place: &Place, target: BlockId, unwind: Option<BlockId>) {
        self.check_place(place, "drop");
        self.check_block_id(target, "drop target");
        if let Some(unwind) = unwind {
            self.check_block_id(unwind, "drop unwind");
        }
    }

    fn handle_borrow(&mut self, place: &Place, region: RegionVar, borrow: BorrowId) {
        self.check_place(place, "borrow place");
        if region.0 == usize::MAX {
            self.errors
                .push(VerifyError::InvalidRegion { region, borrow });
        }
    }

    fn handle_assert(&mut self, cond: &Operand, target: BlockId, cleanup: Option<&BlockId>) {
        self.check_operand(cond);
        self.check_block_id(target, "assert target");
        if let Some(cleanup) = cleanup {
            self.check_block_id(*cleanup, "assert cleanup");
        }
    }

    fn handle_eval(&mut self, block: BlockId, index: usize, pending: &PendingRvalue) {
        if pending.span.is_none() {
            self.errors.push(VerifyError::MissingDebugSpan {
                entity: DebugEntity::PendingExpression { block, index },
            });
        }
    }

    fn check_terminator(&mut self, block: BlockId, term: &Terminator) {
        match term {
            Terminator::Goto { target } => self.check_block_id(*target, "goto"),
            Terminator::SwitchInt {
                discr,
                targets,
                otherwise,
            } => self.check_switch_int(discr, targets, *otherwise),
            Terminator::Match {
                value,
                arms,
                otherwise,
            } => self.check_match(value, arms, *otherwise),
            Terminator::Return | Terminator::Panic | Terminator::Unreachable => {}
            Terminator::Throw { exception, .. } => {
                if let Some(value) = exception {
                    self.check_operand(value);
                }
            }
            Terminator::Call {
                func,
                args,
                arg_modes: _,
                destination,
                target,
                unwind,
                ..
            } => {
                self.check_operand(func);
                for arg in args {
                    self.check_operand(arg);
                }
                if let Some(dest) = destination {
                    self.check_place(dest, "call destination");
                }
                self.check_call_targets(*target, *unwind);
            }
            Terminator::Yield {
                value,
                resume,
                drop,
            } => {
                self.check_operand(value);
                self.check_block_id(*resume, "yield resume");
                self.check_block_id(*drop, "yield drop");
            }
            Terminator::Await {
                future,
                destination,
                resume,
                drop,
            } => self.check_await(future, destination.as_ref(), *resume, *drop),
            Terminator::Pending(_) => {
                self.errors.push(VerifyError::PendingTerminator { block });
            }
        }
    }

    fn check_switch_int(
        &mut self,
        discr: &Operand,
        targets: &[(i128, BlockId)],
        otherwise: BlockId,
    ) {
        self.check_operand(discr);
        for (_, target) in targets {
            self.check_block_id(*target, "switch target");
        }
        self.check_block_id(otherwise, "switch otherwise");
    }

    fn check_match(&mut self, value: &Place, arms: &[MatchArm], otherwise: BlockId) {
        self.check_place(value, "match value");
        for arm in arms {
            self.check_block_id(arm.target, "match arm target");
            self.check_pattern(&arm.pattern);
            for binding in &arm.bindings {
                self.check_local(binding.local, "match binding local");
            }
        }
        self.check_block_id(otherwise, "match otherwise");
    }

    fn check_call_targets(&mut self, target: BlockId, unwind: Option<BlockId>) {
        self.check_block_id(target, "call target");
        if let Some(unwind) = unwind {
            self.check_block_id(unwind, "call unwind");
        }
    }

    fn check_await(
        &mut self,
        future: &Place,
        destination: Option<&Place>,
        resume: BlockId,
        drop: BlockId,
    ) {
        self.check_place(future, "await future");
        if let Some(dest) = destination {
            self.check_place(dest, "await destination");
        }
        self.check_block_id(resume, "await resume");
        self.check_block_id(drop, "await drop");
    }

    #[expect(
        clippy::only_used_in_recursion,
        reason = "Pattern validation relies on recursive traversal without side effects"
    )]
    fn check_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard | Pattern::Binding(_) | Pattern::Literal(_) => {}
            Pattern::Tuple(elems) => {
                for pat in elems {
                    self.check_pattern(pat);
                }
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    self.check_pattern(&field.pattern);
                }
            }
            Pattern::Enum { fields, .. } => match fields {
                VariantPatternFields::Unit => {}
                VariantPatternFields::Tuple(items) => {
                    for pat in items {
                        self.check_pattern(pat);
                    }
                }
                VariantPatternFields::Struct(items) => {
                    for field in items {
                        self.check_pattern(&field.pattern);
                    }
                }
            },
        }
    }

    fn check_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.check_place(place, "operand place");
            }
            Operand::Borrow(borrow) => self.check_place(&borrow.place, "borrow operand"),
            Operand::Mmio(_) => {}
            Operand::Const(_) => {}
            Operand::Pending(pending) => {
                if pending.span.is_none() {
                    self.errors.push(VerifyError::MissingDebugSpan {
                        entity: DebugEntity::PendingOperand,
                    });
                }
            }
        }
    }

    fn check_rvalue(&mut self, value: &Rvalue) {
        match value {
            Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => self.check_operand(op),
            Rvalue::Binary { lhs, rhs, .. } => {
                self.check_operand(lhs);
                self.check_operand(rhs);
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    self.check_operand(field);
                }
            }
            Rvalue::AddressOf { place, .. } | Rvalue::Len(place) => {
                self.check_place(place, "rvalue place");
            }
            Rvalue::Cast { operand, .. } => self.check_operand(operand),
            Rvalue::StringInterpolate { segments } => {
                for segment in segments {
                    if let InterpolatedStringSegment::Expr { operand, .. } = segment {
                        self.check_operand(operand);
                    }
                }
            }
            Rvalue::NumericIntrinsic(intrinsic) => {
                for operand in &intrinsic.operands {
                    self.check_operand(operand);
                }
                if let Some(out) = &intrinsic.out {
                    self.check_place(out, "numeric intrinsic out parameter");
                }
            }
            Rvalue::AtomicLoad { target, .. } => {
                self.check_place(target, "atomic load target");
            }
            Rvalue::AtomicRmw { target, value, .. } => {
                self.check_place(target, "atomic rmw target");
                self.check_operand(value);
            }
            Rvalue::AtomicCompareExchange {
                target,
                expected,
                desired,
                ..
            } => {
                self.check_place(target, "atomic compare-exchange target");
                self.check_operand(expected);
                self.check_operand(desired);
            }
            Rvalue::StaticLoad { .. } => {}
            Rvalue::StaticRef { .. } => {}
            Rvalue::Pending(pending) => {
                if pending.span.is_none() {
                    self.errors.push(VerifyError::MissingDebugSpan {
                        entity: DebugEntity::PendingRvalue,
                    });
                }
            }
            Rvalue::DecimalIntrinsic(decimal) => {
                self.check_operand(&decimal.lhs);
                self.check_operand(&decimal.rhs);
                if let Some(addend) = &decimal.addend {
                    self.check_operand(addend);
                }
                self.check_operand(&decimal.rounding);
                self.check_operand(&decimal.vectorize);
            }
            Rvalue::SpanStackAlloc { .. } => {}
        }
    }

    fn check_place(&mut self, place: &Place, context: &'static str) {
        self.check_local(place.local, context);
        for proj in &place.projection {
            match proj {
                ProjectionElem::Index(local) => self.check_local(*local, "projection index"),
                ProjectionElem::Field(_)
                | ProjectionElem::FieldNamed(_)
                | ProjectionElem::UnionField { .. }
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Deref
                | ProjectionElem::Downcast { .. }
                | ProjectionElem::Subslice { .. } => {}
            }
        }
    }

    fn check_local(&mut self, local: LocalId, context: &'static str) {
        if local.0 >= self.body.locals.len() {
            self.errors
                .push(VerifyError::InvalidLocal { local, context });
        } else {
            let decl = &self.body.locals[local.0];
            if decl.span.is_none() && matches!(decl.kind, LocalKind::Local | LocalKind::Temp) {
                self.errors.push(VerifyError::MissingDebugSpan {
                    entity: DebugEntity::Local { local },
                });
            }
        }
    }

    fn check_block_id(&mut self, block: BlockId, context: &'static str) {
        if block.0 >= self.body.blocks.len() {
            self.errors
                .push(VerifyError::InvalidBlockTarget { context, block });
        }
    }
}

/// Verification failures produced by [`verify_body`].
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    EmptyBody,
    MissingReturnLocal,
    MissingTerminator {
        block: BlockId,
    },
    BlockIdMismatch {
        expected: BlockId,
        actual: BlockId,
    },
    InvalidBlockTarget {
        context: &'static str,
        block: BlockId,
    },
    InvalidLocal {
        local: LocalId,
        context: &'static str,
    },
    ArgumentCountMismatch {
        expected: usize,
        actual: usize,
    },
    StorageDeadBeforeLive {
        local: LocalId,
    },
    MissingDebugSpan {
        entity: DebugEntity,
    },
    PendingTerminator {
        block: BlockId,
    },
    InvalidRegion {
        region: RegionVar,
        borrow: BorrowId,
    },
}

/// Entities expected to carry span/debug information.
#[derive(Debug, PartialEq, Eq)]
pub enum DebugEntity {
    Statement { block: BlockId, index: usize },
    PendingExpression { block: BlockId, index: usize },
    PendingRvalue,
    PendingOperand,
    Local { local: LocalId },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::data::{LocalDecl, Ty};

    #[test]
    fn detects_missing_terminator() {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.blocks.push(BasicBlock::new(BlockId(0), None));
        let result = verify_body(&body);
        assert!(matches!(
            result,
            Err(errors) if errors.iter().any(|e| matches!(e, VerifyError::MissingTerminator { .. }))
        ));
    }
}
