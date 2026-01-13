use super::*;
use crate::frontend::ast::CasePattern;
use crate::mir::builder::support::{ListDestructurePlan, ListIndexSpec};
use crate::syntax::expr::parse_expression;
use std::char;

pub(super) struct ListPatternPlanResult {
    pub(super) pre_guards: Vec<Expression>,
    pub(super) post_guards: Vec<Expression>,
    pub(super) bindings: Vec<BindingSpec>,
    pub(super) plan: ListDestructurePlan,
}

body_builder_impl! {

    pub(super) fn extract_pattern_bindings(pattern: &Pattern, span: Option<Span>) -> Vec<BindingSpec> {
        let mut bindings = Vec::new();
        let mut prefix = Vec::new();
        Self::collect_pattern_bindings(pattern, span, &mut prefix, &mut bindings);
        bindings
    }
    fn collect_pattern_bindings(
        pattern: &Pattern,
        span: Option<Span>,
                prefix: &mut Vec<PatternProjectionElem>,
        out: &mut Vec<BindingSpec>,
    ) {
        match pattern {
            Pattern::Binding(binding) => {
                out.push(BindingSpec {
                    name: binding.name.clone(),
                    projection: prefix.clone(),
                    span,
                    mutability: binding.mutability,
                    mode: binding.mode,
                                    });
            }
            Pattern::Struct { fields, .. } => {
                for field in fields {
                    prefix.push(PatternProjectionElem::FieldNamed(field.name.clone()));
                    Self::collect_pattern_bindings(&field.pattern, span, prefix, out);
                    prefix.pop();
                }
            }
            Pattern::Tuple(items) => {
                for (index, item) in items.iter().enumerate() {
                    let Ok(idx) = u32::try_from(index) else {
                        continue;
                    };
                    prefix.push(PatternProjectionElem::FieldIndex(idx));
                    Self::collect_pattern_bindings(item, span, prefix, out);
                    prefix.pop();
                }
            }
            Pattern::Enum {
                path,
                variant,
                fields,
            } => {
                prefix.push(PatternProjectionElem::Variant {
                    path: path.clone(),
                    variant: variant.clone(),
                });
                match fields {
                    VariantPatternFields::Unit => {}
                    VariantPatternFields::Tuple(items) => {
                        for (index, item) in items.iter().enumerate() {
                            let Ok(idx) = u32::try_from(index) else {
                                continue;
                            };
                            prefix.push(PatternProjectionElem::FieldIndex(idx));
                            Self::collect_pattern_bindings(item, span, prefix, out);
                            prefix.pop();
                        }
                    }
                    VariantPatternFields::Struct(struct_fields) => {
                        for field in struct_fields {
                            prefix.push(PatternProjectionElem::FieldNamed(field.name.clone()));
                            Self::collect_pattern_bindings(&field.pattern, span, prefix, out);
                            prefix.pop();
                        }
                    }
                }
                prefix.pop();
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    #[allow(dead_code)]
    pub(super) fn pattern_contains_binding(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Binding(_) => true,
            Pattern::Tuple(items) => items.iter().any(Self::pattern_contains_binding),
            Pattern::Struct { fields, .. } => fields
                .iter()
                .any(|field| Self::pattern_contains_binding(&field.pattern)),
            Pattern::Enum { fields, .. } => match fields {
                VariantPatternFields::Unit => false,
                VariantPatternFields::Tuple(items) => {
                    items.iter().any(Self::pattern_contains_binding)
                }
                VariantPatternFields::Struct(items) => items
                    .iter()
                    .any(|field| Self::pattern_contains_binding(&field.pattern)),
            },
            Pattern::Wildcard | Pattern::Literal(_) => false,
        }
    }

    pub(super) fn parse_case_pattern(
        &mut self,
        pattern: &CasePattern,
        binding_name: &str,
    ) -> Result<ParsedCasePattern, LoweringDiagnostic> {
        let expr = &pattern.raw;
        let text = expr.text.trim();
        if text.is_empty() {
            return Err(LoweringDiagnostic {
                message: "pattern cannot be empty".into(),
                span: expr.span,
                            });
        }

        let ast = if let Some(ast) = &pattern.ast {
            ast.clone()
        } else {
            match parse_pattern(text, expr.span) {
                Ok(ast) => ast,
                Err(err) => {
                    return Err(LoweringDiagnostic {
                        message: err.message,
                        span: err.span.or(expr.span),
                    });
                }
            }
        };

        let mut pre_guards = Vec::new();
        let mut post_guards = Vec::new();
        let mut bindings = Vec::new();
        let mut list_plan = None;

        if let PatternNode::List(list) = &ast.node {
            let plan =
                self.plan_list_pattern(list, binding_name, ast.span.or(expr.span))?;
            pre_guards = plan.pre_guards;
            post_guards = plan.post_guards;
            bindings = plan.bindings;
            list_plan = Some(plan.plan);
            return Ok(ParsedCasePattern {
                kind: CasePatternKind::Complex(Pattern::Wildcard),
                key: None,
                pre_guards,
                post_guards,
                bindings,
                list_plan,
            });
        }

        let Some(pattern) = self.lower_pattern_ast(&ast, expr.span) else {
            return Err(LoweringDiagnostic {
                message: "unsupported pattern shape".into(),
                span: ast.span.or(expr.span),
            });
        };

        if bindings.is_empty() {
            bindings = Self::extract_pattern_bindings(&pattern, ast.span.or(expr.span));
        }
        let mut dispatch_pattern = pattern.clone();
        let needs_structural_guard = matches!(
            pattern,
            Pattern::Struct { .. }
                | Pattern::Tuple(_)
                | Pattern::Enum {
                    fields: VariantPatternFields::Tuple(_) | VariantPatternFields::Struct(_),
                    ..
                }
        ) || Self::pattern_requires_guard(&ast.node);

        if needs_structural_guard {
            dispatch_pattern = Pattern::Wildcard;
            let guard_text = format!(
                "{binding_name} is {}",
                Self::pattern_guard_string(&ast.node)
            );
            pre_guards.push(self.parse_guard_expression(
                guard_text,
                ast.span.or(expr.span),
            )?);
        }

        let kind = match &dispatch_pattern {
            Pattern::Wildcard => CasePatternKind::Wildcard,
            Pattern::Literal(value) => CasePatternKind::Literal(value.clone()),
            _ => CasePatternKind::Complex(dispatch_pattern.clone()),
        };
        let key = match &kind {
            CasePatternKind::Literal(value) => Some(literal_key_from_const(value)),
            CasePatternKind::Wildcard => None,
            CasePatternKind::Complex(_) => Some(text.to_string()),
        };

        Ok(ParsedCasePattern {
            kind,
            key,
            pre_guards,
            post_guards,
            bindings,
            list_plan,
        })
    }

    pub(super) fn plan_list_pattern(
        &mut self,
        list: &ListPatternNode,
        binding_name: &str,
        span: Option<Span>,
    ) -> Result<ListPatternPlanResult, LoweringDiagnostic> {
        let list_span = list.span.or(span);
        let prefix_len = list.prefix.len();
        let suffix_len = list.suffix.len();
        let minimum = prefix_len + suffix_len;
        let mut pre_guards = Vec::new();
        let post_guards = Vec::new();
        let mut bindings = Vec::new();

        let len_guard = if list.slice.is_none() && suffix_len == 0 {
            format!("{binding_name}.Length == {minimum}")
        } else {
            format!("{binding_name}.Length >= {minimum}")
        };
        pre_guards.push(self.parse_guard_expression(len_guard, list_span)?);

        let len_local = self.create_temp(list_span);
        if let Some(decl) = self.locals.get_mut(len_local.0) {
            decl.ty = Ty::named("usize");
        }
        let mut indices = Vec::new();

        for (index, element) in list.prefix.iter().enumerate() {
            self.append_list_element_plan(
                element,
                binding_name,
                index,
                false,
                list_span,
                &mut indices,
                &mut pre_guards,
                &mut bindings,
            )?;
        }

        for (rev_index, element) in list.suffix.iter().enumerate() {
            let offset = list.suffix.len().saturating_sub(rev_index);
            self.append_list_element_plan(
                element,
                binding_name,
                offset,
                true,
                list_span,
                &mut indices,
                &mut pre_guards,
                &mut bindings,
            )?;
        }

        if let Some(slice) = &list.slice {
            match slice.as_ref() {
                PatternNode::Wildcard => {}
                PatternNode::Binding(binding) => {
                    bindings.push(BindingSpec {
                        name: binding.name.clone(),
                        projection: vec![PatternProjectionElem::Subslice {
                            from: prefix_len,
                            to: suffix_len,
                        }],
                        span: binding.span.or(list.slice_span).or(list_span).or(span),
                        mutability: binding.mutability,
                        mode: binding.mode,
                    });
                }
                _ => {
                    return Err(LoweringDiagnostic {
                        message: "list slice patterns only support binding or wildcard targets"
                            .into(),
                        span: list.slice_span.or(list_span).or(span),
                    });
                }
            }
        }

        Ok(ListPatternPlanResult {
            pre_guards,
            post_guards,
            bindings,
            plan: ListDestructurePlan {
                length_local: len_local,
                indices,
                span: list_span,
            },
        })
    }

    fn append_list_element_plan(
        &mut self,
        element: &PatternNode,
        binding_name: &str,
        offset: usize,
        from_end: bool,
        span: Option<Span>,
        indices: &mut Vec<ListIndexSpec>,
        pre_guards: &mut Vec<Expression>,
        bindings: &mut Vec<BindingSpec>,
    ) -> Result<(), LoweringDiagnostic> {
        let element_expr = Self::element_expr(binding_name, offset, from_end);
        if let Some(guard_text) = Self::element_guard_expression(element, &element_expr) {
            pre_guards.push(self.parse_guard_expression(guard_text, span)?);
        }

        let wants_binding = Self::pattern_node_contains_binding(element);
        if wants_binding {
            let Some(lowered) = self.lower_pattern_node(element, span) else {
                return Err(LoweringDiagnostic {
                    message: "unsupported pattern inside list element".into(),
                    span,
                });
            };
            let index_local = self.create_temp(span);
            if let Some(decl) = self.locals.get_mut(index_local.0) {
                decl.ty = Ty::named("usize");
            }
            indices.push(ListIndexSpec {
                local: index_local,
                offset,
                from_end,
            });

            let mut prefix = vec![PatternProjectionElem::Index(index_local)];
            Self::collect_pattern_bindings(&lowered, span, &mut prefix, bindings);
        }

        Ok(())
    }

    fn parse_guard_expression(
        &self,
        text: String,
        span: Option<Span>,
    ) -> Result<Expression, LoweringDiagnostic> {
        match parse_expression(&text) {
            Ok(node) => Ok(Expression::with_node(text, span, node)),
            Err(err) => Err(LoweringDiagnostic {
                message: err.message,
                span: err.span.or(span),
            }),
        }
    }

    fn pattern_node_contains_binding(node: &PatternNode) -> bool {
        match node {
            PatternNode::Binding(_) => true,
            PatternNode::Tuple(items) => items.iter().any(Self::pattern_node_contains_binding),
            PatternNode::Struct { fields, .. } => fields
                .iter()
                .any(|field| Self::pattern_node_contains_binding(&field.pattern)),
            PatternNode::Record(record) => record
                .fields
                .iter()
                .any(|field| Self::pattern_node_contains_binding(&field.pattern)),
            PatternNode::Enum { fields, .. } => match fields {
                VariantPatternFieldsNode::Unit => false,
                VariantPatternFieldsNode::Tuple(items) => {
                    items.iter().any(Self::pattern_node_contains_binding)
                }
                VariantPatternFieldsNode::Struct(items) => items
                    .iter()
                    .any(|field| Self::pattern_node_contains_binding(&field.pattern)),
            },
            PatternNode::Positional { elements, .. } => elements
                .iter()
                .any(Self::pattern_node_contains_binding),
            PatternNode::Type { subpattern, .. } => subpattern
                .as_deref()
                .map(Self::pattern_node_contains_binding)
                .unwrap_or(false),
            PatternNode::Relational { .. } => false,
            PatternNode::Binary { left, right, .. } => {
                Self::pattern_node_contains_binding(left)
                    || Self::pattern_node_contains_binding(right)
            }
            PatternNode::Not(inner) => Self::pattern_node_contains_binding(inner),
            PatternNode::List(list) => {
                list.prefix.iter().any(Self::pattern_node_contains_binding)
                    || list
                        .slice
                        .as_deref()
                        .map(Self::pattern_node_contains_binding)
                        .unwrap_or(false)
                    || list.suffix.iter().any(Self::pattern_node_contains_binding)
            }
            PatternNode::Wildcard | PatternNode::Literal(_) => false,
        }
    }

    fn element_guard_expression(element: &PatternNode, element_expr: &str) -> Option<String> {
        match element {
            PatternNode::Wildcard | PatternNode::Binding(_) => None,
            PatternNode::Literal(value) => Some(format!(
                "{element_expr} == {}",
                Self::const_to_guard_string(value)
            )),
            other => Some(format!(
                "{element_expr} is {}",
                Self::pattern_guard_string(other)
            )),
        }
    }

    fn element_expr(binding_name: &str, offset: usize, from_end: bool) -> String {
        if from_end {
            format!("{binding_name}[({binding_name}.Length) - {offset}]")
        } else {
            format!("{binding_name}[{offset}]")
        }
    }

    fn pattern_guard_string(node: &PatternNode) -> String {
        match node {
            PatternNode::Binding(_) | PatternNode::Wildcard => "_".into(),
            PatternNode::Literal(value) => Self::const_to_guard_string(value),
            PatternNode::Tuple(elements) => {
                let parts = elements
                    .iter()
                    .map(Self::pattern_guard_string)
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
                            Self::pattern_guard_string(&field.pattern)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {} }}", path.join("."), body)
            }
            PatternNode::Enum {
                path,
                variant,
                fields,
            } => {
                let ty = if path.is_empty() {
                    variant.clone()
                } else {
                    format!("{}.{}", path.join("."), variant)
                };
                match fields {
                    VariantPatternFieldsNode::Unit => ty,
                    VariantPatternFieldsNode::Tuple(items) => {
                        let parts = items
                            .iter()
                            .map(Self::pattern_guard_string)
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
                                    Self::pattern_guard_string(&field.pattern)
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{ty} {{ {body} }}")
                    }
                }
            }
            PatternNode::Positional { path, elements } => {
                let ty = path.join(".");
                let parts = elements
                    .iter()
                    .map(Self::pattern_guard_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{ty}({parts})")
            }
            PatternNode::Type { path, subpattern } => {
                let ty = path.join(".");
                if let Some(inner) = subpattern {
                    format!("{ty} {}", Self::pattern_guard_string(inner))
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
                format!("{op_str} {}", expr.text)
            }
            PatternNode::Binary { left, op, right } => {
                let op_str = match op {
                    PatternBinaryOp::And => "and",
                    PatternBinaryOp::Or => "or",
                };
                format!(
                    "({} {op_str} {})",
                    Self::pattern_guard_string(left),
                    Self::pattern_guard_string(right)
                )
            }
            PatternNode::Not(inner) => format!("not {}", Self::pattern_guard_string(inner)),
            PatternNode::List(list) => {
                let mut parts = Vec::new();
                for item in &list.prefix {
                    parts.push(Self::pattern_guard_string(item));
                }
                if let Some(slice) = &list.slice {
                    parts.push(format!("..{}", Self::pattern_guard_string(slice)));
                }
                for item in &list.suffix {
                    parts.push(Self::pattern_guard_string(item));
                }
                format!("[{}]", parts.join(", "))
            }
            PatternNode::Record(record) => {
                let body = record
                    .fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{}: {}",
                            field.name,
                            Self::pattern_guard_string(&field.pattern)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                if let Some(path) = &record.path {
                    format!("{} {{ {body} }}", path.join("."))
                } else {
                    format!("{{ {body} }}")
                }
            }
        }
    }

    pub(super) fn pattern_requires_guard(node: &PatternNode) -> bool {
        match node {
            PatternNode::Struct { .. }
            | PatternNode::Tuple(_)
            | PatternNode::Enum { .. }
            | PatternNode::Positional { .. }
            | PatternNode::Type { .. }
            | PatternNode::Relational { .. }
            | PatternNode::Binary { .. }
            | PatternNode::Not(_)
            | PatternNode::List(_)
            | PatternNode::Record(_) => true,
            PatternNode::Binding(_) | PatternNode::Literal(_) | PatternNode::Wildcard => false,
        }
    }

    pub(super) fn lower_pattern_ast(
        &mut self,
        pattern: &PatternAst,
        fallback_span: Option<Span>,
    ) -> Option<Pattern> {
        self.lower_pattern_node(&pattern.node, pattern.span.or(fallback_span))
    }
    pub(super) fn lower_pattern_node(&mut self, node: &PatternNode, span: Option<Span>) -> Option<Pattern> {
        match node {
            PatternNode::Wildcard => Some(Pattern::Wildcard),
            PatternNode::Literal(value) => {
                Some(Pattern::Literal(self.normalise_const(value.clone(), span)))
            }
            PatternNode::Binding(binding) => {
                Some(Pattern::Binding(BindingPattern {
                    name: binding.name.clone(),
                    mutability: binding.mutability,
                    mode: binding.mode,
                }))
            }
            PatternNode::Tuple(elements) => {
                let mut lowered = Vec::with_capacity(elements.len());
                for element in elements {
                    let sub = self.lower_pattern_node(element, span)?;
                    lowered.push(sub);
                }
                Some(Pattern::Tuple(lowered))
            }
            PatternNode::Struct { path, fields } => {
                let mut lowered = Vec::with_capacity(fields.len());
                for field in fields {
                    let sub = self.lower_pattern_node(&field.pattern, span)?;
                    lowered.push(PatternField {
                        name: field.name.clone(),
                        pattern: sub,
                    });
                }
                Some(Pattern::Struct {
                    path: path.clone(),
                    fields: lowered,
                })
            }
            PatternNode::Record(record) => {
                let mut lowered = Vec::with_capacity(record.fields.len());
                for field in &record.fields {
                    let sub = self.lower_pattern_node(&field.pattern, span)?;
                    lowered.push(PatternField {
                        name: field.name.clone(),
                        pattern: sub,
                    });
                }
                Some(Pattern::Struct {
                    path: record.path.clone().unwrap_or_default(),
                    fields: lowered,
                })
            }
            PatternNode::Enum {
                path,
                variant,
                fields,
            } => {
                let lowered_fields = match fields {
                    VariantPatternFieldsNode::Unit => VariantPatternFields::Unit,
                    VariantPatternFieldsNode::Tuple(items) => {
                        let mut lowered = Vec::with_capacity(items.len());
                        for item in items {
                            let sub = self.lower_pattern_node(item, span)?;
                            lowered.push(sub);
                        }
                        VariantPatternFields::Tuple(lowered)
                    }
                    VariantPatternFieldsNode::Struct(items) => {
                        let mut lowered = Vec::with_capacity(items.len());
                        for item in items {
                            let sub = self.lower_pattern_node(&item.pattern, span)?;
                            lowered.push(PatternField {
                                name: item.name.clone(),
                                pattern: sub,
                            });
                        }
                        VariantPatternFields::Struct(lowered)
                    }
                };
                Some(Pattern::Enum {
                    path: path.clone(),
                    variant: variant.clone(),
                    fields: lowered_fields,
                })
            }
            PatternNode::Positional { path, elements } => {
                self.lower_positional_pattern(path, elements, span)
            }
            PatternNode::Type { .. }
            | PatternNode::Relational { .. }
            | PatternNode::Binary { .. }
            | PatternNode::Not(..)
            | PatternNode::List(..) => Some(Pattern::Wildcard),
        }
    }

    pub(super) fn lower_positional_pattern(
        &mut self,
        path: &[String],
        elements: &[PatternNode],
        span: Option<Span>,
            ) -> Option<Pattern> {
        if let Some(pattern) = self.lower_positional_struct(path, elements, span) {
            return Some(pattern);
        }
        if let Some(pattern) = self.lower_positional_enum(path, elements, span) {
            return Some(pattern);
        }
        self.diagnostics.push(LoweringDiagnostic {
            message: "unable to resolve positional pattern target".into(),
            span,
                    });
        None
    }

    pub(super) fn lower_positional_struct(
        &mut self,
        path: &[String],
        elements: &[PatternNode],
        span: Option<Span>,
            ) -> Option<Pattern> {
        let qualified = self.qualify_pattern_path(path)?;
        let positional_fields = {
            let layout = self
                .lookup_struct_layout_by_name(&qualified)
                .or_else(|| self.lookup_class_layout_by_name(&qualified))?;

            if elements.len() > layout.positional.len() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "positional pattern expected at most {} elements",
                        layout.positional.len()
                    ),
                    span,
                });
                return None;
            }

            let mut positional_fields = Vec::new();
            for slot in &layout.positional {
                let Some(field) = layout
                    .fields
                    .iter()
                    .find(|candidate| candidate.index == slot.field_index)
                else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "unable to map positional element to field".into(),
                        span: slot.span.or(span),
                    });
                    return None;
                };
                positional_fields.push((field.name.clone(), slot.span));
            }
            positional_fields
        };

        let mut lowered_fields = Vec::new();
        for (index, element) in elements.iter().enumerate() {
            let Some((field_name, _span_hint)) = positional_fields.get(index).cloned() else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "unable to map positional element to field".into(),
                    span,
                });
                return None;
            };
            let sub = self.lower_pattern_node(element, span)?;
            lowered_fields.push(PatternField {
                name: field_name,
                pattern: sub,
            });
        }

        Some(Pattern::Struct {
            path: path.to_vec(),
            fields: lowered_fields,
        })
    }

    pub(super) fn lower_positional_enum(
        &mut self,
        path: &[String],
        elements: &[PatternNode],
        span: Option<Span>,
            ) -> Option<Pattern> {
        if path.is_empty() {
            return None;
        }

        let variant_name = path.last().cloned()?;
        let type_segments = if path.len() > 1 {
            path[..path.len() - 1].to_vec()
        } else {
            Vec::new()
        };

        let enum_name = if type_segments.is_empty() {
            self.find_enum_for_variant(&variant_name)?
        } else {
            self.qualify_pattern_path(&type_segments)?
        };

        let enum_layout = self.lookup_enum_layout(&enum_name)?;
        let variant_layout = enum_layout
            .variants
            .iter()
            .find(|variant| variant.name == variant_name)?;

        if variant_layout.fields.len() != elements.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "variant `{}` expects {} element(s)",
                    variant_name,
                    variant_layout.fields.len()
                ),
                span,
                            });
            return None;
        }

        let mut lowered = Vec::with_capacity(elements.len());
        for element in elements {
            let sub = self.lower_pattern_node(element, span)?;
            lowered.push(sub);
        }

        Some(Pattern::Enum {
            path: type_segments,
            variant: variant_name,
            fields: VariantPatternFields::Tuple(lowered),
        })
    }

    pub(super) fn find_enum_for_variant(&self, variant: &str) -> Option<String> {
        self.type_layouts.types.iter().find_map(|(name, layout)| {
            if let TypeLayout::Enum(enum_layout) = layout
                && enum_layout.variants.iter().any(|v| v.name == variant)
            {
                return Some(name.clone());
            }
            None
        })
    }

    pub(super) fn lookup_layout_candidate(&self, name: &str) -> Option<String> {
        let current_type = self.current_self_type_name();
        crate::mir::builder::support::resolve_type_layout_name(
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
            name,
        )
    }

    pub(super) fn lookup_struct_layout_by_name(&self, name: &str) -> Option<&StructLayout> {
        let candidate = if self.type_layouts.types.contains_key(name) {
            Some(name.to_string())
        } else {
            self.lookup_layout_candidate(name)
        }?;

        match self.type_layouts.types.get(&candidate)? {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => Some(layout),
            _ => None,
        }
    }

    pub(super) fn lookup_class_layout_by_name(&self, name: &str) -> Option<&StructLayout> {
        if let Some(TypeLayout::Class(layout)) = self.type_layouts.types.get(name) {
            return Some(layout);
        }
        None
    }

    pub(super) fn lookup_enum_layout(&self, name: &str) -> Option<&EnumLayout> {
        let candidate = if self.type_layouts.types.contains_key(name) {
            Some(name.to_string())
        } else {
            self.lookup_layout_candidate(name)
        }?;

        match self.type_layouts.types.get(&candidate)? {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        }
    }

    pub(super) fn lookup_union_layout(&self, name: &str) -> Option<&UnionLayout> {
        let candidate = if self.type_layouts.types.contains_key(name) {
            Some(name.to_string())
        } else {
            self.lookup_layout_candidate(name)
        }?;

        match self.type_layouts.types.get(&candidate)? {
            TypeLayout::Union(layout) => Some(layout),
            _ => None,
        }
    }

    pub(super) fn resolve_ty_name(&self, ty: &Ty) -> Option<String> {
        if let Ty::Named(name) = ty {
            if !name.args.is_empty() {
                return Some(ty.canonical_name());
            }
            if name.as_str() == "Self" {
                return self.current_self_type_name();
            }
            if self.type_layouts.types.contains_key(name.as_str()) {
                return Some(name.as_str().to_string());
            }
            return self.lookup_layout_candidate(name.as_str());
        }

        match ty {
            Ty::Tuple(tuple) => Some(tuple.canonical_name()),
            Ty::String => Some("string".to_string()),
            Ty::Str => Some("str".to_string()),
            Ty::Fn(fn_ty) => Some(fn_ty.canonical_name()),
            Ty::Span(span) => Some(Ty::Span(span.clone()).canonical_name()),
            Ty::ReadOnlySpan(span) => Some(Ty::ReadOnlySpan(span.clone()).canonical_name()),
            Ty::Vec(vec) => Some(Ty::Vec(vec.clone()).canonical_name()),
            Ty::Array(array) => Some(Ty::Array(array.clone()).canonical_name()),
            Ty::Ref(reference) => self.resolve_ty_name(&reference.element),
            Ty::Nullable(inner) => {
                let name = format!("{}?", inner.canonical_name());
                if self.type_layouts.types.contains_key(&name) {
                    Some(name)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(super) fn qualify_pattern_path(&self, path: &[String]) -> Option<String> {
        if path.is_empty() {
            return None;
        }
        let joined = path.join("::");
        if self.type_layouts.types.contains_key(&joined) {
            Some(joined)
        } else {
            self.lookup_layout_candidate(&joined)
        }
    }

    pub(super) fn const_to_guard_string(value: &ConstValue) -> String {
        match value {
            ConstValue::Str { value, .. } | ConstValue::RawStr(value) => {
                format!("\"{value}\"")
            }
            ConstValue::Int(v) | ConstValue::Int32(v) => v.to_string(),
            ConstValue::UInt(v) => v.to_string(),
            ConstValue::Float(v) => v.display(),
            ConstValue::Decimal(v) => v.into_decimal().to_string(),
            ConstValue::Bool(v) => v.to_string(),
            ConstValue::Char(c) => {
                let mut repr = String::with_capacity(4);
                repr.push('\'');
                if let Some(scalar) = char::from_u32(u32::from(*c)) {
                    repr.extend(scalar.escape_default());
                } else {
                    repr.push_str(&format!("\\u{c:04X}"));
                }
                repr.push('\'');
                repr
            }
            ConstValue::Struct { type_name, .. } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short} {{ .. }}")
            }
            ConstValue::Null => String::from("null"),
            ConstValue::Unit => String::from("()"),
            ConstValue::Unknown => String::from("0"),
            ConstValue::Symbol(sym) => sym.clone(),
            ConstValue::Enum {
                type_name,
                variant,
                ..
            } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short}::{variant}")
            }
        }
    }
}
