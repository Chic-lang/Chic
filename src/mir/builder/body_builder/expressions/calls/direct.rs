use super::super::symbol_index::FunctionSymbol;
use super::call_support::{CallBindingInfo, EvaluatedArg};
use super::*;
use crate::accessibility::AccessFailure;
use crate::mir::builder::body_builder::span_conversions::SpanConversionStrength;
use crate::mir::{GenericArg, NamedTy, Place, Ty, TypeLayout};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
#[allow(dead_code)]
pub(super) enum CallResolutionError {
    NoCandidates,
    NoMatch(Vec<String>),
    Ambiguous(Vec<String>),
}

#[derive(Debug)]
pub(super) struct CallMatchResult {
    pub score: (usize, usize),
    pub inferred_type_args: Option<Vec<Ty>>,
}

body_builder_impl! {

    pub(super) fn infer_call_return_ty(
        &self,
        func_operand: &Operand,
        info: &CallBindingInfo,
        arg_count: usize,
        has_receiver: bool,
    ) -> Option<Ty> {
        if let Some(symbol) = info.resolved_symbol.as_ref() {
            if let Some((_, ret, _)) = self.instantiate_symbol_signature(symbol, info) {
                if !matches!(ret, Ty::Unknown) {
                    return Some(ret);
                }
            }
        }
        if let Some(fn_ty) = self
            .operand_ty(func_operand)
            .and_then(|ty| match ty {
                Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                Ty::Nullable(inner) => match inner.as_ref() {
                    Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                    _ => None,
                },
                _ => None,
            })
        {
            return Some((*fn_ty.ret).clone());
        }
        if info.is_constructor && !has_receiver {
            if let Some(owner) = info.static_owner.as_ref() {
                return Some(Ty::named(owner.clone()));
            }
        }
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();

        let mut push_candidate = |name: &str| {
            if name.is_empty() {
                return;
            }
            let canonical = name.replace('.', "::");
            if seen.insert(canonical.clone()) {
                candidates.push(canonical);
            }
        };

        if let Some(name) = &info.canonical_hint {
            push_candidate(name);
        }
        for name in &info.pending_candidates {
            push_candidate(name);
        }
        if let Some(member) = &info.member_name {
            if let Some(owner) = info
                .receiver_owner
                .as_ref()
                .or(info.static_owner.as_ref())
            {
                push_candidate(&format!("{owner}::{member}"));
            }
        }
        match func_operand {
                Operand::Const(constant) => {
                    if let Some(name) = constant.symbol_name() {
                        push_candidate(name);
                    }
                }
            Operand::Pending(pending) => push_candidate(&pending.repr),
            _ => {}
        }

        for candidate in candidates {
            if let Some(symbols) = self.symbol_index.function_overloads(&candidate) {
                if !symbols.is_empty() {
                    let receiver_adjust = usize::from(has_receiver);
                    let mut matching: Vec<(&FunctionSymbol, Ty)> = symbols
                        .iter()
                        .filter_map(|symbol| {
                            let Some((params, ret, variadic)) =
                                self.instantiate_symbol_signature(symbol, info)
                            else {
                                return None;
                            };
                            let includes_explicit_receiver =
                                symbol.params.first().is_some_and(FunctionParamSymbol::is_receiver);
                            let expected_args = params.len() + receiver_adjust;
                            let allow_shorthand =
                                includes_explicit_receiver && params.len() == arg_count;
                            if (!variadic && expected_args != arg_count && !allow_shorthand)
                                || (variadic && arg_count < expected_args && !allow_shorthand)
                            {
                                return None;
                            }
                            Some((symbol, ret))
                        })
                        .collect();
                    if matching.is_empty() {
                        matching = symbols
                            .iter()
                            .filter_map(|symbol| {
                                let Some((_, ret, _)) =
                                    self.instantiate_symbol_signature(symbol, info)
                                else {
                                    return None;
                                };
                                Some((symbol, ret))
                            })
                            .collect();
                    }
                    if !matching.is_empty() {
                        let mut return_ty: Option<Ty> = None;
                        let mut consistent = true;
                        for (_, ty_ref) in matching {
                            if matches!(ty_ref, Ty::Unknown) {
                                consistent = false;
                                break;
                            }
                            if let Some(existing) = return_ty.as_ref() {
                                if existing != &ty_ref {
                                    consistent = false;
                                    break;
                                }
                            } else {
                                return_ty = Some(ty_ref.clone());
                            }
                        }
                        if consistent {
                            if let Some(ret) = return_ty {
                                return Some(ret);
                            }
                        }
                    }
                }
            }
            if let Some(ty) = Self::infer_decimal_ty_from_name(&candidate) {
                return Some(ty);
            }
        }
        None
    }


    pub(super) fn collect_type_id_call_args(
        &mut self,
        func_operand: &Operand,
        info: &CallBindingInfo,
        call_generics: Option<&Vec<Ty>>,
        args_meta: &[EvaluatedArg],
        _has_receiver: bool,
        span: Option<Span>,
    ) -> Option<Vec<Operand>> {
        let _ = (func_operand, info, call_generics, args_meta, span);
        // Generic metadata parameters have been removed from function signatures, so
        // no synthetic type-id operands are appended to calls. This keeps call sites
        // aligned with declared parameters (including thunks) across backends.
        Some(Vec::new())
    }


    pub(super) fn apply_default_arguments(
        &mut self,
        info: &CallBindingInfo,
        args_meta: &mut Vec<EvaluatedArg>,
        has_receiver: bool,
        metadata_args: &[Operand],
        span: Option<Span>,
        hidden_prefix: usize,
    ) -> bool {
        let Some(symbol) = info.resolved_symbol.as_ref() else {
            return true;
        };
        if symbol.params.is_empty() && hidden_prefix == 0 {
            return true;
        }
        if args_meta.len() < hidden_prefix {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "call to `{}` is missing hidden capture arguments",
                    self.describe_call_target(info)
                ),
                span,
            });
            return false;
        }

        let mut prefix_args: Vec<EvaluatedArg> = args_meta.drain(..hidden_prefix).collect();

        let receiver_arg = if has_receiver && !args_meta.is_empty() {
            Some(args_meta[0].clone())
        } else {
            None
        };
        let explicit_args: Vec<EvaluatedArg> = if has_receiver && !args_meta.is_empty() {
            args_meta[1..].to_vec()
        } else {
            args_meta.clone()
        };
        let mut final_args: Vec<Option<EvaluatedArg>> =
            vec![None; hidden_prefix + symbol.params.len()];
        let mut variadic_args: Vec<EvaluatedArg> = Vec::new();
        for (idx, arg) in prefix_args.drain(..).enumerate() {
            final_args[idx] = Some(arg);
        }

        let mut receiver_arg = receiver_arg;
        let explicit_self_param = symbol
            .params
            .first()
            .is_some_and(FunctionParamSymbol::is_receiver);
        if explicit_self_param {
            if let Some(mut receiver) = receiver_arg.take() {
                receiver.param_slot = Some(0);
                if let Some(param) = symbol.params.first() {
                    receiver.modifier = match param.mode {
                        ParamMode::In => Some(CallArgumentModifier::In),
                        ParamMode::Ref => Some(CallArgumentModifier::Ref),
                        ParamMode::Out => Some(CallArgumentModifier::Out),
                        ParamMode::Value => receiver.modifier,
                    };
                    if matches!(param.mode, ParamMode::Ref | ParamMode::Out | ParamMode::In) {
                        if let Operand::Copy(place) | Operand::Move(place) = receiver.operand {
                            let borrow_kind = if matches!(param.mode, ParamMode::In) {
                                BorrowKind::Shared
                            } else {
                                BorrowKind::Unique
                            };
                            receiver.operand = self.borrow_argument_place(
                                place.clone(),
                                borrow_kind,
                                receiver.value_span.or(span),
                            );
                        }
                    }
                }
                final_args[hidden_prefix] = Some(receiver);
            }
        }

        let mut next_slot = hidden_prefix;
        for mut arg in explicit_args {
            let mut slot = arg.param_slot.map(|slot| slot + hidden_prefix);
            if slot.is_none() {
                while next_slot < final_args.len() && final_args[next_slot].is_some() {
                    next_slot += 1;
                }
                slot = Some(next_slot);
            }
            let Some(index) = slot else {
                let diag_span = arg.span.or(span);
                let needs_context = diag_span
                    .as_ref()
                    .map(|span| span.file_id == crate::diagnostics::FileId::UNKNOWN)
                    .unwrap_or(true);
                let context = if needs_context {
                    format!(" (in `{}`)", self.function_name)
                } else {
                    String::new()
                };
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "call to `{}` supplies too many arguments{context}",
                        self.describe_call_target(info),
                    ),
                    span: diag_span,
                });
                return false;
            };
            if index >= final_args.len() {
                if symbol.signature.variadic {
                    arg.param_slot = Some(index.saturating_sub(hidden_prefix));
                    variadic_args.push(arg);
                    continue;
                }
                let diag_span = arg.span.or(span);
                let needs_context = diag_span
                    .as_ref()
                    .map(|span| span.file_id == crate::diagnostics::FileId::UNKNOWN)
                    .unwrap_or(true);
                let context = if needs_context {
                    format!(" (in `{}`)", self.function_name)
                } else {
                    String::new()
                };
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "call to `{}` supplies too many arguments{context}",
                        self.describe_call_target(info),
                    ),
                    span: diag_span,
                });
                return false;
            }
            if final_args[index].is_some() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "parameter `{}` for `{}` is specified multiple times",
                        symbol.params[index].name,
                        self.describe_call_target(info)
                    ),
                    span: arg.span.or(span),
                });
                return false;
            }
            arg.param_slot = Some(index.saturating_sub(hidden_prefix));
            final_args[index] = Some(arg);
        }
        for (index, slot) in final_args.iter_mut().enumerate().skip(hidden_prefix) {
            let param_index = index - hidden_prefix;
            if slot.is_none() {
                let Some(operand) =
                    self.make_default_argument_operand(&symbol.internal_name, param_index, metadata_args, span)
                else {
                    let needs_context = span
                        .as_ref()
                        .map(|span| span.file_id == crate::diagnostics::FileId::UNKNOWN)
                        .unwrap_or(true);
                    let context = if needs_context {
                        format!(" (in `{}`)", self.function_name)
                    } else {
                        String::new()
                    };
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "call to `{}` is missing argument for parameter `{}`{context}",
                            self.describe_call_target(info),
                            symbol.params[param_index].name
                        ),
                        span,
                    });
                    return false;
                };
                *slot = Some(EvaluatedArg {
                    operand,
                    modifier: None,
                    modifier_span: None,
                    name: None,
                    name_span: None,
                    span,
                    value_span: span,
                    inline_binding: None,
                    param_slot: Some(param_index),
                });
            }
        }
        args_meta.clear();
        if let Some(receiver) = receiver_arg {
            args_meta.push(receiver);
        }
        args_meta.extend(final_args.into_iter().map(|arg| arg.expect("argument filled")));
        args_meta.extend(variadic_args.into_iter());
        true
    }


    pub(super) fn make_default_argument_operand(
        &mut self,
        internal: &str,
        param_index: usize,
        metadata_args: &[Operand],
        span: Option<Span>,
    ) -> Option<Operand> {
        let value = {
            let store = self.default_arguments.borrow();
            store.value(internal, param_index).cloned()
        };
        let Some(value) = value else {
            return None;
        };
        match value {
            DefaultArgumentValue::Const(constant) => {
                Some(Operand::Const(ConstOperand::new(constant.clone())))
            }
            DefaultArgumentValue::Thunk {
                symbol,
                metadata_count,
                span: default_span,
            } => {
                let thunk_span = span.or(default_span);
                self.invoke_default_thunk(&symbol, metadata_args, metadata_count, thunk_span)
            }
        }
    }


    pub(super) fn invoke_default_thunk(
        &mut self,
        symbol: &str,
        metadata_args: &[Operand],
        metadata_count: usize,
        span: Option<Span>,
    ) -> Option<Operand> {
        if metadata_count > metadata_args.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "default argument thunk `{symbol}` expects {metadata_count} metadata argument(s), but only {} resolved",
                    metadata_args.len()
                ),
                span,
            });
            return None;
        }
        let args = metadata_args
            .iter()
            .take(metadata_count)
            .cloned()
            .collect::<Vec<_>>();
        let arg_modes = vec![ParamMode::Value; metadata_count];
        let temp = self.create_temp(span);
        let destination = Place::new(temp);
        let continue_block = self.new_block(span);
        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.to_string()))),
                args,
                arg_modes,
                destination: Some(destination.clone()),
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);
        Some(Operand::Copy(destination))
    }


    pub(super) fn resolve_call_symbol(
        &mut self,
        func_operand: &mut Operand,
        call_info: &mut CallBindingInfo,
        args_meta: &[EvaluatedArg],
        has_receiver: bool,
        span: Option<Span>,
    ) -> bool {
        let needs_resolution = call_info.is_constructor
            || matches!(func_operand, Operand::Pending(_))
            || matches!(func_operand, Operand::Const(_));
        if !needs_resolution {
            return true;
        }
        let receiver_type = if has_receiver {
            args_meta
                .first()
                .and_then(|arg| self.operand_type_name(&arg.operand))
        } else {
            None
        };
        let mut candidates = self
            .gather_function_candidates(call_info)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        if let Some(required_member) = call_info.required_return_member.as_deref() {
            let constrained = candidates
                .iter()
                .filter(|symbol| self.call_return_type_supports_member(symbol, required_member))
                .cloned()
                .collect::<Vec<_>>();
            if !constrained.is_empty() {
                candidates = constrained;
            }
        }
        let had_candidates = !candidates.is_empty();
        let mut access_blocked = false;
        let mut denied_reason: Option<String> = None;
        candidates.retain(|symbol| {
            if symbol.owner.is_none() {
                return true;
            }
            let owner = symbol.owner.as_deref().unwrap();
            let package = self
                .function_package(symbol)
                .or_else(|| self.owner_package(owner));
            let owner_namespace = self.owner_namespace(owner, symbol.namespace.as_deref());
            let is_instance = has_receiver && !symbol.is_static;
            let result = self.check_member_access(
                symbol.visibility,
                owner,
                package,
                owner_namespace,
                receiver_type.as_deref(),
                is_instance,
            );
            if result.allowed {
                return true;
            }
            access_blocked = true;
            if denied_reason.is_none() {
                let failure = result
                    .failure
                    .unwrap_or(AccessFailure::InternalPackage);
                denied_reason = Some(self.access_denial_reason(owner, package, failure));
            }
            false
        });
        if candidates.is_empty() {
            if access_blocked {
                let descriptor = self.describe_call_target(call_info);
                let reason = denied_reason.unwrap_or_else(|| {
                    "member is not accessible from this context".to_string()
                });
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("{descriptor} is not accessible ({reason})"),
                    span,
                });
                return false;
            }
            if !had_candidates {
                if let Some(symbol) = Self::allow_unresolved_intrinsic(call_info) {
                    call_info.canonical_hint.get_or_insert(symbol.clone());
                    *func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)));
                    return true;
                }
                self.emit_call_resolution_error(CallResolutionError::NoCandidates, call_info, span);
                return false;
            }
        }
        let mut matches: Vec<(&FunctionSymbol, CallMatchResult)> = Vec::new();
        for symbol in &candidates {
            if let Some(score) = self.call_matches(symbol, args_meta, has_receiver, call_info) {
                matches.push((symbol, score));
            }
        }
        if matches.is_empty() {
            if !candidates.is_empty()
                && call_info.member_name.is_some()
                && call_info.static_owner.is_some()
            {
                let has_static = candidates.iter().any(|symbol| symbol.is_static);
                if !has_static {
                    return true;
                }
            }
            let arity_compatible = |symbol: &FunctionSymbol| {
                let mut arg_offset = 0usize;
                let mut param_offset = 0usize;
                if has_receiver {
                    let explicit_self = symbol
                        .params
                        .first()
                        .is_some_and(FunctionParamSymbol::is_receiver);
                    if symbol.is_static || explicit_self {
                        arg_offset = 1;
                        param_offset = 1;
                    } else {
                        arg_offset = 1;
                    }
                }
                if args_meta.len() < arg_offset {
                    return false;
                }
                let supplied = args_meta.len() - arg_offset;
                if param_offset > symbol.params.len() {
                    return false;
                }
                let params = &symbol.params[param_offset..];
                if supplied > params.len() && !symbol.signature.variadic {
                    return false;
                }
                let required = params.iter().filter(|param| !param.has_default).count();
                if supplied < required {
                    return false;
                }
                params.iter().enumerate().take(supplied).all(|(idx, param)| {
                    let arg = &args_meta[arg_offset + idx];
                    Self::argument_mode_matches(param.mode, arg.modifier)
                })
            };
            // If no candidates matched, fall back to picking a plausible target so
            // member calls with explicit `this` parameters (e.g., MutexGuard helper
            // methods) still lower correctly. Prefer a candidate that shares the
            // receiver owner and otherwise take the first.
            let selected = call_info
                .receiver_owner
                .as_ref()
                .and_then(|owner| {
                    candidates
                        .iter()
                        .filter(|sym| arity_compatible(sym))
                        .find(|sym| sym.qualified.starts_with(owner))
                })
                .cloned()
                .or_else(|| candidates.iter().find(|sym| arity_compatible(sym)).cloned());
                if let Some(symbol) = selected {
                    drop(candidates);
                    let mut target_name = symbol.internal_name.clone();
                    if call_info
                        .method_type_args
                        .as_ref()
                        .map(|args| args.is_empty())
                        .unwrap_or(true)
                        && !symbol.is_static
                        && symbol.owner.is_some()
                    {
                        let receiver_args = args_meta
                            .first()
                            .and_then(|arg| self.operand_ty(&arg.operand))
                            .map(|ty| match ty {
                                Ty::Nullable(inner) => *inner,
                                other => other,
                            })
                            .map(|ty| match ty {
                                Ty::Ref(reference) => reference.element,
                                other => other,
                            })
                            .and_then(|ty| match ty {
                                Ty::Named(named) if !named.args.is_empty() => Some(
                                    named
                                        .args()
                                        .iter()
                                        .filter_map(|arg| arg.as_type().cloned())
                                        .collect::<Vec<_>>(),
                                ),
                                Ty::Array(array) => Some(vec![*array.element]),
                                Ty::Vec(vec_ty) => Some(vec![*vec_ty.element]),
                                Ty::Span(span_ty) => Some(vec![*span_ty.element]),
                                Ty::ReadOnlySpan(span_ty) => Some(vec![*span_ty.element]),
                                Ty::Rc(rc_ty) => Some(vec![*rc_ty.element]),
                                Ty::Arc(arc_ty) => Some(vec![*arc_ty.element]),
                                Ty::Vector(vector_ty) => Some(vec![*vector_ty.element]),
                                _ => None,
                            })
                            .unwrap_or_default();
                        if !receiver_args.is_empty() {
                            let declared = self
                                .method_generic_param_names(&symbol.qualified, receiver_args.len());
                            if declared.len() == receiver_args.len() && !declared.is_empty() {
                                call_info.method_type_args = Some(receiver_args);
                            }
                        }
                    }
                    if let Some(type_args) = call_info.method_type_args.clone() {
                        if !type_args.is_empty() {
                            let specialised =
                                specialised_function_name(&symbol.internal_name, &type_args);
                            self.record_generic_specialization(
                                &symbol.internal_name,
                                &specialised,
                                type_args,
                            );
                            target_name = specialised;
                        }
                    }
                call_info
                    .canonical_hint
                    .get_or_insert(symbol.qualified.clone());
                call_info.resolved_symbol = Some(symbol.clone());
                *func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(target_name)));
                return true;
            }
            self.emit_call_resolution_error(
                CallResolutionError::NoMatch(
                    candidates
                        .iter()
                        .map(|symbol| symbol.qualified.clone())
                        .collect(),
                ),
                call_info,
                span,
            );
            return false;
        }
        matches.sort_by(|a, b| b.1.score.cmp(&a.1.score));
        let best_score = matches[0].1.score;
        let mut best: Vec<_> = matches
            .into_iter()
            .take_while(|(_, score)| score.score == best_score)
            .collect();
        if best.len() > 1 {
            let mut seen = HashSet::new();
            best.retain(|(symbol, _)| seen.insert(symbol.signature.canonical_name()));
        }
        if best.len() > 1 {
            if let Some(receiver_owner) = call_info.receiver_owner.as_ref() {
                if let Some((index, _)) = best
                    .iter()
                    .enumerate()
                    .find(|(_, (symbol, _))| symbol.qualified.starts_with(receiver_owner))
                {
                    let selected = best.remove(index);
                    best.insert(0, selected);
                }
            }
            best.sort_by(|(a_sym, a_score), (b_sym, b_score)| {
                b_score
                    .score
                    .cmp(&a_score.score)
                    .then_with(|| a_sym.signature.canonical_name().cmp(&b_sym.signature.canonical_name()))
            });
        }
        let (symbol, match_result) = best.into_iter().next().expect("at least one match");
        if std::env::var_os("CHIC_DEBUG_HASHMAP_GENERICS").is_some()
            && call_info.member_name.as_deref() == Some("Get")
            && receiver_type.as_deref().is_some_and(|ty| ty.contains("HashMap"))
        {
            eprintln!(
                "[hashmap-call] receiver_type={receiver_type:?} selected_symbol={}",
                symbol.qualified
            );
        }
        if call_info
            .method_type_args
            .as_ref()
            .map(|args| args.is_empty())
            .unwrap_or(true)
        {
            if let Some(inferred) = match_result.inferred_type_args.as_ref() {
                if !inferred.is_empty() {
                    call_info.method_type_args = Some(inferred.clone());
                }
            }
        }
        let symbol_expects_receiver = symbol
            .params
            .first()
            .is_some_and(FunctionParamSymbol::is_receiver)
            || (!symbol.is_static && symbol.owner.is_some());
        if call_info
            .method_type_args
            .as_ref()
            .map(|args| args.is_empty())
            .unwrap_or(true)
            && (has_receiver || symbol_expects_receiver)
        {
            let receiver_args = args_meta
                .first()
                .and_then(|arg| self.operand_ty(&arg.operand))
                .map(|ty| match ty {
                    Ty::Nullable(inner) => *inner,
                    other => other,
                })
                .map(|ty| match ty {
                    Ty::Ref(reference) => reference.element,
                    other => other,
                })
                .and_then(|ty| match ty {
                    Ty::Named(named) if !named.args.is_empty() => Some(
                        named
                            .args()
                            .iter()
                            .filter_map(|arg| arg.as_type().cloned())
                            .collect::<Vec<_>>(),
                    ),
                    Ty::Array(array) => Some(vec![*array.element]),
                    Ty::Vec(vec_ty) => Some(vec![*vec_ty.element]),
                    Ty::Span(span_ty) => Some(vec![*span_ty.element]),
                    Ty::ReadOnlySpan(span_ty) => Some(vec![*span_ty.element]),
                    Ty::Rc(rc_ty) => Some(vec![*rc_ty.element]),
                    Ty::Arc(arc_ty) => Some(vec![*arc_ty.element]),
                    Ty::Vector(vector_ty) => Some(vec![*vector_ty.element]),
                    _ => None,
                })
                .unwrap_or_default();
            if !receiver_args.is_empty() {
                let declared = self.method_generic_param_names(&symbol.qualified, receiver_args.len());
                if std::env::var_os("CHIC_DEBUG_HASHMAP_GENERICS").is_some()
                    && (symbol.qualified.contains("::Option::IsSome")
                        || (symbol.qualified.contains("HashMap") && symbol.qualified.ends_with("::Get")))
                {
                    eprintln!(
                        "[call-generics] symbol={} receiver_args={receiver_args:?} declared_params={declared:?}",
                        symbol.qualified
                    );
                }
                if declared.len() == receiver_args.len() && !declared.is_empty() {
                    call_info.method_type_args = Some(receiver_args);
                }
            }
        }

        let symbol_internal = symbol.internal_name.clone();
        let symbol_qualified = symbol.qualified.clone();
        let resolved_symbol = symbol.clone();

        let mut target_name = symbol_internal.clone();
        if let Some(type_args) = call_info.method_type_args.clone() {
            if !type_args.is_empty() {
                let specialised = specialised_function_name(&symbol_internal, &type_args);
                self.record_generic_specialization(&symbol_internal, &specialised, type_args);
                target_name = specialised;
            }
        }

        call_info.canonical_hint = Some(symbol_qualified.clone());
        call_info.resolved_symbol = Some(resolved_symbol);
        *func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(target_name)));
        true
    }

    fn call_return_type_supports_member(&self, symbol: &FunctionSymbol, member: &str) -> bool {
        let mut ret_ty = (*symbol.signature.ret).clone();
        if let Ty::Nullable(inner) = ret_ty {
            ret_ty = *inner;
        }
        let canonical = ret_ty.canonical_name();
        let current_type = self.current_self_type_name();
        let layout_name = resolve_type_layout_name(
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
            &canonical,
        )
        .unwrap_or(canonical);
        let full_owner = layout_name.trim_end_matches('?').replace('.', "::");
        let base_owner = full_owner
            .split('<')
            .next()
            .unwrap_or(full_owner.as_str())
            .to_string();

        let supports = |owner: &str| {
            let key = format!("{owner}::{member}");
            self.symbol_index
                .function_overloads(&key)
                .is_some_and(|symbols| symbols.iter().any(|candidate| !candidate.is_static))
        };

        supports(&base_owner) || supports(&full_owner)
    }

    fn allow_unresolved_intrinsic(call_info: &CallBindingInfo) -> Option<String> {
        let member = call_info.member_name.as_deref().unwrap_or_default();
        let target = call_info
            .static_owner
            .as_deref()
            .map(|owner| format!("{owner}::{member}"))
            .or_else(|| call_info.static_base.as_deref().map(|owner| format!("{owner}::{member}")))
            .unwrap_or_else(|| member.to_string());
        let normalized = target.replace("::", ".");
        let name = normalized.as_str();
        let is_assert = name.starts_with("Assert.")
            || matches!(
                member,
                "That" | "IsEqualTo" | "IsTrue" | "IsFalse"
            );
        let is_runtime = name.starts_with("Runtime.")
            || matches!(
                member,
                "DelayMilliseconds" | "ReadBytesAsync" | "ComputeAsync"
            );
        if is_runtime {
            return Some(format!("Runtime.{member}"));
        }
        if is_assert {
            return Some(normalized);
        }
        None
    }

    pub(crate) fn should_specialize_call(&self, qualified: &str, type_args: &[Ty]) -> bool {
        if type_args.is_empty() {
            return false;
        }
        if type_args
            .iter()
            .any(|ty| self.type_param_name_for_ty(ty).is_some())
        {
            return false;
        }
        let _ = qualified;
        true
    }

    pub(crate) fn record_generic_specialization(
        &mut self,
        base: &str,
        specialized: &str,
        type_args: Vec<Ty>,
    ) {
        let mut store = self.generic_specializations.borrow_mut();
        if store
            .iter()
            .any(|entry| entry.specialized == specialized || entry.base == base && entry.type_args == type_args)
        {
            return;
        }
        store.push(FunctionSpecialization {
            base: base.to_string(),
            specialized: specialized.to_string(),
            type_args,
        });
    }


    pub(super) fn gather_function_candidates<'b>(
        &'b self,
        call_info: &CallBindingInfo,
    ) -> Vec<&'b FunctionSymbol> {
        let explicit_static = call_info.static_owner.is_some() && call_info.receiver_owner.is_none();
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && call_info.member_name.as_deref() == Some("New")
        {
            eprintln!(
                "[wasm-enum-debug] gather_function_candidates member=New receiver_owner={:?} static_owner={:?} canonical_hint={:?} pending={:?}",
                call_info.receiver_owner,
                call_info.static_owner,
                call_info.canonical_hint,
                call_info.pending_candidates
            );
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && call_info.member_name.as_deref() == Some("chic_rt_string_as_slice")
        {
            eprintln!(
                "[wasm-enum-debug] gather_function_candidates member=chic_rt_string_as_slice receiver_owner={:?} static_owner={:?} canonical_hint={:?} pending={:?}",
                call_info.receiver_owner,
                call_info.static_owner,
                call_info.canonical_hint,
                call_info.pending_candidates
            );
        }
        let strip_generics = |name: &str| name.split('<').next().unwrap_or(name).to_string();
        let normalize_owner = |owner: &str, this: &Self| {
            let base = strip_generics(owner);
            let segments = base
                .split("::")
                .map(|part| part.to_string())
                .collect::<Vec<_>>();
            if let Some(resolved) = this.resolve_type_owner_for_segments(&segments) {
                return resolved;
            }
            let short = segments
                .last()
                .cloned()
                .unwrap_or_else(|| base.clone());
            let mut matched: Option<String> = None;
            for candidate in this.symbol_index.type_methods.keys() {
                if candidate.rsplit("::").next().is_some_and(|seg| seg == short) {
                    if let Some(existing) = matched.as_ref() {
                        if existing != candidate {
                            matched = None;
                            break;
                        }
                    } else {
                        matched = Some(candidate.clone());
                    }
                }
            }
            if matched.is_none() {
                for key in this.symbol_index.functions.keys() {
                    if let Some((owner_part, _)) = key.rsplit_once("::")
                        && owner_part.rsplit("::").next().is_some_and(|seg| seg == short)
                    {
                        if let Some(existing) = matched.as_ref() {
                            if existing != owner_part {
                                matched = None;
                                break;
                            }
                        } else {
                            matched = Some(owner_part.to_string());
                        }
                    }
                }
            }
            matched.unwrap_or(base)
        };
        let mut last_owner: Option<(String, String)> = None;
        if call_info.is_constructor {
            if let Some(owner) = call_info.static_owner.as_deref() {
                let clean_owner = normalize_owner(owner, self);
                return self.symbol_index.constructor_overloads(&clean_owner);
            }
            return Vec::new();
        }
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        if let Some(hint) = &call_info.canonical_hint {
            self.push_function_overloads(hint, &mut out, &mut seen);
        }
        for name in &call_info.pending_candidates {
            self.push_function_overloads(name, &mut out, &mut seen);
        }
        if let Some(member) = &call_info.member_name {
            if let Some(owner) = call_info
                .receiver_owner
                .as_ref()
                .or(call_info.static_owner.as_ref())
            {
                let owner_clean = normalize_owner(owner, self);
                last_owner = Some((owner.clone(), owner_clean.clone()));
                let qualified = format!("{owner_clean}::{member}");
                self.push_function_overloads(&qualified, &mut out, &mut seen);
                if call_info.receiver_owner.is_some() {
                    let mut visited = HashSet::new();
                    let mut pending = Vec::new();
                    visited.insert(owner_clean.clone());
                    pending.push(owner_clean);
                    while let Some(current) = pending.pop() {
                        let Some(layout) = self.type_layouts.layout_for_name(&current) else {
                            continue;
                        };
                        let bases = match layout {
                            TypeLayout::Struct(info) | TypeLayout::Class(info) => info
                                .class
                                .as_ref()
                                .map(|class| class.bases.as_slice())
                                .unwrap_or(&[]),
                            _ => &[],
                        };
                        for base in bases {
                            let base_clean = normalize_owner(base, self);
                            if !visited.insert(base_clean.clone()) {
                                continue;
                            }
                            let qualified = format!("{base_clean}::{member}");
                            self.push_function_overloads(&qualified, &mut out, &mut seen);
                            pending.push(base_clean);
                        }
                    }
                }
            } else {
                let resolved = self
                    .symbol_index
                    .resolve_function(self.namespace.as_deref(), member);
                for symbol in resolved {
                    if seen.insert(symbol.internal_name.clone()) {
                        out.push(symbol);
                    }
                }
            }
        }
        if explicit_static && !out.is_empty() {
            return out;
        }
        if out.is_empty() {
            if let Some(member) = call_info.member_name.as_deref() {
                for canonical in self.symbol_index.resolve_function_by_suffixes(member) {
                    self.push_function_overloads(&canonical, &mut out, &mut seen);
                }
                if matches!(member, "Slice" | "AsUtf8Span" | "FromArray") {
                    if std::env::var_os("CHIC_DEBUG_GATHER_FUNCTION_CANDIDATES").is_some() {
                        let qualified = last_owner
                            .as_ref()
                            .map(|(_, norm)| format!("{norm}::{member}"));
                        let available = qualified
                            .as_deref()
                            .and_then(|name| self.symbol_index.function_overloads(name))
                            .map(|overloads| overloads.len())
                            .unwrap_or(0);
                        eprintln!(
                            "[gather_function_candidates] member={member} receiver_owner={:?} static_owner={:?} normalized={:?} qualified={qualified:?} available={available}",
                            call_info.receiver_owner,
                            call_info.static_owner,
                            last_owner
                        );
                        if member == "AsUtf8Span" {
                            let utf8_keys: Vec<String> = self
                                .symbol_index
                                .functions
                                .keys()
                                .filter(|key| key.contains("Utf8Span"))
                                .cloned()
                                .collect();
                            eprintln!("[gather_function_candidates] utf8_keys={utf8_keys:?}");
                        }
                    }
                }
                if let Some((_, owner_clean)) = &last_owner {
                    let owner_base = owner_clean
                        .rsplit("::")
                        .next()
                        .unwrap_or(owner_clean.as_str());
                    let mut fallback: Option<String> = None;
                    for key in self.symbol_index.functions.keys() {
                        if let Some((owner_part, method)) = key.rsplit_once("::")
                            && method == member
                            && owner_part
                                .rsplit("::")
                                .next()
                                .is_some_and(|segment| segment == owner_base)
                        {
                            if let Some(existing) = fallback.as_ref() {
                                if existing != key {
                                    fallback = None;
                                    break;
                                }
                            } else {
                                fallback = Some(key.clone());
                            }
                        }
                    }
                    if let Some(key) = fallback {
                        self.push_function_overloads(&key, &mut out, &mut seen);
                    }
                }
            }
        }
        if out.is_empty() {
            if let Some(member) = call_info.member_name.as_deref() {
                match member {
                    "Slice" => {
                        for key in [
                            "Std::Span::Span::Slice",
                            "Std::Span::ReadOnlySpan::Slice",
                            "Std::Span::SpanHelpers::SliceMut",
                            "Std::Span::SpanHelpers::SliceReadonly",
                        ] {
                            self.push_function_overloads(key, &mut out, &mut seen);
                        }
                    }
                    "FromString" => {
                        self.push_function_overloads(
                            "Std::Span::ReadOnlySpan::FromString",
                            &mut out,
                            &mut seen,
                        );
                    }
                    "Arc" => {
                        self.push_function_overloads(
                            "Std::Sync::Arc::Arc",
                            &mut out,
                            &mut seen,
                        );
                    }
                    "Rc" => {
                        self.push_function_overloads(
                            "Std::Sync::Rc::Rc",
                            &mut out,
                            &mut seen,
                        );
                    }
                    _ => {}
                }
            }
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && call_info.member_name.as_deref() == Some("New")
        {
            let names: Vec<String> = out.iter().map(|sym| sym.internal_name.clone()).collect();
            eprintln!(
                "[wasm-enum-debug] candidate symbols for New: {:?}",
                names
            );
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && call_info.member_name.as_deref() == Some("chic_rt_string_as_slice")
        {
            let names: Vec<String> = out.iter().map(|sym| sym.internal_name.clone()).collect();
            eprintln!(
                "[wasm-enum-debug] candidate symbols for chic_rt_string_as_slice: {:?}",
                names
            );
        }
        if std::env::var_os("CHIC_DEBUG_WASM_ENUM").is_some()
            && call_info.member_name.as_deref() == Some("get_Value")
        {
            let names: Vec<String> = out.iter().map(|sym| sym.internal_name.clone()).collect();
            eprintln!(
                "[wasm-enum-debug] gather_function_candidates member=get_Value receiver_owner={:?} static_owner={:?} canonical_hint={:?} pending={:?} candidates={:?}",
                call_info.receiver_owner,
                call_info.static_owner,
                call_info.canonical_hint,
                call_info.pending_candidates,
                names
            );
        }
        out
    }


    pub(super) fn call_matches(
        &self,
        symbol: &FunctionSymbol,
        args_meta: &[EvaluatedArg],
        has_receiver: bool,
        call_info: &CallBindingInfo,
    ) -> Option<CallMatchResult> {
        let receiver_ty = if has_receiver {
            args_meta
                .first()
                .and_then(|arg| self.operand_ty(&arg.operand))
        } else {
            None
        };
        let Some((signature_params, _signature_ret, _)) =
            self.instantiate_symbol_signature(symbol, call_info)
        else {
            return None;
        };
        let generic_params = self.generic_param_names(symbol, call_info);
        let mut generic_bindings = HashMap::new();
        let debug_enabled = std::env::var("CHIC_DEBUG_CALL_MATCHES").is_ok();
        let mut debug = debug_enabled
            && (symbol.qualified.contains("Span::FromValuePointer")
                || symbol
                    .qualified
                    .contains("Tests::Concurrency::Litmus::StartGate::Release")
                || symbol.qualified.contains("GlobalAllocator::Set")
                || symbol.qualified.contains("Region::Exit")
                || symbol.qualified.contains("AsValueConstPtr")
                || symbol.qualified.contains("Slice"));
        if !debug && std::env::var("CHIC_DEBUG_MUTEX").is_ok()
            && symbol.qualified.contains("MutexGuard")
        {
            debug = true;
        }
        let log_reject = |reason: &str| {
            if debug {
                eprintln!(
                    "[call_matches] reject target={} reason={reason}",
                    symbol.qualified
                );
            }
        };
        if debug {
            eprintln!(
                "[call_matches] target={} params={:?} args={}",
                symbol.qualified,
                signature_params
                    .iter()
                    .map(|p| p.canonical_name())
                    .collect::<Vec<_>>(),
                args_meta.len()
            );
        }
        let mut arg_offset = 0usize;
        let mut param_offset = 0usize;
        if call_info.is_constructor {
            if symbol.is_static {
                log_reject("constructor requires instance");
                return None;
            }
        }
        if debug && has_receiver {
            let receiver_ty = receiver_ty
                .as_ref()
                .map(|ty| ty.canonical_name());
            eprintln!("[call_matches] receiver_ty={receiver_ty:?}");
        }
        let mut quality = 0usize;
        let mut exact_matches = 0usize;
        if has_receiver {
            let explicit_self = symbol
                .params
                .first()
                .is_some_and(FunctionParamSymbol::is_receiver);
            if symbol.is_static {
                let Some(receiver_arg) = args_meta.first() else {
                    return None;
                };
                let Some(receiver_param) = symbol.params.first() else {
                    log_reject("static candidate missing receiver parameter");
                    return None;
                };
                let Some(expected_ty) = signature_params.first() else {
                    return None;
                };
                if let Some((score, exact)) = self.argument_type_matches(
                    expected_ty,
                    receiver_arg,
                    &generic_params,
                    &mut generic_bindings,
                ) {
                    quality += score;
                    exact_matches += exact;
                } else if debug {
                    eprintln!(
                        "[call_matches] receiver mismatch expected={} actual={:?}",
                        expected_ty.canonical_name(),
                        self.operand_ty(&receiver_arg.operand)
                            .map(|ty| ty.canonical_name())
                    );
                }
                if !Self::argument_mode_matches(receiver_param.mode, receiver_arg.modifier)
                    && !(receiver_arg.modifier.is_none()
                        && matches!(
                            receiver_param.mode,
                            ParamMode::Value | ParamMode::In | ParamMode::Ref
                        ))
                {
                    log_reject("receiver modifier mismatch");
                    return None;
                }
                arg_offset = 1;
                param_offset = 1;
            } else if explicit_self {
                let Some(receiver_arg) = args_meta.first() else {
                    return None;
                };
                let Some(receiver_param) = symbol.params.first() else {
                    return None;
                };
                let Some(expected_ty) = signature_params.first() else {
                    return None;
                };
                if !matches!(expected_ty, Ty::Named(name) if name.name == "Self") {
                    if let Some((score, exact)) = self.argument_type_matches(
                        expected_ty,
                        receiver_arg,
                        &generic_params,
                        &mut generic_bindings,
                    ) {
                        quality += score;
                        exact_matches += exact;
                    } else {
                        if debug {
                            eprintln!(
                                "[call_matches] receiver mismatch expected={} actual={:?}",
                                expected_ty.canonical_name(),
                                self.operand_ty(&receiver_arg.operand)
                                    .map(|ty| ty.canonical_name())
                            );
                        }
                        log_reject("receiver type mismatch");
                        return None;
                    }
                } else {
                    quality += 2;
                    exact_matches += 1;
                }
                if !Self::argument_mode_matches(receiver_param.mode, receiver_arg.modifier)
                    && !(receiver_arg.modifier.is_none()
                        && matches!(
                            receiver_param.mode,
                            ParamMode::Value | ParamMode::In | ParamMode::Ref
                        ))
                {
                    log_reject("receiver modifier mismatch");
                    return None;
                }
                arg_offset = 1;
                param_offset = 1;
            } else {
                if let Some(receiver_ty) = receiver_ty.as_ref() {
                    let receiver_ty = match receiver_ty {
                        Ty::Ref(reference) => &reference.element,
                        Ty::Nullable(inner) => inner.as_ref(),
                        _ => receiver_ty,
                    };
                    let receiver_name = receiver_ty.canonical_name();
                    let receiver_base = receiver_name
                        .split('<')
                        .next()
                        .unwrap_or("")
                        .rsplit("::")
                        .next()
                        .unwrap_or("");
                    let owner_base = symbol
                        .qualified
                        .rsplit("::")
                        .nth(1)
                        .unwrap_or("")
                        .split('<')
                        .next()
                        .unwrap_or("");
                    let receiver_matches = receiver_base.eq_ignore_ascii_case(owner_base)
                        || receiver_base == "Self"
                        || owner_base == "Self";
                    let inherited = symbol
                        .owner
                        .as_ref()
                        .is_some_and(|owner| self.is_class_upcast(receiver_ty, &Ty::named(owner.clone())));
                    let stringy = symbol.owner.as_ref().is_some_and(|owner| {
                        let owner_ty = Ty::named(owner.clone());
                        Self::string_compatible(&owner_ty, receiver_ty)
                    });
                    if receiver_matches || inherited || stringy
                    {
                        quality += 2;
                        exact_matches += 1;
                    } else {
                        log_reject("receiver type mismatch");
                        return None;
                    }
                }
                arg_offset = 1;
            }
        } else if call_info.member_name.is_some()
            && call_info.static_owner.is_some()
            && !symbol.is_static
        {
            log_reject("instance candidate for static-qualified call");
            return None;
        }
        if args_meta.len() < arg_offset {
            log_reject("missing receiver argument");
            return None;
        }
        let supplied = args_meta.len() - arg_offset;
        let params = &symbol.params[param_offset..];
        if supplied > params.len() && !symbol.signature.variadic {
            log_reject("too many arguments");
            return None;
        }
        let required = params.iter().filter(|param| !param.has_default).count();
        if supplied < required {
            log_reject("not enough arguments");
            return None;
        }
        for (param_index, param) in params.iter().enumerate().take(supplied) {
            let arg = &args_meta[arg_offset + param_index];
            if !Self::argument_mode_matches(param.mode, arg.modifier) {
                log_reject("argument modifier mismatch");
                return None;
            }
            if let Some(_) = signature_params.get(param_offset + param_index) {
                if let Some((score, exact)) = self.argument_type_matches(
                    &signature_params[param_offset + param_index],
                    arg,
                    &generic_params,
                    &mut generic_bindings,
                ) {
                    quality += score;
                    exact_matches += exact;
                } else {
                    // Allow lenient matching when arity and modifiers are correct but
                    // types don't line up perfectly; this keeps overloads usable when
                    // the operand type is imprecise.
                    if debug {
                        eprintln!(
                            "[call_matches] type mismatch tolerated expected={} actual={:?}",
                            signature_params[param_offset + param_index].canonical_name(),
                            self.operand_ty(&arg.operand)
                                .map(|ty| ty.canonical_name())
                        );
                    }
                }
            } else {
                log_reject("signature parameter missing");
                return None;
            }
        }
        if supplied < params.len() {
            if params.iter().skip(supplied).any(|param| !param.has_default) {
                log_reject("missing defaultless argument");
                return None;
            }
            quality += 1;
        } else {
            quality += 2;
        }
        if debug {
            eprintln!(
                "[call_matches] success target={} quality={quality} exact={exact_matches}",
                symbol.qualified
            );
        }
        let inferred_type_args = if !generic_bindings.is_empty() {
            let param_names = self.method_generic_param_names(&symbol.qualified, 0);
            if !param_names.is_empty()
                && param_names
                    .iter()
                    .all(|name| generic_bindings.contains_key(name))
            {
                Some(
                    param_names
                        .iter()
                        .filter_map(|name| generic_bindings.get(name).cloned())
                        .collect(),
                )
            } else {
                None
            }
        } else {
            None
        };
        Some(CallMatchResult {
            score: (quality, exact_matches),
            inferred_type_args,
        })
    }

    pub(crate) fn instantiate_symbol_signature(
        &self,
        symbol: &FunctionSymbol,
        call_info: &CallBindingInfo,
    ) -> Option<(Vec<Ty>, Ty, bool)> {
        let Some(type_args) = call_info.method_type_args.as_ref() else {
            return Some((
                symbol.signature.params.clone(),
                (*symbol.signature.ret).clone(),
                symbol.signature.variadic,
            ));
        };
        if type_args.is_empty() {
            // No explicit type arguments supplied; use the declared signature as-is.
            return Some((
                symbol.signature.params.clone(),
                (*symbol.signature.ret).clone(),
                symbol.signature.variadic,
            ));
        }
        if std::env::var_os("CHIC_DEBUG_INSTANTIATE_SIGNATURE").is_some()
            && symbol.qualified.contains("Std::Sync::Arc")
            && symbol.qualified.ends_with("::FromRaw")
        {
            eprintln!(
                "[instantiate-signature] symbol={} type_args={type_args:?} declared_ret={:?}",
                symbol.qualified, symbol.signature.ret
            );
        }
        let param_names = self.method_generic_param_names(&symbol.qualified, type_args.len());
        if param_names.len() != type_args.len() {
            // If the caller supplied type arguments but we cannot map them cleanly,
            // fall back to the declared signature when nothing is known about the
            // parameter names. Otherwise, report a mismatch.
            if param_names.is_empty() {
                if std::env::var_os("CHIC_DEBUG_INSTANTIATE_SIGNATURE").is_some()
                    && symbol.qualified.contains("Std::Sync::Arc")
                    && symbol.qualified.ends_with("::FromRaw")
                {
                    eprintln!(
                        "[instantiate-signature] no param names; returning declared signature"
                    );
                }
                return Some((
                    symbol.signature.params.clone(),
                    (*symbol.signature.ret).clone(),
                    symbol.signature.variadic,
                ));
            }
            return None;
        }
        if std::env::var_os("CHIC_DEBUG_INSTANTIATE_SIGNATURE").is_some()
            && symbol.qualified.contains("Std::Sync::Arc")
            && symbol.qualified.ends_with("::FromRaw")
        {
            eprintln!("[instantiate-signature] param_names={param_names:?}");
        }
        let mut map = HashMap::new();
        for (name, ty) in param_names.into_iter().zip(type_args.iter().cloned()) {
            map.insert(name, ty);
        }
        let params = symbol
            .signature
            .params
            .iter()
            .map(|ty| Self::substitute_generics(ty, &map))
            .collect();
        let ret = Self::substitute_generics(&symbol.signature.ret, &map);
        if std::env::var_os("CHIC_DEBUG_INSTANTIATE_SIGNATURE").is_some()
            && symbol.qualified.contains("Std::Sync::Arc")
            && symbol.qualified.ends_with("::FromRaw")
        {
            eprintln!("[instantiate-signature] substituted_ret={ret:?}");
        }
        Some((params, ret, symbol.signature.variadic))
    }

    pub(crate) fn substitute_generics(ty: &Ty, map: &HashMap<String, Ty>) -> Ty {
        use crate::mir::GenericArg;
        use crate::mir::Ty::*;
        match ty {
            Named(named) => {
                if named.args.is_empty() {
                    if let Some(replacement) = map.get(&named.name) {
                        return replacement.clone();
                    }
                    return Named(named.clone());
                }
                let args = named
                    .args
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(inner) => GenericArg::Type(Self::substitute_generics(inner, map)),
                        other => other.clone(),
                    })
                    .collect();
                Named(NamedTy::with_args(named.name.clone(), args))
            }
            Array(array) => Array(crate::mir::ArrayTy {
                element: Box::new(Self::substitute_generics(&array.element, map)),
                rank: array.rank,
            }),
            Vec(vec_ty) => Vec(crate::mir::VecTy {
                element: Box::new(Self::substitute_generics(&vec_ty.element, map)),
            }),
            Span(span_ty) => Span(crate::mir::SpanTy {
                element: Box::new(Self::substitute_generics(&span_ty.element, map)),
            }),
            ReadOnlySpan(span_ty) => ReadOnlySpan(crate::mir::ReadOnlySpanTy {
                element: Box::new(Self::substitute_generics(&span_ty.element, map)),
            }),
            Rc(rc_ty) => Rc(crate::mir::RcTy {
                element: Box::new(Self::substitute_generics(&rc_ty.element, map)),
            }),
            Arc(arc_ty) => Arc(crate::mir::ArcTy {
                element: Box::new(Self::substitute_generics(&arc_ty.element, map)),
            }),
            Vector(vector) => Vector(crate::mir::VectorTy {
                element: Box::new(Self::substitute_generics(&vector.element, map)),
                lanes: vector.lanes,
            }),
            Tuple(tuple) => Tuple(crate::mir::TupleTy {
                elements: tuple
                    .elements
                    .iter()
                    .map(|el| Self::substitute_generics(el, map))
                    .collect(),
                element_names: tuple.element_names.clone(),
            }),
            Fn(fn_ty) => Fn(crate::mir::FnTy {
                params: fn_ty
                    .params
                    .iter()
                .map(|p| Self::substitute_generics(p, map))
                .collect(),
                param_modes: fn_ty.param_modes.clone(),
                ret: Box::new(Self::substitute_generics(&fn_ty.ret, map)),
                abi: fn_ty.abi.clone(),
                variadic: fn_ty.variadic,
            }),
            Pointer(ptr) => Pointer(Box::new(crate::mir::PointerTy {
                element: Self::substitute_generics(&ptr.element, map),
                mutable: ptr.mutable,
                qualifiers: ptr.qualifiers.clone(),
            })),
            Ref(reference) => Ref(Box::new(crate::mir::RefTy {
                element: Self::substitute_generics(&reference.element, map),
                readonly: reference.readonly,
            })),
            Nullable(inner) => Nullable(Box::new(Self::substitute_generics(inner, map))),
            TraitObject(obj) => TraitObject(obj.clone()),
            Unknown => Unknown,
            Unit => Unit,
            String => String,
            Str => Str,
        }
    }


    pub(super) fn emit_call_resolution_error(
        &mut self,
        error: CallResolutionError,
        info: &CallBindingInfo,
        span: Option<Span>,
    ) {
        let target = self.describe_call_target(info);
        let message = match error {
            CallResolutionError::NoCandidates => {
                format!("cannot resolve call target for `{target}`")
            }
            CallResolutionError::NoMatch(candidates) => {
                if candidates.is_empty() {
                    format!("no overload of `{target}` matches the provided arguments")
                } else {
                    format!(
                        "no overload of `{target}` matches the provided arguments; available: {}",
                        candidates.join(", ")
                    )
                }
            }
            CallResolutionError::Ambiguous(candidates) => format!(
                "call to `{target}` is ambiguous; matching overloads: {}",
                candidates.join(", ")
            ),
        };
        self.diagnostics.push(LoweringDiagnostic { message, span });
    }


    pub(super) fn describe_call_target(&self, info: &CallBindingInfo) -> String {
        if let Some(hint) = &info.canonical_hint {
            return hint.clone();
        }
        if info.is_constructor {
            return info
                .static_owner
                .clone()
                .unwrap_or_else(|| "<constructor>".into());
        }
        if let Some(member) = &info.member_name {
            if let Some(owner) = info
                .receiver_owner
                .as_ref()
                .or(info.static_owner.as_ref())
            {
                return format!("{owner}::{member}");
            }
            return member.clone();
        }
        if let Some(candidate) = info.pending_candidates.first() {
            return candidate.clone();
        }
        "<call>".into()
    }


    pub(super) fn push_function_overloads<'b>(
        &'b self,
        name: &str,
        out: &mut Vec<&'b FunctionSymbol>,
        seen: &mut HashSet<String>,
    ) {
        if let Some(symbols) = self.symbol_index.function_overloads(name) {
            for symbol in symbols {
                if seen.insert(symbol.internal_name.clone()) {
                    out.push(symbol);
                }
            }
        }
    }


    pub(super) fn bind_implicit_receiver(
        &mut self,
        call_info: &mut CallBindingInfo,
    ) -> Option<Operand> {
        if call_info.member_name.is_some()
            || call_info.receiver_owner.is_some()
            || call_info.is_constructor
        {
            return None;
        }
        let Some(owner) = self.current_self_type_name() else {
            return None;
        };
        let Some(self_local) = self.lookup_name("self") else {
            return None;
        };
        let has_instance_candidate = self
            .overload_candidate_names(call_info)
            .iter()
            .any(|name| {
                self.symbol_index
                    .function_overloads(name)
                    .is_some_and(|symbols| {
                        symbols.iter().any(|symbol| {
                            !symbol.is_static
                                && symbol
                                    .qualified
                                    .rsplit_once("::")
                                    .map(|(symbol_owner, _)| {
                                        symbol_owner == owner
                                            || self.inherits_from(owner.as_str(), symbol_owner)
                                    })
                                    .unwrap_or(false)
                        })
                    })
            });
        if !has_instance_candidate {
            return None;
        }
        let mut place = Place::new(self_local);
        self.normalise_place(&mut place);
        call_info.receiver_owner = Some(owner);
        Some(Operand::Copy(place))
    }


    pub(super) fn record_thread_spawn_constraints(
        &mut self,
        func_operand: &Operand,
        info: &CallBindingInfo,
        args: &[EvaluatedArg],
        has_receiver: bool,
        span: Option<Span>,
    ) {
        let Some(canonical) = self.canonical_name_for_call(func_operand, info) else {
            return;
        };
        let lowered = canonical.to_ascii_lowercase();
        let is_thread_spawn = lowered == "std::thread::thread::spawn";
        let is_builder_spawn = lowered == "std::thread::threadbuilder::spawn";
        if !is_thread_spawn && !is_builder_spawn {
            return;
        }
        let arg_index = if has_receiver { 1 } else { 0 };
        if args.len() <= arg_index {
            return;
        }
        let Some(Ty::Arc(arc_ty)) = self.operand_ty(&args[arg_index].operand) else {
            return;
        };
        let payload_ty = (*arc_ty.element).clone();
        let ty_name = payload_ty.canonical_name();
        self.constraints.push(TypeConstraint::new(
            ConstraintKind::RequiresAutoTrait {
                function: self.function_name.clone(),
                target: "Thread::Spawn payload".to_string(),
                ty: ty_name,
                trait_kind: AutoTraitKind::ThreadSafe,
                origin: AutoTraitConstraintOrigin::ThreadSpawn,
            },
            span,
        ));
        if let ThreadRuntimeMode::Unsupported { backend } = self.thread_runtime_mode {
            self.constraints.push(TypeConstraint::new(
                ConstraintKind::ThreadingBackendAvailable {
                    function: self.function_name.clone(),
                    backend: backend.to_string(),
                    call: canonical,
                },
                span,
            ));
        }
    }


    pub(super) fn overload_candidate_names(&self, info: &CallBindingInfo) -> Vec<String> {
        let mut names = Vec::new();
        if let Some(hint) = &info.canonical_hint {
            names.push(Self::canonical_overload_key(hint));
        }
        names.extend(info.pending_candidates.iter().cloned());
        if names.is_empty() {
            if let Some(member) = &info.member_name {
                if let Some(owner) = info
                    .receiver_owner
                    .as_ref()
                    .or(info.static_owner.as_ref())
                {
                    names.push(format!("{owner}::{member}"));
                }
            }
        }
        names
    }


    pub(super) fn canonical_overload_key(name: &str) -> String {
        match name.find('#') {
            Some(index) => name[..index].to_string(),
            None => name.to_string(),
        }
    }


    pub(super) fn func_operand_owner_repr(func_operand: &Operand) -> Option<String> {
        match func_operand {
            Operand::Pending(pending) => {
                if let Some((owner, _)) = pending.repr.rsplit_once('.') {
                    Some(owner.to_string())
                } else if let Some((owner, _)) = pending.repr.rsplit_once("::") {
                    Some(owner.to_string())
                } else {
                    None
                }
            }
            Operand::Const(constant) => {
                let symbol = constant.symbol_name()?;
                symbol
                    .rsplit_once("::")
                    .map(|(owner, _)| owner.to_string())
            }
            _ => None,
        }
    }


    pub(super) fn canonical_name_for_call(
        &self,
        func_operand: &Operand,
        info: &CallBindingInfo,
    ) -> Option<String> {
        let raw = match func_operand {
            Operand::Const(constant) => constant.symbol_name(),
            Operand::Pending(pending) => Some(pending.repr.as_str()),
            _ => None,
        }
        .or_else(|| info.canonical_hint.as_deref())?;
        let name = raw.replace('.', "::");
        if let Some(owner) = info.receiver_owner.as_deref() {
            if let Some(member) = info.member_name.as_deref() {
                return Some(format!("{owner}::{member}"));
            }
        }
        Some(name)
    }


    pub(super) fn argument_mode_matches(
        expected: ParamMode,
        modifier: Option<CallArgumentModifier>,
    ) -> bool {
        match (expected, modifier) {
            (ParamMode::Value, None) => true,
            (ParamMode::In, Some(CallArgumentModifier::In)) => true,
            (ParamMode::In, Some(CallArgumentModifier::Ref)) => true,
            (ParamMode::Ref, Some(CallArgumentModifier::Ref)) => true,
            (ParamMode::Out, Some(CallArgumentModifier::Out)) => true,
            (ParamMode::Value, Some(CallArgumentModifier::In | CallArgumentModifier::Ref)) => true,
            (ParamMode::Value, Some(_)) => false,
            _ => false,
        }
    }


    pub(super) fn argument_type_matches(
        &self,
        expected: &Ty,
        arg: &EvaluatedArg,
        generic_params: &HashSet<String>,
        bindings: &mut HashMap<String, Ty>,
    ) -> Option<(usize, usize)> {
        if matches!(expected, Ty::Unknown) || expected.is_var_placeholder() {
            return Some((1, 0));
        }
        let Some(mut actual) = self.operand_ty(&arg.operand) else {
            return None;
        };
        if matches!(actual, Ty::Unknown) {
            return None;
        }
        if !matches!(expected, Ty::Ref(_)) {
            if let Ty::Ref(reference) = actual {
                actual = reference.element.clone();
            }
        }
        let expected_is_generic_placeholder = match expected {
            Ty::Named(named) => named.args.is_empty() && generic_params.contains(&named.name),
            _ => false,
        };
        if let Ty::Nullable(inner) = expected {
            if inner.as_ref().canonical_name() == actual.canonical_name() {
                // Passing a non-null value where a nullable is expected should be preferred over
                // cross-string compatibility edges (e.g. `string`->`str?`).
                return Some((3, 0));
            }
        }
        if Self::string_compatible(expected, &actual) {
            // `string` <-> `str` is an allowed compatibility edge, but it is not an exact match.
            // Keep it stronger than generic catch-all overloads, while still allowing exact
            // `string`/`string?` matches to win when available.
            return Some((2, 0));
        }
        let expected_name = expected.canonical_name();
        let actual_name = actual.canonical_name();
        let base_match = expected_name
            .rsplit("::")
            .next()
            .zip(actual_name.rsplit("::").next())
            .is_some_and(|(a, b)| a == b);
        if base_match && expected_name == actual_name {
            return Some((3, 1));
        }
        if matches!(
            arg.modifier,
            Some(
                CallArgumentModifier::Ref
                    | CallArgumentModifier::In
                    | CallArgumentModifier::Out
            )
        ) {
            if let Ty::Ref(reference) = actual {
                actual = reference.element.clone();
            }
            if matches!(actual, Ty::Unknown) {
                return Some((1, 0));
            }
        }
        if expected_name == actual_name {
            return Some((3, 1));
        }

        if let Some(strength) = self.span_conversion_strength(&actual, expected) {
            if matches!(strength, SpanConversionStrength::Implicit) {
                Self::bind_span_generics(expected, &actual, generic_params, bindings);
                return Some((2, 0));
            }
        }
        if Self::ty_matches(expected, &actual, generic_params, bindings) {
            // Prefer non-placeholder matches over generic catch-all parameters.
            return Some((if expected_is_generic_placeholder { 1 } else { 2 }, 0));
        }
        if Self::numeric_compatible(expected, &actual) {
            return Some((1, 0));
        }
        if matches!(expected, Ty::Pointer(_)) && matches!(actual, Ty::Pointer(_)) {
            return Some((1, 0));
        }
        None
    }

    fn ty_matches(
        expected: &Ty,
        actual: &Ty,
        generic_params: &HashSet<String>,
        bindings: &mut HashMap<String, Ty>,
    ) -> bool {
        let names_equivalent = |a: &str, b: &str| {
            if a == b {
                return true;
            }
            let a_base = a.rsplit("::").next().unwrap_or(a);
            let b_base = b.rsplit("::").next().unwrap_or(b);
            a_base == b_base
        };
        match expected {
            Ty::Unknown => true,
            Ty::Vector(expected_vec) => {
                if let Ty::Vector(actual_vec) = actual {
                    expected_vec.lanes == actual_vec.lanes
                        && Self::ty_matches(
                            &expected_vec.element,
                            &actual_vec.element,
                            generic_params,
                            bindings,
                        )
                } else {
                    false
                }
            }
            Ty::Named(named) => {
                if named.name == "Self" {
                    return true;
                }
                if named.args.is_empty() && generic_params.contains(&named.name) {
                    if let Some(bound) = bindings.get(&named.name) {
                        return names_equivalent(&bound.canonical_name(), &actual.canonical_name());
                    }
                    bindings.insert(named.name.clone(), actual.clone());
                    return true;
                }
                if let Ty::Named(actual_named) = actual {
                    if actual_named.name == "Self" {
                        return true;
                    }
                    if !names_equivalent(&named.name, &actual_named.name)
                        || named.args.len() != actual_named.args.len()
                    {
                        return false;
                    }
                    for (expected_arg, actual_arg) in named.args.iter().zip(actual_named.args.iter()) {
                        match (expected_arg, actual_arg) {
                            (GenericArg::Type(exp_ty), GenericArg::Type(act_ty)) => {
                                if !Self::ty_matches(exp_ty, act_ty, generic_params, bindings) {
                                    return false;
                                }
                            }
                            _ => {
                                if expected_arg != actual_arg {
                                    return false;
                                }
                            }
                        }
                    }
                    true
                } else {
                    names_equivalent(&expected.canonical_name(), &actual.canonical_name())
                }
            }
            Ty::Array(exp) => match actual {
                Ty::Array(act) => {
                    exp.rank == act.rank
                        && Self::ty_matches(&exp.element, &act.element, generic_params, bindings)
                }
                _ => false,
            },
            Ty::Vec(exp) => match actual {
                Ty::Vec(act) => Self::ty_matches(&exp.element, &act.element, generic_params, bindings),
                _ => false,
            },
            Ty::Span(exp) => match actual {
                Ty::Span(act) => {
                    matches!(&*act.element, Ty::Unknown)
                        || Self::ty_matches(&exp.element, &act.element, generic_params, bindings)
                        || exp.element.canonical_name().rsplit("::").last()
                            == act.element.canonical_name().rsplit("::").last()
                }
                _ => false,
            },
            Ty::ReadOnlySpan(exp) => match actual {
                Ty::ReadOnlySpan(act) => {
                    matches!(&*act.element, Ty::Unknown)
                        || Self::ty_matches(&exp.element, &act.element, generic_params, bindings)
                        || exp.element.canonical_name().rsplit("::").last()
                            == act.element.canonical_name().rsplit("::").last()
                }
                _ => false,
            },
            Ty::Rc(exp) => match actual {
                Ty::Rc(act) => Self::ty_matches(&exp.element, &act.element, generic_params, bindings),
                _ => false,
            },
            Ty::Arc(exp) => match actual {
                Ty::Arc(act) => Self::ty_matches(&exp.element, &act.element, generic_params, bindings),
                _ => false,
            },
            Ty::Pointer(exp) => match actual {
                Ty::Pointer(act) => {
                    let mutability_ok = exp.mutable == act.mutable
                        || (!exp.mutable && act.mutable);
                    mutability_ok
                        && Self::ty_matches(&exp.element, &act.element, generic_params, bindings)
                }
                _ => false,
            },
            Ty::Ref(exp) => match actual {
                Ty::Ref(act) => {
                    exp.readonly == act.readonly
                        && Self::ty_matches(&exp.element, &act.element, generic_params, bindings)
                }
                _ => false,
            },
            Ty::Tuple(exp) => match actual {
                Ty::Tuple(act) => {
                    if exp.elements.len() != act.elements.len() {
                        return false;
                    }
                    exp.elements
                        .iter()
                        .zip(act.elements.iter())
                        .all(|(e, a)| Self::ty_matches(e, a, generic_params, bindings))
                }
                _ => false,
            },
            Ty::Fn(exp) => match actual {
                Ty::Fn(act) => {
                    if exp.params.len() != act.params.len() || exp.abi != act.abi {
                        return false;
                    }
                    exp.params
                        .iter()
                        .zip(act.params.iter())
                        .all(|(e, a)| Self::ty_matches(e, a, generic_params, bindings))
                        && Self::ty_matches(&exp.ret, &act.ret, generic_params, bindings)
                }
                _ => false,
            },
            Ty::Nullable(exp) => match actual {
                Ty::Nullable(act) => Self::ty_matches(exp, act, generic_params, bindings),
                _ => Self::ty_matches(exp, actual, generic_params, bindings),
            },
            Ty::TraitObject(_) => names_equivalent(&expected.canonical_name(), &actual.canonical_name()),
            Ty::Unit | Ty::String | Ty::Str => names_equivalent(&expected.canonical_name(), &actual.canonical_name()),
        }
    }

    fn bind_span_generics(
        expected: &Ty,
        actual: &Ty,
        generic_params: &HashSet<String>,
        bindings: &mut HashMap<String, Ty>,
    ) {
        let expected_element = match expected {
            Ty::Span(span) => Some((*span.element).clone()),
            Ty::ReadOnlySpan(span) => Some((*span.element).clone()),
            _ => None,
        };
        let actual_element = Self::span_like_element(actual);
        if let (Some(expected), Some(actual)) = (expected_element, actual_element) {
            let _ = Self::ty_matches(&expected, &actual, generic_params, bindings);
        }
    }

    fn span_like_element(ty: &Ty) -> Option<Ty> {
        match ty {
            Ty::Array(array) if array.rank == 1 => Some((*array.element).clone()),
            Ty::Span(span) => Some((*span.element).clone()),
            Ty::ReadOnlySpan(span) => Some((*span.element).clone()),
            Ty::String | Ty::Str => Some(Ty::named("byte")),
            _ => None,
        }
    }

    fn numeric_compatible(expected: &Ty, actual: &Ty) -> bool {
        Self::is_integral_ty(expected) && Self::is_integral_ty(actual)
    }

    fn string_compatible(expected: &Ty, actual: &Ty) -> bool {
        let expected = match expected {
            Ty::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        let actual = match actual {
            Ty::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        matches!(expected, Ty::String) && matches!(actual, Ty::Str)
            || matches!(expected, Ty::Str) && matches!(actual, Ty::String)
    }

    fn is_integral_ty(ty: &Ty) -> bool {
        match ty {
            Ty::Named(named) => matches!(
                named.name.as_str(),
                "byte"
                    | "sbyte"
                    | "short"
                    | "ushort"
                    | "int"
                    | "uint"
                    | "long"
                    | "ulong"
                    | "nint"
                    | "nuint"
                    | "usize"
                    | "isize"
                    | "Std::Numeric::Decimal::DecimalRoundingEncoding"
            ),
            Ty::Nullable(inner) => Self::is_integral_ty(inner),
            _ => false,
        }
    }

    fn generic_param_names(
        &self,
        symbol: &FunctionSymbol,
        call_info: &CallBindingInfo,
    ) -> HashSet<String> {
        let declared_len = call_info
            .method_type_args
            .as_ref()
            .map(|args| args.len())
            .unwrap_or(0);
        let declared = self.method_generic_param_names(&symbol.qualified, declared_len);
        if !declared.is_empty() {
            return declared.into_iter().collect();
        }
        let mut inferred = HashSet::new();
        for ty in &symbol.signature.params {
            Self::gather_generic_candidates(ty, &mut inferred);
        }
        Self::gather_generic_candidates(&symbol.signature.ret, &mut inferred);
        inferred
    }

    fn gather_generic_candidates(ty: &Ty, out: &mut HashSet<String>) {
        match ty {
            Ty::Named(named) => {
                let looks_generic = !named.name.contains("::")
                    && named
                        .name
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
                if looks_generic {
                    out.insert(named.name.clone());
                }
                for arg in &named.args {
                    if let GenericArg::Type(inner) = arg {
                        Self::gather_generic_candidates(inner, out);
                    }
                }
            }
            Ty::Array(array) => Self::gather_generic_candidates(&array.element, out),
            Ty::Vec(vec_ty) => Self::gather_generic_candidates(&vec_ty.element, out),
            Ty::Vector(vec_ty) => Self::gather_generic_candidates(&vec_ty.element, out),
            Ty::Span(span_ty) => Self::gather_generic_candidates(&span_ty.element, out),
            Ty::ReadOnlySpan(span_ty) => Self::gather_generic_candidates(&span_ty.element, out),
            Ty::Rc(rc_ty) => Self::gather_generic_candidates(&rc_ty.element, out),
            Ty::Arc(arc_ty) => Self::gather_generic_candidates(&arc_ty.element, out),
            Ty::Tuple(tuple) => {
                for element in &tuple.elements {
                    Self::gather_generic_candidates(element, out);
                }
            }
            Ty::Fn(fn_ty) => {
                for param in &fn_ty.params {
                    Self::gather_generic_candidates(param, out);
                }
                Self::gather_generic_candidates(&fn_ty.ret, out);
            }
            Ty::Pointer(ptr) => Self::gather_generic_candidates(&ptr.element, out),
            Ty::Ref(reference) => Self::gather_generic_candidates(&reference.element, out),
            Ty::Nullable(inner) => Self::gather_generic_candidates(inner, out),
            Ty::TraitObject(obj) => {
                for name in &obj.traits {
                    out.insert(name.clone());
                }
            }
            Ty::Unknown | Ty::Unit | Ty::String | Ty::Str => {}
        }
    }


        pub(super) fn lower_call_callee(&mut self, callee: &ExprNode, span: Option<Span>) -> Option<Operand> {
            if let ExprNode::Identifier(name) = callee {
                if let Some(owner) = self.current_self_type_name() {
                    let qualified = format!("{owner}::{name}");
                    if let Some(symbols) = self.symbol_index.function_overloads(&qualified) {
                        if symbols.len() == 1 {
                            return Some(Operand::Const(ConstOperand::new(ConstValue::Symbol(
                                symbols[0].internal_name.clone(),
                            ))));
                        }
                        let candidates = symbols
                            .iter()
                            .map(|symbol| PendingFunctionCandidate {
                                qualified: symbol.qualified.clone(),
                                signature: symbol.signature.clone(),
                                is_static: symbol.is_static,
                            })
                            .collect::<Vec<_>>();
                        return Some(Operand::Pending(PendingOperand {
                            category: ValueCategory::Pending,
                            repr: name.to_string(),
                            span,
                            info: Some(Box::new(PendingOperandInfo::FunctionGroup {
                                path: qualified,
                                candidates,
                                receiver: None,
                            })),
                        }));
                    }
                }

                if let Some(index) = self.lookup_local_function_entry(name) {
                    let operand = self.instantiate_local_function(index, span);
                    if std::env::var("CHIC_DEBUG_CALLS").is_ok() {
                        eprintln!("[call-callee] name={name} operand={operand:?}");
                    }
                return operand;
            }
            if let Some(id) = self.lookup_name(name) {
                let mut place = Place::new(id);
                self.normalise_place(&mut place);
                let operand = Some(Operand::Copy(place));
                if std::env::var("CHIC_DEBUG_CALLS").is_ok() {
                    let local_name = self
                        .locals
                        .get(id.0)
                        .and_then(|decl| decl.name.as_deref())
                        .unwrap_or("<unnamed>");
                    eprintln!(
                        "[call-callee] name={name} operand={operand:?} local={} ({local_name})",
                        id.0
                    );
                }
                return operand;
            }
            if let Some(func_operand) = self.resolve_function_operand(name, span) {
                if std::env::var("CHIC_DEBUG_CALLS").is_ok() {
                    eprintln!("[call-callee] name={name} operand={func_operand:?}");
                }
                return Some(func_operand);
            }
            let operand = self.lower_identifier_expr(name, span);
            if std::env::var("CHIC_DEBUG_CALLS").is_ok() {
                if let Some(Operand::Copy(place)) | Some(Operand::Move(place)) =
                    operand.as_ref()
                {
                    let local = place.local.0;
                    let local_name = self
                        .locals
                        .get(local)
                        .and_then(|decl| decl.name.as_deref())
                        .unwrap_or("<unnamed>");
                    eprintln!(
                        "[call-callee] name={name} operand={operand:?} local={local} ({local_name})"
                    );
                    for (scope_idx, scope) in self.scopes.iter().enumerate() {
                        if let Some(id) = scope.bindings.get(name) {
                            let bound_name = self
                                .locals
                                .get(id.0)
                                .and_then(|decl| decl.name.as_deref())
                                .unwrap_or("<unnamed>");
                            eprintln!(
                                "  [call-callee] scope {scope_idx} binds {name} -> {id:?} ({bound_name})"
                            );
                        }
                    }
                } else {
                    eprintln!("[call-callee] name={name} operand={operand:?}");
                }
            }
            return operand;
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "unsupported call callee".into(),
                span,
                            });
            None
        }
    }


    pub(super) fn lower_closure_to_fn_ptr(
        &mut self,
        receiver: Operand,
        span: Option<Span>,
                capture_result: bool,
    ) -> Option<Operand> {
        let type_name = match self.operand_type_name(&receiver) {
            Some(name) => name,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "cannot determine closure type for `.to_fn_ptr()`".into(),
                    span,
                                    });
                return None;
            }
        };

        let Some(info) = self.closure_registry.get(&type_name).cloned() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "`.to_fn_ptr()` is not supported for `{type_name}` (closure metadata missing)"
                ),
                span,
                            });
            return None;
        };

        let pointer_operand = self.convert_closure_operand_to_fn_ptr(receiver, &info, span)?;
        if capture_result {
            Some(pointer_operand)
        } else {
            Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
        }
    }


    pub(super) fn closure_symbol_for_operand(&self, operand: &Operand) -> Option<FunctionSymbol> {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => place,
            _ => return None,
        };
        let type_name = self.place_type_name(place)?;
        let info = self.closure_registry.get(&type_name)?;
        Some(FunctionSymbol {
            qualified: info.invoke_symbol.clone(),
            internal_name: info.invoke_symbol.clone(),
            signature: info.fn_ty.clone(),
            params: info.params.clone(),
            is_unsafe: false,
            is_static: false,
            visibility: crate::frontend::ast::Visibility::Private,
            namespace: None,
            owner: None,
        })
    }


}
