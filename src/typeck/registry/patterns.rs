use super::*;
use crate::frontend::ast::PatternGuard;
use crate::syntax::pattern::PatternAst;
use std::collections::HashSet;

impl<'a> TypeChecker<'a> {
    pub(super) fn validate_case_pattern(
        &mut self,
        pattern: &crate::frontend::ast::CasePattern,
        guards: &[PatternGuard],
    ) {
        if let Some(ast) = &pattern.ast {
            self.validate_pattern_ast_bindings(ast);
        }
        self.validate_pattern_guards(guards);
    }

    fn validate_pattern_ast_bindings(&mut self, ast: &PatternAst) {
        let mut bindings = HashSet::new();
        for binding in &ast.metadata.bindings {
            if !bindings.insert(binding.name.clone()) {
                self.emit_error(
                    codes::PATTERN_BINDING_CONFLICT,
                    binding.span.or(ast.span),
                    format!(
                        "pattern binding `{}` is declared multiple times",
                        binding.name
                    ),
                );
            }
        }

        let mut record_fields = HashSet::new();
        for field in &ast.metadata.record_fields {
            let key = (field.path.clone(), field.name.clone());
            if !record_fields.insert(key) {
                self.emit_error(
                    codes::PATTERN_FIELD_DUPLICATE,
                    field.name_span.or(field.pattern_span).or(ast.span),
                    format!("record field `{}` appears more than once", field.name),
                );
            }
        }

        if ast.metadata.list_slices.len() > 1 {
            for slice in &ast.metadata.list_slices[1..] {
                self.emit_error(
                    codes::PATTERN_BINDING_CONFLICT,
                    slice.span.or(ast.span),
                    "list patterns may only include a single slice binding",
                );
            }
        }
    }

    fn validate_pattern_guards(&mut self, guards: &[PatternGuard]) {
        let mut previous_depth = None;
        for guard in guards {
            if let Some(prev) = previous_depth {
                if guard.depth < prev {
                    self.emit_error(
                        codes::PATTERN_GUARD_ORDER,
                        guard.keyword_span.or(guard.expression.span),
                        format!(
                            "`when` guard at depth {} must not precede guard at depth {}",
                            guard.depth, prev
                        ),
                    );
                }
            }
            previous_depth = Some(guard.depth);
        }
    }
}
