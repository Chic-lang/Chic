use super::*;
use crate::frontend::ast::PropertyAccessorKind;
use crate::mir::builder::symbol_index::{PropertyAccessorMetadata, PropertySymbol};

#[derive(Clone, Copy)]
enum NullCoalesceKind {
    Struct,
    Pointer,
}

#[derive(Clone)]
pub(super) struct NullablePlaceInfo {
    decl_ty: Ty,
    payload_ty: Ty,
    kind: NullCoalesceKind,
}

struct NullCoalesceBranches {
    non_null: BlockId,
    null: BlockId,
    join: BlockId,
    flag_place: Place,
    value_place: Place,
}

body_builder_impl! {
    pub(super) fn lower_null_coalesce_expr(
        &mut self,
        left: ExprNode,
        right: ExprNode,
        span: Option<Span>,
            ) -> Option<Operand> {
        let left_repr = Self::expr_to_string(&left);
        let left_operand = self.lower_expr_node(left, span)?;
        let left_local = self.ensure_operand_local(left_operand, span);
        let left_place = Place::new(left_local);

        let Some(info) = self.nullable_place_info(&left_place, &left_repr, "??", span) else {
            return None;
        };
        self.ensure_nullable_layout(&info);

        let Some(branches) = self.build_null_branch(&left_place, &info, &left_repr, span) else {
            return None;
        };

        let result_local = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(result_local.0) {
            local.ty = info.payload_ty.clone();
            local.is_nullable = matches!(info.kind, NullCoalesceKind::Pointer)
                || matches!(local.ty, Ty::Nullable(_));
        }

        self.switch_to_block(branches.non_null);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(result_local),
                value: Rvalue::Use(Operand::Copy(branches.value_place.clone())),
            },
        });
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.null);
        let mut rhs_operand = match self.lower_expr_node(right, span) {
            Some(op) => op,
            None => return None,
        };
        rhs_operand = self.coerce_operand_to_ty(rhs_operand, &info.payload_ty, false, span);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(result_local),
                value: Rvalue::Use(rhs_operand),
            },
        });
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.join);
        Some(Operand::Copy(Place::new(result_local)))
    }

    pub(super) fn lower_null_coalesce_assignment(
        &mut self,
        target: ExprNode,
        value: ExprNode,
        span: Option<Span>,
            ) -> bool {
        if self
            .target_looks_like_static_member(&target)
            .unwrap_or(false)
        {
            return false;
        }

        if let ExprNode::Member { base, member, .. } = target {
            return self.lower_property_null_coalesce_assignment(*base, member, value, span);
        }

        let repr = Self::expr_to_string(&target);
        let Some(mut place) = self.lower_place_expr(target, span) else {
            return false;
        };
        self.normalise_place(&mut place);

        let Some(info) = self.nullable_place_info(&place, &repr, "??=", span) else {
            return false;
        };
        self.ensure_nullable_layout(&info);

        let Some(branches) = self.build_null_branch(&place, &info, &repr, span) else {
            return false;
        };

        self.switch_to_block(branches.non_null);
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.null);
        let mut rhs_operand = match self.lower_expr_node(value, span) {
            Some(op) => op,
            None => return false,
        };
        rhs_operand = self.coerce_operand_to_ty(rhs_operand, &info.payload_ty, false, span);
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: branches.value_place.clone(),
                value: Rvalue::Use(rhs_operand),
            },
        });
        if matches!(info.kind, NullCoalesceKind::Struct) {
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: branches.flag_place.clone(),
                    value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Bool(true)))),
                },
            });
        }
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.join);
        true
    }
}

body_builder_impl! {
    fn lower_property_null_coalesce_assignment(
        &mut self,
        base_expr: ExprNode,
        member: String,
        value_expr: ExprNode,
        span: Option<Span>,
            ) -> bool {
        if self.member_chain_unresolved(&base_expr) {
            if let Some(owner) = self.resolve_static_owner_expr(&base_expr) {
                if let Some(symbol) = self.symbol_index.property(&owner, &member) {
                    if symbol.is_static {
                        return self.lower_static_property_null_coalesce_assignment(
                            &owner,
                            &member,
                            symbol,
                            value_expr,
                            span,
                        );
                    }
                }
            }
        }

        let base_operand = match self.lower_expr_node(base_expr, span) {
            Some(op) => op,
            None => return false,
        };

        let Some((type_name, symbol_ref)) =
            self.property_symbol_from_operand(&base_operand, &member)
        else {
            return false;
        };
        let symbol = symbol_ref.clone();

        if symbol.is_static {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "static property `{}.{}` must be accessed using the type name",
                    type_name, member
                ),
                span: symbol.span.or(span),
            });
            return false;
        }

        let Some(getter) = symbol.accessors.get(&PropertyAccessorKind::Get) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "property `{}.{}` does not provide a getter",
                    type_name, member
                ),
                span: symbol.span.or(span),
                            });
            return false;
        };

        let Some((setter_metadata, accessor_kind)) =
            self.property_setter_metadata(&symbol, &type_name, &member, span)
        else {
            return false;
        };

        if !self.validate_property_setter_context(
            accessor_kind,
            &type_name,
            &member,
            &base_operand,
            symbol.span,
            span,
                    ) {
            return false;
        }

        let mut getter_args = Vec::new();
        getter_args.push(base_operand.clone());
        let return_ty = Ty::named(symbol.ty.clone());
        let getter_value = match self.emit_property_call(
            &getter.function,
            getter_args,
            Some((return_ty, symbol.is_nullable)),
            span,
                    ) {
            Some(value) => value,
            None => return false,
        };

        let temp_local = self.ensure_operand_local(getter_value, span);
        let temp_place = Place::new(temp_local);
        let repr = format!("{}.{}", type_name, member);

        let Some(info) = self.nullable_place_info(&temp_place, &repr, "??=", span) else {
            return false;
        };
        self.ensure_nullable_layout(&info);

        let Some(branches) = self.build_null_branch(&temp_place, &info, &repr, span) else {
            return false;
        };

        self.switch_to_block(branches.non_null);
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.null);
        let mut rhs_operand = match self.lower_expr_node(value_expr, span) {
            Some(op) => op,
            None => return false,
        };
        rhs_operand = self.coerce_operand_to_ty(rhs_operand, &info.payload_ty, false, span);
        let mut setter_args = Vec::new();
        setter_args.push(base_operand);
        setter_args.push(rhs_operand);

        if self
            .emit_property_call(&setter_metadata.function, setter_args, None, span)
            .is_none()
        {
            return false;
        }
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.join);
        true
    }

    fn lower_static_property_null_coalesce_assignment(
        &mut self,
        owner: &str,
        member: &str,
        symbol: &PropertySymbol,
        value_expr: ExprNode,
        span: Option<Span>,
    ) -> bool {
        let descriptor = format!("property `{owner}.{member}`");
        if !self.check_static_visibility(
            owner,
            symbol.namespace.as_deref(),
            symbol.visibility,
            span.or(symbol.span),
            &descriptor,
        ) {
            return false;
        }
        let getter_value =
            match self.lower_static_property_value(owner, None, member, symbol, span) {
                Some(value) => value,
                None => return false,
            };
        let temp_local = self.ensure_operand_local(getter_value, span);
        let temp_place = Place::new(temp_local);
        let repr = format!("{}.{}", owner, member);

        let Some(info) = self.nullable_place_info(&temp_place, &repr, "??=", span) else {
            return false;
        };
        self.ensure_nullable_layout(&info);

        let Some(branches) = self.build_null_branch(&temp_place, &info, &repr, span) else {
            return false;
        };

        self.switch_to_block(branches.non_null);
        self.ensure_goto(branches.join, span);

        let Some((setter_metadata, accessor_kind)) =
            self.property_setter_metadata(symbol, owner, member, span)
        else {
            return false;
        };

        if !self.validate_static_property_setter_context(
            accessor_kind,
            owner,
            member,
            symbol.span,
            span,
        ) {
            return false;
        }

        self.switch_to_block(branches.null);
        let mut rhs_operand = match self.lower_expr_node(value_expr, span) {
            Some(op) => op,
            None => return false,
        };
        rhs_operand = self.coerce_operand_to_ty(rhs_operand, &info.payload_ty, false, span);

        if self
            .emit_property_call(&setter_metadata.function, vec![rhs_operand], None, span)
            .is_none()
        {
            return false;
        }
        self.ensure_goto(branches.join, span);

        self.switch_to_block(branches.join);
        true
    }
}

body_builder_impl! {
    pub(super) fn nullable_place_info(
        &mut self,
        place: &Place,
        repr: &str,
        operator: &str,
        span: Option<Span>,
            ) -> Option<NullablePlaceInfo> {
        let place_ty = match self.place_ty(place) {
            Some(ty) => ty,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("cannot determine the type of `{repr}` for `{operator}`"),
                    span,
                });
                return None;
            }
        };
        if matches!(place_ty, Ty::Unknown) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("cannot determine the type of `{repr}` for `{operator}`"),
                span,
            });
            return None;
        }
        let binding_nullable = self.place_is_nullable(place).unwrap_or(false);

        if let Ty::Nullable(inner) = &place_ty {
            return Some(NullablePlaceInfo {
                decl_ty: place_ty.clone(),
                payload_ty: (**inner).clone(),
                kind: NullCoalesceKind::Struct,
            });
        }

        if binding_nullable && matches!(place_ty, Ty::Pointer(_)) {
            return Some(NullablePlaceInfo {
                decl_ty: place_ty.clone(),
                payload_ty: place_ty,
                kind: NullCoalesceKind::Pointer,
            });
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: format!("`{repr}` must be nullable to use the `{operator}` operator"),
            span,
        });
        None
    }

    pub(super) fn ensure_nullable_layout(&mut self, info: &NullablePlaceInfo) {
        match info.kind {
            NullCoalesceKind::Struct => {
                self.ensure_ty_layout_for_ty(&info.decl_ty);
                self.ensure_ty_layout_for_ty(&info.payload_ty);
            }
            NullCoalesceKind::Pointer => {
                self.ensure_ty_layout_for_ty(&info.payload_ty);
            }
        }
    }

    fn build_null_branch(
        &mut self,
        place: &Place,
        info: &NullablePlaceInfo,
        _repr: &str,
        span: Option<Span>,
            ) -> Option<NullCoalesceBranches> {
        let (flag_place, value_place) = match info.kind {
            NullCoalesceKind::Struct => {
                let mut flag_place = place.clone();
                flag_place
                    .projection
                    .push(ProjectionElem::FieldNamed("HasValue".into()));
                self.normalise_place(&mut flag_place);

                let mut value_place = place.clone();
                value_place
                    .projection
                    .push(ProjectionElem::FieldNamed("Value".into()));
                self.normalise_place(&mut value_place);
                (flag_place, value_place)
            }
            NullCoalesceKind::Pointer => {
                let flag_local = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(flag_local.0) {
                    local.ty = Ty::named("bool");
                    local.is_nullable = false;
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(flag_local),
                        value: Rvalue::Binary {
                            op: BinOp::Ne,
                            lhs: Operand::Copy(place.clone()),
                            rhs: Operand::Const(ConstOperand::new(ConstValue::Null)),
                            rounding: None,
                        },
                    },
                });
                (Place::new(flag_local), place.clone())
            }
        };

        let null_block = self.new_block(span);
        let non_null_block = self.new_block(span);
        let join_block = self.new_block(span);

        self.set_terminator(
            span,
                        Terminator::SwitchInt {
            discr: Operand::Copy(flag_place.clone()),
            targets: vec![(0, null_block)],
            otherwise: non_null_block,
        },
    );

        Some(NullCoalesceBranches {
            non_null: non_null_block,
            null: null_block,
            join: join_block,
            flag_place,
            value_place,
        })
    }

    pub(super) fn property_setter_metadata(
        &mut self,
        symbol: &PropertySymbol,
        type_name: &str,
        member: &str,
        span: Option<Span>,
            ) -> Option<(PropertyAccessorMetadata, PropertyAccessorKind)> {
        if let Some(metadata) = symbol.accessors.get(&PropertyAccessorKind::Set) {
            return Some((metadata.clone(), PropertyAccessorKind::Set));
        }

        if let Some(metadata) = symbol.accessors.get(&PropertyAccessorKind::Init) {
            return Some((metadata.clone(), PropertyAccessorKind::Init));
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "property `{}.{}` does not provide a setter",
                type_name, member
            ),
            span: symbol.span.or(span),
                    });
        None
    }

    pub(super) fn validate_property_setter_context(
        &mut self,
        accessor_kind: PropertyAccessorKind,
        type_name: &str,
        member: &str,
        base_operand: &Operand,
        symbol_span: Option<Span>,
        span: Option<Span>,
            ) -> bool {
        if accessor_kind == PropertyAccessorKind::Init {
            if self.function_kind != FunctionKind::Constructor {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "init-only property `{}.{}` can only be assigned during construction",
                        type_name, member
                    ),
                    span: symbol_span.or(span),
                                    });
                return false;
            }
            if !self.is_self_operand(base_operand) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "init accessor on `{}.{}` must be invoked on `self`",
                        type_name, member
                    ),
                    span: symbol_span.or(span),
                                    });
                return false;
            }
        }
        true
    }
}
