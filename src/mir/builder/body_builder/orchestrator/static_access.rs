use super::super::*;
use crate::mir::{PendingFunctionCandidate, PendingOperandInfo, StaticId, StaticVar};
use std::collections::HashSet;

#[derive(Clone)]
pub(crate) enum StaticUsingCandidate {
    Field {
        owner: String,
        symbol: FieldSymbol,
    },
    Property {
        owner: String,
        symbol: PropertySymbol,
    },
    Const {
        owner: String,
        symbol: ConstSymbol,
    },
}

impl StaticUsingCandidate {
    fn owner(&self) -> &str {
        match self {
            StaticUsingCandidate::Field { owner, .. }
            | StaticUsingCandidate::Property { owner, .. }
            | StaticUsingCandidate::Const { owner, .. } => owner,
        }
    }
}

body_builder_impl! {
    #[allow(dead_code)]
    pub(crate) fn visibility_keyword(visibility: Visibility) -> &'static str {
        match visibility {
            Visibility::Public => "public",
            Visibility::Internal => "internal",
            Visibility::Protected => "protected",
            Visibility::Private => "private",
            Visibility::ProtectedInternal => "protected internal",
            Visibility::PrivateProtected => "private protected",
        }
    }

    pub(crate) fn namespace_root(path: Option<&str>) -> Option<&str> {
        path.and_then(|ns| ns.split("::").find(|segment| !segment.is_empty()))
    }

    pub(crate) fn resolve_static_owner_expr(&self, expr: &ExprNode) -> Option<String> {
        let segments = collect_path_segments(expr)?;
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some() {
            if let Some(first) = segments.first() {
                if first == "Vec" || first == "VecIntrinsics" {
                    eprintln!("[wasm-enum-debug] resolve_static_owner_expr segments {:?}", segments);
                }
            }
        }
        self.resolve_static_type_name(&segments)
    }

    pub(crate) fn is_within_type(&self, owner: &str) -> bool {
        self.current_self_type_name()
            .as_deref()
            .map(|ty| ty == owner)
            .unwrap_or(false)
    }

    pub(crate) fn current_namespace_root(&self) -> Option<String> {
        let scope = match self.function_kind {
            FunctionKind::Method | FunctionKind::Constructor => {
                self.current_self_type_name()
                    .and_then(|ty| ty.rsplit_once("::").map(|(ns, _)| ns.to_string()))
            }
            _ => self.namespace.clone(),
        };
        scope.and_then(|value| Self::namespace_root(Some(value.as_str())).map(|root| root.to_string()))
    }

    pub(crate) fn resolve_static_type_name(&self, segments: &[String]) -> Option<String> {
        if segments.is_empty() {
            return None;
        }
        if segments[0].eq_ignore_ascii_case("self") {
            let mut base = self.current_self_type_name()?;
            for segment in &segments[1..] {
                base.push_str("::");
                base.push_str(segment);
            }
            return Some(base);
        }
        if segments.len() == 1 {
            match segments[0].as_str() {
                "Vec" => return Some("Foundation::Collections::Vec".to_string()),
                "TimeZones" => return Some("Std::Datetime::TimeZones".to_string()),
                "Arc" => return Some("Std::Sync::Arc".to_string()),
                "Rc" => return Some("Std::Sync::Rc".to_string()),
                _ => {}
            }
        }
        let candidate = segments.join("::");
        let current_type = self.current_self_type_name();
        resolve_type_layout_name(
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
            &candidate,
        )
        .or_else(|| self.symbol_index.contains_type(&candidate).then_some(candidate.clone()))
        .or_else(|| {
            if segments.len() != 1 {
                return None;
            }
            let suffix = segments.last().cloned().unwrap_or_default();
            let mut matched: Option<String> = None;
            for ty in self.symbol_index.types() {
                if ty.rsplit("::").next().is_some_and(|seg| seg == suffix) {
                    matched.get_or_insert_with(|| ty.clone());
                }
            }
            matched
        })
    }

    #[allow(dead_code)]
    pub(crate) fn inherits_from(&self, derived: &str, base: &str) -> bool {
        if derived == base {
            return true;
        }
        let mut stack = vec![derived.to_string()];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(bases) = self.class_bases.get(&current) {
                for candidate in bases {
                    if candidate == base {
                        return true;
                    }
                    stack.push(candidate.clone());
                }
            }
        }
        false
    }

    pub(crate) fn namespaces_match(&self, member_namespace: Option<&str>) -> bool {
        let current_root = self.current_namespace_root();
        let member_root = Self::namespace_root(member_namespace).map(str::to_string);
        match (current_root.as_ref(), member_root.as_ref()) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }

    pub(crate) fn check_static_visibility(
        &mut self,
        owner: &str,
        member_namespace: Option<&str>,
        visibility: Visibility,
        span: Option<Span>,
        descriptor: &str,
    ) -> bool {
        let cleaned_namespace = member_namespace
            .filter(|ns| !ns.is_empty())
            .map(|ns| ns.replace('.', "::"));
        let namespace = self
            .owner_namespace(owner, cleaned_namespace.as_deref())
            .map(|ns| ns.to_string());
        let owner_package = self.owner_package(owner).map(|pkg| pkg.to_string());
        self.member_accessible(
            visibility,
            owner,
            owner_package.as_deref(),
            namespace.as_deref(),
            None,
            false,
            span,
            descriptor,
        )
    }

    pub(crate) fn pending_static_operand(&self, type_name: &str, member: &str, span: Option<Span>) -> Operand {
        Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: format!("{type_name}.{member}"),
            span,
            info: None,
        })
    }

    pub(crate) fn lower_static_member_operand(
        &mut self,
        base: &ExprNode,
        member: &str,
        span: Option<Span>,
    ) -> Option<Operand> {
        let owner_expr = Self::expr_to_string(base);
        let owner = self.resolve_static_owner_expr(base)?;
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some() && member == "New" {
            eprintln!(
                "[wasm-enum-debug] static member <base>.{} resolved owner {}",
                member, owner
            );
        }
        if let Some(symbol) = self.symbol_index.property(&owner, member) {
            if symbol.is_static {
                return self.lower_static_property_value(&owner, Some(&owner_expr), member, symbol, span);
            }
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("property `{owner}.{member}` is not static"),
                span: symbol.span.or(span),
            });
            return Some(self.pending_static_operand(&owner, member, span));
        }
        if let Some(field) = self.symbol_index.field_symbol(&owner, member) {
            return self.lower_static_field_value(&owner, member, field, span);
        }
        if let Some(symbol) = self.symbol_index.type_const(&owner, member) {
            let value = self.const_symbol_value(symbol, span)?;
            let value = self.normalise_const(value, span);
            return Some(Operand::Const(ConstOperand::new(value)));
        }
        let qualified = format!("{owner}::{member}");
        if let Some(overloads) = self.symbol_index.function_overloads(&qualified) {
            let mut static_overloads = Vec::new();
            for symbol in overloads {
                if symbol.is_static {
                    static_overloads.push(symbol.clone());
                }
            }
            let selected = if !static_overloads.is_empty() {
                static_overloads
            } else {
                overloads.to_vec()
            };
            if !selected.is_empty() {
                if selected.len() == 1 {
                    return Some(Operand::Const(ConstOperand::new(ConstValue::Symbol(
                        selected[0].internal_name.clone(),
                    ))));
                }
                let candidates = selected
                    .iter()
                    .map(|symbol| PendingFunctionCandidate {
                        qualified: symbol.qualified.clone(),
                        signature: symbol.signature.clone(),
                        is_static: symbol.is_static,
                    })
                    .collect::<Vec<_>>();
                return Some(Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                    repr: qualified.clone(),
                    span,
                    info: Some(Box::new(PendingOperandInfo::FunctionGroup {
                        path: qualified,
                        candidates,
                        receiver: None,
                    })),
                }));
            }
        }
        None
    }

    pub(crate) fn try_static_assignment(
        &mut self,
        target: &ExprNode,
        op: AssignOp,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        let ExprNode::Member { base, member, .. } = target else {
            return None;
        };
        let owner = match self.resolve_static_owner_expr(base) {
            Some(owner) => owner,
            None => {
                if self.member_chain_unresolved(base) {
                    return Some(false);
                }
                return None;
            }
        };
        if let Some(symbol) = self.symbol_index.property(&owner, member) {
            return self.lower_static_property_assignment(&owner, member, symbol, op, value_expr, span);
        }
        if let Some(field) = self.symbol_index.field_symbol(&owner, member) {
            if op != AssignOp::Assign {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "compound assignment on field `{owner}.{member}` is not supported"
                    ),
                    span,
                });
                return Some(false);
            }
            let value_operand = match self.lower_expr_node(value_expr, span) {
                Some(operand) => operand,
                None => return Some(false),
            };
            return Some(self.emit_static_store(&owner, member, field, value_operand, span));
        }
        None
    }

    pub(crate) fn try_same_type_static_assignment(
        &mut self,
        name: &str,
        op: AssignOp,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        let owner = self.current_self_type_name()?;
        if let Some(symbol) = self.symbol_index.property(&owner, name) {
            if !symbol.is_static {
                return None;
            }
            return self.lower_static_property_assignment(&owner, name, symbol, op, value_expr, span);
        }
        if let Some(field) = self.symbol_index.field_symbol(&owner, name) {
            if !field.is_static {
                return None;
            }
            if op != AssignOp::Assign {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "compound assignment on field `{owner}.{name}` is not supported"
                    ),
                    span,
                });
                return Some(false);
            }
            let value_operand = match self.lower_expr_node(value_expr, span) {
                Some(operand) => operand,
                None => return Some(false),
            };
            return Some(self.emit_static_store(&owner, name, field, value_operand, span));
        }
        None
    }

    pub(crate) fn lower_namespace_static_identifier(
        &mut self,
        name: &str,
        span: Option<Span>,
    ) -> Option<Operand> {
        let Some((id, var)) =
            self.static_registry
                .lookup_in_namespace(self.namespace.as_deref(), name)
        else {
            return None;
        };
        self.lower_namespace_static_value(id, var, span)
    }

    pub(crate) fn lower_namespace_static_address(
        &mut self,
        name: &str,
        mutable: bool,
        span: Option<Span>,
    ) -> Option<Operand> {
        let Some((id, var)) =
            self.static_registry
                .lookup_in_namespace(self.namespace.as_deref(), name)
        else {
            return None;
        };
        self.lower_namespace_static_address_value(id, var, mutable, span)
    }

    pub(crate) fn lower_namespace_static_path(
        &mut self,
        base: &ExprNode,
        member: &str,
        span: Option<Span>,
    ) -> Option<Operand> {
        let Some(mut segments) = collect_path_segments(base) else {
            return None;
        };
        segments.push(member.to_string());
        let qualified = segments.join("::");
        let Some((id, var)) = self.static_registry.lookup_qualified(&qualified) else {
            return None;
        };
        self.lower_namespace_static_value(id, var, span)
    }

    pub(crate) fn lower_namespace_static_address_path(
        &mut self,
        base: &ExprNode,
        member: &str,
        mutable: bool,
        span: Option<Span>,
    ) -> Option<Operand> {
        let Some(mut segments) = collect_path_segments(base) else {
            return None;
        };
        segments.push(member.to_string());
        let qualified = segments.join("::");
        let Some((id, var)) = self.static_registry.lookup_qualified(&qualified) else {
            return None;
        };
        self.lower_namespace_static_address_value(id, var, mutable, span)
    }

    fn lower_namespace_static_value(
        &mut self,
        id: StaticId,
        var: &StaticVar,
        span: Option<Span>,
    ) -> Option<Operand> {
        if !self.check_namespace_static_visibility(var, span) {
            return Some(self.pending_namespace_static_operand(var, span));
        }
        if !var.is_readonly && self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "accessing mutable static `{}` requires an `unsafe` block",
                    var.qualified
                ),
                span: var.span.or(span),
            });
        }
        Some(self.emit_static_load(id, var, span))
    }

    fn lower_namespace_static_address_value(
        &mut self,
        id: StaticId,
        var: &StaticVar,
        mutable: bool,
        span: Option<Span>,
    ) -> Option<Operand> {
        if !self.check_namespace_static_visibility(var, span) {
            return Some(self.pending_namespace_static_operand(var, span));
        }
        if mutable && var.is_readonly {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static `{}` is immutable and cannot be taken as `&mut`",
                    var.qualified
                ),
                span: var.span.or(span),
            });
            return Some(self.pending_namespace_static_operand(var, span));
        }
        if !var.is_readonly && self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "taking the address of mutable static `{}` requires an `unsafe` block",
                    var.qualified
                ),
                span: var.span.or(span),
            });
        }
        self.ensure_ty_layout_for_ty(&var.ty);
        let temp = self.create_temp(span);
        let pointer_ty = Ty::Pointer(Box::new(crate::mir::PointerTy::new(
            var.ty.clone(),
            mutable,
        )));
        self.hint_local_ty(temp, pointer_ty);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.is_nullable = var.is_weak_import;
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::StaticRef { id },
            },
        });
        Some(Operand::Copy(Place::new(temp)))
    }

    fn emit_static_load(&mut self, id: StaticId, var: &StaticVar, span: Option<Span>) -> Operand {
        self.ensure_ty_layout_for_ty(&var.ty);
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = var.ty.clone();
            local.is_nullable = matches!(var.ty, Ty::Nullable(_));
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::StaticLoad { id },
            },
        });
        Operand::Copy(Place::new(temp))
    }

    pub(crate) fn try_namespace_static_assignment(
        &mut self,
        name: &str,
        op: AssignOp,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        let Some((id, var)) = self
            .static_registry
            .lookup_in_namespace(self.namespace.as_deref(), name)
        else {
            return None;
        };

        if op != AssignOp::Assign {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "compound assignment on static `{}` is not supported",
                    var.qualified
                ),
                span,
            });
            return Some(false);
        }
        if var.is_readonly {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("static `{}` is immutable", var.qualified),
                span: var.span.or(span),
            });
            return Some(false);
        }
        if self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "assigning to mutable static `{}` requires an `unsafe` block",
                    var.qualified
                ),
                span: var.span.or(span),
            });
        }

        let value_operand = match self.lower_expr_node(value_expr, span) {
            Some(operand) => operand,
            None => return Some(false),
        };
        let coerced = self.coerce_operand_to_ty(value_operand, &var.ty, false, span);
        let value_local = self.ensure_operand_local(coerced, span);
        let store_operand = Operand::Copy(Place::new(value_local));
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::StaticStore { id, value: store_operand },
        });
        Some(true)
    }

    fn check_namespace_static_visibility(
        &mut self,
        var: &StaticVar,
        span: Option<Span>,
    ) -> bool {
        match var.visibility {
            Visibility::Public => true,
            Visibility::Internal => self.namespaces_match(var.namespace.as_deref()),
            Visibility::Private => self.namespace == var.namespace,
            _ => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "static `{}` uses unsupported visibility {:?}",
                        var.qualified, var.visibility
                    ),
                    span: var.span.or(span),
                });
                false
            }
        }
    }

    fn pending_namespace_static_operand(
        &self,
        var: &StaticVar,
        span: Option<Span>,
    ) -> Operand {
        Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: var.qualified.clone(),
            span,
            info: None,
        })
    }

    pub(crate) fn lower_static_field_value(
        &mut self,
        owner: &str,
        member: &str,
        symbol: &FieldSymbol,
        span: Option<Span>,
    ) -> Option<Operand> {
        if !symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("field `{owner}.{member}` is not static"),
                span: symbol.span.or(span),
            });
            return Some(self.pending_static_operand(owner, member, span));
        }
        let descriptor = format!("field `{owner}.{member}`");
        if !self.check_static_visibility(
            owner,
            symbol.namespace.as_deref(),
            symbol.visibility,
            span.or(symbol.span),
            &descriptor,
        ) {
            return Some(self.pending_static_operand(owner, member, span));
        }
        let Some((id, var)) = self.static_registry.lookup(owner, member) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static field `{owner}.{member}` is not registered in this module"
                ),
                span,
            });
            return Some(self.pending_static_operand(owner, member, span));
        };
        self.ensure_ty_layout_for_ty(&var.ty);
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = var.ty.clone();
            local.is_nullable = matches!(var.ty, Ty::Nullable(_));
        }
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::StaticLoad { id },
            },
        });
        Some(Operand::Copy(Place::new(temp)))
    }

    pub(crate) fn emit_static_store(
        &mut self,
        owner: &str,
        member: &str,
        symbol: &FieldSymbol,
        mut value_operand: Operand,
        span: Option<Span>,
    ) -> bool {
        if !symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("field `{owner}.{member}` is not static"),
                span: symbol.span.or(span),
            });
            return false;
        }
        if symbol.is_readonly && !self.is_within_type(owner) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "field `{owner}.{member}` is readonly and may only be assigned within `{owner}`"
                ),
                span: symbol.span.or(span),
            });
            return false;
        }
        let descriptor = format!("field `{owner}.{member}`");
        if !self.check_static_visibility(
            owner,
            symbol.namespace.as_deref(),
            symbol.visibility,
            span.or(symbol.span),
            &descriptor,
        ) {
            return false;
        }
        let Some((id, var)) = self.static_registry.lookup(owner, member) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static field `{owner}.{member}` is not registered in this module"
                ),
                span,
            });
            return false;
        };
        value_operand = self.coerce_operand_to_ty(value_operand, &var.ty, false, span);
        let value_local = self.ensure_operand_local(value_operand, span);
        let store_operand = Operand::Copy(Place::new(value_local));
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::StaticStore { id, value: store_operand },
        });
        true
    }

    pub(crate) fn lower_static_property_value(
        &mut self,
        owner: &str,
        owner_expr: Option<&str>,
        member: &str,
        symbol: &PropertySymbol,
        span: Option<Span>,
    ) -> Option<Operand> {
        if !symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("property `{owner}.{member}` is not static"),
                span: symbol.span.or(span),
            });
            return Some(self.pending_static_operand(owner, member, span));
        }
        let descriptor = format!("property `{owner}.{member}`");
        if !self.check_static_visibility(
            owner,
            symbol.namespace.as_deref(),
            symbol.visibility,
            span.or(symbol.span),
            &descriptor,
        ) {
            return Some(self.pending_static_operand(owner, member, span));
        }
        let Some(getter) = symbol.accessors.get(&PropertyAccessorKind::Get) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "property `{owner}.{member}` does not provide a getter"
                ),
                span: symbol.span.or(span),
            });
            return Some(self.pending_static_operand(owner, member, span));
        };
        let return_ty = parse_type_expression_text(symbol.ty.as_str())
            .map(|expr| Ty::from_type_expr(&expr))
            .unwrap_or_else(|| Ty::named(symbol.ty.clone()));
        let return_ty = owner_expr
            .and_then(|owner| {
                let instantiated = self.instantiate_member_type_from_owner_name(owner, &return_ty);
                (!matches!(instantiated, Ty::Unknown)).then_some(instantiated)
            })
            .unwrap_or(return_ty);
        self.emit_property_call(
            &getter.function,
            Vec::new(),
            Some((return_ty, symbol.is_nullable)),
            span,
        )
    }

    pub(crate) fn validate_static_property_setter_context(
        &mut self,
        accessor_kind: PropertyAccessorKind,
        owner: &str,
        member: &str,
        symbol_span: Option<Span>,
        span: Option<Span>,
    ) -> bool {
        if accessor_kind == PropertyAccessorKind::Init {
            if self.function_kind != FunctionKind::Constructor || !self.is_within_type(owner) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "init-only property `{owner}.{member}` can only be assigned during construction of `{owner}`"
                    ),
                    span: symbol_span.or(span),
                });
                return false;
            }
        }
        true
    }

    pub(crate) fn lower_static_property_assignment(
        &mut self,
        owner: &str,
        member: &str,
        symbol: &PropertySymbol,
        op: AssignOp,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> Option<bool> {
        if !symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("property `{owner}.{member}` is not static"),
                span: symbol.span.or(span),
            });
            return Some(false);
        }
        if op != AssignOp::Assign {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "compound assignment on property `{owner}.{member}` is not supported"
                ),
                span,
            });
            return Some(false);
        }
        let descriptor = format!("property `{owner}.{member}`");
        if !self.check_static_visibility(
            owner,
            symbol.namespace.as_deref(),
            symbol.visibility,
            span.or(symbol.span),
            &descriptor,
        ) {
            return Some(false);
        }
        let Some((metadata, accessor_kind)) =
            self.property_setter_metadata(symbol, owner, member, span)
        else {
            return Some(false);
        };
        if !self.validate_static_property_setter_context(
            accessor_kind,
            owner,
            member,
            symbol.span,
            span,
        ) {
            return Some(false);
        }
        let value_operand = match self.lower_expr_node(value_expr, span) {
            Some(operand) => operand,
            None => return Some(false),
        };
        let value_ty = Ty::named(symbol.ty.clone());
        let coerced = self.coerce_operand_to_ty(value_operand, &value_ty, false, span);
        if self
            .emit_property_call(&metadata.function, vec![coerced], None, span)
            .is_none()
        {
            return Some(false);
        }
        Some(true)
    }

    pub(crate) fn collect_static_imports(
        import_resolver: &'a ImportResolver,
        namespace: Option<&str>,
    ) -> Vec<String> {
        let scope = import_resolver.combined_scope(namespace);
        let mut seen = HashSet::new();
        let mut imports = Vec::new();
        for segments in scope.static_imports {
            if segments.is_empty() {
                continue;
            }
            let canonical = segments.join("::");
            if seen.insert(canonical.clone()) {
                imports.push(canonical);
            }
        }
        imports
    }

    pub(crate) fn lower_static_using_identifier(&mut self, name: &str, span: Option<Span>) -> Option<Operand> {
        if self.static_import_types.is_empty() {
            return None;
        }

        let mut matches = Vec::new();
        for owner in &self.static_import_types {
            if let Some(field) = self.symbol_index.field_symbol(owner, name) {
                if field.is_static {
                    matches.push(StaticUsingCandidate::Field {
                        owner: owner.clone(),
                        symbol: field.clone(),
                    });
                }
            }
            if let Some(property) = self.symbol_index.property(owner, name) {
                if property.is_static {
                    matches.push(StaticUsingCandidate::Property {
                        owner: owner.clone(),
                        symbol: property.clone(),
                    });
                }
            }
            if let Some(constant) = self.symbol_index.type_const(owner, name) {
                matches.push(StaticUsingCandidate::Const {
                    owner: owner.clone(),
                    symbol: constant.clone(),
                });
            }
        }

        if matches.is_empty() {
            return None;
        }
        if matches.len() > 1 {
            self.report_static_using_ambiguity(name, &matches, span);
            return Some(self.pending_static_operand(matches[0].owner(), name, span));
        }

        match matches.remove(0) {
            StaticUsingCandidate::Field { owner, symbol } => {
                self.lower_static_field_value(owner.as_str(), name, &symbol, span)
            }
            StaticUsingCandidate::Property { owner, symbol } => {
                self.lower_static_property_value(owner.as_str(), None, name, &symbol, span)
            }
            StaticUsingCandidate::Const { owner, symbol } => {
                if let Some(value) = self.const_symbol_value(&symbol, span) {
                    let value = self.normalise_const(value, span);
                    Some(Operand::Const(ConstOperand::new(value)))
                } else {
                    Some(self.pending_static_operand(owner.as_str(), name, span))
                }
            }
        }
    }

    pub(crate) fn report_static_using_ambiguity(
        &mut self,
        name: &str,
        candidates: &[StaticUsingCandidate],
        span: Option<Span>,
    ) {
        let owners = candidates
            .iter()
            .map(|candidate| format!("`{}::{name}`", candidate.owner()))
            .collect::<Vec<_>>()
            .join(", ");
        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "reference to `{name}` is ambiguous between {owners}; qualify the access or remove conflicting `import static` directives"
            ),
            span,
        });
    }

    pub(crate) fn resolve_static_using_method_owner(
        &mut self,
        name: &str,
        span: Option<Span>,
    ) -> Option<String> {
        if self.static_import_types.is_empty() {
            return None;
        }

        let mut owners: Vec<String> = Vec::new();
        for owner in &self.static_import_types {
            if !self
                .symbol_index
                .static_method_overloads(owner, name)
                .is_empty()
            {
                owners.push(owner.clone());
            }
        }
        owners.sort_unstable();
        owners.dedup();
        match owners.len() {
            0 => None,
            1 => Some(owners[0].clone()),
            _ => {
                let formatted = owners
                    .iter()
                    .map(|owner| format!("`{owner}::{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "reference to `{name}` is ambiguous between {formatted}; qualify the call or remove conflicting `import static` directives"
                    ),
                    span,
                });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::Expression;
    use crate::frontend::ast::TypeExpr;
    use std::collections::HashMap;

    #[test]
    fn static_using_candidate_owner_reports_all_variants() {
        let field_symbol = FieldSymbol {
            ty: TypeExpr::simple("int"),
            visibility: Visibility::Public,
            is_static: true,
            is_readonly: false,
            is_required: false,
            span: None,
            namespace: None,
        };
        let property_symbol = PropertySymbol {
            ty: "int".into(),
            is_static: true,
            accessors: HashMap::new(),
            span: None,
            is_required: false,
            is_nullable: false,
            visibility: Visibility::Public,
            namespace: None,
        };
        let const_symbol = ConstSymbol {
            qualified: "Demo::Value".into(),
            name: "Value".into(),
            owner: Some("Demo::Constants".into()),
            namespace: Some("Demo".into()),
            ty: TypeExpr::simple("int"),
            initializer: Expression::new("1", None),
            visibility: Visibility::Public,
            modifiers: Vec::new(),
            span: None,
            value: Some(ConstValue::Int(1)),
        };

        let field = StaticUsingCandidate::Field {
            owner: "Demo::FieldOwner".into(),
            symbol: field_symbol.clone(),
        };
        let property = StaticUsingCandidate::Property {
            owner: "Demo::PropOwner".into(),
            symbol: property_symbol.clone(),
        };
        let constant = StaticUsingCandidate::Const {
            owner: "Demo::ConstOwner".into(),
            symbol: const_symbol.clone(),
        };

        assert_eq!(field.owner(), "Demo::FieldOwner");
        assert_eq!(property.owner(), "Demo::PropOwner");
        assert_eq!(constant.owner(), "Demo::ConstOwner");
    }
}
