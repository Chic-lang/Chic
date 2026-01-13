use super::*;
use crate::diagnostics::FileId;
use crate::mir::casts::is_builtin_primitive;

body_builder_impl! {
    pub(crate) fn resolve_overloaded_unary(
        &mut self,
        op: UnOp,
        operand: &Operand,
        span: Option<Span>,
            ) -> OperatorResolution {
        let span_has_source = span.is_some_and(|span| span.file_id != FileId::UNKNOWN);
        let error_context = if span_has_source {
            String::new()
        } else {
            format!(" (in {})", self.function_name)
        };
        if matches!(op, UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut) {
            return OperatorResolution::Skip;
        }
        let Some(operand_ty) = self.operand_type_name(operand) else {
            return OperatorResolution::Skip;
        };

        match self.operator_registry.resolve_unary(&operand_ty, op) {
            OperatorMatch::Found(overload) => OperatorResolution::Handled(overload.clone()),
            OperatorMatch::Ambiguous(candidates) => {
                let names = candidates
                    .iter()
                    .map(|candidate| candidate.function.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "ambiguous operator `{}` for operand type `{}`; candidates: {names}",
                        unary_operator_symbol(op),
                        operand_ty
                    ) + &error_context,
                    span,
                                    });
                OperatorResolution::Error
            }
            OperatorMatch::None => {
                if is_builtin_primitive(self.primitive_registry, &operand_ty) {
                    return OperatorResolution::Skip;
                }
                if self.try_require_numeric_trait_unary(op, &operand_ty, span) {
                    return OperatorResolution::Skip;
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "operator `{}` is not defined for operand type `{}`",
                        unary_operator_symbol(op),
                        operand_ty
                    ) + &error_context,
                    span,
                                    });
                OperatorResolution::Error
            }
        }
    }
    pub(crate) fn resolve_overloaded_binary(
        &mut self,
        op: BinOp,
        lhs: &Operand,
        rhs: &Operand,
        span: Option<Span>,
            ) -> OperatorResolution {
        let span_has_source = span.is_some_and(|span| span.file_id != FileId::UNKNOWN);
        let error_context = if span_has_source {
            String::new()
        } else {
            format!(" (in {})", self.function_name)
        };
        let Some(lhs_ty) = self.operand_type_name(lhs) else {
            return OperatorResolution::Skip;
        };
        let Some(rhs_ty) = self.operand_type_name(rhs) else {
            return OperatorResolution::Skip;
        };

        if matches!(op, BinOp::Eq | BinOp::Ne)
            && lhs_ty == rhs_ty
            && self.lookup_enum_layout(&lhs_ty).is_some()
        {
            return OperatorResolution::Skip;
        }
        if matches!(op, BinOp::Eq | BinOp::Ne)
            && lhs_ty.starts_with("fn ")
            && lhs_ty == rhs_ty
        {
            // Raw function pointers compare by address; treat them as built-in
            // equality candidates so nullable hook slots can be probed without
            // user-defined overloads.
            return OperatorResolution::Skip;
        }
        if matches!(op, BinOp::Eq | BinOp::Ne) && lhs_ty == rhs_ty {
            let stripped = lhs_ty.strip_suffix('?').unwrap_or(&lhs_ty);
            let candidate = if self.type_layouts.types.contains_key(stripped) {
                Some(stripped.to_string())
            } else {
                self.lookup_layout_candidate(stripped)
            };
            if let Some(candidate) = candidate {
                if matches!(
                    self.type_layouts.types.get(&candidate),
                    Some(TypeLayout::Class(_))
                ) {
                    return OperatorResolution::Skip;
                }
            }
        }

        match self
            .operator_registry
            .resolve_binary(&lhs_ty, &rhs_ty, op)
        {
            OperatorMatch::Found(overload) => OperatorResolution::Handled(overload.clone()),
            OperatorMatch::Ambiguous(candidates) => {
                let names = candidates
                    .iter()
                    .map(|candidate| candidate.function.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "ambiguous operator `{}` for operand types `{}` and `{}`; candidates: {names}",
                        binary_operator_symbol(op),
                        lhs_ty,
                        rhs_ty
                    ) + &error_context,
                    span,
                                    });
                OperatorResolution::Error
            }
            OperatorMatch::None => {
                if is_builtin_primitive(self.primitive_registry, &lhs_ty)
                    && is_builtin_primitive(self.primitive_registry, &rhs_ty)
                {
                    return OperatorResolution::Skip;
                }
                if self.try_require_numeric_trait(op, &lhs_ty, &rhs_ty, span) {
                    return OperatorResolution::Skip;
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "operator `{}` is not defined for operand types `{}` and `{}`",
                        binary_operator_symbol(op),
                        lhs_ty,
                        rhs_ty
                    ) + &error_context,
                    span,
                                    });
                OperatorResolution::Error
            }
        }
    }
    pub(crate) fn emit_operator_call(
        &mut self,
        overload: OperatorOverload,
        args: Vec<Operand>,
        span: Option<Span>,
            ) -> Option<Operand> {
        let destination = if overload.result == "void" {
            None
        } else {
            let temp = self.create_temp(span);
            self.locals[temp.0].ty = Ty::named(overload.result.clone());
            Some(Place::new(temp))
        };

        let func_operand = Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: overload.function.clone(),
            span,
                        info: None,
        });

        let continue_block = self.new_block(span);
        let destination_clone = destination.clone();
        let arg_modes = vec![ParamMode::Value; args.len()];
        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: func_operand,
                args,
                arg_modes,
                destination,
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);

        match destination_clone {
            Some(place) => Some(Operand::Copy(place)),
            None => Some(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        }
    }
    pub(crate) fn emit_property_call(
        &mut self,
        function_name: &str,
        args: Vec<Operand>,
        return_ty: Option<(Ty, bool)>,
        span: Option<Span>,
            ) -> Option<Operand> {
        let destination = if let Some((ty, is_nullable)) = return_ty {
            let temp = self.create_temp(span);
            self.locals[temp.0].ty = ty;
            self.locals[temp.0].is_nullable = is_nullable;
            Some(Place::new(temp))
        } else {
            None
        };

        let func_operand = Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: function_name.to_string(),
            span,
                        info: None,
        });

        let continue_block = self.new_block(span);
        let destination_clone = destination.clone();
        let arg_modes = vec![ParamMode::Value; args.len()];
        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: func_operand,
                args,
                arg_modes,
                destination,
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);

        match destination_clone {
            Some(place) => Some(Operand::Copy(place)),
            None => Some(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        }
    }

    fn try_require_numeric_trait(
        &mut self,
        op: BinOp,
        lhs_ty: &str,
        rhs_ty: &str,
        span: Option<Span>,
    ) -> bool {
        let trait_name = match op {
            BinOp::Add => Some("Std::Numeric::IAdditionOperators"),
            BinOp::Sub => Some("Std::Numeric::ISubtractionOperators"),
            BinOp::Mul => Some("Std::Numeric::IMultiplyOperators"),
            BinOp::Div => Some("Std::Numeric::IDivisionOperators"),
            BinOp::Rem => Some("Std::Numeric::IModulusOperators"),
            BinOp::Eq | BinOp::Ne => Some("Std::Numeric::IEqualityOperators"),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                Some("Std::Numeric::IComparisonOperators")
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                Some("Std::Numeric::IBitwiseOperators")
            }
            BinOp::Shl | BinOp::Shr => Some("Std::Numeric::IShiftOperators"),
            _ => None,
        };
        let Some(trait_name) = trait_name else {
            return false;
        };
        let lhs_is_builtin = is_builtin_primitive(self.primitive_registry, lhs_ty);
        let rhs_is_builtin = is_builtin_primitive(self.primitive_registry, rhs_ty);
        let lhs_is_type_param = self.generic_param_index.contains_key(lhs_ty);
        let rhs_is_type_param = self.generic_param_index.contains_key(rhs_ty);

        if lhs_is_builtin && rhs_is_builtin {
            return false;
        }

        let requires_trait = lhs_ty == rhs_ty || !lhs_is_builtin;
        if requires_trait {
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::RequiresTrait {
                    function: self.function_name.clone(),
                    ty: lhs_ty.to_string(),
                    trait_name: trait_name.to_string(),
                },
                span,
            ));
            // For generic type parameters we defer operator support to the
            // specialization phase (which substitutes concrete operand types).
            if lhs_is_type_param && rhs_is_type_param && lhs_ty == rhs_ty {
                return true;
            }
            return false;
        }

        false
    }

    fn try_require_numeric_trait_unary(
        &mut self,
        op: UnOp,
        operand_ty: &str,
        span: Option<Span>,
    ) -> bool {
        let trait_name = match op {
            UnOp::Neg => Some("Std::Numeric::IUnaryNegationOperators"),
            UnOp::UnaryPlus => Some("Std::Numeric::IUnaryPlusOperators"),
            UnOp::Increment => Some("Std::Numeric::IIncrementOperators"),
            UnOp::Decrement => Some("Std::Numeric::IDecrementOperators"),
            UnOp::BitNot => Some("Std::Numeric::IBitwiseOperators"),
            UnOp::Not => None,
            UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut => None,
        };
        let Some(trait_name) = trait_name else {
            return false;
        };
        self.constraints.push(TypeConstraint::new(
            ConstraintKind::RequiresTrait {
                function: self.function_name.clone(),
                ty: operand_ty.to_string(),
                trait_name: trait_name.to_string(),
            },
            span,
        ));
        self.generic_param_index.contains_key(operand_ty)
    }
}
