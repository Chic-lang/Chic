use super::*;

body_builder_impl! {
    pub(super) fn lower_switch_as_int(
        &mut self,
        discr_local: LocalId,
        cases: &[SwitchCase],
        fallback_block: BlockId,
        switch_span: Option<Span>,
    ) {
        let mut check_blocks = Vec::with_capacity(cases.len());
        for case in cases {
            let block = self.new_block(case.span);
            check_blocks.push(block);
        }

        if let Some(first) = check_blocks.first().copied() {
            self.ensure_goto(first, switch_span);
        }

        for (index, case) in cases.iter().enumerate() {
            let check_block = check_blocks[index];
            let next_block = if index + 1 < check_blocks.len() {
                check_blocks[index + 1]
            } else {
                fallback_block
            };
            self.switch_to_block(check_block);
            self.lower_switch_case(case, discr_local, next_block);
        }

        self.switch_to_block(fallback_block);
    }
}
