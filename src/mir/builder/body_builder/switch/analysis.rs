use super::*;
use std::collections::HashMap;

pub(super) struct SwitchAnalysis {
    pub(super) cases: Vec<SwitchCase>,
    pub(super) sections: Vec<SwitchSectionInfo>,
    pub(super) has_complex_pattern: bool,
}

body_builder_impl! {
    pub(super) fn collect_switch_sections(
        &mut self,
        statement: &AstStatement,
        switch: &SwitchStatement,
        match_binding: &str,
        binding_locals: &mut HashMap<String, SwitchBindingLocal>,
    ) -> SwitchAnalysis {
        let mut cases = Vec::new();
        let mut section_infos = Vec::new();
        let mut has_complex_pattern = false;

        for (index, section) in switch.sections.iter().enumerate() {
            if section.labels.is_empty() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "switch section requires at least one label".into(),
                    span: section
                        .statements
                        .first()
                        .and_then(|s| s.span)
                        .or(statement.span),
                                    });
                continue;
            }

            let section_span = switch_section_span(section, statement.span);
            let body_block = self.new_block(section_span);
            section_infos.push(SwitchSectionInfo {
                body_block,
                section_index: index,
                span: section_span,
                            });

            for label in &section.labels {
                match label {
                    SwitchLabel::Case(case_label) => {
                        let label_span = case_label
                            .pattern
                            .raw
                            .span
                            .or_else(|| case_label.guards.iter().find_map(|g| g.expression.span));
                        let parsed = match self.parse_case_pattern(&case_label.pattern, match_binding)
                        {
                            Ok(pattern) => pattern,
                            Err(diag) => {
                                self.diagnostics.push(diag);
                                continue;
                            }
                        };

                        let ParsedCasePattern {
                            kind,
                            key,
                            pre_guards,
                            post_guards,
                            bindings: parsed_bindings,
                            list_plan,
                        } = parsed;

                        if matches!(kind, CasePatternKind::Complex(_)) {
                            has_complex_pattern = true;
                        }

                        let mut pre_guard_meta = Vec::new();
                        for guard_expression in pre_guards {
                            let node = guard_expression
                                .node
                                .clone()
                                .or_else(|| self.expression_node(&guard_expression));
                            pre_guard_meta.push(GuardMetadata {
                                expr: guard_expression,
                                node,
                            });
                        }

                        let mut guard_meta = Vec::new();
                        for guard_expression in post_guards
                            .into_iter()
                            .chain(case_label.guards.iter().cloned().map(|guard| guard.expression))
                        {
                            let node = guard_expression
                                .node
                                .clone()
                                .or_else(|| self.expression_node(&guard_expression));
                            guard_meta.push(GuardMetadata {
                                expr: guard_expression,
                                node,
                            });
                        }

                        let pattern_span = case_label.pattern.raw.span;
                        let mut case_bindings = parsed_bindings;
                        if case_bindings.is_empty() {
                            if let CasePatternKind::Complex(pattern) = &kind {
                                let specs = Self::extract_pattern_bindings(
                                    pattern,
                                    pattern_span.or(label_span),
                                );
                                case_bindings.extend(specs);
                            }
                        }

                        let mut resolved_bindings = Vec::new();
                        for spec in case_bindings {
                            let BindingSpec {
                                name,
                                projection,
                                span,
                                mutability,
                                mode,
                            } = spec;
                            let span = span.or(pattern_span).or(label_span);
                            let mutable_flag = matches!(mutability, PatternBindingMutability::Mutable);
                            let local_id = match binding_locals.entry(name.clone()) {
                                std::collections::hash_map::Entry::Occupied(entry) => {
                                    let existing = entry.get();
                                    if existing.mode != mode {
                                        self.diagnostics.push(LoweringDiagnostic {
                                            message: format!(
                                                "pattern binding `{}` must use the same borrow mode across cases",
                                                name
                                            ),
                                            span,
                                        });
                                    }
                                    if existing.mutability != mutability {
                                        self.diagnostics.push(LoweringDiagnostic {
                                            message: format!(
                                                "pattern binding `{}` cannot switch between `let` and `var` across cases",
                                                name
                                            ),
                                            span,
                                        });
                                    }
                                    existing.local
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    let decl = LocalDecl::new(
                                        Some(name.clone()),
                                        Ty::Unknown,
                                        mutable_flag,
                                        span,
                                        LocalKind::Local,
                                    );
                                    let id = self.push_local(decl);
                                    entry.insert(SwitchBindingLocal {
                                        local: id,
                                        mutability,
                                        mode,
                                    });
                                    self.bind_name(&name, id);
                                    id
                                }
                            };

                            resolved_bindings.push(PatternBinding {
                                name,
                                local: local_id,
                                projection,
                                span,
                                mutability,
                                mode,
                            });
                        }

                        let allows_goto =
                            pre_guard_meta.is_empty() && guard_meta.is_empty() && key.is_some();

                        if let Some(key) = key.clone() {
                            let label_scope_depth = self.scope_depth();
                            if let Some(ctx) = self.current_switch_context_mut() {
                                match ctx.label_map.entry(key.clone()) {
                                    std::collections::hash_map::Entry::Occupied(_) => {
                                        self.diagnostics.push(LoweringDiagnostic {
                                            message: format!(
                                                "duplicate `case` label for pattern `{key}`"
                                            ),
                                            span: pattern_span.or(label_span),
                                                                                    });
                                    }
                                    std::collections::hash_map::Entry::Vacant(entry) => {
                                        entry.insert(SwitchTarget {
                                            block: body_block,
                                            allows_goto,
                                            scope_depth: label_scope_depth,
                                        });
                                    }
                                }
                            }
                        }

                        cases.push(SwitchCase {
                            pattern: kind,
                            pre_guards: pre_guard_meta,
                            guards: guard_meta,
                            body_block,
                            span: label_span,
                            pattern_span: case_label.pattern.raw.span,
                            bindings: resolved_bindings,
                            list_plan,
                        });
                    }
                    SwitchLabel::Default => {
                        let label_scope_depth = self.scope_depth();
                        if let Some(ctx) = self.current_switch_context_mut() {
                            if ctx.default_target.is_some() {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: "duplicate `default` label in switch".into(),
                                    span: section_span,
                                                                    });
                            } else {
                                ctx.default_target = Some(SwitchTarget {
                                    block: body_block,
                                    allows_goto: true,
                                    scope_depth: label_scope_depth,
                                });
                            }
                        }
                    }
                }
            }
        }

        SwitchAnalysis {
            cases,
            sections: section_infos,
            has_complex_pattern,
        }
    }
}
