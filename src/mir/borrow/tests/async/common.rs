use crate::frontend::diagnostics::Span;
use crate::mir::AsyncFramePolicy;
use crate::mir::data::{BlockId, LocalId};
use crate::mir::layout::{AutoTraitOverride, AutoTraitSet, StructLayout, TypeLayout, TypeRepr};
use crate::mir::state::{AsyncStateMachine, AsyncSuspendPoint};

use super::super::util::BorrowTestHarness;

pub(super) fn async_harness(name: &str) -> BorrowTestHarness {
    BorrowTestHarness::new(name).mark_async()
}

pub(super) fn single_suspend_point(
    future: LocalId,
    destination: Option<LocalId>,
    await_block: BlockId,
    resume_block: BlockId,
    drop_block: BlockId,
    pinned: Vec<LocalId>,
    span: Option<Span>,
) -> AsyncStateMachine {
    AsyncStateMachine {
        suspend_points: vec![AsyncSuspendPoint {
            id: 0,
            await_block,
            resume_block,
            drop_block,
            future,
            destination,
            span,
        }],
        pinned_locals: pinned,
        cross_locals: Vec::new(),
        frame_fields: Vec::new(),
        result_local: None,
        result_ty: None,
        context_local: None,
        policy: AsyncFramePolicy::default(),
    }
}

pub(super) fn register_layout(
    harness: &mut BorrowTestHarness,
    name: &str,
    auto_traits: AutoTraitSet,
    overrides: AutoTraitOverride,
    size: Option<usize>,
    align: Option<usize>,
) {
    harness.layouts_mut().types.insert(
        name.into(),
        TypeLayout::Struct(StructLayout {
            name: name.into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size,
            align,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits,
            overrides,
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}
