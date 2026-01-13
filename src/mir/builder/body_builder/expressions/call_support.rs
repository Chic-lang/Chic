use super::*;
use crate::frontend::import_resolver::ImportResolution;
use crate::mir::Ty;
use std::collections::HashSet;

fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CallBindingInfo {
    pub(crate) canonical_hint: Option<String>,
    pub(crate) pending_candidates: Vec<String>,
    pub(crate) member_name: Option<String>,
    pub(crate) receiver_owner: Option<String>,
    pub(crate) static_owner: Option<String>,
    pub(crate) static_base: Option<String>,
    pub(crate) is_constructor: bool,
    pub(crate) resolved_symbol: Option<FunctionSymbol>,
    pub(crate) force_base_receiver: bool,
    pub(crate) method_type_args: Option<Vec<Ty>>,
}

#[derive(Clone)]
pub(crate) struct EvaluatedArg {
    pub(crate) operand: Operand,
    pub(crate) modifier: Option<CallArgumentModifier>,
    pub(crate) modifier_span: Option<Span>,
    pub(crate) name: Option<String>,
    pub(crate) name_span: Option<Span>,
    pub(crate) span: Option<Span>,
    pub(crate) value_span: Option<Span>,
    pub(crate) inline_binding: Option<InlineBindingMeta>,
    pub(crate) param_slot: Option<usize>,
}

#[derive(Clone)]
pub(crate) struct InlineBindingMeta {
    pub(crate) local: LocalId,
}

#[derive(Debug)]
pub(crate) enum BindFailure {
    UnknownName {
        name: String,
        span: Option<Span>,
    },
    DuplicateName {
        name: String,
        span: Option<Span>,
    },
    TooManyArguments {
        expected: usize,
        found: usize,
        span: Option<Span>,
    },
    MissingArguments {
        missing: Vec<String>,
    },
    ModifierMismatch {
        param: String,
        expected: ParamMode,
        found: Option<CallArgumentModifier>,
        span: Option<Span>,
    },
}

body_builder_impl! {

    pub(crate) fn bind_named_arguments(
        &mut self,
        func_operand: &mut Operand,
        evaluated_args: Vec<EvaluatedArg>,
        info: CallBindingInfo,
        span: Option<Span>,
            ) -> Option<(Vec<EvaluatedArg>, Option<String>)> {
        if let Some(symbol) = info.resolved_symbol.as_ref() {
            match Self::match_candidate(symbol, &evaluated_args) {
                Ok(mapping) => {
                    let mut storage: Vec<Option<EvaluatedArg>> =
                        evaluated_args.into_iter().map(Some).collect();
                    let mut ordered = Vec::with_capacity(mapping.len());
                    for (param_index, arg_index) in mapping.into_iter().enumerate() {
                        let Some(arg_index) = arg_index else { continue };
                        if let Some(arg) = storage[arg_index].as_mut() {
                            arg.param_slot = Some(param_index);
                        }
                        if let Some(arg) = storage[arg_index].take() {
                            ordered.push(arg);
                        }
                    }
                    if symbol.signature.variadic {
                        for arg in storage.into_iter().flatten() {
                            ordered.push(arg);
                        }
                    }
                    return Some((ordered, None));
                }
                Err(err) => {
                    self.emit_bind_error(
                        &evaluated_args,
                        vec![err],
                        span,
                        info.member_name.as_deref(),
                    );
                    return None;
                }
            }
        }

        let mut candidates: Vec<&FunctionSymbol> = Vec::new();
        let mut seen = HashSet::new();

        if let Some(name) = &info.canonical_hint {
            if let Some(symbols) = self.symbol_index.function_overloads(name) {
                for symbol in symbols {
                    if seen.insert(symbol.qualified.clone()) {
                        candidates.push(symbol);
                    }
                }
            }
        }

        for name in &info.pending_candidates {
            if let Some(symbols) = self.symbol_index.function_overloads(name) {
                for symbol in symbols {
                    if seen.insert(symbol.qualified.clone()) {
                        candidates.push(symbol);
                    }
                }
            }
        }

        if let Some(member) = &info.member_name {
            if let Some(owner) = info
                .receiver_owner
                .as_ref()
                .or(info.static_owner.as_ref())
            {
                let qualified = format!("{owner}::{member}");
                if let Some(symbols) = self.symbol_index.function_overloads(&qualified) {
                    for symbol in symbols {
                        if seen.insert(symbol.qualified.clone()) {
                            candidates.push(symbol);
                        }
                    }
                }
            }
        }

        if info.is_constructor {
            if let Some(owner) = info.static_owner.as_ref() {
                for symbol in self.symbol_index.constructor_overloads(owner) {
                    if seen.insert(symbol.qualified.clone()) {
                        candidates.push(symbol);
                    }
                }
            }
        }

        if candidates.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot resolve call target for named arguments".into(),
                span,
                            });
            return None;
        }

        let mut successes: Vec<(&FunctionSymbol, Vec<Option<usize>>)> = Vec::new();
        let mut failures: Vec<BindFailure> = Vec::new();

        for candidate in candidates {
            match Self::match_candidate(candidate, &evaluated_args) {
                Ok(mapping) => successes.push((candidate, mapping)),
                Err(err) => failures.push(err),
            }
        }

        if successes.is_empty() {
            self.emit_bind_error(&evaluated_args, failures, span, info.member_name.as_deref());
            return None;
        }

        if successes.len() > 1 {
            let names = successes
                .iter()
                .map(|(symbol, _)| symbol.qualified.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("named arguments are ambiguous; matching overloads: {names}"),
                span,
                            });
            return None;
        }

        let (symbol, mapping) = successes.remove(0);
        if symbol.is_unsafe && self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "call to unsafe function `{}` requires an `unsafe` block",
                    symbol.qualified
                ),
                span,
                            });
        }
        let mut storage: Vec<Option<EvaluatedArg>> =
            evaluated_args.into_iter().map(Some).collect();
        let mut ordered = Vec::with_capacity(mapping.len());
        for (param_index, arg_index) in mapping.into_iter().enumerate() {
            let Some(arg_index) = arg_index else { continue };
            if let Some(arg) = storage[arg_index].as_mut() {
                arg.param_slot = Some(param_index);
            }
            if let Some(arg) = storage[arg_index].take() {
                ordered.push(arg);
            }
        }

        let canonical = if matches!(func_operand, Operand::Pending(_)) {
            Some(symbol.qualified.clone())
        } else {
            None
        };

        Some((ordered, canonical))
    }
    pub(crate) fn validate_argument_modes_for_call(
        &mut self,
        func_operand: &Operand,
        args_meta: &[EvaluatedArg],
        has_receiver: bool,
        span: Option<Span>,
            ) {
        let Some(symbol) = (match func_operand {
            Operand::Const(constant) => constant.symbol_name(),
            _ => None,
        }) else {
            return;
        };

        let Some(overloads) = self.symbol_index.function_overloads(symbol) else {
            return;
        };
        let function = if overloads.len() == 1 {
            &overloads[0]
        } else {
            return;
        };

        if function.is_unsafe && self.unsafe_depth == 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "call to unsafe function `{}` requires an `unsafe` block",
                    function.qualified
                ),
                span,
                            });
        }

        let params = &function.params;
        let expected_args = params.len() + usize::from(has_receiver);
        if args_meta.len() != expected_args {
            // Additional synthetic arguments (e.g. closure captures) are present; skip validation for now.
            return;
        }

        let start = usize::from(has_receiver);
        if args_meta.len() < start + params.len() {
            return;
        }
        for (index, param) in params.iter().enumerate() {
            let arg = &args_meta[start + index];
            let expected = param.mode;
            let actual = match arg.modifier {
                Some(CallArgumentModifier::In) => ParamMode::In,
                Some(CallArgumentModifier::Ref) => ParamMode::Ref,
                Some(CallArgumentModifier::Out) => ParamMode::Out,
                None => ParamMode::Value,
            };

            if expected != actual {
                let diag_span = arg
                    .modifier_span
                    .or(arg.span)
                    .or(arg.value_span)
                    .or(span);
                let expected_kw = Self::param_mode_display(expected);
                let found_kw = Self::argument_modifier_display(arg.modifier);
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "argument `{}` must be passed with {expected_kw}, but call uses {found_kw}",
                        param.name
                    ),
                    span: diag_span,
                                    });
            }
            if let Some(binding) = &arg.inline_binding {
                if let Some(param_ty) = function.signature.params.get(index) {
                    self.hint_local_ty(binding.local, param_ty.clone());
                }
            }
        }
    }
    pub(crate) fn emit_bind_error(
        &mut self,
        _args: &[EvaluatedArg],
        failures: Vec<BindFailure>,
        span: Option<Span>,
                member_name: Option<&str>,
    ) {
        if let Some(failure) = failures.into_iter().next() {
            match failure {
                BindFailure::UnknownName { name, span: name_span } => {
                    let diag_span = name_span.or(span);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("unknown named argument '{name}'"),
                        span: diag_span,
                                            });
                }
                BindFailure::DuplicateName { name, span: dup_span } => {
                    let diag_span = dup_span.or(span);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("named argument '{name}' specified multiple times"),
                        span: diag_span,
                                            });
                }
                BindFailure::TooManyArguments {
                    expected,
                    found,
                    span: arg_span,
                                    } => {
                    let diag_span = arg_span.or(span);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "too many arguments for call; expected {expected} but found {found}"
                        ),
                        span: diag_span,
                                            });
                }
                BindFailure::MissingArguments { missing } => {
                    let joined = missing.join(", ");
                    let target = member_name.unwrap_or("call");
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "call to `{target}` is missing argument(s): {joined}"
                        ),
                        span,
                                            });
                }
                BindFailure::ModifierMismatch {
                    param,
                    expected,
                    found,
                    span: mod_span,
                                    } => {
                    let diag_span = mod_span.or(span);
                    let expected_kw = Self::param_mode_display(expected);
                    let found_kw = Self::argument_modifier_display(found);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "argument `{param}` must be passed with {expected_kw}, but call uses {found_kw}"
                        ),
                        span: diag_span,
                                            });
                }
            }
        } else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot match named arguments to parameter list".into(),
                span,
                            });
        }
    }
    pub(crate) fn match_candidate(
        candidate: &FunctionSymbol,
        args: &[EvaluatedArg],
    ) -> Result<Vec<Option<usize>>, BindFailure> {
        let param_len = candidate.params.len();
        let mut mapping: Vec<Option<usize>> = vec![None; param_len];
        let mut positional_index = 0usize;
        let variadic = candidate.signature.variadic;

        for (arg_index, arg) in args.iter().enumerate() {
            if let Some(name) = &arg.name {
                if let Some(position) = candidate
                    .params
                    .iter()
                    .position(|param| param.name == *name)
                {
                    if mapping[position].is_some() {
                        return Err(BindFailure::DuplicateName {
                            name: name.clone(),
                            span: arg.name_span,
                                                    });
                    }
                    Self::check_argument_modifier(&candidate.params[position], arg)?;
                    mapping[position] = Some(arg_index);
                } else {
                    return Err(BindFailure::UnknownName {
                        name: name.clone(),
                        span: arg.name_span,
                                            });
                }
            } else {
                while positional_index < param_len && mapping[positional_index].is_some() {
                    positional_index += 1;
                }
                if positional_index >= param_len {
                    if variadic {
                        continue;
                    }
                    return Err(BindFailure::TooManyArguments {
                        expected: param_len,
                        found: args.len(),
                        span: arg.span,
                    });
                }
                Self::check_argument_modifier(&candidate.params[positional_index], arg)?;
                mapping[positional_index] = Some(arg_index);
                positional_index += 1;
            }
        }

        let missing = candidate
            .params
            .iter()
            .enumerate()
            .filter_map(|(index, param)| {
                if mapping[index].is_none() && !param.has_default {
                    Some(param.name.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(BindFailure::MissingArguments { missing });
        }
        // Leave variadic arguments in their original order after the fixed arguments.
        Ok(mapping.into_iter().collect())
    }
    pub(crate) fn check_argument_modifier(
        param: &FunctionParamSymbol,
        arg: &EvaluatedArg,
    ) -> Result<(), BindFailure> {
        let expected = param.mode;
        let actual = match arg.modifier {
            Some(CallArgumentModifier::In) => ParamMode::In,
            Some(CallArgumentModifier::Ref) => ParamMode::Ref,
            Some(CallArgumentModifier::Out) => ParamMode::Out,
            None => ParamMode::Value,
        };
        if expected != actual {
            let span = arg
                .modifier_span
                .or(arg.span)
                .or(arg.value_span)
                .or(arg.name_span);
            return Err(BindFailure::ModifierMismatch {
                param: param.name.clone(),
                expected,
                found: arg.modifier,
                span,
                            });
        }
        Ok(())
    }
    pub(crate) fn param_mode_display(mode: ParamMode) -> &'static str {
        match mode {
            ParamMode::Value => "no modifier",
            ParamMode::In => "`in`",
            ParamMode::Ref => "`ref`",
            ParamMode::Out => "`out`",
        }
    }
    pub(crate) fn argument_modifier_display(modifier: Option<CallArgumentModifier>) -> &'static str {
        match modifier {
            Some(CallArgumentModifier::In) => "`in`",
            Some(CallArgumentModifier::Ref) => "`ref`",
            Some(CallArgumentModifier::Out) => "`out`",
            None => "no modifier",
        }
    }
    pub(crate) fn resolve_type_owner_for_segments(&self, segments: &[String]) -> Option<String> {
        if segments.is_empty() {
            return None;
        }
        let cleaned = segments
            .iter()
            .map(|segment| segment.split('<').next().unwrap_or(segment).to_string())
            .collect::<Vec<_>>();
        if cleaned.len() == 1 {
            match cleaned[0].as_str() {
                "Vec" => return Some("Foundation::Collections::Vec".to_string()),
                "TimeZones" => return Some("Std::Datetime::TimeZones".to_string()),
                "Arc" => return Some("Std::Sync::Arc".to_string()),
                "Rc" => return Some("Std::Sync::Rc".to_string()),
                _ => {}
            }
        }
        let candidate = cleaned.join("::");
        let current_type = self.current_self_type_name();
        if let ImportResolution::Found(resolved) =
            self.import_resolver
                .resolve_type(&cleaned, self.namespace.as_deref(), current_type.as_deref(), |name| {
                    self.symbol_index.contains_type(name)
                })
        {
            return Some(resolved);
        }
        if let Some(resolved) = resolve_type_layout_name(
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
            &candidate,
        ) {
            return Some(resolved);
        }
        if cleaned.len() == 1 {
            if let Some(key) = self
                .type_layouts
                .resolve_type_key(&cleaned[0])
                .map(str::to_string)
            {
                return Some(key);
            }
        }
        if self.symbol_index.contains_type(&candidate) {
            return Some(candidate);
        }
        if let Some(namespace) = self.namespace.as_deref() {
            let qualified = format!("{namespace}::{candidate}");
            if self.symbol_index.contains_type(&qualified) {
                return Some(qualified);
            }
        }
        let requested = cleaned.clone();
        for ty in self.symbol_index.types() {
            let segments = ty
                .split("::")
                .map(|seg| strip_generics(seg))
                .collect::<Vec<_>>();
            if segments.len() >= requested.len() {
                if segments[segments.len() - requested.len()..] == requested {
                    return Some(ty.clone());
                }
            } else if requested.len() == 1 {
                // Single-segment fallback that still respects segment boundaries.
                if segments.last().is_some_and(|seg| seg == &requested[0]) {
                    return Some(ty.clone());
                }
            }
        }
        None
    }
}
