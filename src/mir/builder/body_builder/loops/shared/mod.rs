use super::*;

mod binding;

pub(crate) use binding::parse_foreach_binding;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForeachBindingMode {
    Value,
    In,
    Ref,
    RefReadonly,
}

#[derive(Clone, Debug)]
pub(crate) struct ForeachBindingInfo {
    pub(crate) mode: ForeachBindingMode,
    pub(crate) mutable: bool,
    pub(crate) ty: Ty,
    pub(crate) name: String,
}

#[derive(Clone, Copy)]
pub(crate) struct LoopBlockPlan {
    pub(crate) condition: BlockId,
    pub(crate) body: BlockId,
    pub(crate) exit: BlockId,
    pub(crate) iterator: Option<BlockId>,
}

impl LoopBlockPlan {
    pub(crate) fn new(
        builder: &mut BodyBuilder<'_>,
        condition_span: Option<Span>,
        body_span: Option<Span>,
        exit_span: Option<Span>,
        iterator_span: Option<Span>,
    ) -> Self {
        let condition = builder.new_block(condition_span);
        let body = builder.new_block(body_span);
        let exit = builder.new_block(exit_span);
        let iterator = if let Some(span) = iterator_span {
            Some(builder.new_block(Some(span)))
        } else {
            None
        };

        Self {
            condition,
            body,
            exit,
            iterator,
        }
    }
}

pub(crate) struct ForeachLocals {
    pub(crate) iter_local: LocalId,
    pub(crate) sequence_local: LocalId,
    pub(crate) enumerator_local: Option<LocalId>,
    pub(crate) sequence_name: String,
    pub(crate) enumerator_name: String,
}

pub(crate) struct ForeachBlocks {
    pub(crate) condition: BlockId,
    pub(crate) prepare: BlockId,
    pub(crate) body: BlockId,
    pub(crate) cleanup: BlockId,
    pub(crate) break_cleanup: BlockId,
    pub(crate) exit: BlockId,
}

pub(crate) fn plan_foreach_blocks(
    builder: &mut BodyBuilder<'_>,
    condition_span: Option<Span>,
    binding_span: Option<Span>,
    body_span: Option<Span>,
    exit_span: Option<Span>,
) -> ForeachBlocks {
    let condition = builder.new_block(condition_span);
    let prepare = builder.new_block(binding_span);
    let body = builder.new_block(body_span);
    let cleanup = builder.new_block(binding_span);
    let break_cleanup = builder.new_block(binding_span);
    let exit = builder.new_block(exit_span);

    ForeachBlocks {
        condition,
        prepare,
        body,
        cleanup,
        break_cleanup,
        exit,
    }
}

pub(crate) fn initialise_foreach_locals(
    builder: &mut BodyBuilder<'_>,
    binding: &ForeachBindingInfo,
    binding_span: Option<Span>,
    expression_span: Option<Span>,
) -> ForeachLocals {
    let foreach_id = builder.next_foreach_id;
    builder.next_foreach_id += 1;

    let sequence_name = format!("__foreach_seq_{foreach_id}");
    let enumerator_name = format!("__foreach_enum_{foreach_id}");

    let iter_local_decl = LocalDecl::new(
        Some(binding.name.clone()),
        binding.ty.clone(),
        binding.mutable,
        binding_span,
        LocalKind::Local,
    );
    let iter_local = builder.push_local(iter_local_decl);
    builder.bind_name(&binding.name, iter_local);

    let sequence_local = builder.push_local(LocalDecl::new(
        Some(sequence_name.clone()),
        Ty::Unknown,
        false,
        expression_span,
        LocalKind::Local,
    ));
    builder.bind_name(&sequence_name, sequence_local);
    storage_live_local(builder, sequence_local, expression_span);

    ForeachLocals {
        iter_local,
        sequence_local,
        enumerator_local: None,
        sequence_name,
        enumerator_name,
    }
}

pub(crate) fn ensure_enumerator_local(
    builder: &mut BodyBuilder<'_>,
    locals: &mut ForeachLocals,
    span: Option<Span>,
) -> LocalId {
    if let Some(local) = locals.enumerator_local {
        return local;
    }

    let enumerator_local = builder.push_local(LocalDecl::new(
        Some(locals.enumerator_name.clone()),
        Ty::Unknown,
        true,
        span,
        LocalKind::Local,
    ));
    builder.bind_name(&locals.enumerator_name, enumerator_local);
    storage_live_local(builder, enumerator_local, span);
    locals.enumerator_local = Some(enumerator_local);
    enumerator_local
}

pub(crate) fn storage_live_local(
    builder: &mut BodyBuilder<'_>,
    local: LocalId,
    span: Option<Span>,
) {
    builder.push_statement(MirStatement {
        span,
        kind: MirStatementKind::StorageLive(local),
    });
    builder.record_local(local, span);
}

pub(crate) fn storage_dead_local(
    builder: &mut BodyBuilder<'_>,
    local: LocalId,
    span: Option<Span>,
) {
    builder.push_statement(MirStatement {
        span,
        kind: MirStatementKind::StorageDead(local),
    });
}

pub(crate) fn defer_drop_for_place(
    builder: &mut BodyBuilder<'_>,
    place: Place,
    span: Option<Span>,
) {
    builder.push_statement(MirStatement {
        span,
        kind: MirStatementKind::DeferDrop { place },
    });
}
