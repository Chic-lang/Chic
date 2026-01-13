use super::*;
use crate::mir::builder::body_builder::expressions::CallBindingInfo;
use crate::mir::builder::support::type_size_and_align_for_ty;
use crate::mir::{ArrayTy, ReadOnlySpanTy, SpanTy};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpanConversionStrength {
    Implicit,
    Explicit,
}

#[derive(Clone, Debug)]
enum SpanConversionKind {
    ArrayToSpan(ArrayTy, SpanTy),
    ArrayToReadOnly(ArrayTy, ReadOnlySpanTy),
    SpanToReadOnly(SpanTy, ReadOnlySpanTy),
    ReadOnlyToReadOnly(ReadOnlySpanTy, ReadOnlySpanTy),
    StringToReadOnly(ReadOnlySpanTy),
}

#[derive(Clone, Debug)]
struct SpanConversionPlan {
    kind: SpanConversionKind,
    strength: SpanConversionStrength,
}

body_builder_impl! {
    pub(super) fn first_class_spans(&self) -> bool {
        self.first_class_spans
    }

    pub(super) fn try_span_conversion(
        &mut self,
        operand: Operand,
        target: &Ty,
        allow_explicit: bool,
        span: Option<Span>,
    ) -> Option<Operand> {
        let Some(source_ty) = self.operand_ty(&operand) else {
            return None;
        };
        let Some(plan) = self.span_conversion_plan(&source_ty, target) else {
            return None;
        };
        match plan.strength {
            SpanConversionStrength::Implicit => {}
            SpanConversionStrength::Explicit if allow_explicit => {}
            SpanConversionStrength::Explicit => return None,
        }
        self.emit_span_conversion(plan, operand, span)
    }

    pub(super) fn span_conversion_pair_exists(&self, source: &Ty, target: &Ty) -> bool {
        self.span_conversion_plan(source, target).is_some()
    }

    pub(super) fn span_conversion_strength(
        &self,
        source: &Ty,
        target: &Ty,
    ) -> Option<SpanConversionStrength> {
        self.span_conversion_plan(source, target)
            .map(|plan| plan.strength)
    }

    fn span_conversion_plan(&self, source: &Ty, target: &Ty) -> Option<SpanConversionPlan> {
        if !self.first_class_spans() {
            return None;
        }
        match (source, target) {
            (Ty::Array(array), Ty::Span(span)) if array.rank == 1 => {
                let strength = self.element_conversion_strength(&array.element, &span.element)?;
                Some(SpanConversionPlan {
                    kind: SpanConversionKind::ArrayToSpan(array.clone(), span.clone()),
                    strength,
                })
            }
            (Ty::Array(array), Ty::ReadOnlySpan(span)) if array.rank == 1 => {
                let strength = self.element_conversion_strength(&array.element, &span.element)?;
                Some(SpanConversionPlan {
                    kind: SpanConversionKind::ArrayToReadOnly(array.clone(), span.clone()),
                    strength,
                })
            }
            (Ty::Span(actual), Ty::ReadOnlySpan(expected)) => {
                self.element_conversion_strength(&actual.element, &expected.element)
                    .map(|strength| SpanConversionPlan {
                        kind: SpanConversionKind::SpanToReadOnly(actual.clone(), expected.clone()),
                        strength,
                    })
            }
            (Ty::ReadOnlySpan(actual), Ty::ReadOnlySpan(expected)) => {
                self.element_conversion_strength(&actual.element, &expected.element)
                    .map(|strength| SpanConversionPlan {
                        kind: SpanConversionKind::ReadOnlyToReadOnly(
                            actual.clone(),
                            expected.clone(),
                        ),
                        strength,
                    })
            }
            (Ty::String, Ty::ReadOnlySpan(span)) if self.string_span_target(span) => {
                Some(SpanConversionPlan {
                    kind: SpanConversionKind::StringToReadOnly(span.clone()),
                    strength: SpanConversionStrength::Implicit,
                })
            }
            _ => None,
        }
    }

    fn element_conversion_strength(
        &self,
        source: &Ty,
        target: &Ty,
    ) -> Option<SpanConversionStrength> {
        if source == target || source.canonical_name() == target.canonical_name() {
            return Some(SpanConversionStrength::Implicit);
        }
        if self.is_class_upcast(source, target) {
            return Some(SpanConversionStrength::Implicit);
        }
        if self.is_class_downcast(source, target) {
            return Some(SpanConversionStrength::Explicit);
        }
        None
    }

    fn emit_span_conversion(
        &mut self,
        plan: SpanConversionPlan,
        operand: Operand,
        span: Option<Span>,
    ) -> Option<Operand> {
        match plan.kind {
            SpanConversionKind::ArrayToSpan(source, target) => {
                let _ = &source;
                self.emit_span_helper_call(
                    "Std::Span::Span::FromArray",
                    &[target.element.as_ref().clone()],
                    vec![operand],
                    span,
                )
            }
            SpanConversionKind::ArrayToReadOnly(source, target) => {
                let _ = &source;
                self.emit_span_helper_call(
                    "Std::Span::ReadOnlySpan::FromArray",
                    &[target.element.as_ref().clone()],
                    vec![operand],
                    span,
                )
            }
            SpanConversionKind::SpanToReadOnly(source, target) => {
                let element_layout_ok =
                    self.span_element_layouts_match(&source.element, &target.element);
                if source.element == target.element {
                    self.emit_span_helper_call(
                        "Std::Span::Span::AsReadOnly",
                        &[target.element.as_ref().clone()],
                        vec![operand],
                        span,
                    )
                } else if element_layout_ok {
                    let readonly = self.emit_span_helper_call(
                        "Std::Span::Span::AsReadOnly",
                        &[source.element.as_ref().clone()],
                        vec![operand],
                        span,
                    )?;
                    self.bitcast_span_operand(readonly, Ty::ReadOnlySpan(target), span)
                } else {
                    None
                }
            }
            SpanConversionKind::ReadOnlyToReadOnly(source, target) => {
                if source.element == target.element
                    || self.span_element_layouts_match(&source.element, &target.element)
                {
                    self.bitcast_span_operand(
                        operand,
                        Ty::ReadOnlySpan(target),
                        span,
                    )
                } else {
                    None
                }
            }
            SpanConversionKind::StringToReadOnly(target) => self.emit_span_helper_call(
                "Std::Span::ReadOnlySpan::FromString",
                &[],
                vec![operand],
                span,
            )
            .and_then(|converted| {
                let byte_ty = Ty::named("byte");
                if target.element.as_ref() == &byte_ty {
                    return Some(converted);
                }
                let char_ty = Ty::named("char");
                if target.element.as_ref() == &char_ty
                    && self.span_element_layouts_match(&byte_ty, &char_ty)
                {
                    return self
                        .bitcast_span_operand(converted, Ty::ReadOnlySpan(target), span);
                }
                None
            }),
        }
    }

    fn bitcast_span_operand(&mut self, operand: Operand, target_ty: Ty, span: Option<Span>) -> Option<Operand> {
        let local = self.ensure_operand_local(operand, span);
        let temp = self.create_temp(span);
        if let Some(decl) = self.locals.get_mut(temp.0) {
            decl.ty = target_ty;
            decl.is_nullable = false;
        }
        let place = Place::new(temp);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: place.clone(),
                value: Rvalue::Use(Operand::Copy(Place::new(local))),
            },
        });
        Some(Operand::Copy(place))
    }

    fn span_element_layouts_match(&self, source: &Ty, target: &Ty) -> bool {
        let source_layout = type_size_and_align_for_ty(
            source,
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            None,
        );
        let target_layout = type_size_and_align_for_ty(
            target,
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            None,
        );
        match (source_layout, target_layout) {
            (Some((s_size, s_align)), Some((t_size, t_align))) => s_size == t_size && s_align == t_align,
            _ => false,
        }
    }

    pub(super) fn emit_span_helper_call(
        &mut self,
        function: &str,
        type_args: &[Ty],
        args: Vec<Operand>,
        span: Option<Span>,
    ) -> Option<Operand> {
        let mut symbols = self
            .symbol_index
            .function_overloads(function)
            .map(|list| list.to_vec())
            .unwrap_or_default();
        if symbols.is_empty() {
            return None;
        }
        symbols.retain(|symbol| {
            symbol.params.len() == args.len()
                || (symbol.signature.variadic && args.len() >= symbol.params.len())
        });
        let Some(symbol) = symbols.first() else {
            return None;
        };

        let call_info = CallBindingInfo {
            method_type_args: if type_args.is_empty() {
                None
            } else {
                Some(type_args.to_vec())
            },
            canonical_hint: Some(symbol.qualified.clone()),
            ..CallBindingInfo::default()
        };
        let (_, inst_ret, _) = self
            .instantiate_symbol_signature(symbol, &call_info)
            .unwrap_or_else(|| {
                (
                    symbol.signature.params.clone(),
                    (*symbol.signature.ret).clone(),
                    symbol.signature.variadic,
                )
            });

        let call_name = if self.should_specialize_call(&symbol.qualified, type_args) {
            let specialized = specialised_function_name(&symbol.internal_name, type_args);
            self.record_generic_specialization(
                &symbol.internal_name,
                &specialized,
                type_args.to_vec(),
            );
            specialized
        } else {
            symbol.internal_name.clone()
        };

        let mut prepared_args = Vec::new();
        let mut arg_modes = Vec::new();
        for (operand, param) in args.into_iter().zip(symbol.params.iter()) {
            let adjusted = self.adjust_operand_for_mode(operand, param.mode, span);
            prepared_args.push(adjusted);
            arg_modes.push(param.mode);
        }

        let destination = if inst_ret == Ty::Unit {
            None
        } else {
            let temp = self.create_temp(span);
            if let Some(decl) = self.locals.get_mut(temp.0) {
                decl.ty = inst_ret.clone();
                decl.is_nullable = matches!(inst_ret, Ty::Nullable(_));
            }
            Some(Place::new(temp))
        };
        let continue_block = self.new_block(span);
        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: Operand::Const(ConstOperand::new(ConstValue::Symbol(call_name))),
                args: prepared_args,
                arg_modes,
                destination: destination.clone(),
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);
        destination.map(Operand::Copy).or_else(|| {
            Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
        })
    }

    fn adjust_operand_for_mode(
        &mut self,
        operand: Operand,
        mode: ParamMode,
        span: Option<Span>,
    ) -> Operand {
        match mode {
            ParamMode::In => self.ensure_borrow_operand(operand, BorrowKind::Shared, span),
            ParamMode::Ref | ParamMode::Out => {
                self.ensure_borrow_operand(operand, BorrowKind::Unique, span)
            }
            ParamMode::Value => operand,
        }
    }

    fn ensure_borrow_operand(
        &mut self,
        operand: Operand,
        kind: BorrowKind,
        span: Option<Span>,
    ) -> Operand {
        match operand {
            Operand::Borrow(_) => operand,
            Operand::Copy(place) | Operand::Move(place) => {
                self.borrow_argument_place(place, kind, span)
            }
            other => {
                let local = self.ensure_operand_local(other, span);
                self.borrow_argument_place(Place::new(local), kind, span)
            }
        }
    }

    fn string_span_target(&self, target: &ReadOnlySpanTy) -> bool {
        let element_name = target.element.canonical_name();
        matches!(element_name.as_str(), "byte" | "char")
    }
}
