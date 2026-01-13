use super::super::support::resolve_type_layout_name;
use super::*;
use crate::frontend::parser::parse_type_expression_text;

body_builder_impl! {
    pub(crate) fn lower_index_expr(
        &mut self,
        base: ExprNode,
        indices: Vec<ExprNode>,
        span: Option<Span>,
            ) -> Option<Operand> {
        let place = self.lower_place_expr(
            ExprNode::Index {
                base: Box::new(base),
                indices,
                null_conditional: false,
            },
            span,
                    )?;
        Some(Operand::Copy(place))
    }
    pub(crate) fn lower_range_index_expr(
        &mut self,
        base: ExprNode,
        range: ExprNode,
        span: Option<Span>,
            ) -> Option<Operand> {
        let range_span = match &range {
            ExprNode::Range(info) => info.span,
            _ => span,
        };
        let arg_span = range_span.or(span);
        let argument = CallArgument::positional(range, arg_span, arg_span);
        let callee = ExprNode::Member {
            base: Box::new(base),
            member: "Slice".to_string(),
            null_conditional: false,
        };
        self.lower_call(callee, vec![argument], None, span, true)
    }
    pub(crate) fn lower_index_from_end_expr(
        &mut self,
        expr: ExprNode,
        span: Option<Span>,
            ) -> Option<Operand> {
        let endpoint = RangeEndpoint::new(expr, true, span);
        self.lower_range_endpoint(endpoint, span)
    }
    pub(crate) fn lower_range_value(
        &mut self,
        range: RangeExpr,
        span: Option<Span>,
            ) -> Option<Operand> {
        let index_ty = Ty::named("Std::Range::Index");
        let index_name = index_ty.canonical_name();

        let start_operand = match range.start {
            Some(start) => Some(self.lower_range_endpoint(*start, span)?),
            None => None,
        };
        let end_operand = match range.end {
            Some(end) => Some(self.lower_range_endpoint(*end, span)?),
            None => None,
        };

        let (range_name, fields) = match (start_operand, end_operand, range.inclusive) {
            (Some(start), Some(end), false) => ("Std::Range::Range".to_string(), vec![start, end]),
            (Some(start), None, _) => ("Std::Range::RangeFrom".to_string(), vec![start]),
            (None, Some(end), false) => ("Std::Range::RangeTo".to_string(), vec![end]),
            (Some(start), Some(end), true) => {
                ("Std::Range::RangeInclusive".to_string(), vec![start, end])
            }
            (None, Some(end), true) => {
                let zero = ConstOperand::new(ConstValue::UInt(0));
                let default_start = self.materialise_index_operand(
                    Operand::Const(zero),
                    false,
                    span,
                    &index_name,
                )?;
                ("Std::Range::RangeInclusive".to_string(), vec![default_start, end])
            }
            (None, None, _) => ("Std::Range::RangeFull".to_string(), Vec::new()),
        };

        let local = self.create_temp(span);
        if let Some(decl) = self.locals.get_mut(local.0) {
            decl.ty = Ty::named(range_name.clone());
            decl.is_nullable = false;
        }
        let range_place = Place::new(local);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: range_place.clone(),
                value: Rvalue::Aggregate {
                    kind: AggregateKind::Adt {
                        name: range_name,
                        variant: None,
                    },
                    fields,
                },
            },
        });
        Some(Operand::Copy(range_place))
    }
    pub(crate) fn lower_range_endpoint(
        &mut self,
        endpoint: RangeEndpoint,
        span: Option<Span>,
            ) -> Option<Operand> {
        let expr_value = self.lower_expr_node(*endpoint.expr, span)?;
        self.materialise_index_operand(
            expr_value,
            endpoint.from_end,
            endpoint.span.or(span),
            "Std::Range::Index",
        )
    }
    fn materialise_index_operand(
        &mut self,
        value: Operand,
        from_end: bool,
        span: Option<Span>,
        ty_name: &str,
            ) -> Option<Operand> {
        let value_local = self.ensure_operand_local(value, span);
        if let Some(decl) = self.locals.get_mut(value_local.0) {
            if decl.ty == Ty::Unknown {
                decl.ty = Ty::named("usize");
                decl.is_nullable = false;
            }
        }
        let local = self.create_temp(span);
        if let Some(decl) = self.locals.get_mut(local.0) {
            decl.ty = Ty::named(ty_name);
            decl.is_nullable = false;
        }
        let fields = vec![
            Operand::Copy(Place::new(value_local)),
            Operand::Const(ConstOperand::new(ConstValue::Bool(from_end))),
        ];
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(local),
                value: Rvalue::Aggregate {
                    kind: AggregateKind::Adt {
                        name: ty_name.to_string(),
                        variant: None,
                    },
                    fields,
                },
            },
        });
        Some(Operand::Copy(Place::new(local)))
    }
    fn lower_from_end_index(
        &mut self,
        place: &Place,
        expr: ExprNode,
        span: Option<Span>,
            ) -> Option<LocalId> {
        let len_local = self.create_temp(span);
        if let Some(decl) = self.locals.get_mut(len_local.0) {
            decl.ty = Ty::named("usize");
            decl.is_nullable = false;
        }
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(len_local),
                value: Rvalue::Len(place.clone()),
            },
        });

        let value_operand = self.lower_expr_node(expr, span)?;
        let value_local = self.ensure_operand_local(value_operand, span);
        if let Some(decl) = self.locals.get_mut(value_local.0) {
            if decl.ty == Ty::Unknown {
                decl.ty = Ty::named("usize");
                decl.is_nullable = false;
            }
        }

        let result_local = self.create_temp(span);
        if let Some(decl) = self.locals.get_mut(result_local.0) {
            decl.ty = Ty::named("usize");
            decl.is_nullable = false;
        }

        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(result_local),
                value: Rvalue::Binary {
                    op: BinOp::Sub,
                    lhs: Operand::Copy(Place::new(len_local)),
                    rhs: Operand::Copy(Place::new(value_local)),
                    rounding: None,
                },
            },
        });

        Some(result_local)
    }
    pub(crate) fn lower_member_expr(
        &mut self,
        base: ExprNode,
        member: &str,
        span: Option<Span>,
        ) -> Option<Operand> {
        let base_repr = Self::expr_to_string(&base);
        if let Some(segments) = collect_path_segments(&base) {
            let base_is_bound_value = segments
                .first()
                .is_some_and(|name| segments.len() == 1 && self.lookup_name(name).is_some());
            if !base_is_bound_value {
                if let Some(operand) = self.lower_static_member_operand(&base, member, span) {
                    return Some(operand);
                }
                if let Some(const_operand) = self.resolve_static_const(&base, member, span) {
                    return Some(const_operand);
                }
                if let Some(static_operand) = self.lower_namespace_static_path(&base, member, span)
                {
                    return Some(static_operand);
                }
            }
        }
        if self.member_chain_unresolved(&base) {
            let repr = format!("{base_repr}.{member}");
            return Some(Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr,
                span,
                                info: None,
            }));
        }

        let base_operand = self.lower_expr_node(base, span)?;
        if let Some(result) = self.lower_property_member(&base_operand, member, span) {
            return Some(result);
        }
        if let Some(owner) = self.operand_type_name(&base_operand) {
            let resolved_owner = self
                .resolve_ty_name(&Ty::named(owner.clone()))
                .unwrap_or(owner.clone());
            let mut field_owner: Option<(String, &FieldSymbol)> = None;
            for owner_name in [resolved_owner.as_str(), owner.as_str()] {
                if let Some(symbol) = self.symbol_index.field_symbol(owner_name, member) {
                    field_owner = Some((owner_name.to_string(), symbol));
                    break;
                }
            }
            if let Some((field_owner, field_symbol)) = field_owner {
                if field_symbol.is_static {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "static field `{field_owner}.{member}` must be accessed using the type name"
                        ),
                        span: field_symbol.span.or(span),
                    });
                    return Some(self.pending_static_operand(&field_owner, member, span));
                }
                let owner_namespace = self
                    .owner_namespace(&field_owner, field_symbol.namespace.as_deref())
                    .map(|ns| ns.to_string());
                let receiver_type = self.operand_type_name(&base_operand);
                let owner_package = self
                    .owner_package(&field_owner)
                    .map(|pkg| pkg.to_string());
                let descriptor = format!("field `{field_owner}.{member}`");
                if self.member_accessible(
                    field_symbol.visibility,
                    &field_owner,
                    owner_package.as_deref(),
                    owner_namespace.as_deref(),
                    receiver_type.as_deref(),
                    true,
                    span.or(field_symbol.span),
                    &descriptor,
                ) {
                    return Some(self.project_member_operand(base_operand, member, span));
                }
                return Some(Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                    repr: format!("{field_owner}.{member}"),
                    span,
                    info: None,
                }));
            }
            let mut candidates = Vec::new();
            for owner_name in [resolved_owner.as_str(), owner.as_str()] {
                let qualified = format!("{owner_name}::{member}");
                if let Some(overloads) = self.symbol_index.function_overloads(&qualified) {
                    for symbol in overloads {
                        if symbol.is_static {
                            continue;
                        }
                        candidates.push(PendingFunctionCandidate {
                            qualified: symbol.qualified.clone(),
                            signature: symbol.signature.clone(),
                            is_static: symbol.is_static,
                        });
                    }
                }
                if !candidates.is_empty() {
                    break;
                }
            }
            if !candidates.is_empty() {
                return Some(Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                    repr: format!("{owner}.{member}"),
                    span,
                    info: Some(Box::new(PendingOperandInfo::FunctionGroup {
                        path: format!("{resolved_owner}::{member}"),
                        candidates,
                        receiver: Some(Box::new(base_operand)),
                    })),
                }));
            }
        }
        Some(self.project_member_operand(base_operand, member, span))
    }
    pub(crate) fn const_symbol_value(
        &mut self,
        symbol: &ConstSymbol,
        span: Option<Span>,
            ) -> Option<ConstValue> {
        symbol.value.clone().or_else(|| {
            if let Some(node) = symbol.initializer.node.as_ref() {
                if let ExprNode::Literal(literal) = node {
                    return Some(literal.value.clone());
                }
            }
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "constant `{}` could not be evaluated",
                    symbol.qualified
                ),
                span,
                            });
            None
        })
    }
    pub(crate) fn resolve_static_const(
        &mut self,
        base: &ExprNode,
        member: &str,
        span: Option<Span>,
            ) -> Option<Operand> {
        let mut segments = collect_path_segments(base)?;
        segments.push(member.to_string());
        let qualified = segments.join("::");

        let mut candidates = Vec::new();
        candidates.push(qualified.clone());
        if let Some(ns) = self.namespace.as_deref() {
            let mut current = Some(ns);
            while let Some(prefix) = current {
                candidates.push(format!("{prefix}::{qualified}"));
                current = prefix.rfind("::").map(|idx| &prefix[..idx]);
            }
        }

        let current_type = self.current_self_type_name();
        for candidate in candidates {
            let canonical = candidate.replace('.', "::");

            if let Some(symbol) = self.symbol_index.const_symbol(&canonical) {
                let value = self.const_symbol_value(symbol, span)?;
                let value = self.normalise_const(value, span);
                return Some(Operand::Const(ConstOperand::new(value)));
            }

            if let Some((type_name, variant_name)) = canonical.rsplit_once("::") {
                let resolved = resolve_type_layout_name(
                    self.type_layouts,
                    Some(self.import_resolver),
                    self.namespace.as_deref(),
                    current_type.as_deref(),
                    type_name,
                )
                .or_else(|| {
                    if self.type_layouts.types.contains_key(type_name) {
                        Some(type_name.to_string())
                    } else {
                        None
                    }
                });

                if let Some(layout_name) = resolved {
                    if self
                        .symbol_index
                        .has_enum_variant(&layout_name, variant_name)
                    {
                        if let Some(TypeLayout::Enum(enum_layout)) =
                            self.type_layouts.types.get(&layout_name)
                        {
                            if let Some(variant) = enum_layout
                                .variants
                                .iter()
                                .find(|entry| entry.name == variant_name)
                            {
                                let value = ConstValue::Enum {
                                    type_name: layout_name.clone(),
                                    variant: variant_name.to_string(),
                                    discriminant: variant.discriminant,
                                };
                                let value = self.normalise_const(value, span);
                                return Some(Operand::Const(ConstOperand::new(value)));
                            }
                        }
                    }
                }
            }

            match canonical.as_str() {
                status if status.contains("DecimalStatus::") => {
                    let variant = status.rsplit("::").next().unwrap_or(status);
                    let discriminant = match variant {
                        "Success" => 0,
                        "Overflow" => 1,
                        "DivideByZero" => 2,
                        "InvalidRounding" => 3,
                        "InvalidFlags" => 4,
                        "InvalidPointer" => 5,
                        "InvalidOperand" => 6,
                        _ => continue,
                    };
                    let value = ConstValue::Enum {
                        type_name: "Std::Numeric::Decimal::DecimalStatus".into(),
                        variant: variant.to_string(),
                        discriminant,
                    };
                    let value = self.normalise_const(value, span);
                    return Some(Operand::Const(ConstOperand::new(value)));
                }
                variant if variant.contains("DecimalIntrinsicVariant::") => {
                    let name = variant.rsplit("::").next().unwrap_or(variant);
                    let discriminant = match name {
                        "Scalar" => 0,
                        _ => continue,
                    };
                    let value = ConstValue::Enum {
                        type_name: "Std::Numeric::Decimal::DecimalIntrinsicVariant".into(),
                        variant: name.to_string(),
                        discriminant,
                    };
                    let value = self.normalise_const(value, span);
                    return Some(Operand::Const(ConstOperand::new(value)));
                }
                rounding if rounding.contains("DecimalRoundingMode::") => {
                    let name = rounding.rsplit("::").next().unwrap_or(rounding);
                    let discriminant = match name {
                        "TiesToEven" => 0,
                        "TowardZero" => 1,
                        "AwayFromZero" => 2,
                        "TowardPositive" => 3,
                        "TowardNegative" => 4,
                        _ => continue,
                    };
                    let value = ConstValue::Enum {
                        type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
                        variant: name.to_string(),
                        discriminant,
                    };
                    let value = self.normalise_const(value, span);
                    return Some(Operand::Const(ConstOperand::new(value)));
                }
                hint if hint.contains("DecimalVectorizeHint::") => {
                    let name = hint.rsplit("::").next().unwrap_or(hint);
                    let discriminant = match name {
                        "None" => 0,
                        "Decimal" => 1,
                        _ => continue,
                    };
                    let value = ConstValue::Enum {
                        type_name: "Std::Numeric::Decimal::DecimalVectorizeHint".into(),
                        variant: name.to_string(),
                        discriminant,
                    };
                    let value = self.normalise_const(value, span);
                    return Some(Operand::Const(ConstOperand::new(value)));
                }
                _ => {}
            }
        }

        None
    }
    pub(crate) fn lower_property_member(
        &mut self,
        base_operand: &Operand,
        member: &str,
        span: Option<Span>,
            ) -> Option<Operand> {
        let member_lower = member.to_ascii_lowercase();
        if matches!(member_lower.as_str(), "length" | "count") {
            if let Some(len_operand) = self.lower_sequence_length(base_operand, span) {
                return Some(len_operand);
            }
        }

        let (type_name, symbol_ref) = self.property_symbol_from_operand(base_operand, member)?;
        let symbol = symbol_ref.clone();

        let Some(getter) = symbol.accessors.get(&PropertyAccessorKind::Get) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "property `{type_name}.{member}` does not provide a getter"
                ),
                span: symbol.span.or(span),
                            });
            return Some(Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: format!("{type_name}.{member}"),
                span,
                info: None,
            }));
        };

        if symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static property `{type_name}.{member}` must be accessed using the type name"
                ),
                span: symbol.span.or(span),
            });
            return Some(self.pending_static_operand(&type_name, member, span));
        }

        let owner_namespace = self
            .owner_namespace(&type_name, symbol.namespace.as_deref())
            .map(|ns| ns.to_string());
        let receiver_type = self.operand_type_name(base_operand);
        let owner_package = self
            .owner_package(&type_name)
            .map(|pkg| pkg.to_string());
        let descriptor = format!("property `{type_name}.{member}`");
        if !self.member_accessible(
            symbol.visibility,
            &type_name,
            owner_package.as_deref(),
            owner_namespace.as_deref(),
            receiver_type.as_deref(),
            true,
            span.or(symbol.span),
            &descriptor,
        ) {
            return Some(Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: format!("{type_name}.{member}"),
                span,
                info: None,
            }));
        }

        let mut args = Vec::new();
        args.push(base_operand.clone());
        let return_ty = parse_type_expression_text(symbol.ty.as_str())
            .map(|expr| Ty::from_type_expr(&expr))
            .unwrap_or_else(|| Ty::named(symbol.ty.clone()));
        let receiver_ty = self
            .operand_type_name(base_operand)
            .unwrap_or_else(|| type_name.clone());
        let return_ty = self.instantiate_member_type_from_owner_name(&receiver_ty, &return_ty);
        self.emit_property_call(
            &getter.function,
            args,
            Some((return_ty, symbol.is_nullable)),
            span,
                    )
    }

    pub(crate) fn instantiate_member_type_from_owner_name(&self, owner: &str, ty: &Ty) -> Ty {
        use crate::mir::GenericArg;
        use std::collections::HashMap;

        let debug = std::env::var_os("CHIC_DEBUG_GENERIC_INSTANTIATION").is_some();
        let Some(owner_expr) = parse_type_expression_text(owner) else {
            if debug {
                eprintln!("[chic-debug] instantiate_member_type: failed to parse owner `{owner}`");
            }
            return ty.clone();
        };
        let mut owner_ty = Ty::from_type_expr(&owner_expr);
        let named = loop {
            match &owner_ty {
                Ty::Named(named) => break named.clone(),
                Ty::Ref(reference) => {
                    owner_ty = reference.element.clone();
                }
                Ty::Nullable(inner) => {
                    owner_ty = (**inner).clone();
                }
                _ => {
                    if debug {
                        eprintln!(
                            "[chic-debug] instantiate_member_type: owner `{owner}` parsed to non-named `{}`",
                            owner_ty.canonical_name()
                        );
                    }
                    return ty.clone();
                }
            }
        };
        if named.args.is_empty() {
            if debug {
                eprintln!(
                    "[chic-debug] instantiate_member_type: owner `{owner}` has no args; base={}",
                    named.name
                );
            }
            return ty.clone();
        }

        let base_name = self
            .resolve_ty_name(&Ty::named(named.name.clone()))
            .or_else(|| self.lookup_layout_candidate(named.name.as_str()))
            .unwrap_or_else(|| named.name.clone());

        let base_name = self
            .symbol_index
            .resolve_type_generics_owner(base_name.as_str())
            .unwrap_or(base_name);

        let Some(params) = self.symbol_index.type_generics(base_name.as_str()) else {
            if debug {
                eprintln!(
                    "[chic-debug] instantiate_member_type: no generics recorded for base `{base_name}` (from owner `{owner}`)"
                );
            }
            return ty.clone();
        };
        if params.len() != named.args.len() {
            if debug {
                eprintln!(
                    "[chic-debug] instantiate_member_type: base `{base_name}` param/arg mismatch; params={} args={}",
                    params.len(),
                    named.args.len()
                );
            }
            return ty.clone();
        }

        let mut map = HashMap::new();
        for (param, arg) in params.iter().zip(named.args.iter()) {
            if let GenericArg::Type(arg_ty) = arg {
                map.insert(param.name.clone(), arg_ty.clone());
            }
        }

        if debug {
            let substitutions: Vec<_> = map
                .iter()
                .map(|(k, v)| format!("{k}={}", v.canonical_name()))
                .collect();
            eprintln!(
                "[chic-debug] instantiate_member_type: owner `{owner}` base `{base_name}` -> [{}]",
                substitutions.join(", ")
            );
            eprintln!(
                "[chic-debug] instantiate_member_type: before={} after={}",
                ty.canonical_name(),
                Self::substitute_generics(ty, &map).canonical_name()
            );
        }

        Self::substitute_generics(ty, &map)
    }
    pub(crate) fn lower_sequence_length(
        &mut self,
        base_operand: &Operand,
        span: Option<Span>,
            ) -> Option<Operand> {
        let ty = self.operand_ty(base_operand)?;
        let mut core = ty.clone();
        while let Ty::Nullable(inner) = core {
            core = *inner;
        }

        let mut supports_length = matches!(
            core,
            Ty::Vec(_) | Ty::Array(_) | Ty::Span(_) | Ty::ReadOnlySpan(_) | Ty::String | Ty::Str
        );
        if !supports_length {
            supports_length = matches!(
                self.primitive_registry.kind_for(&core),
                Some(crate::primitives::PrimitiveKind::String)
                    | Some(crate::primitives::PrimitiveKind::Str)
            );
        }
        if !supports_length {
            return None;
        }

        let place = match base_operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let mut place = place.clone();
                self.normalise_place(&mut place);
                place
            }
            _ => {
                let temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(temp.0) {
                    local.ty = ty.clone();
                    local.is_nullable = matches!(ty, Ty::Nullable(_));
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(base_operand.clone()),
                    },
                });
                Place::new(temp)
            }
        };

        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = Ty::named("usize");
            local.is_nullable = false;
        }
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Len(place),
            },
        });
        Some(Operand::Copy(Place::new(temp)))
    }
    pub(crate) fn lower_place_expr(&mut self, expr: ExprNode, span: Option<Span>) -> Option<Place> {
        match expr {
            ExprNode::Identifier(name) => {
                if let Some(id) = self.lookup_name(&name) {
                    let mut place = Place::new(id);
                    self.normalise_place(&mut place);
                    Some(place)
                } else if let Some(place) = self.resolve_self_field_place(&name) {
                    Some(place)
                } else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("unknown identifier `{name}` in expression"),
                        span,
                                            });
                    None
                }
            }
            ExprNode::Member {
                base,
                member,
                null_conditional: _,
            } => {
                let mut place = self.lower_place_expr(*base, span)?;
                if let Some((type_name, symbol)) = self.property_symbol_from_place(&place, &member)
                {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "property `{type_name}.{member}` is not an assignable place"
                        ),
                        span: symbol.span.or(span),
                                            });
                    return None;
                }
                place.projection.push(ProjectionElem::FieldNamed(member));
                self.normalise_place(&mut place);
                Some(place)
            }
            ExprNode::Index {
                base,
                indices,
                null_conditional: _,
            } => {
                let mut place = self.lower_place_expr(*base, span)?;
                let index_count = indices.len();

                if let Some(base_ty) = self.place_ty(&place) {
                    let canonical_ty = Self::strip_nullable(&base_ty);
                    match self.indexable_kind(&base_ty) {
                        Some(IndexableKind::Array(rank)) => {
                            if index_count != rank {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "array type `{}` expects {rank} index expression(s) but {index_count} were supplied",
                                        canonical_ty.canonical_name()
                                    ),
                                    span,
                                                                    });
                                return None;
                            }
                        }
                        Some(IndexableKind::Vec) => {
                            if index_count != 1 {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "vector type `{}` expects exactly one index expression",
                                        canonical_ty.canonical_name()
                                    ),
                                    span,
                                                                    });
                                return None;
                            }
                        }
                        Some(IndexableKind::Span | IndexableKind::ReadOnlySpan) => {
                            if index_count != 1 {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "span type `{}` expects exactly one index expression",
                                        canonical_ty.canonical_name()
                                    ),
                                    span,
                                                                    });
                                return None;
                            }
                        }
                        None => {
                            if !matches!(canonical_ty, Ty::Unknown) {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "type `{}` does not support indexing operations",
                                        canonical_ty.canonical_name()
                                    ),
                                    span,
                                                                    });
                                return None;
                            }
                        }
                    }
                }

                for index_expr in indices {
                    let index_local = match index_expr {
                        ExprNode::IndexFromEnd(from_end) => {
                            self.lower_from_end_index(&place, *from_end.expr, span)?
                        }
                        ExprNode::Range(range) => {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: "range indices produce slices; use a single range when indexing"
                                    .to_string(),
                                span: range.span.or(span),
                                                            });
                            return None;
                        }
                        other => {
                            let index_operand = self.lower_expr_node(other, span)?;
                            self.ensure_operand_local(index_operand, span)
                        }
                    };
                    place.projection.push(ProjectionElem::Index(index_local));
                }
                self.normalise_place(&mut place);
                Some(place)
            }
            ExprNode::Unary {
                op: UnOp::Deref,
                expr,
                ..
            } => {
                if self.unsafe_depth == 0 {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "dereferencing a pointer requires an `unsafe` block".into(),
                        span,
                    });
                }
                let operand = self.lower_expr_node(*expr, span)?;
                let local = self.ensure_operand_local(operand, span);
                let mut place = Place::new(local);
                place.projection.push(ProjectionElem::Deref);
                Some(place)
            }
            ExprNode::Parenthesized(inner) => self.lower_place_expr(*inner, span),
            other => {
                let repr = Self::expr_to_string(&other);
                if let Some(operand) = self.lower_expr_node(other, span) {
                    match operand {
                        Operand::Copy(mut place) | Operand::Move(mut place) => {
                            self.normalise_place(&mut place);
                            return Some(place);
                        }
                        Operand::Borrow(borrow) => {
                            let mut place = borrow.place.clone();
                            self.normalise_place(&mut place);
                            return Some(place);
                        }
                        _ => {}
                    }
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("expression `{repr}` is not an assignable place"),
                    span,
                });
                None
            }
        }
    }
}
