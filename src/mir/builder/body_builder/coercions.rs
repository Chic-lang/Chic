use super::*;
use crate::diagnostics::FileId;
use crate::drop_glue::drop_type_identity;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::AggregateKind;
use crate::mir::PointerTy;
use crate::mir::RefTy;
use crate::mir::data::{ConstOperand, ConstValue, PendingOperandInfo};
use crate::mir::layout::AutoTraitSet;
use crate::mir::operators::ConversionResolution;
use std::collections::HashMap;

body_builder_impl! {
    pub(super) fn coerce_operand_to_ty(
        &mut self,
        operand: Operand,
        target: &Ty,
        allow_explicit: bool,
        span: Option<Span>,
            ) -> Operand {
        if let Ty::Nullable(inner) = target {
            if let Operand::Const(constant) = &operand {
                if matches!(constant.value(), ConstValue::Null) {
                    return operand;
                }
            }

            if let Some(operand_ty) = self.operand_ty(&operand)
                && operand_ty == *target
            {
                return operand;
            }

            let payload_operand =
                self.coerce_operand_to_ty(operand, inner.as_ref(), allow_explicit, span);

            let has_value = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));
            let nullable_ty = Ty::Nullable(Box::new(inner.as_ref().clone()));
            self.ensure_ty_layout_for_ty(&nullable_ty);
            self.ensure_ty_layout_for_ty(inner.as_ref());
            let type_name = format!("{}?", inner.as_ref().canonical_name());

            let temp = self.create_temp(span);
            if let Some(local) = self.locals.get_mut(temp.0) {
                local.ty = nullable_ty.clone();
                local.is_nullable = true;
            }

            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value: Rvalue::Aggregate {
                        kind: AggregateKind::Adt {
                            name: type_name,
                            variant: None,
                        },
                        fields: vec![has_value, payload_operand],
                    },
                },
            });

            return Operand::Copy(Place::new(temp));
        }

        if let Ty::Named(named) = target {
            match self.primitive_registry.kind_for_name(named.as_str()) {
                Some(crate::primitives::PrimitiveKind::String) => {
                    return self.coerce_operand_to_ty(operand, &Ty::String, allow_explicit, span);
                }
                Some(crate::primitives::PrimitiveKind::Str) => {
                    return self.coerce_operand_to_ty(operand, &Ty::Str, allow_explicit, span);
                }
                _ => {}
            }
        }
        if let Operand::Pending(pending) = &operand {
            if pending.repr == "default" {
                return self.lower_default_for_target(target.clone(), span);
            }
        }
        if let Ty::Pointer(expected_ptr) = target {
            return self.coerce_pointer_operand(operand, expected_ptr, allow_explicit, span);
        }
        match target {
            Ty::Named(_) | Ty::String | Ty::Str => {
                if let Some((delegate_name, delegate_sig)) = self.delegate_signature_for_ty(target)
                {
                    return self.coerce_operand_to_delegate(
                        operand,
                        &delegate_name,
                        &delegate_sig,
                        span,
                        Some(target.clone()),
                    );
                }
                if let Some(name) = self.resolve_ty_name(target) {
                    self.coerce_operand_to_type_name(operand, &name, allow_explicit, span)
                } else {
                    operand
                }
            }
            Ty::Span(_) | Ty::ReadOnlySpan(_) => self
                .try_span_conversion(operand.clone(), target, allow_explicit, span)
                .unwrap_or(operand),
            Ty::Fn(expected) => self.coerce_operand_to_fn(operand, expected, span),
            Ty::Ref(reference) => self.coerce_operand_to_ref(operand, reference, span),
            _ => operand,
        }
    }

    pub(super) fn coerce_operand_to_place(
        &mut self,
        operand: Operand,
        place: &Place,
        allow_explicit: bool,
        span: Option<Span>,
            ) -> Operand {
        if let Some(target_ty) = self.place_ty(place) {
            if !matches!(target_ty, Ty::Unknown) {
                return self.coerce_operand_to_ty(operand, &target_ty, allow_explicit, span);
            }
        }

        if let Some(name) = self.place_type_name(place) {
            return self.coerce_operand_to_type_name(operand, &name, allow_explicit, span);
        }
        operand
    }

    pub(super) fn coerce_operand_to_type_name(
        &mut self,
        operand: Operand,
        target_name: &str,
        allow_explicit: bool,
        span: Option<Span>,
            ) -> Operand {
        if let Operand::Const(constant) = &operand {
            if matches!(constant.value(), ConstValue::Null) {
                let target_kind = self.primitive_registry.kind_for_name(target_name);
                if Self::type_name_is_nullable(target_name)
                    || target_name == "string"
                    || matches!(target_kind, Some(crate::primitives::PrimitiveKind::String))
                {
                    return operand;
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`null` cannot be assigned to non-nullable type `{target_name}`"
                    ),
                    span,
                });
                return operand;
            }
        }

        let operand_ty = self.operand_ty(&operand);
        let target_ty = parse_type_expression_text(target_name)
            .as_ref()
            .map(Ty::from_type_expr);

        if let (Some(src_ty), Some(dst_ty)) = (operand_ty.as_ref(), target_ty.as_ref()) {
            if self.span_conversion_pair_exists(src_ty, dst_ty) {
                if let Some(converted) =
                    self.try_span_conversion(operand.clone(), dst_ty, allow_explicit, span)
                {
                    return converted;
                }
                return operand;
            }
            if self.is_class_upcast(src_ty, dst_ty) {
                return operand;
            }
        }

        let Some(source_ty) = self.operand_type_name(&operand) else {
            return operand;
        };

        if source_ty.len() == 1 && source_ty.chars().all(|c| c.is_ascii_uppercase()) {
            return operand;
        }

        let target_base = crate::mir::casts::short_type_name(target_name)
            .split('<')
            .next()
            .unwrap_or(target_name);
        let source_base = crate::mir::casts::short_type_name(&source_ty)
            .split('<')
            .next()
            .unwrap_or(&source_ty);
        if source_base.ends_with("StderrHandle") && target_base.ends_with("StdoutHandle") {
            return operand;
        }
        if matches!(target_base, "ValueMutPtr" | "ValueConstPtr")
            && (source_ty.starts_with("*mut")
                || source_ty.starts_with("*const")
                || source_ty.contains("@expose_address"))
        {
            return operand;
        }

        if crate::mir::casts::is_builtin_primitive(self.primitive_registry, target_name)
            && !crate::mir::casts::is_builtin_primitive(self.primitive_registry, &source_ty)
        {
            return operand;
        }

        let source_base = source_ty
            .strip_prefix("ref readonly ")
            .or_else(|| source_ty.strip_prefix("ref "))
            .unwrap_or(source_ty.as_str());
        let source_kind = self.primitive_registry.kind_for_name(source_base);
        let target_kind = self.primitive_registry.kind_for_name(target_name);

        if matches!(target_kind, Some(crate::primitives::PrimitiveKind::Str)) {
            if matches!(source_kind, Some(crate::primitives::PrimitiveKind::Str)) {
                return operand;
            }
            if matches!(source_kind, Some(crate::primitives::PrimitiveKind::String)) {
                let temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(temp.0) {
                    local.ty = Ty::Str;
                    local.is_nullable = false;
                }
                let place = Place::new(temp);
                let continue_block = self.new_block(span);
                let unwind_target = self.current_unwind_target();
                self.set_terminator(
                    span,
                    Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "chic_rt_string_as_slice".to_string(),
                        ))),
                        args: vec![operand],
                        arg_modes: vec![ParamMode::Value],
                        destination: Some(place.clone()),
                        target: continue_block,
                        unwind: unwind_target,
                        dispatch: None,
                    },
                );
                self.switch_to_block(continue_block);
                return Operand::Copy(place);
            }
        }

        if matches!(target_kind, Some(crate::primitives::PrimitiveKind::String))
            && matches!(source_kind, Some(crate::primitives::PrimitiveKind::Str))
        {
            let temp = self.create_temp(span);
            if let Some(local) = self.locals.get_mut(temp.0) {
                local.ty = Ty::String;
                local.is_nullable = false;
            }
            let place = Place::new(temp);
            let continue_block = self.new_block(span);
            let unwind_target = self.current_unwind_target();
            self.set_terminator(
                span,
                Terminator::Call {
                    func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                        "chic_rt_string_from_slice".to_string(),
                    ))),
                    args: vec![operand],
                    arg_modes: vec![ParamMode::Value],
                    destination: Some(place.clone()),
                    target: continue_block,
                    unwind: unwind_target,
                    dispatch: None,
                },
            );
            self.switch_to_block(continue_block);
            return Operand::Copy(place);
        }

        if types_equivalent(&source_ty, target_name) {
            return operand;
        }

        if crate::mir::casts::is_builtin_primitive(self.primitive_registry, &source_ty)
            && crate::mir::casts::is_builtin_primitive(self.primitive_registry, target_name)
        {
            return operand;
        }

        if target_base.ends_with("ValueConstPtr") && source_base.ends_with("ValueMutPtr") {
            return operand;
        }

        match self
            .operator_registry
            .resolve_conversion(&source_ty, target_name, allow_explicit)
        {
            ConversionResolution::Found(overload) => self
                .emit_operator_call(overload.clone(), vec![operand.clone()], span)
                .unwrap_or(operand),
            ConversionResolution::Ambiguous(candidates) => {
                let names = candidates
                    .iter()
                    .map(|candidate| candidate.function.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "ambiguous conversion from `{source_ty}` to `{target_name}`; candidates: {names}"
                    ),
                    span,
                                    });
                operand
            }
            ConversionResolution::None { explicit_candidates } => {
                if !allow_explicit && !explicit_candidates.is_empty() {
                    let names = explicit_candidates
                        .iter()
                        .map(|candidate| candidate.function.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "explicit conversion from `{source_ty}` to `{target_name}` requires a cast (candidates: {names})"
                        ),
                        span,
                                            });
                } else {
                    let qualifier = if allow_explicit { "" } else { "implicit " };
                    let has_source_span = span
                        .as_ref()
                        .is_some_and(|span| span.file_id != FileId::UNKNOWN);
                    let context = has_source_span
                        .then(String::new)
                        .unwrap_or_else(|| format!(" (in `{}`)", self.function_name));
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "no {qualifier}conversion from `{source_ty}` to `{target_name}` is defined{context}"
                        ),
                        span,
                                            });
                }
                operand
            }
        }
    }

    fn coerce_operand_to_fn(
        &mut self,
        operand: Operand,
        expected: &FnTy,
        span: Option<Span>,
            ) -> Operand {
        match operand {
            Operand::Pending(mut pending) => {
                if let Some(info) = pending.info.take() {
                    let PendingOperandInfo::FunctionGroup {
                        path,
                        candidates,
                        receiver,
                    } = *info;
                    let debug = std::env::var("CHIC_DEBUG_FN_COERCE").is_ok();
                    if debug {
                        eprintln!(
                            "[coerce-fn] target={} candidates={} receiver={}",
                            path,
                            candidates.len(),
                            receiver.is_some()
                        );
                    }
                    let matching = candidates
                        .iter()
                        .filter(|candidate| candidate.signature == *expected)
                        .collect::<Vec<_>>();
                    if matching.len() == 1 {
                        if matches!(expected.abi, Abi::Extern(_)) {
                            if receiver.is_some() {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "cannot coerce method group `{path}` to `{}`; C ABI function pointers must reference a free `@extern(\"C\")` function",
                                        expected.canonical_name()
                                    ),
                                    span: span.or(pending.span),
                                });
                                pending.info = Some(
                                    PendingOperandInfo::FunctionGroup {
                                        path,
                                        candidates,
                                        receiver,
                                    }
                                    .into(),
                                );
                                return Operand::Pending(pending);
                            }
                            return Operand::Const(ConstOperand::new(ConstValue::Symbol(
                                matching[0].qualified.clone(),
                            )));
                        }

                        let context_operand = receiver
                            .as_ref()
                            .map(|op| *op.clone())
                            .unwrap_or_else(|| {
                                Operand::Const(ConstOperand::new(ConstValue::Null))
                            });
                        let context_ty = receiver
                            .as_ref()
                            .and_then(|op| self.operand_ty(op.as_ref()))
                            .or_else(|| {
                                Some(Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))))
                            });
                        let adapter = self.build_plain_fn_adapter(
                            expected,
                            &matching[0].qualified,
                            span.or(pending.span),
                            context_ty,
                            receiver.is_some() && !matching[0].is_static,
                        );
                        let type_id = Operand::Const(ConstOperand::new(ConstValue::UInt(
                            drop_type_identity(&expected.canonical_name()).into(),
                        )));
                        return self.build_fn_pointer_value(
                            expected,
                            adapter,
                            context_operand,
                            Operand::Const(ConstOperand::new(ConstValue::Null)),
                            type_id,
                            Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                            span.or(pending.span),
                        );
                    }

                    let error_span = span.or(pending.span);
                    let expected_name = expected.canonical_name();
                    if matching.is_empty() {
                        let available = candidates
                            .iter()
                            .map(|candidate| candidate.signature.canonical_name())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let availability = if available.is_empty() {
                            String::from("no overloads are registered")
                        } else {
                            format!("available signatures: {available}")
                        };
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "function `{path}` does not have an overload matching `{expected_name}` ({availability})"
                            ),
                            span: error_span,
                        });
                    } else {
                        let names = matching
                            .iter()
                            .map(|candidate| candidate.qualified.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "function `{path}` has multiple overloads matching `{expected_name}`; candidates: {names}"
                            ),
                            span: error_span,
                        });
                    }

                    pending.info = Some(
                        PendingOperandInfo::FunctionGroup {
                            path,
                            candidates,
                            receiver,
                        }
                        .into(),
                    );
                }
                Operand::Pending(pending)
            }
            other => {
                if let Operand::Const(constant) = &other
                    && let ConstValue::Symbol(symbol) = constant.value()
                {
                    if matches!(expected.abi, Abi::Extern(_)) {
                        return other;
                    }
                    let adapter =
                        self.build_plain_fn_adapter(expected, symbol, span, None, false);
                    let type_id = Operand::Const(ConstOperand::new(ConstValue::UInt(
                        drop_type_identity(&expected.canonical_name()).into(),
                    )));
                    return self.build_fn_pointer_value(
                        expected,
                        adapter,
                        Operand::Const(ConstOperand::new(ConstValue::Null)),
                        Operand::Const(ConstOperand::new(ConstValue::Null)),
                        type_id,
                        Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                        Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                        span,
                    );
                }

                if let Some(result) =
                    self.try_closure_to_fn_pointer(&other, expected, span)
                {
                    return match result {
                        Ok(converted) => converted,
                        Err(_) => other,
                    };
                }
        if let Some(source_ty) = self.operand_ty(&other)
            && let Some((_, source_sig)) = self.delegate_signature_for_ty(&source_ty)
            && self.fn_signature_equivalent(&source_sig, expected)
        {
            if matches!(expected.abi, Abi::Extern(_)) {
                if let Ty::Fn(fn_ty) = &source_ty {
                    if matches!(fn_ty.abi, Abi::Extern(_)) {
                        return other;
                    }
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot convert delegate `{}` to C ABI function pointer `{}`",
                        source_ty.canonical_name(),
                        expected.canonical_name()
                            ),
                            span,
                        });
                        return other;
                    }
                    if let Operand::Copy(place) | Operand::Move(place) = &other {
                        let mut base_place = place.clone();
                        self.normalise_place(&mut base_place);
                        let load_field = |index: u32| {
                            let mut p = base_place.clone();
                            p.projection.push(ProjectionElem::Field(index));
                            Operand::Copy(p)
                        };
                        let temp = self.create_temp(span);
                        if let Some(local) = self.locals.get_mut(temp.0) {
                            local.ty = Ty::Fn(expected.clone());
                            local.is_nullable = false;
                        }
                        self.ensure_ty_layout_for_ty(&Ty::Fn(expected.clone()));
                        let value = Rvalue::Aggregate {
                            kind: AggregateKind::Adt {
                                name: expected.canonical_name(),
                                variant: None,
                            },
                            fields: vec![
                                load_field(0),
                                load_field(1),
                                load_field(2),
                                load_field(3),
                                load_field(4),
                                load_field(5),
                            ],
                        };
                        self.push_statement(MirStatement {
                            span,
                            kind: MirStatementKind::Assign {
                                place: Place::new(temp),
                                value,
                            },
                        });
                        return Operand::Copy(Place::new(temp));
                    }
                }
                let Some(source_ty) = self.operand_type_name(&other) else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "cannot determine function pointer type for operand (use `.to_fn_ptr()` on closures with captures)".into(),
                    span,
                                    });
                return other;
            };

            let expected_name = expected.canonical_name();
            if source_ty == expected_name {
                    return other;
                }

                eprintln!(
                    "[coerce_fn] fn={} source={source_ty} expected={expected_name} span={:?} operand={:?}",
                    self.function_name,
                    span,
                    other
                );
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "function pointer of type `{source_ty}` cannot be used where `{expected_name}` is required; for closures with captures, call `.to_fn_ptr()` explicitly"
                    ),
                    span,
                                    });
                other
            }
        }
    }

    pub(super) fn coerce_operand_to_delegate(
        &mut self,
        operand: Operand,
        delegate_name: &str,
        signature: &FnTy,
        span: Option<Span>,
        delegate_ty: Option<Ty>,
    ) -> Operand {
        let Some(type_name) = self.operand_type_name(&operand) else {
            return operand;
        };
        let parsed_source_ty =
            parse_type_expression_text(&type_name).map(|expr| Ty::from_type_expr(&expr));
        let normalize = |name: &str| {
            parse_type_expression_text(name)
                .map(|expr| Ty::from_type_expr(&expr).canonical_name())
                .unwrap_or_else(|| name.to_string())
        };
        let names_equivalent = |a: &str, b: &str| normalize(a) == normalize(b);
        if names_equivalent(&type_name, delegate_name) {
            return operand;
        }

        let source_delegate = parsed_source_ty
            .as_ref()
            .and_then(|ty| self.delegate_signature_for_ty(ty));
        let delegate_display = delegate_ty
            .as_ref()
            .and_then(|ty| self.resolve_ty_name(ty))
            .unwrap_or_else(|| delegate_name.to_string());
        if let Some((source_name, source_sig)) = source_delegate {
            if source_sig.params.len() != signature.params.len()
                || source_sig.param_modes.len() != signature.param_modes.len()
                || source_sig.param_modes != signature.param_modes
            {
                if std::env::var("CHIC_DEBUG_DELEGATE_SIG").is_ok() {
                    eprintln!(
                        "[delegate-coerce] source params/modes: {}/{} target: {}/{}",
                        source_sig.params.len(),
                        source_sig.param_modes.len(),
                        signature.params.len(),
                        signature.param_modes.len()
                    );
                }
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot convert `{type_name}` to delegate `{delegate_display}`; signatures are incompatible"
                    ),
                    span,
                });
                return operand;
            }
            if let Operand::Copy(place) | Operand::Move(place) = &operand {
                let mut base_place = place.clone();
                self.normalise_place(&mut base_place);
                let load_field = |index: u32| {
                    let mut p = base_place.clone();
                    p.projection.push(ProjectionElem::Field(index));
                    Operand::Copy(p)
                };

                let temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(temp.0) {
                    local.ty = delegate_ty.unwrap_or_else(|| Ty::named(delegate_name.to_string()));
                    local.is_nullable = false;
                }
                self.type_layouts.ensure_delegate_layout(delegate_name);
                if let Some(traits) = self
                    .type_layouts
                    .delegate_auto_traits_for_key(&source_name)
                {
                    self.type_layouts
                        .record_delegate_auto_traits(delegate_name.to_string(), traits);
                }
                let value = Rvalue::Aggregate {
                    kind: AggregateKind::Adt {
                        name: delegate_name.to_string(),
                        variant: None,
                    },
                    fields: vec![
                        load_field(0),
                        load_field(1),
                        load_field(2),
                        load_field(3),
                        load_field(4),
                        load_field(5),
                    ],
                };
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value,
                    },
                });
                return Operand::Copy(Place::new(temp));
            }
        }

        if let Some(info) = self.closure_registry.get(&type_name).cloned() {
            let ty_matches =
                |actual: &Ty, expected: &Ty| matches!(actual, Ty::Unknown) || actual == expected;
            if info.fn_ty.params.len() == signature.params.len()
                && info.fn_ty.param_modes.len() == signature.param_modes.len()
                && info
                    .fn_ty
                    .param_modes
                    .iter()
                    .zip(signature.param_modes.iter())
                    .all(|(a, e)| a == e)
                && info
                    .fn_ty
                    .params
                    .iter()
                    .zip(signature.params.iter())
                    .all(|(a, e)| ty_matches(a, e))
                && ty_matches(&info.fn_ty.ret, &signature.ret)
                && info.fn_ty.abi == signature.abi
            {
                self.specialise_closure_signature(&type_name, &info.invoke_symbol, signature);
                let updated_info = self
                    .closure_registry
                    .get(&type_name)
                    .cloned()
                    .unwrap_or(info);
                if let Some(converted) =
                    self.convert_closure_operand_to_delegate(operand.clone(), &updated_info, delegate_name, signature, span)
                {
                    return converted;
                }
            }
        }

        match operand {
            Operand::Pending(mut pending) => {
                if let Some(info) = pending.info.take() {
                    let PendingOperandInfo::FunctionGroup {
                        path,
                        candidates,
                        receiver,
                    } = *info;
                    let debug = std::env::var("CHIC_DEBUG_FN_COERCE").is_ok();
                    if debug {
                        eprintln!(
                            "[coerce-delegate] target={} candidates={} receiver={}",
                            path,
                            candidates.len(),
                            receiver.is_some()
                        );
                    }
                    let matching = candidates
                        .iter()
                        .filter(|candidate| candidate.signature == *signature)
                        .collect::<Vec<_>>();
                    if matching.len() == 1 {
                        let context_operand = receiver
                            .as_ref()
                            .map(|op| *op.clone())
                            .unwrap_or_else(|| {
                                Operand::Const(ConstOperand::new(ConstValue::Null))
                            });
                        let context_ty = receiver
                            .as_ref()
                            .and_then(|op| self.operand_ty(op.as_ref()))
                            .or_else(|| {
                                Some(Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))))
                            });
                        let adapter = self.build_plain_fn_adapter(
                            signature,
                            &matching[0].qualified,
                            span.or(pending.span),
                            context_ty,
                            receiver.is_some() && !matching[0].is_static,
                        );
                        let type_id = Operand::Const(ConstOperand::new(ConstValue::UInt(
                            drop_type_identity(delegate_name).into(),
                        )));
                        return self.build_delegate_value(
                            delegate_name,
                            adapter,
                            context_operand,
                            Operand::Const(ConstOperand::new(ConstValue::Null)),
                            type_id,
                            Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                            Some(AutoTraitSet::all_yes()),
                            span.or(pending.span),
                            delegate_ty.clone(),
                        );
                    }

                    let error_span = span.or(pending.span);
                    if matching.is_empty() {
                        let available = candidates
                            .iter()
                            .map(|candidate| candidate.signature.canonical_name())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let availability = if available.is_empty() {
                            String::from("no overloads are registered")
                        } else {
                            format!("available signatures: {available}")
                        };
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "function `{path}` does not have an overload matching delegate `{delegate_name}` ({availability})"
                            ),
                            span: error_span,
                        });
                    } else {
                        let names = matching
                            .iter()
                            .map(|candidate| candidate.qualified.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "function `{path}` has multiple overloads matching delegate `{delegate_name}`; candidates: {names}"
                            ),
                            span: error_span,
                        });
                    }
                    pending.info = Some(
                        PendingOperandInfo::FunctionGroup {
                            path,
                            candidates,
                            receiver,
                        }
                        .into(),
                    );
                }
                Operand::Pending(pending)
            }
            other => {
                if let Operand::Const(constant) = &other
                    && let ConstValue::Symbol(symbol) = constant.value()
                {
                    let adapter =
                        self.build_plain_fn_adapter(signature, symbol, span, None, false);
                    let type_id = Operand::Const(ConstOperand::new(ConstValue::UInt(
                        drop_type_identity(delegate_name).into(),
                    )));
                    return self.build_delegate_value(
                        delegate_name,
                        adapter,
                        Operand::Const(ConstOperand::new(ConstValue::Null)),
                        Operand::Const(ConstOperand::new(ConstValue::Null)),
                        type_id,
                        Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                        Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                        Some(AutoTraitSet::all_yes()),
                        span,
                        delegate_ty.clone(),
                    );
                }

                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot convert `{type_name}` to delegate `{delegate_display}`"
                    ),
                    span,
                });
                other
            }
        }
    }

    fn type_name_is_nullable(name: &str) -> bool {
        name.trim().ends_with('?')
    }

    pub(super) fn coerce_pointer_operand(
        &mut self,
        operand: Operand,
        expected: &PointerTy,
        allow_explicit: bool,
        span: Option<Span>,
    ) -> Operand {
        if let Operand::Const(constant) = &operand {
            if matches!(constant.value(), ConstValue::Null) {
                return operand;
            }
        }

        let operand_ty = self.operand_ty(&operand);
        let operand_ptr = match operand_ty.as_ref() {
            Some(Ty::Pointer(ptr)) => Some(ptr.clone()),
            Some(Ty::Nullable(inner)) => match inner.as_ref() {
                Ty::Pointer(ptr) => Some(ptr.clone()),
                _ => None,
            },
            _ => None,
        };
        let Some(source_ptr) = operand_ptr else {
            return operand;
        };

        let expected_elem = expected.element.canonical_name();
        let source_elem = source_ptr.element.canonical_name();

        if !expected.mutable
            && source_ptr.mutable
            && expected_elem == source_elem
            && expected.qualifiers == source_ptr.qualifiers
        {
            return self.retag_pointer_operand(operand, Ty::Pointer(Box::new(expected.clone())), span);
        }

        let expected_is_void = matches!(expected.element, Ty::Unit);
        let source_is_void = matches!(source_ptr.element, Ty::Unit);

        if expected_is_void && !source_is_void {
            if self.ffi_pointer_context {
                return self.retag_pointer_operand(
                    operand,
                    Ty::Pointer(Box::new(expected.clone())),
                    span,
                );
            }
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "implicit pointer cast from `{source_elem}` to `void*` is only allowed in `@extern(\"C\")` contexts"
                ),
                span,
            });
            return operand;
        }

        if source_is_void && !expected_is_void {
            if !allow_explicit {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot implicitly convert `void*` to `{}`; use an explicit cast inside an `unsafe` block",
                        Ty::Pointer(Box::new(expected.clone())).canonical_name()
                    ),
                    span,
                });
                return operand;
            }
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "casting `void*` to `{}` requires an `unsafe` block",
                        Ty::Pointer(Box::new(expected.clone())).canonical_name()
                    ),
                    span,
                });
            }
            return self.retag_pointer_operand(
                operand,
                Ty::Pointer(Box::new(expected.clone())),
                span,
            );
        }

        if allow_explicit {
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "pointer cast from `{}` to `{}` requires an `unsafe` block",
                        Ty::Pointer(source_ptr.clone()).canonical_name(),
                        Ty::Pointer(Box::new(expected.clone())).canonical_name()
                    ),
                    span,
                });
            }
            return self.retag_pointer_operand(
                operand,
                Ty::Pointer(Box::new(expected.clone())),
                span,
            );
        }

        operand
    }

    fn retag_pointer_operand(
        &mut self,
        operand: Operand,
        target: Ty,
        span: Option<Span>,
    ) -> Operand {
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = target.clone();
            local.is_nullable = matches!(target, Ty::Nullable(_));
        }
        self.ensure_ty_layout_for_ty(&target);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Use(operand),
            },
        });
        Operand::Copy(Place::new(temp))
    }

    pub(super) fn delegate_signature_for_ty(&self, ty: &Ty) -> Option<(String, FnTy)> {
        if let Ty::Fn(fn_ty) = ty {
            if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                return None;
            }
            return Some((fn_ty.canonical_name(), fn_ty.clone()));
        }
        let mut base_ty = ty;
        if let Ty::Nullable(inner) = ty {
            base_ty = inner;
        }
        if std::env::var("CHIC_DEBUG_DELEGATE_SIG").map(|v| !v.is_empty()).unwrap_or(false) {
            eprintln!("[delegate-sig] query ty={}", base_ty.canonical_name());
        }
        let canonical_name = base_ty.canonical_name();
        let lookup_name = if let Ty::Named(named) = base_ty {
            if !named.args.is_empty() {
                self.resolve_ty_name(&Ty::named(named.name.clone()))
                    .or_else(|| self.lookup_layout_candidate(named.name.as_str()))
                    .unwrap_or_else(|| named.name.clone())
            } else {
                self.resolve_ty_name(base_ty).unwrap_or_else(|| named.name.clone())
            }
        } else {
            self.resolve_ty_name(base_ty)?
        };
        if std::env::var("CHIC_DEBUG_DELEGATE_SIG").is_ok()
            && canonical_name.contains("Converter")
        {
            eprintln!("[delegate-sig] target={canonical_name} lookup={lookup_name}");
        }
        let instantiate = |sig: &FnTy| self.instantiate_delegate_signature_from_ty(base_ty, sig);
        if let Some(sig) = self.symbol_index.delegate_signature(&lookup_name) {
            let instantiated = instantiate(sig);
            if std::env::var("CHIC_DEBUG_DELEGATE_SIG").is_ok()
                && canonical_name.contains("Converter")
            {
                eprintln!(
                    "[delegate-sig] resolved {canonical_name} via symbol index params={} modes={}",
                    instantiated.params.len(),
                    instantiated.param_modes.len()
                );
            }
            return Some((canonical_name.clone(), instantiated));
        }
        if let Some(sig) = self.type_layouts.delegate_signature(&lookup_name) {
            let instantiated = instantiate(sig);
            if std::env::var("CHIC_DEBUG_DELEGATE_SIG").is_ok()
                && canonical_name.contains("Converter")
            {
                eprintln!(
                    "[delegate-sig] resolved {canonical_name} via layouts params={} modes={}",
                    instantiated.params.len(),
                    instantiated.param_modes.len()
                );
            }
            return Some((canonical_name.clone(), instantiated));
        }
        None
    }

    pub(super) fn instantiate_delegate_signature_from_ty(
        &self,
        ty: &Ty,
        sig: &FnTy,
    ) -> FnTy {
        use crate::mir::GenericArg;

        let Ty::Named(named) = ty else {
            return sig.clone();
        };
        if named.args.is_empty() {
            return sig.clone();
        }
        let base_name = self
            .resolve_ty_name(&Ty::named(named.name.clone()))
            .or_else(|| self.lookup_layout_candidate(named.name.as_str()))
            .or_else(|| self.resolve_ty_name(ty));
        let Some(base_name) = base_name else {
            return sig.clone();
        };
        let Some(params) = self.symbol_index.type_generics(&base_name) else {
            return sig.clone();
        };
        if params.len() != named.args.len() {
            return sig.clone();
        }
        let mut map = HashMap::new();
        for (param, arg) in params.iter().zip(named.args.iter()) {
            if let GenericArg::Type(arg_ty) = arg {
                map.insert(param.name.clone(), arg_ty.clone());
            }
        }
        FnTy {
            params: sig
                .params
                .iter()
                .map(|p| Self::substitute_generics(p, &map))
                .collect(),
            param_modes: sig.param_modes.clone(),
            ret: Box::new(Self::substitute_generics(&sig.ret, &map)),
            abi: sig.abi.clone(),
            variadic: sig.variadic,
        }
    }
}

fn types_equivalent(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let left_short = crate::mir::casts::short_type_name(left);
    let right_short = crate::mir::casts::short_type_name(right);
    if left_short == right_short {
        return true;
    }
    let left_base = left_short.split('<').next().unwrap_or(left_short);
    let right_base = right_short.split('<').next().unwrap_or(right_short);
    if (left_base.ends_with("ValueMutPtr") && right_base.ends_with("ValueConstPtr"))
        || (left_base.ends_with("ValueConstPtr") && right_base.ends_with("ValueMutPtr"))
    {
        return true;
    }
    left_base == right_base
}

impl<'a> BodyBuilder<'a> {
    fn fn_signature_equivalent(&self, actual: &FnTy, expected: &FnTy) -> bool {
        if actual.abi != expected.abi {
            return false;
        }
        if actual.params.len() != expected.params.len() {
            return false;
        }
        if actual.param_modes != expected.param_modes {
            return false;
        }
        for (a, e) in actual.params.iter().zip(expected.params.iter()) {
            if !types_equivalent(&a.canonical_name(), &e.canonical_name()) {
                return false;
            }
        }
        types_equivalent(&actual.ret.canonical_name(), &expected.ret.canonical_name())
    }

    fn coerce_operand_to_ref(
        &mut self,
        operand: Operand,
        target: &RefTy,
        span: Option<Span>,
    ) -> Operand {
        match operand {
            Operand::Borrow(borrow) => {
                self.validate_borrow_for_ref(&borrow, target, span);
                Operand::Borrow(borrow)
            }
            Operand::Copy(place) => {
                self.validate_ref_place(&place, target, span);
                Operand::Copy(place)
            }
            Operand::Move(place) => {
                self.validate_ref_place(&place, target, span);
                Operand::Move(place)
            }
            Operand::Const(_) | Operand::Pending(_) | Operand::Mmio(_) => {
                let source = self
                    .operand_type_name(&operand)
                    .unwrap_or_else(|| "<unknown>".into());
                let has_source_span = span
                    .as_ref()
                    .is_some_and(|span| span.file_id != FileId::UNKNOWN);
                let context = has_source_span
                    .then(String::new)
                    .unwrap_or_else(|| format!(" (in `{}`)", self.function_name));
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "type `{source}` cannot be assigned to `{}`; only `ref` values may be stored{context}",
                        format_ref_type(target),
                    ),
                    span,
                });
                operand
            }
        }
    }

    fn validate_borrow_for_ref(
        &mut self,
        borrow: &BorrowOperand,
        target: &RefTy,
        span: Option<Span>,
    ) {
        match borrow.kind {
            BorrowKind::Raw => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "raw borrows cannot be assigned to `{}`; take a `ref` or `ref readonly` value instead",
                        format_ref_type(target)
                    ),
                    span,
                });
                return;
            }
            BorrowKind::Shared => {
                if !target.readonly {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "cannot assign `{}` to `{}`; mutable `ref` values require unique borrows",
                            self.format_borrow_label(borrow),
                            format_ref_type(target)
                        ),
                        span,
                    });
                }
            }
            BorrowKind::Unique => {}
        }

        if let Some(source_ty) = self.place_ty(&borrow.place) {
            if !ref_referent_matches(&source_ty, target) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cannot assign `{}` to `{}`; referent type `{}` does not match `{}`",
                        self.format_borrow_label(borrow),
                        format_ref_type(target),
                        source_ty.canonical_name(),
                        target.element.canonical_name()
                    ),
                    span,
                });
            }
        }
    }

    fn validate_ref_place(&mut self, place: &Place, target: &RefTy, span: Option<Span>) {
        let Some(source_ty) = self.place_ty(place) else {
            return;
        };
        match source_ty {
            Ty::Ref(inner) => {
                let inner = inner.as_ref();
                if inner.readonly && !target.readonly {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "cannot assign `{}` to `{}`; mutable `ref` values require unique borrows",
                            format_ref_type(&inner),
                            format_ref_type(target)
                        ),
                        span,
                    });
                }
                if !ref_referent_matches(&inner.element, target) {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "cannot assign `{}` to `{}`; referent type `{}` does not match `{}`",
                            format_ref_type(&inner),
                            format_ref_type(target),
                            inner.element.canonical_name(),
                            target.element.canonical_name()
                        ),
                        span,
                    });
                }
            }
            _ => {
                let has_source_span = span
                    .as_ref()
                    .is_some_and(|span| span.file_id != FileId::UNKNOWN);
                let context = has_source_span
                    .then(String::new)
                    .unwrap_or_else(|| format!(" (in `{}`)", self.function_name));
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "type `{}` cannot be assigned to `{}`; only `ref` values may be stored{context}",
                        source_ty.canonical_name(),
                        format_ref_type(target),
                    ),
                    span,
                });
            }
        }
    }
}

impl<'a> BodyBuilder<'a> {
    fn lower_default_for_target(&mut self, target: Ty, span: Option<Span>) -> Operand {
        if matches!(target, Ty::Unknown) {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot lower `default` without a known target type; add a type annotation or use `default(T)`".into(),
                span,
            });
            return Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: "default".into(),
                span,
                info: None,
            });
        }
        self.ensure_ty_layout_for_ty(&target);
        self.zero_init_operand(target, span)
    }

    fn format_borrow_label(&self, borrow: &BorrowOperand) -> String {
        let prefix = match borrow.kind {
            BorrowKind::Shared => "ref readonly",
            BorrowKind::Unique => "ref",
            BorrowKind::Raw => "raw ref",
        };
        let referent = self
            .place_type_name(&borrow.place)
            .or_else(|| self.place_ty(&borrow.place).map(|ty| ty.canonical_name()))
            .unwrap_or_else(|| "<unknown>".into());
        format!("{prefix} {referent}")
    }
}

fn ref_referent_matches(source: &Ty, target: &RefTy) -> bool {
    source.canonical_name() == target.element.canonical_name()
}

fn format_ref_type(reference: &RefTy) -> String {
    let prefix = if reference.readonly {
        "ref readonly"
    } else {
        "ref"
    };
    format!("{prefix} {}", reference.element.canonical_name())
}
