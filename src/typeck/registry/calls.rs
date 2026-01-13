use super::*;
use crate::mir::ParamMode;
use crate::mir::builder::FunctionSymbol;
use crate::syntax::expr::ExprNode;
use crate::syntax::expr::builders::CallArgument;
use std::collections::HashSet;

pub(crate) struct CallCandidateSet {
    pub(crate) display: String,
    pub(crate) candidates: Vec<FunctionSymbol>,
}

#[derive(Debug)]
pub(crate) struct CallMatch {
    pub(crate) score: usize,
}

#[derive(Debug)]
pub(crate) struct CallMismatch {
    pub(crate) message: String,
    pub(crate) span: Option<Span>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MemoryOrderVariant {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

impl MemoryOrderVariant {
    fn from_str(name: &str) -> Option<Self> {
        match name {
            "Relaxed" => Some(Self::Relaxed),
            "Acquire" => Some(Self::Acquire),
            "Release" => Some(Self::Release),
            "AcqRel" => Some(Self::AcqRel),
            "SeqCst" => Some(Self::SeqCst),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Relaxed => "Relaxed",
            Self::Acquire => "Acquire",
            Self::Release => "Release",
            Self::AcqRel => "AcqRel",
            Self::SeqCst => "SeqCst",
        }
    }

    fn failure_allowed(self, failure: Self) -> bool {
        match failure {
            Self::Relaxed => true,
            Self::Acquire => matches!(self, Self::Acquire | Self::AcqRel | Self::SeqCst),
            Self::SeqCst => matches!(self, Self::SeqCst),
            Self::Release | Self::AcqRel => false,
        }
    }
}

impl<'a> TypeChecker<'a> {
    pub(crate) fn validate_call_expression(
        &mut self,
        function_name: &str,
        callee: &ExprNode,
        args: &[CallArgument],
        span: Option<Span>,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        let inferred_owner = self.infer_call_owner(function_name);
        let owner_hint = if let Some(context) = context_type {
            if self.types.contains_key(context) {
                Some(context)
            } else {
                inferred_owner.as_deref()
            }
        } else {
            inferred_owner.as_deref()
        };
        let Some(candidates) = self.collect_call_candidates(callee, namespace, owner_hint) else {
            return;
        };
        self.resolve_overload_for_call(&candidates, args, span);
    }

    pub(crate) fn collect_call_candidates(
        &self,
        callee: &ExprNode,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> Option<CallCandidateSet> {
        let segments = expr_path_segments(callee)?;
        let display = self.expr_display(callee);
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut symbols = Vec::new();

        if segments.len() > 1 {
            let owner_segments = &segments[..segments.len() - 1];
            let member = segments.last().expect("member segment exists");
            let owner_candidate = owner_segments.join("::");
            let owner = self
                .resolve_call_owner_name(&owner_candidate, namespace, context_type)
                .unwrap_or(owner_candidate);
            let canonical = format!("{owner}::{member}");
            self.push_call_overloads(&canonical, &mut symbols, &mut seen);
        } else if segments.len() == 1 && segments[0].contains("::") {
            self.push_call_overloads(&segments[0], &mut symbols, &mut seen);
        }

        if segments.len() == 1 {
            if let Some(context) = context_type {
                let candidate = format!("{}::{}", context, segments[0]);
                self.push_call_overloads(&candidate, &mut symbols, &mut seen);
            }
        }

        if symbols.is_empty() && segments.len() == 1 {
            if let Some(context) = context_type {
                for base in self.base_class_chain(context) {
                    let candidate = format!("{base}::{}", segments[0]);
                    self.push_call_overloads(&candidate, &mut symbols, &mut seen);
                    if !symbols.is_empty() {
                        break;
                    }
                }
            }
        }

        if symbols.is_empty() && segments.len() == 1 {
            let resolved = self.symbol_index.resolve_function(namespace, &segments[0]);
            for symbol in resolved {
                let key = (symbol.qualified.clone(), symbol.signature.canonical_name());
                if seen.insert(key) {
                    symbols.push(symbol.clone());
                }
            }
        }

        if symbols.is_empty() {
            return None;
        }

        Some(CallCandidateSet {
            display,
            candidates: symbols,
        })
    }

    fn base_class_chain(&self, type_name: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut current = type_name.to_string();
        let mut visited = HashSet::new();
        while let Some(info) = self.resolve_type_info(&current) {
            let TypeKind::Class { bases, .. } = &info.kind else {
                break;
            };
            let mut next: Option<String> = None;
            for base in bases {
                if let Some(base_info) = self.resolve_type_info(&base.name) {
                    if matches!(base_info.kind, TypeKind::Class { .. }) {
                        next = Some(base.name.clone());
                        break;
                    }
                }
            }
            let Some(next) = next else {
                break;
            };
            if !visited.insert(next.clone()) {
                break;
            }
            chain.push(next.clone());
            current = next;
        }
        chain
    }

    pub(crate) fn resolve_call_owner_name(
        &self,
        owner: &str,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> Option<String> {
        if owner == "Self" {
            return context_type.map(str::to_string);
        }
        if owner.contains("::") {
            return Some(owner.to_string());
        }
        if self.symbol_index.contains_type(owner) {
            return Some(owner.to_string());
        }
        // Permit suffix-based disambiguation when `using` brings a single matching type into scope.
        let mut matched: Option<String> = None;
        for ty in self.symbol_index.types() {
            if ty.rsplit("::").next().is_some_and(|seg| seg == owner) {
                matched.get_or_insert_with(|| ty.clone());
            }
        }
        if let Some(unique) = matched {
            return Some(unique);
        }
        if let Some(ns) = namespace {
            let mut current = Some(ns);
            while let Some(prefix) = current {
                let candidate = format!("{prefix}::{owner}");
                if self.symbol_index.contains_type(&candidate) {
                    return Some(candidate);
                }
                current = prefix.rfind("::").map(|idx| &prefix[..idx]);
            }
        }
        // Fall back to owner suffix matching against known function owners.
        let mut matched_owner: Option<String> = None;
        for key in self.symbol_index.functions.keys() {
            if let Some((owner_part, _)) = key.rsplit_once("::") {
                if owner_part
                    .rsplit("::")
                    .next()
                    .is_some_and(|segment| segment == owner)
                {
                    if let Some(existing) = matched_owner.as_ref() {
                        if existing != owner_part {
                            matched_owner = None;
                            break;
                        }
                    } else {
                        matched_owner = Some(owner_part.to_string());
                    }
                }
            }
        }
        if let Some(owner_match) = matched_owner {
            return Some(owner_match);
        }
        None
    }

    fn infer_call_owner(&self, function_name: &str) -> Option<String> {
        let (owner, _) = function_name.rsplit_once("::")?;
        self.methods.contains_key(owner).then(|| owner.to_string())
    }

    fn push_call_overloads(
        &self,
        qualified: &str,
        output: &mut Vec<FunctionSymbol>,
        seen: &mut HashSet<(String, String)>,
    ) {
        if let Some(symbols) = self.symbol_index.function_overloads(qualified) {
            for symbol in symbols {
                let key = (symbol.qualified.clone(), symbol.signature.canonical_name());
                if seen.insert(key) {
                    output.push(symbol.clone());
                }
            }
        }
    }

    pub(crate) fn resolve_overload_for_call(
        &mut self,
        candidates: &CallCandidateSet,
        args: &[CallArgument],
        span: Option<Span>,
    ) {
        let mut matches: Vec<(usize, FunctionSymbol)> = Vec::new();
        let mut mismatch_reason: Option<CallMismatch> = None;
        for symbol in &candidates.candidates {
            match self.call_arguments_match(&candidates.display, symbol, args, span) {
                Ok(result) => {
                    matches.push((result.score, symbol.clone()));
                }
                Err(reason) => {
                    if mismatch_reason.is_none() {
                        mismatch_reason = Some(reason);
                    }
                }
            }
        }

        if matches.is_empty() {
            let mut names: Vec<String> = candidates
                .candidates
                .iter()
                .map(|symbol| symbol.qualified.clone())
                .collect();
            names.sort();
            names.dedup();
            if let Some(reason) = mismatch_reason {
                self.emit_error(codes::CALL_OVERLOAD_NO_MATCH, reason.span, reason.message);
            } else {
                let mut message = format!(
                    "no overload of `{}` matches the provided arguments",
                    candidates.display
                );
                if !names.is_empty() {
                    message.push_str(&format!("; available: {}", names.join(", ")));
                }
                self.emit_error(codes::CALL_OVERLOAD_NO_MATCH, span, message);
            }
            return;
        }

        matches.sort_by(|a, b| b.0.cmp(&a.0));
        let best_score = matches[0].0;
        let winners: Vec<FunctionSymbol> = matches
            .iter()
            .take_while(|(score, _)| *score == best_score)
            .map(|(_, symbol)| symbol.clone())
            .collect();

        if winners.len() > 1 && Self::overloads_indistinguishable_for_call(&winners, args) {
            let mut names: Vec<String> = winners
                .iter()
                .map(|symbol| symbol.qualified.clone())
                .collect();
            names.sort();
            names.dedup();
            self.emit_error(
                codes::CALL_OVERLOAD_AMBIGUOUS,
                span,
                format!(
                    "call to `{}` is ambiguous; candidates: {}",
                    candidates.display,
                    names.join(", ")
                ),
            );
            return;
        }

        // Overload ambiguity is resolved during MIR lowering where argument types are available.
        // This registry pass only validates named/positional argument structure, so avoid
        // emitting false positives when multiple overloads share the same arity.
    }

    fn overloads_indistinguishable_for_call(
        candidates: &[FunctionSymbol],
        args: &[CallArgument],
    ) -> bool {
        let Some(first) = candidates.first() else {
            return false;
        };
        let expected = Self::expected_argument_types_for_call(first, args);
        candidates
            .iter()
            .skip(1)
            .all(|candidate| Self::expected_argument_types_for_call(candidate, args) == expected)
    }

    fn expected_argument_types_for_call(
        symbol: &FunctionSymbol,
        args: &[CallArgument],
    ) -> Vec<Option<crate::mir::Ty>> {
        let mut indices = Vec::with_capacity(args.len());
        let mut next_pos = 0usize;
        for arg in args {
            let idx = if let Some(name) = &arg.name {
                symbol
                    .params
                    .iter()
                    .position(|param| param.name == name.text)
                    .unwrap_or(usize::MAX)
            } else {
                let idx = next_pos;
                next_pos += 1;
                idx
            };
            indices.push(idx);
        }

        indices
            .into_iter()
            .map(|idx| symbol.signature.params.get(idx).cloned())
            .collect()
    }

    pub(crate) fn call_arguments_match(
        &self,
        target_display: &str,
        symbol: &FunctionSymbol,
        args: &[CallArgument],
        span: Option<Span>,
    ) -> Result<CallMatch, CallMismatch> {
        let params = &symbol.params;
        if args.is_empty() && params.is_empty() {
            return Ok(CallMatch { score: 2 });
        }
        let mut assigned = vec![false; params.len()];
        let is_variadic = symbol.signature.variadic;
        let mut next_pos = 0usize;
        let mut named_encountered = false;
        let mut explicit = 0usize;
        for arg in args {
            let (index, param_name_span) = if let Some(name) = &arg.name {
                named_encountered = true;
                let idx = params
                    .iter()
                    .position(|param| param.name == name.text)
                    .ok_or_else(|| CallMismatch {
                        message: format!(
                            "`{target_display}` has no parameter named `{}`",
                            name.text
                        ),
                        span: name.span.or(arg.span).or(span),
                    })?;
                if assigned[idx] {
                    return Err(CallMismatch {
                        message: format!(
                            "parameter `{}` for `{target_display}` is specified multiple times",
                            params[idx].name
                        ),
                        span: name.span.or(arg.span).or(span),
                    });
                }
                (idx, name.span)
            } else {
                if named_encountered {
                    return Err(CallMismatch {
                        message: "positional arguments must appear before named arguments"
                            .to_string(),
                        span: arg.span.or(span),
                    });
                }
                if next_pos >= params.len() {
                    if !is_variadic {
                        return Err(CallMismatch {
                            message: format!(
                                "`{target_display}` does not accept {} argument(s)",
                                args.len()
                            ),
                            span: arg.span.or(span),
                        });
                    }
                    if Self::argument_mode(arg) != ParamMode::Value {
                        return Err(CallMismatch {
                            message: format!(
                                "variadic arguments to `{target_display}` must use value passing"
                            ),
                            span: arg.modifier_span.or(arg.span).or(span),
                        });
                    }
                    explicit += 1;
                    continue;
                }
                let idx = next_pos;
                next_pos += 1;
                (idx, None)
            };

            let expected_mode = params[index].mode;
            let supplied_mode = Self::argument_mode(arg);
            if expected_mode != supplied_mode {
                let label = Self::mode_label(expected_mode);
                return Err(CallMismatch {
                    message: format!(
                        "parameter `{}` of `{target_display}` must be passed using `{label}`",
                        params[index].name
                    ),
                    span: arg.modifier_span.or(param_name_span).or(arg.span).or(span),
                });
            }
            assigned[index] = true;
            explicit += 1;
        }

        for (idx, param) in params.iter().enumerate() {
            if !assigned[idx] && !param.has_default {
                return Err(CallMismatch {
                    message: format!(
                        "`{target_display}` is missing an argument for parameter `{}`",
                        param.name
                    ),
                    span,
                });
            }
        }

        let uses_defaults = params.iter().enumerate().any(|(idx, _)| !assigned[idx]);
        let score = explicit.saturating_mul(2) + if uses_defaults { 1 } else { 2 };
        Ok(CallMatch { score })
    }

    fn argument_mode(arg: &CallArgument) -> ParamMode {
        match arg.modifier {
            Some(crate::syntax::expr::builders::CallArgumentModifier::In) => ParamMode::In,
            Some(crate::syntax::expr::builders::CallArgumentModifier::Ref) => ParamMode::Ref,
            Some(crate::syntax::expr::builders::CallArgumentModifier::Out) => ParamMode::Out,
            None => ParamMode::Value,
        }
    }

    fn mode_label(mode: ParamMode) -> &'static str {
        match mode {
            ParamMode::In => "in",
            ParamMode::Ref => "ref",
            ParamMode::Out => "out",
            ParamMode::Value => "value",
        }
    }

    pub(crate) fn check_compare_exchange_call(
        &mut self,
        callee: &ExprNode,
        args: &[CallArgument],
        span: Option<Span>,
    ) {
        if args.len() < 4 {
            return;
        }

        let Some(segments) = expr_path_segments(callee) else {
            return;
        };
        let Some(name) = segments.last() else {
            return;
        };
        if !matches!(name.as_str(), "CompareExchange" | "CompareExchangeWeak") {
            return;
        }

        let success_arg = &args[2];
        let failure_arg = &args[3];

        let Some(success_variant) = self.parse_memory_order_node(&success_arg.value) else {
            return;
        };

        let failure_span = failure_arg.value_span.or(failure_arg.span).or(span);
        let Some(failure_variant) = self.parse_memory_order_node(&failure_arg.value) else {
            return;
        };

        if !success_variant.failure_allowed(failure_variant) {
            self.emit_error(
                codes::ATOMIC_COMPARE_EXCHANGE_ORDER,
                failure_span,
                format!(
                    "failure ordering `{}` cannot be stronger than success ordering `{}` on `CompareExchange`",
                    failure_variant.label(),
                    success_variant.label()
                ),
            );
        }
    }

    pub(crate) fn parse_memory_order_node(&self, node: &ExprNode) -> Option<MemoryOrderVariant> {
        let segments = expr_path_segments(node)?;
        let index = segments.iter().rposition(|part| part == "MemoryOrder")?;
        let variant_index = index.checked_add(1)?;
        let variant = segments.get(variant_index)?;
        MemoryOrderVariant::from_str(variant)
    }

    #[allow(dead_code)]
    pub(crate) fn expr_display(&self, node: &ExprNode) -> String {
        expr_path_segments(node)
            .filter(|segments| !segments.is_empty())
            .map(|segments| segments.join("::"))
            .unwrap_or_else(|| "<expression>".to_string())
    }

    pub(crate) fn emit_memory_order_error(
        &mut self,
        span: Option<Span>,
        expr_text: Option<&str>,
        context: &str,
    ) {
        let display = expr_text
            .map(|text| text.trim())
            .filter(|text| !text.is_empty())
            .unwrap_or("<expression>");
        self.emit_error(
            codes::ATOMIC_ORDERING_EXPECTED,
            span,
            format!(
                "{context} must be a `Std.Sync.MemoryOrder` value, but `{display}` was supplied"
            ),
        );
    }
}

pub(crate) fn expr_path_segments(node: &ExprNode) -> Option<Vec<String>> {
    match node {
        ExprNode::Identifier(name) => Some(vec![name.clone()]),
        ExprNode::Member { base, member, .. } => {
            let mut segments = expr_path_segments(base)?;
            segments.push(member.clone());
            Some(segments)
        }
        ExprNode::Parenthesized(inner) => expr_path_segments(inner),
        _ => None,
    }
}
