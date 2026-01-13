use super::super::*;

#[derive(Clone, Copy)]
pub(crate) struct LoopContext {
    pub break_target: BlockId,
    pub continue_target: BlockId,
    pub scope_depth: usize,
}

body_builder_impl! {
    pub(in crate::mir::builder::body_builder) fn push_switch_context(
        &mut self,
        join_block: BlockId,
        binding: String,
        scope_depth: usize,
    ) {
        self.switch_stack
            .push(SwitchContext::new(join_block, binding, scope_depth));
    }

    pub(in crate::mir::builder::body_builder) fn pop_switch_context(&mut self) -> Option<SwitchContext> {
        self.switch_stack.pop()
    }

    pub(in crate::mir::builder::body_builder) fn current_switch_context(&self) -> Option<&SwitchContext> {
        self.switch_stack.last()
    }

    pub(in crate::mir::builder::body_builder) fn current_switch_context_mut(&mut self) -> Option<&mut SwitchContext> {
        self.switch_stack.last_mut()
    }

    pub(in crate::mir::builder::body_builder) fn current_switch_break_target(&self) -> Option<(BlockId, usize)> {
        self.switch_stack
            .last()
            .map(|ctx| (ctx.join_block, ctx.scope_depth))
    }

    pub(in crate::mir::builder::body_builder) fn current_switch_default_target(&self) -> Option<SwitchTarget> {
        self.switch_stack.last().and_then(|ctx| ctx.default_target)
    }
}
