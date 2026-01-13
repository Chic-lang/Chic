use super::super::*;

body_builder_impl! {
    pub(crate) fn push_statement(&mut self, statement: MirStatement) {
        if let MirStatementKind::Assign { place, .. } = &statement.kind {
            if !self.validate_assignment_target(place, statement.span) {
                return;
            }
        }
        let block = self.ensure_active_block();
        self.blocks[block.0].statements.push(statement);
    }

    pub(crate) fn mark_fallible_handled(
        &mut self,
        local: LocalId,
        span: Option<Span>,
    ) {
        if self.locals.get(local.0).is_none() {
            return;
        }
        let stmt = MirStatement {
            span,
            kind: MirStatementKind::MarkFallibleHandled { local },
        };
        self.push_statement(stmt);
    }

    pub(crate) fn set_terminator(&mut self, span: Option<Span>, terminator: Terminator) {
        let block = self.ensure_active_block();
        self.blocks[block.0].terminator = Some(terminator);
        if self.blocks[block.0].span.is_none() {
            self.blocks[block.0].span = span;
        }
    }

    pub(crate) fn current_unwind_target(&self) -> Option<BlockId> {
        self.try_stack
            .last()
            .and_then(|ctx| ctx.unwind_capture)
    }

    pub(crate) fn ensure_active_block(&mut self) -> BlockId {
        if self.blocks[self.current_block.0].terminator.is_none() {
            return self.current_block;
        }
        let next_id = BlockId(self.blocks.len());
        let span = self.body.span;
        let block = BasicBlock::new(next_id, span);
        self.blocks.push(block);
        self.current_block = next_id;
        self.current_block
    }

    pub(crate) fn new_block(
        &mut self,
        span: Option<Span>,
    ) -> BlockId {
        let id = BlockId(self.blocks.len());
        let block = BasicBlock::new(id, span);
        self.blocks.push(block);
        id
    }

    pub(crate) fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    pub(crate) fn ensure_goto(
        &mut self,
        target: BlockId,
        span: Option<Span>,
    ) {
        let current = self.current_block;
        if self.blocks[current.0].terminator.is_none() {
            self.blocks[current.0].terminator = Some(Terminator::Goto { target });
            if self.blocks[current.0].span.is_none() {
                self.blocks[current.0].span = span;
            }
        }
    }

    pub(crate) fn push_pending(
        &mut self,
        statement: &AstStatement,
        kind: PendingStatementKind,
    ) {
        let detail = statement.span.map(|_| format!("{kind:?} pending lowering"));
        let pending = PendingStatement { kind, detail };
        let stmt = MirStatement {
            span: statement.span,
            kind: MirStatementKind::Pending(pending),
        };
        self.push_statement(stmt);
    }
}
