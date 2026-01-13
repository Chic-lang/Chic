use super::*;

body_builder_impl! {
    #[allow(clippy::too_many_lines)]
    // Recursive pattern collection handles all Chic pattern forms, and lowering must cover every variant in one pass.
    pub(super) fn lower_switch_case(
        &mut self,
        case: &SwitchCase,
        discr_local: LocalId,
        next_block: BlockId,
    ) {
        match &case.pattern {
            CasePatternKind::Wildcard => self.lower_wildcard_case(case, next_block),
            CasePatternKind::Literal(value) => {
                self.lower_literal_case(case, discr_local, value, next_block);
            }
            CasePatternKind::Complex(_) => self.lower_complex_case(case, next_block),
        }
    }

    fn lower_wildcard_case(&mut self, case: &SwitchCase, next_block: BlockId) {
        let post_guard_entry =
            self.lower_guard_chain(&case.guards, case.body_block, next_block, case.span);
        let entry =
            self.lower_guard_chain(&case.pre_guards, post_guard_entry, next_block, case.span);
        self.set_terminator(
            case.span,
            Terminator::Goto {
                target: entry,
            },
        );
    }

    fn lower_literal_case(
        &mut self,
        case: &SwitchCase,
        discr_local: LocalId,
        value: &ConstValue,
        next_block: BlockId,
    ) {
        let discr_operand = Operand::Copy(Place::new(discr_local));
        let temp = self.create_temp(case.pattern_span.or(case.span));
        if let Some(decl) = self.locals.get_mut(temp.0) {
            decl.ty = Ty::named("bool");
            decl.is_nullable = false;
        }
        let assign = MirStatement {
            span: case.pattern_span.or(case.span),
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Binary {
                    op: BinOp::Eq,
                    lhs: discr_operand,
                    rhs: Operand::Const(ConstOperand::new(value.clone())),
                    rounding: None,
                },
            },
        };
        self.push_statement(assign);
        let eq_operand = Operand::Copy(Place::new(temp));

        let guard_entry =
            self.lower_guard_chain(&case.guards, case.body_block, next_block, case.span);
        let pre_entry =
            self.lower_guard_chain(&case.pre_guards, guard_entry, next_block, case.span);
        self.set_terminator(
            case.pattern_span.or(case.span),
            Terminator::SwitchInt {
                discr: eq_operand,
                targets: vec![(1, pre_entry)],
                otherwise: next_block,
            },
        );
    }

    fn lower_complex_case(&mut self, case: &SwitchCase, next_block: BlockId) {
        self.set_terminator(case.span, Terminator::Goto { target: next_block });
    }

}
