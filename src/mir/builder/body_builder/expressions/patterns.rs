use super::*;
use crate::syntax::expr::builders::PatternGuardExpr;
use crate::syntax::expr::parse_expression;

body_builder_impl! {
    pub(crate) fn lower_is_pattern_expr(
        &mut self,
        value: ExprNode,
        pattern: &PatternAst,
        guards: &[PatternGuardExpr],
        span: Option<Span>,
    ) -> Option<Operand> {
        let value_operand = self.lower_expr_node(value, span)?;
        let discr_local = self.ensure_operand_local(value_operand, span);
        if matches!(self.operand_ty(&Operand::Copy(Place::new(discr_local))), Some(Ty::Pointer(_)))
        {
            self.diagnostics.push(LoweringDiagnostic {
                message: "patterns on pointer operands are not supported".into(),
                span: pattern.span.or(span),
            });
            return None;
        }
        if Self::pattern_targets_void(&pattern.node) {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`void` is not a valid operand for `is` patterns".into(),
                span: pattern.span.or(span),
            });
            return None;
        }
        // `is` needs temporary blocks to compute the boolean, but the resulting boolean value
        // must outlive the internal lowering scope so callers (e.g. guard chains) can branch on it.
        let result_temp = self.create_temp(span);
        self.locals[result_temp.0].ty = Ty::named("bool");
        self.locals[result_temp.0].is_nullable = false;

        self.push_scope();
        let match_binding_name = self.register_match_binding(discr_local);

        let mut guard_meta = Vec::new();
        let mut mir_pattern = None;
        let allow_generated_guards = !self.in_guard_expression;
        if allow_generated_guards && let PatternNode::List(list) = &pattern.node {
            match self.plan_list_pattern(list, &match_binding_name, pattern.span.or(span)) {
                Ok(plan) => {
                    if !plan.bindings.is_empty() {
                        self.diagnostics.push(LoweringDiagnostic {
                            message:
                                "list pattern bindings in `is` expressions are not supported yet"
                                    .into(),
                            span: pattern.span.or(span),
                        });
                        self.pop_scope();
                        return None;
                    }
                    for guard_expression in plan
                        .pre_guards
                        .into_iter()
                        .chain(plan.post_guards.into_iter())
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
                }
                Err(diag) => {
                    self.diagnostics.push(diag);
                    self.pop_scope();
                    return None;
                }
            }
            mir_pattern = Some(Pattern::Wildcard);
        } else if allow_generated_guards {
            if let Some(predicate) = self.pattern_predicate_for_is(
                &pattern.node,
                &match_binding_name,
                pattern.span.or(span),
            ) {
                if let Some(meta) = self.guard_from_text(predicate, pattern.span.or(span)) {
                    guard_meta.push(meta);
                    mir_pattern = Some(Pattern::Wildcard);
                }
            }
        }

        let mir_pattern = if let Some(pattern) = mir_pattern {
            pattern
        } else {
            let Some(pattern) = self.prepare_is_pattern(pattern, span) else {
                self.pop_scope();
                return None;
            };
            pattern
        };

        guard_meta.extend(
            guards
            .iter()
            .map(|guard| {
                let expr_text = Self::expr_to_string(&guard.expr);
                GuardMetadata {
                    expr: Expression::with_node(expr_text, guard.span, guard.expr.clone()),
                    node: Some(guard.expr.clone()),
                }
            }),
        );

        let result = self.emit_is_pattern_boolean(
            result_temp,
            discr_local,
            mir_pattern,
            &guard_meta,
            span,
        );

        self.pop_scope();
        Some(result)
    }

    pub(crate) fn prepare_is_pattern(&mut self, pattern: &PatternAst, span: Option<Span>) -> Option<Pattern> {
        let pattern_span = pattern.span.or(span);

        let Some(mir_pattern) = self.lower_is_pattern_node(&pattern.node, pattern_span) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "unsupported pattern in `is` expression".into(),
                span: pattern_span,
                            });
            return None;
        };

        Some(mir_pattern)
    }

    fn lower_is_pattern_node(&mut self, node: &PatternNode, span: Option<Span>) -> Option<Pattern> {
        match node {
            PatternNode::Type { .. }
            | PatternNode::Relational { .. }
            | PatternNode::Binary { .. }
            | PatternNode::Not(..)
            | PatternNode::List(_) => Some(Pattern::Wildcard),
            _ => self.lower_pattern_node(node, span),
        }
    }

    fn guard_from_text(&mut self, text: String, span: Option<Span>) -> Option<GuardMetadata> {
        match parse_expression(&text) {
            Ok(node) => Some(GuardMetadata {
                expr: Expression::with_node(text, span, node.clone()),
                node: Some(node),
            }),
            Err(err) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "failed to lower pattern guard `{text}`: {}",
                        err.message
                    ),
                    span: err.span.or(span),
                });
                None
            }
        }
    }

    fn pattern_predicate_for_is(
        &mut self,
        node: &PatternNode,
        value_expr: &str,
        span: Option<Span>,
    ) -> Option<String> {
        match node {
            PatternNode::Wildcard | PatternNode::Binding(_) => Some("true".into()),
            PatternNode::Literal(value) => Some(format!(
                "{value_expr} == {}",
                Self::const_to_guard_string(value)
            )),
            PatternNode::Relational { op, expr } => {
                let op_str = match op {
                    RelationalOp::Less => "<",
                    RelationalOp::LessEqual => "<=",
                    RelationalOp::Greater => ">",
                    RelationalOp::GreaterEqual => ">=",
                };
                Some(format!("{value_expr} {op_str} {}", expr.text))
            }
            PatternNode::Binary { op, left, right } => {
                let left_guard = self.pattern_predicate_for_is(left, value_expr, span)?;
                let right_guard = self.pattern_predicate_for_is(right, value_expr, span)?;
                let op_str = match op {
                    PatternBinaryOp::And => "&&",
                    PatternBinaryOp::Or => "||",
                };
                Some(format!("({left_guard}) {op_str} ({right_guard})"))
            }
            PatternNode::Not(inner) => {
                let guard = self.pattern_predicate_for_is(inner, value_expr, span)?;
                Some(format!("!({guard})"))
            }
            PatternNode::Type { path, subpattern } => {
                if let Some(inner) = subpattern {
                    return self.pattern_predicate_for_is(inner, value_expr, span);
                }
                Some(format!(
                    "{value_expr} is {}",
                    Self::join_pattern_path(path)
                ))
            }
            PatternNode::Struct { fields, .. } => {
                let mut guards = Vec::new();
                for field in fields {
                    let field_expr = format!("{value_expr}.{}", field.name);
                    let Some(field_guard) =
                        self.pattern_predicate_for_is(&field.pattern, &field_expr, span)
                    else {
                        return None;
                    };
                    guards.push(field_guard);
                }
                Some(if guards.is_empty() {
                    String::from("true")
                } else {
                    guards.join(" && ")
                })
            }
            PatternNode::Record(record) => {
                let mut guards = Vec::new();
                for field in &record.fields {
                    let field_expr = format!("{value_expr}.{}", field.name);
                    let Some(field_guard) =
                        self.pattern_predicate_for_is(&field.pattern, &field_expr, span)
                    else {
                        return None;
                    };
                    guards.push(field_guard);
                }
                if guards.is_empty() {
                    Some("true".into())
                } else {
                    Some(guards.join(" && "))
                }
            }
            PatternNode::Tuple(_) | PatternNode::List(_) => Some(format!(
                "{value_expr} is {}",
                Self::pattern_node_to_string(node)
            )),
            PatternNode::Enum { .. } | PatternNode::Positional { .. } => None,
        }
    }

    fn pattern_targets_void(node: &PatternNode) -> bool {
        match node {
            PatternNode::Type { path, .. } | PatternNode::Struct { path, .. } => {
                Self::join_pattern_path(path).eq_ignore_ascii_case("void")
            }
            PatternNode::Record(record) => record
                .path
                .as_ref()
                .map(|path| Self::join_pattern_path(path).eq_ignore_ascii_case("void"))
                .unwrap_or(false),
            _ => false,
        }
    }

    pub(crate) fn emit_is_pattern_boolean(
        &mut self,
        temp: LocalId,
        discr_local: LocalId,
        pattern: Pattern,
        guards: &[GuardMetadata],
        span: Option<Span>,
    ) -> Operand {
        let true_block = self.new_block(span);
        let false_block = self.new_block(span);
        let join_block = self.new_block(span);

        let guard_entry = self.lower_guard_chain(guards, true_block, false_block, span);

        self.set_terminator(
            span,
            Terminator::Match {
                value: Place::new(discr_local),
                arms: vec![MatchArm {
                    pattern,
                    guard: None,
                    bindings: Vec::new(),
                    target: guard_entry,
                }],
                otherwise: false_block,
            },
        );

        self.switch_to_block(true_block);
        self.assign_boolean_temp(temp, true, span);
        self.ensure_goto(join_block, span);

        self.switch_to_block(false_block);
        self.assign_boolean_temp(temp, false, span);
        self.ensure_goto(join_block, span);

        self.switch_to_block(join_block);
        Operand::Copy(Place::new(temp))
    }

    pub(crate) fn assign_boolean_temp(&mut self, temp: LocalId, value: bool, span: Option<Span>) {
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Bool(value)))),
            },
        });
    }
    pub(crate) fn pattern_ast_to_string(pattern: &PatternAst) -> String {
        Self::pattern_node_to_string(&pattern.node)
    }
    pub(crate) fn pattern_node_to_string(node: &PatternNode) -> String {
        match node {
            PatternNode::Wildcard => "_".into(),
            PatternNode::Literal(value) => Self::const_to_string(value),
            PatternNode::Binding(binding) => Self::binding_to_string(binding),
            PatternNode::Tuple(elements) => {
                let parts = elements
                    .iter()
                    .map(Self::pattern_node_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({parts})")
            }
            PatternNode::Struct { path, fields } => {
                let body = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            field.name,
                            Self::pattern_node_to_string(&field.pattern)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {} }}", Self::join_pattern_path(path), body)
            }
            PatternNode::Record(record) => {
                let body = record
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            field.name,
                            Self::pattern_node_to_string(&field.pattern)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                match &record.path {
                    Some(path) => format!("{} {{ {} }}", path.join("::"), body),
                    None => format!("{{ {} }}", body),
                }
            }
            PatternNode::Enum {
                path,
                variant,
                fields,
            } => {
                let ty = if path.is_empty() {
                    variant.clone()
                } else {
                    format!("{}::{}", path.join("::"), variant)
                };
                match fields {
                    VariantPatternFieldsNode::Unit => ty,
                    VariantPatternFieldsNode::Tuple(items) => {
                        let parts = items
                            .iter()
                            .map(Self::pattern_node_to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{ty}({parts})")
                    }
                    VariantPatternFieldsNode::Struct(items) => {
                        let body = items
                            .iter()
                            .map(|field| {
                                format!(
                                    "{}: {}",
                                    field.name,
                                    Self::pattern_node_to_string(&field.pattern)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{ty} {{ {body} }}")
                    }
                }
            }
            PatternNode::Positional { path, elements } => {
                let ty = Self::join_pattern_path(path);
                let parts = elements
                    .iter()
                    .map(Self::pattern_node_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{ty}({parts})")
            }
            PatternNode::Type { path, subpattern } => {
                let ty = Self::join_pattern_path(path);
                if let Some(inner) = subpattern {
                    format!("{ty} {}", Self::pattern_node_to_string(inner))
                } else {
                    ty
                }
            }
            PatternNode::Relational { op, expr } => {
                let op_str = match op {
                    RelationalOp::Less => "<",
                    RelationalOp::LessEqual => "<=",
                    RelationalOp::Greater => ">",
                    RelationalOp::GreaterEqual => ">=",
                };
                format!("{} {}", op_str, expr.text.trim())
            }
            PatternNode::Binary { op, left, right } => {
                let op_str = match op {
                    PatternBinaryOp::And => "and",
                    PatternBinaryOp::Or => "or",
                };
                format!(
                    "({} {} {})",
                    Self::pattern_node_to_string(left),
                    op_str,
                    Self::pattern_node_to_string(right)
                )
            }
            PatternNode::Not(inner) => format!("not {}", Self::pattern_node_to_string(inner)),
            PatternNode::List(ListPatternNode {
                prefix,
                slice,
                suffix,
                ..
            }) => {
                let mut parts = Vec::new();
                for item in prefix {
                    parts.push(Self::pattern_node_to_string(item));
                }
                if let Some(slice) = slice {
                    parts.push(format!("..{}", Self::pattern_node_to_string(slice)));
                }
                for item in suffix {
                    parts.push(Self::pattern_node_to_string(item));
                }
                format!("[{}]", parts.join(", "))
            }
        }
    }
    fn binding_to_string(binding: &BindingPatternNode) -> String {
        let mut parts = Vec::new();
        match binding.mode {
            PatternBindingMode::In => parts.push("in".to_string()),
            PatternBindingMode::Ref => parts.push("ref".to_string()),
            PatternBindingMode::RefReadonly => {
                parts.push("ref".to_string());
                parts.push("readonly".to_string());
            }
            PatternBindingMode::Move => parts.push("move".to_string()),
            PatternBindingMode::Value => {}
        }
        match binding.mutability {
            PatternBindingMutability::Immutable => parts.push("let".to_string()),
            PatternBindingMutability::Mutable => parts.push("var".to_string()),
        }
        parts.push(binding.name.clone());
        parts.join(" ")
    }
    pub(crate) fn join_pattern_path(segments: &[String]) -> String {
        if segments.is_empty() {
            "_".into()
        } else {
            segments.join("::")
        }
    }
}
