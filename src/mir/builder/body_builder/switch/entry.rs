use super::*;
use std::collections::HashMap;

body_builder_impl! {
    pub(crate) fn lower_switch_statement(
        &mut self,
        statement: &AstStatement,
        switch: &SwitchStatement,
    ) {
        self.push_scope();
        self.lower_switch_statement_inner(statement, switch);
        self.pop_scope();
    }

    fn lower_switch_statement_inner(&mut self, statement: &AstStatement, switch: &SwitchStatement) {
        let Some(discriminant_operand) = self.lower_expression_operand(&switch.expression) else {
            self.push_pending(statement, PendingStatementKind::Switch);
            return;
        };
        let discr_local = self.ensure_operand_local(discriminant_operand, switch.expression.span);
        let join_block = self.new_block(statement.span);
        let match_binding = self.register_match_binding(discr_local);
        let scope_depth = self.scope_depth();
        self.push_switch_context(join_block, match_binding.clone(), scope_depth);
        let mut binding_locals: HashMap<String, SwitchBindingLocal> = HashMap::new();

        let analysis = self.collect_switch_sections(
            statement,
            switch,
            &match_binding,
            &mut binding_locals,
        );

        let fallback_block = self
            .current_switch_default_target()
            .map_or(join_block, |target| target.block);

        if analysis.cases.is_empty() {
            self.ensure_goto(fallback_block, statement.span);
        } else if analysis.has_complex_pattern {
            self.lower_switch_as_match(
                discr_local,
                &analysis.cases,
                fallback_block,
                switch.expression.span,
            );
        } else {
            self.lower_switch_as_int(
                discr_local,
                &analysis.cases,
                fallback_block,
                statement.span,
            );
        }

        for info in analysis.sections {
            self.switch_to_block(info.body_block);
            self.push_scope();
            let section = &switch.sections[info.section_index];
            self.predeclare_statement_list(&section.statements);
            for stmt in &section.statements {
                self.lower_statement(stmt);
            }
            self.pop_scope();
            self.ensure_goto(join_block, info.span.or(statement.span));
        }

        self.pop_switch_context();
        self.switch_to_block(join_block);
    }
}
