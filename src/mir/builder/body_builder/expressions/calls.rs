use super::DefaultArgumentValue;
use super::call_support::{CallBindingInfo, EvaluatedArg, InlineBindingMeta};
use super::*;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::{
    CallDispatch, LocalKind, Place, Rvalue, Statement as MirStatement,
    StatementKind as MirStatementKind, Ty, VirtualDispatch,
};
use crate::syntax::expr::builders::InlineBindingKind;
use crate::typeck::{AutoTraitConstraintOrigin, AutoTraitKind};
use std::collections::HashSet;

mod context;
mod direct;
mod intrinsic;
mod r#virtual;

use self::context::CallContext;
body_builder_impl! {
    pub(crate) fn lower_call_statement(
        &mut self,
        callee: ExprNode,
        args: Vec<CallArgument>,
        generics: Option<Vec<String>>,
        span: Option<Span>,
            ) -> bool {
        self.lower_call(callee, args, generics, span, false).is_some()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Call lowering handles receiver/method, pending operands, and destination setup together."
    )]
    pub(crate) fn lower_call(
        &mut self,
        callee: ExprNode,
        args: Vec<CallArgument>,
        generics: Option<Vec<String>>,
        span: Option<Span>,
        capture_result: bool,
            ) -> Option<Operand> {
        self.lower_call_with_destination(
            callee,
            args,
            generics,
            span,
            capture_result,
            None,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Call lowering handles receiver/method, pending operands, and destination setup together."
    )]
    pub(crate) fn lower_call_with_destination(
        &mut self,
        callee: ExprNode,
        args: Vec<CallArgument>,
        generics: Option<Vec<String>>,
        span: Option<Span>,
                capture_result: bool,
        destination_override: Option<Place>,
    ) -> Option<Operand> {
        let callee_clone = callee.clone();

        if let Some(result) = self.try_lower_drop_glue_intrinsic(&callee_clone, &args, span) {
            return Some(result);
        }
        if let Some(result) =
            self.try_lower_type_id_intrinsic(&callee_clone, &args, &generics, span)
        {
            return Some(result);
        }
        let mut call_info = CallBindingInfo::default();
        let mut method_type_args: Option<Vec<Ty>> = None;
        if let Some(args) = generics.as_ref() {
            let mut parsed = Vec::new();
            let mut parse_ok = true;
            for text in args {
                let Some(expr) = parse_type_expression_text(text) else {
                    parse_ok = false;
                    break;
                };
                parsed.push(Ty::from_type_expr(&expr));
            }
            if parse_ok {
                method_type_args = Some(parsed);
            }
        }
        call_info.method_type_args = method_type_args.clone();
        let mut receiver_operand: Option<Operand> = None;

        if let ExprNode::Identifier(name) = &callee_clone {
            if let Some(type_args) = method_type_args.as_ref()
                && !type_args.is_empty()
            {
                if name == "__sizeof" || name == "__alignof" {
                    let ty = &type_args[0];
                    let intrinsic = if name == "__sizeof" {
                        self.size_for_ty(ty, span)
                    } else {
                        self.align_for_ty(ty, span)
                    };
                    if capture_result {
                        if let Some(value) = intrinsic.clone() {
                            let destination = destination_override.unwrap_or_else(|| {
                                let temp = self.create_temp(span);
                                if let Some(local) = self.locals.get_mut(temp.0) {
                                    local.ty = Ty::named("usize");
                                    local.is_nullable = false;
                                }
                                Place::new(temp)
                            });
                            self.push_statement(MirStatement {
                                span,
                                kind: MirStatementKind::Assign {
                                    place: destination.clone(),
                                    value: Rvalue::Use(value),
                                },
                            });
                            return Some(Operand::Copy(destination));
                        }
                    }
                    return intrinsic;
                }
            }
        }

        let mut func_operand = match callee {
            ExprNode::Member { base, member, .. } => {
                let member_name = member.clone();
                call_info.member_name = Some(member_name.clone());
                let is_base_receiver = matches!(
                    base.as_ref(),
                    ExprNode::Identifier(name) if name.eq_ignore_ascii_case("base")
                );
                if is_base_receiver {
                    let Some(base_owner) = self.base_receiver_owner() else {
                        let ty = self.current_self_type_name().unwrap_or_else(|| "<type>".into());
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "`base` cannot be used because `{ty}` does not declare a base class"
                            ),
                            span,
                        });
                        return None;
                    };
                    call_info.receiver_owner = Some(base_owner);
                    call_info.force_base_receiver = true;
                    let Some(self_operand) = self.make_self_operand(span) else {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "`base` calls are only valid within instance methods".into(),
                            span,
                        });
                        return None;
                    };
                    receiver_operand = Some(self_operand);
                    Operand::Pending(PendingOperand {
                        category: ValueCategory::Pending,
                        repr: format!("base.{member_name}"),
                        span,
                info: None,
            })
        } else {
            let base_repr = Self::expr_to_string(&base);
            let mut owner_type_args: Option<Vec<Ty>> = None;
            if let ExprNode::Call {
                args,
                generics: Some(list),
                ..
            } = base.as_ref()
            {
                if args.is_empty() {
                    let mut parsed = Vec::new();
                    for text in list {
                        if let Some(expr) = parse_type_expression_text(text) {
                            parsed.push(Ty::from_type_expr(&expr));
                        } else if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok() {
                            eprintln!(
                                "[owner-args-debug] failed to parse base type argument `{text}` for {base_repr}"
                            );
                        }
                    }
                    if !parsed.is_empty() {
                        owner_type_args = Some(parsed);
                    }
                }
            }
            if owner_type_args.is_none() {
                let looks_like_type = matches!(base.as_ref(), ExprNode::Identifier(_))
                    || matches!(base.as_ref(), ExprNode::Member { .. })
                    || matches!(
                        base.as_ref(),
                        ExprNode::Call { args, .. } if args.is_empty()
                    );
                let avoid_call_like = base_repr.contains('(') || base_repr.contains(')');
                if looks_like_type && !avoid_call_like {
                    owner_type_args = self
                        .parse_type_arguments(&base_repr, span)
                        .and_then(|args| if args.is_empty() { None } else { Some(args) });
                }
            }
            if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok() && owner_type_args.is_some() {
                eprintln!(
                    "[owner-args-debug] base_expr={base:?} base_repr={base_repr} member={member_name} method_type_args={:?} owner_type_args={owner_type_args:?}",
                    method_type_args
                );
            }
            if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok()
                && member_name == "FromValuePointer"
                && owner_type_args.is_none()
            {
                eprintln!(
                    "[owner-args-debug] no owner args inferred for base_expr={base:?} base_repr={base_repr}"
                );
            }
            let mut treat_as_static = false;
            if let Some(segments) = collect_path_segments(&base) {
                if let Some(owner) = self.resolve_type_owner_for_segments(&segments) {
                    treat_as_static = true;
                    call_info.static_owner = Some(owner);
                }
            }

            if !treat_as_static && self.member_chain_unresolved(base.as_ref()) {
                treat_as_static = true;
            }

            if treat_as_static && call_info.static_owner.is_none() {
                let fallback_segments =
                    collect_path_segments(&base).unwrap_or_else(|| vec![base_repr.clone()]);
                if let Some(owner) = self.resolve_type_owner_for_segments(&fallback_segments) {
                    call_info.static_owner = Some(owner);
                } else {
                    call_info.static_owner = Some(base_repr.clone());
                }
            }

            if treat_as_static {
                if let Some(args) = owner_type_args {
                    let merged: Vec<Ty> = if let Some(existing) = method_type_args.take() {
                        if existing.is_empty() {
                            args
                        } else {
                            args.iter().cloned().chain(existing.into_iter()).collect()
                        }
                    } else {
                        args
                    };
                    method_type_args = Some(merged.clone());
                    call_info.method_type_args = Some(merged);
                }
                call_info.static_base = Some(base_repr.clone());
                call_info.static_owner.get_or_insert(base_repr.clone());
                Operand::Pending(PendingOperand {
                    category: ValueCategory::Pending,
                            repr: format!("{base_repr}.{member_name}"),
                            span,
                            info: None,
                        })
                    } else {
                        let receiver = self.lower_expr_node(*base, span)?;
                        match receiver {
                            Operand::Pending(_) => Operand::Pending(PendingOperand {
                                category: ValueCategory::Pending,
                                repr: format!("{base_repr}.{member_name}"),
                                span,
                                info: None,
                            }),
                            operand => {
                                if let Some(owner) = self.receiver_owner_from_operand(&operand) {
                                    call_info.receiver_owner = Some(owner);
                                }
                                receiver_operand = Some(operand);
                                Operand::Pending(PendingOperand {
                                    category: ValueCategory::Pending,
                                    repr: format!("{base_repr}.{member_name}"),
                                    span,
                                    info: None,
                                })
                            }
                        }
                    }
                }
            }
            other => self.lower_call_callee(&other, span)?,
        };

        if let Some(member) = call_info.member_name.as_deref()
            && member.starts_with("chic_rt_decimal_")
        {
            func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(format!(
                "Std::Numeric::Decimal::RuntimeIntrinsics::{member}"
            ))));
        }

        match &func_operand {
            Operand::Const(constant) => {
                if let Some(name) = constant.symbol_name() {
                    call_info.canonical_hint = Some(name.to_string());
                }
            }
            Operand::Pending(pending) => {
                if let Some(info) = &pending.info {
                    let PendingOperandInfo::FunctionGroup { candidates, .. } = info.as_ref();
                    call_info
                        .pending_candidates
                        .extend(candidates.iter().map(|candidate| candidate.qualified.clone()));
                }
            }
            _ => {}
        }

        if let Some(hint) = call_info.canonical_hint.clone() {
            if let Some((owner, _)) = hint.rsplit_once("::init#") {
                call_info.is_constructor = true;
                if call_info.static_owner.is_none() {
                    call_info.static_owner = Some(owner.to_string());
                }
            }
        }

        if call_info.canonical_hint.is_none()
            && call_info.member_name.is_none()
            && call_info.receiver_owner.is_none()
        {
            if call_info.static_owner.is_none() {
                if let ExprNode::Identifier(name) = &callee_clone {
                    if let Some(owner) = self.current_self_type_name() {
                        let qualified = format!("{owner}::{name}");
                        if self
                            .symbol_index
                            .function_overloads(&qualified)
                            .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.is_static))
                        {
                            call_info.member_name = Some(name.clone());
                            call_info.static_owner = Some(owner);
                        }
                    }
                }
            }
            if call_info.static_owner.is_none() {
                let owner = match &callee_clone {
                    ExprNode::Identifier(name) => {
                        self.resolve_type_owner_for_segments(&[name.clone()])
                    }
                    _ => collect_path_segments(&callee_clone)
                        .and_then(|segments| self.resolve_type_owner_for_segments(&segments)),
                };
                if let Some(owner) = owner {
                    call_info.static_owner = Some(owner);
                    call_info.static_base = Some(Self::expr_to_string(&callee_clone));
                }
            }
            if call_info.static_owner.is_some() && call_info.member_name.is_none() {
                call_info.is_constructor = true;
            } else if call_info.member_name.is_none() {
                if let ExprNode::Identifier(name) = &callee_clone {
                    if let Some(owner) = self.resolve_static_using_method_owner(name, span) {
                        call_info.member_name = Some(name.clone());
                        call_info.static_owner = Some(owner);
                    }
                }
            }
        }

        if call_info.static_owner.is_none() {
            if let Some(owner) = self.resolve_static_owner_expr(&callee_clone) {
                call_info.static_owner = Some(owner);
            }
        }
        if call_info.canonical_hint.is_none() {
            if let (Some(owner), Some(member)) =
                (call_info.static_owner.as_ref(), call_info.member_name.as_ref())
            {
                call_info.canonical_hint = Some(format!("{owner}::{member}"));
            }
        }

        if receiver_operand.is_none() {
            if let Some(implicit_receiver) = self.bind_implicit_receiver(&mut call_info) {
                receiver_operand = Some(implicit_receiver);
            }
        }

        if std::env::var("CHIC_DEBUG_OWNER_TYPE_ARGS").is_ok()
            && call_info.member_name.as_deref() == Some("FromValuePointer")
        {
            eprintln!(
                "[owner-args-call] static_owner={:?} static_base={:?} receiver_owner={:?} method_type_args={:?}",
                call_info.static_owner, call_info.static_base, call_info.receiver_owner, call_info.method_type_args
            );
        }

        if self.should_elide_conditional_call(&call_info, span) {
            self.push_debug_note(
                format!(
                    "conditional call removed: {}",
                    call_info
                        .canonical_hint
                        .clone()
                        .unwrap_or_else(|| Self::expr_to_string(&callee_clone))
                ),
                span,
            );
            return Some(Operand::Const(ConstOperand::new(ConstValue::Unit)));
        }

        let mut evaluated_args = Vec::with_capacity(args.len());
        let mut seen_named_labels: HashSet<String> = HashSet::new();
        let mut has_named_arguments = false;
        let mut positional_after_named_span: Option<Span> = None;

        for arg in args.into_iter() {
            let CallArgument {
                name,
                value,
                span: arg_span,
                value_span,
                modifier,
                modifier_span,
                inline_binding,
            } = arg;
            let mut inline_meta: Option<InlineBindingMeta> = None;
            if let Some(binding) = inline_binding.clone() {
                if self.lookup_name(&binding.name).is_some() {
                    let diag_span = binding.keyword_span.or(binding.name_span).or(arg_span);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("identifier `{}` already defined in this scope", binding.name),
                        span: diag_span,
                    });
                    return None;
                }
                let mut decl = LocalDecl::new(
                    Some(binding.name.clone()),
                    Ty::Unknown,
                    true,
                    binding.keyword_span.or(binding.name_span),
                    LocalKind::Local,
                );
                if let InlineBindingKind::Typed {
                    type_name,
                    type_span,
                } = &binding.kind
                {
                    if let Some(type_expr) = parse_type_expression_text(type_name) {
                        let ty = Ty::from_type_expr(&type_expr);
                        decl.ty = ty.clone();
                        decl.is_nullable = type_expr.is_nullable();
                        self.ensure_ty_layout_for_ty(&ty);
                    } else {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!("unable to parse type `{type_name}` for inline binding"),
                            span: *type_span,
                        });
                    }
                }
                let local_id = self.push_local(decl);
                self.bind_name(&binding.name, local_id);
                let live_span = binding.keyword_span.or(binding.name_span).or(arg_span);
                self.push_statement(MirStatement {
                    span: live_span,
                    kind: MirStatementKind::StorageLive(local_id),
                });
                self.record_local(local_id, live_span);

                if let Some(initializer) = binding.initializer.clone() {
                    let init_span = binding
                        .initializer_span
                        .or(binding.name_span)
                        .or(arg_span)
                        .or(span);
                    let mut operand = self.lower_expr_node(initializer, init_span)?;
                    let operand_ty = self.operand_ty(&operand);
                    let operand_fn_ty = self.operand_fn_ty(&operand);
                    let operand_type_name = self.operand_type_name(&operand);

                    let mut target_ty: Option<Ty> = None;
                    if let Some(local_decl) = self.locals.get_mut(local_id.0) {
                        if matches!(local_decl.ty, Ty::Unknown) {
                            if let Some(op_ty) = operand_ty.clone() {
                                if !matches!(op_ty, Ty::Unknown) {
                                    local_decl.is_nullable = matches!(op_ty, Ty::Nullable(_));
                                    local_decl.ty = op_ty;
                                }
                            }
                        }
                        if matches!(local_decl.ty, Ty::Unknown) {
                            if let Some(fn_ty) = operand_fn_ty.clone() {
                                local_decl.ty = Ty::Fn(fn_ty);
                            } else if let Some(type_name) = operand_type_name.clone() {
                                if self.closure_registry.contains_key(&type_name) {
                                    local_decl.ty = Ty::named(type_name);
                                }
                            }
                        }
                        if !matches!(local_decl.ty, Ty::Unknown) {
                            local_decl.is_nullable = matches!(local_decl.ty, Ty::Nullable(_));
                        }
                        target_ty = Some(local_decl.ty.clone());
                    }

                    if let Some(ty) = target_ty.clone() {
                        if !matches!(ty, Ty::Unknown) {
                            self.ensure_ty_layout_for_ty(&ty);
                            operand = self.coerce_operand_to_ty(operand, &ty, false, init_span);
                        }
                    }

                    self.push_statement(MirStatement {
                        span: init_span,
                        kind: MirStatementKind::Assign {
                            place: Place::new(local_id),
                            value: Rvalue::Use(operand),
                        },
                    });
                }

                inline_meta = Some(InlineBindingMeta {
                    local: local_id,
                });
            }
            let (arg_name, name_span) = match name {
                Some(name) => {
                    has_named_arguments = true;
                    let label = name.text.clone();
                    if !seen_named_labels.insert(label.clone()) {
                        let diag_span = name.span.or(arg_span).or(span);
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "named argument `{}` appears more than once",
                                label
                            ),
                            span: diag_span,
                                                    });
                        return None;
                    }
                    (Some(label), name.span)
                }
                None => {
                    if has_named_arguments && positional_after_named_span.is_none() {
                        positional_after_named_span = arg_span.or(span);
                    }
                    (None, None)
                }
            };
            let operand: Operand;
            match modifier {
                Some(CallArgumentModifier::Ref) => {
                    let place_span = value_span.or(arg_span).or(span);
                    let lowered = self.lower_place_expr(value, place_span)?;
                    operand = self.borrow_argument_place(
                        lowered.clone(),
                        BorrowKind::Unique,
                        modifier_span.or(arg_span).or(span),
                    );
                }
                Some(CallArgumentModifier::In) => {
                    let place_span = value_span.or(arg_span).or(span);
                    let lowered = self.lower_place_expr(value, place_span)?;
                    operand = self.borrow_argument_place(
                        lowered.clone(),
                        BorrowKind::Shared,
                        modifier_span.or(arg_span).or(span),
                    );
                }
                Some(CallArgumentModifier::Out) => {
                    let place_span = value_span.or(arg_span).or(span);
                    let lowered = self.lower_place_expr(value, place_span)?;
                    operand = self.borrow_argument_place(
                        lowered.clone(),
                        BorrowKind::Unique,
                        modifier_span.or(arg_span).or(span),
                    );
                }
                None => {
                    operand = self.lower_expr_node(value, arg_span.or(span))?;
                }
            }
            evaluated_args.push(EvaluatedArg {
                operand,
                modifier,
                modifier_span,
                name: arg_name,
                name_span,
                span: arg_span,
                value_span,
                inline_binding: inline_meta,
                param_slot: None,
            });
        }

        if matches!(call_info.member_name.as_deref(), Some("to_fn_ptr")) {
            if !evaluated_args.is_empty() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "`.to_fn_ptr()` does not take arguments".into(),
                    span,
                                    });
                return None;
            }
            let Some(receiver) = receiver_operand.clone() else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "`.to_fn_ptr()` requires a closure receiver".into(),
                    span,
                                    });
                return None;
            };
            return self.lower_closure_to_fn_ptr(receiver, span, capture_result);
        }

        if let Some(violation_span) = positional_after_named_span {
            self.diagnostics.push(LoweringDiagnostic {
                message: "positional arguments cannot follow named arguments".into(),
                span: Some(violation_span),
                            });
            return None;
        }

        if call_info.resolved_symbol.is_none() {
            if let Some(symbol) = self.closure_symbol_for_operand(&func_operand) {
                if call_info.canonical_hint.is_none() {
                    call_info.canonical_hint = Some(symbol.qualified.clone());
                }
                call_info.resolved_symbol = Some(symbol);
            }
        }

        let member_name = call_info.member_name.clone();
        let (mut ordered_args_meta, canonical_override) = if has_named_arguments {
            let (bound_args, canonical) = self.bind_named_arguments(
                &mut func_operand,
                evaluated_args,
                call_info.clone(),
                span,
                            )?;
            if let Some(name) = canonical.as_ref() {
                call_info.canonical_hint = Some(name.clone());
            }
            (bound_args, canonical)
        } else {
            (evaluated_args, None)
        };

        if let Some(name) = canonical_override {
            func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(name)));
        }

        let has_receiver = receiver_operand.is_some();

        if let Some(receiver) = receiver_operand {
            let receiver_meta = EvaluatedArg {
                operand: receiver,
                modifier: None,
                modifier_span: None,
                name: None,
                name_span: None,
                span,
                value_span: span,
                inline_binding: None,
                param_slot: None,
            };
            let mut with_receiver = Vec::with_capacity(ordered_args_meta.len() + 1);
            with_receiver.push(receiver_meta);
            with_receiver.extend(ordered_args_meta);
            ordered_args_meta = with_receiver;
        }

        let mut call_dispatch: Option<CallDispatch> = None;
        if has_receiver {
            if let (Some(first_arg), Some(member)) =
                (ordered_args_meta.first_mut(), member_name.as_deref())
            {
                if let Some(dispatch) = self.try_build_trait_object_dispatch(
                    &mut first_arg.operand,
                    member,
                    span,
                ) {
                    func_operand = Operand::Const(ConstOperand::new(ConstValue::Unit));
                    call_dispatch = Some(CallDispatch::Trait(dispatch));
                }
            }
        }

        if call_dispatch.is_none() && has_receiver {
            if let Some(dispatch) = self.try_build_virtual_dispatch(&call_info, 0) {
                call_dispatch = Some(CallDispatch::Virtual(dispatch));
            }
        }

        if !has_named_arguments {
            self.validate_argument_modes_for_call(
                &func_operand,
                &ordered_args_meta,
                has_receiver,
                span,
            );
        }

        let mut destination_place = if let Some(place) = destination_override {
            Some(place)
        } else if capture_result {
            let temp = self.create_temp(span);
            Some(Place::new(temp))
        } else {
            None
        };

        {
            let ctx = CallContext::new(span, &call_info, has_receiver);
            if let Some(result_operand) = self.try_lower_numeric_intrinsic(
                &func_operand,
                &ctx,
                &ordered_args_meta,
                destination_place.clone(),
            ) {
                return Some(result_operand);
            }
            if let Some(result_operand) = self.try_lower_decimal_intrinsic(
                &func_operand,
                &ctx,
                &ordered_args_meta,
                destination_place.clone(),
            ) {
                return Some(result_operand);
            }

            if let Some(result_operand) = self.try_lower_span_intrinsic(
                &func_operand,
                &ctx,
                &ordered_args_meta,
                destination_place.clone(),
            ) {
                return Some(result_operand);
            }

            if let Some(result_operand) = self.try_lower_zero_init_intrinsic(
                &func_operand,
                &ctx,
                &ordered_args_meta,
            ) {
                return Some(result_operand);
            }
        }

        self.record_thread_spawn_constraints(
            &func_operand,
            &call_info,
            &ordered_args_meta,
            has_receiver,
            span,
        );

        if let Some(result) = self.try_lower_delegate_call(
            &func_operand,
            &ordered_args_meta,
            destination_place.clone(),
            span,
        ) {
            return result;
        }

        let skip_symbol_resolution = matches!(call_dispatch, Some(CallDispatch::Trait(_)));
        if !skip_symbol_resolution
            && !self.resolve_call_symbol(
                &mut func_operand,
                &mut call_info,
                &ordered_args_meta,
                has_receiver,
                span,
            )
        {
            return None;
        }

        let mut hidden_prefix = 0usize;
        if let Some((invoke_symbol, captures)) = self.prepare_closure_call(&func_operand) {
            hidden_prefix = captures.len();
            let mut capture_meta = captures
                .into_iter()
                .map(|operand| EvaluatedArg {
                    operand,
                    modifier: None,
                    modifier_span: None,
                    name: None,
                    name_span: None,
                    span,
                    value_span: span,
                    inline_binding: None,
                    param_slot: None,
                })
                .collect::<Vec<_>>();
            capture_meta.extend(ordered_args_meta);
            ordered_args_meta = capture_meta;
            func_operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(invoke_symbol)));
        }

        let type_metadata_args = self.collect_type_id_call_args(
            &func_operand,
            &call_info,
            method_type_args.as_ref(),
            &ordered_args_meta,
            has_receiver,
            span,
        )?;
        if !self.apply_default_arguments(
            &call_info,
            &mut ordered_args_meta,
            has_receiver,
            &type_metadata_args,
            span,
            hidden_prefix,
        ) {
            return None;
        }

        {
            let ctx = CallContext::new(span, &call_info, has_receiver);
            if let Some(result_operand) = self.try_lower_atomic_call(
                &func_operand,
                &ctx,
                &ordered_args_meta,
                destination_place.clone(),
            ) {
                return Some(result_operand);
            }
        }

        let mut ordered_args = ordered_args_meta
            .iter()
            .map(|arg| arg.operand.clone())
            .collect::<Vec<_>>();
        let arg_count = ordered_args_meta.len();
        let mut arg_modes = ordered_args_meta
            .iter()
            .map(|arg| match arg.modifier {
                Some(CallArgumentModifier::In) => ParamMode::In,
                Some(CallArgumentModifier::Ref) => ParamMode::Ref,
                Some(CallArgumentModifier::Out) => ParamMode::Out,
                None => ParamMode::Value,
            })
            .collect::<Vec<_>>();

        let prior_ffi_pointer_context = self.ffi_pointer_context;
        let ffi_pointer_context = self.call_targets_ffi_abi(&call_info, &func_operand);
        self.ffi_pointer_context = ffi_pointer_context;

        if let Some(CallDispatch::Trait(dispatch)) = call_dispatch.as_ref() {
            if let Some(trait_info) = self.trait_registry.get(dispatch.trait_name.as_str()) {
                if let Some(method_info) = trait_info
                    .methods
                    .iter()
                    .find(|method| method.name == dispatch.method)
                {
                    let signature_params = method_info
                        .signature
                        .parameters
                        .iter()
                        .map(|param| Ty::from_type_expr(&param.ty))
                        .collect::<Vec<_>>();
                    if signature_params.len() + 1 <= ordered_args.len() {
                        for (offset, expected) in signature_params.iter().enumerate() {
                            let index = offset + 1;
                            if !matches!(arg_modes.get(index), Some(ParamMode::Value)) {
                                continue;
                            }
                            let span_hint = ordered_args_meta
                                .get(index)
                                .and_then(|meta| meta.value_span.or(meta.span))
                                .or(span);
                            if let Some(binding) = ordered_args_meta
                                .get(index)
                                .and_then(|arg| arg.inline_binding.as_ref())
                            {
                                self.hint_local_ty(binding.local, expected.clone());
                                self.ensure_ty_layout_for_ty(expected);
                            }
                            ordered_args[index] = self.coerce_operand_to_ty(
                                ordered_args[index].clone(),
                                expected,
                                false,
                                span_hint,
                            );
                        }
                    }
                }
            }
        }

        if let Some(symbol) = call_info.resolved_symbol.as_ref() {
            if let Some((signature_params, _, _)) =
                self.instantiate_symbol_signature(symbol, &call_info)
            {
                let signature_params = self.align_params_with_receiver(
                    has_receiver,
                    &ordered_args,
                    signature_params,
                    symbol
                        .owner
                        .as_deref()
                        .or_else(|| call_info.receiver_owner.as_deref()),
                );
                let fixed_len = signature_params.len();
                if fixed_len <= ordered_args.len() {
                    for (index, (operand, expected)) in ordered_args
                        .iter_mut()
                        .zip(signature_params.iter())
                        .enumerate()
                    {
                        if std::env::var_os("CHIC_DEBUG_CALL_COERCE").is_some() {
                            let op_ty = self.operand_ty(operand).map(|ty| ty.canonical_name());
                            eprintln!(
                                "[call-coerce] func={:?} index={index} operand_ty={op_ty:?} expected={}",
                                call_info
                                    .resolved_symbol
                                    .as_ref()
                                    .map(|s| s.qualified.as_str())
                                    .or_else(|| symbol.owner.as_deref()),
                                expected.canonical_name()
                            );
                        }
                        if let Some(binding) = ordered_args_meta
                            .get(index)
                            .and_then(|arg| arg.inline_binding.as_ref())
                        {
                            self.hint_local_ty(binding.local, expected.clone());
                            self.ensure_ty_layout_for_ty(expected);
                        }
                        if !matches!(arg_modes.get(index), Some(ParamMode::Value)) {
                            continue;
                        }
                        let span_hint = ordered_args_meta
                            .get(index)
                            .and_then(|meta| meta.value_span.or(meta.span))
                            .or(span);
                        *operand =
                            self.coerce_operand_to_ty(operand.clone(), expected, false, span_hint);
                    }
                }
            }
        }

        let mut type_metadata_args = type_metadata_args.clone();
        if let Some(symbol) = call_info.resolved_symbol.as_ref() {
            let expected_len = self
                .instantiate_symbol_signature(symbol, &call_info)
                .map(|(params, _, _)| {
                    self.align_params_with_receiver(
                        has_receiver,
                        &ordered_args,
                        params,
                        symbol
                            .owner
                            .as_deref()
                            .or_else(|| call_info.receiver_owner.as_deref()),
                    )
                    .len()
                })
                .unwrap_or_else(|| symbol.signature.params.len());
            if expected_len <= ordered_args.len() {
                type_metadata_args.clear();
            } else {
                let allowed = expected_len - ordered_args.len();
                if type_metadata_args.len() > allowed {
                    type_metadata_args.truncate(allowed);
                }
            }
        }

        for operand in type_metadata_args {
            arg_modes.push(ParamMode::Value);
            ordered_args.push(operand);
        }

        if call_dispatch.is_none() {
            if let Some(symbol) = call_info.resolved_symbol.clone() {
                if let Some((signature_params, _, _)) =
                    self.instantiate_symbol_signature(&symbol, &call_info)
                {
                    let signature_params = self.align_params_with_receiver(
                        has_receiver,
                        &ordered_args,
                        signature_params,
                        symbol
                            .owner
                            .as_deref()
                            .or_else(|| call_info.receiver_owner.as_deref()),
                    );
                    for (index, (operand, expected)) in
                        ordered_args.iter_mut().zip(signature_params.iter()).enumerate()
                    {
                        if !matches!(arg_modes.get(index), Some(ParamMode::Value)) {
                            continue;
                        }
                        if !matches!(expected, Ty::Fn(_)) {
                            continue;
                        }
                        if std::env::var_os("CHIC_DEBUG_FN_COERCE").is_some() {
                            let op_ty =
                                self.operand_ty(operand).map(|ty| ty.canonical_name());
                            let target = call_info
                                .canonical_hint
                                .as_deref()
                                .unwrap_or("<unknown>");
                            eprintln!(
                                "[fn-coerce] target={target} index={index} expected={} operand_ty={op_ty:?}",
                                expected.canonical_name()
                            );
                        }
                        let span_hint = ordered_args_meta
                            .get(index)
                            .and_then(|meta| meta.value_span.or(meta.span))
                            .or(span);
                        let before = if std::env::var_os("CHIC_DEBUG_FN_COERCE").is_some() {
                            Some(format!("{operand:?}"))
                        } else {
                            None
                        };
                        let coerced =
                            self.coerce_operand_to_ty(operand.clone(), expected, false, span_hint);
                        if let Some(prev) = before {
                            if prev != format!("{coerced:?}") {
                                eprintln!("[fn-coerce] updated operand index={index} before={prev} after={coerced:?}");
                            }
                        }
                        *operand = coerced;
                    }
                }
            }
        }

        let inferred_return_ty = if call_info.is_constructor {
            None
        } else {
            self.infer_call_return_ty(&func_operand, &call_info, arg_count, has_receiver)
                .and_then(|ty| (!matches!(ty, Ty::Unknown)).then_some(ty))
                .or_else(|| {
                    call_dispatch
                        .as_ref()
                        .and_then(|dispatch| match dispatch {
                            CallDispatch::Trait(trait_dispatch) => {
                                let lookup_ret = |candidate: &str| {
                                    self.symbol_index
                                        .function_overloads(candidate)
                                        .and_then(|symbols| {
                                            symbols.iter().find_map(|symbol| {
                                                let receiver_adjust = usize::from(symbol.params.first().is_some_and(|param| param.is_receiver()));
                                                let expected_args = symbol.signature.params.len() + receiver_adjust;
                                                let matches_arity = if symbol.signature.variadic {
                                                    arg_count >= expected_args
                                                } else {
                                                    arg_count == expected_args
                                                };
                                                let ret = (*symbol.signature.ret).clone();
                                                (!matches!(ret, Ty::Unknown)
                                                    && matches_arity)
                                                    .then_some(ret)
                                            })
                                        })
                                };

                                if let Some(impl_type) = trait_dispatch.impl_type.as_ref() {
                                    let segments = impl_type
                                        .split("::")
                                        .map(|seg| seg.to_string())
                                        .collect::<Vec<_>>();
                                    let mut candidates = Vec::new();
                                    if let Some(owner) =
                                        self.resolve_type_owner_for_segments(&segments)
                                    {
                                        candidates.push(format!(
                                            "{owner}::{}",
                                            trait_dispatch.method
                                        ));
                                    }
                                    if let Some(namespace) = self.namespace.as_deref() {
                                        let ns = namespace.replace('.', "::");
                                        let impl_name = impl_type.replace('.', "::");
                                        candidates.push(format!(
                                            "{ns}::{impl_name}::{}",
                                            trait_dispatch.method
                                        ));
                                    }
                                    candidates.push(format!(
                                        "{}::{}",
                                        impl_type.replace('.', "::"),
                                        trait_dispatch.method
                                    ));

                                    for candidate in candidates {
                                        if let Some(ret) = lookup_ret(&candidate) {
                                            return Some(ret);
                                        }
                                    }

                                    let impl_segment = impl_type
                                        .split("::")
                                        .last()
                                        .unwrap_or(impl_type)
                                        .split('<')
                                        .next()
                                        .unwrap_or(impl_type);
                                    for candidate in self
                                        .symbol_index
                                        .resolve_function_by_suffixes(&trait_dispatch.method)
                                    {
                                        if !candidate
                                            .split("::")
                                            .any(|seg| seg == impl_segment)
                                        {
                                            continue;
                                        }
                                        if let Some(ret) = lookup_ret(&candidate) {
                                            return Some(ret);
                                        }
                                    }
                                }

                                let candidate = format!("{}::{}", trait_dispatch.trait_name, trait_dispatch.method);
                                lookup_ret(&candidate)
                            }
                            CallDispatch::Virtual(_) => None,
                        })
                })
        };

        if !call_info.is_constructor
            && destination_place.is_none()
            && inferred_return_ty
                .as_ref()
                .is_some_and(|ty| !matches!(ty, Ty::Unit))
        {
            let temp = self.create_temp(span);
            if let Some(ty) = inferred_return_ty.as_ref() {
                self.hint_local_ty(temp, ty.clone());
            }
            destination_place = Some(Place::new(temp));
        }

        let result_type_hint = if call_info.is_constructor {
            None
        } else {
            destination_place
                .as_ref()
                .and_then(|place| inferred_return_ty.as_ref().map(|ty| (place.local, ty.clone())))
        };

        let constructor_receiver_mode = if call_info.is_constructor {
            let owner_is_class = destination_place
                .as_ref()
                .and_then(|place| self.place_type_name(place))
                .or_else(|| {
                    call_info
                        .resolved_symbol
                        .as_ref()
                        .and_then(|symbol| symbol.owner.clone())
                })
                .and_then(|type_name| self.type_layouts.layout_for_name(&type_name).cloned())
                .is_some_and(|layout| matches!(layout, crate::mir::layout::TypeLayout::Class(_)))
                || destination_place
                    .as_ref()
                    .and_then(|place| self.place_type_name(place))
                    .or_else(|| {
                        call_info
                            .resolved_symbol
                            .as_ref()
                            .and_then(|symbol| symbol.owner.clone())
                    })
                    .and_then(|type_name| self.symbol_index.reflection_descriptor(&type_name))
                    .is_some_and(|descriptor| {
                        matches!(
                            descriptor.kind,
                            crate::frontend::metadata::reflection::TypeKind::Class
                        )
                    });

            if owner_is_class {
                ParamMode::Value
            } else {
                ParamMode::Out
            }
        } else {
            ParamMode::Out
        };

        if call_info.is_constructor && !has_receiver {
            if destination_place.is_none() {
                let temp = self.create_temp(span);
                destination_place = Some(Place::new(temp));
            }
            if let Some(place) = destination_place.clone() {
                let receiver_operand = match constructor_receiver_mode {
                    ParamMode::Value => Operand::Copy(place.clone()),
                    ParamMode::In => self.borrow_argument_place(place.clone(), BorrowKind::Shared, span),
                    ParamMode::Ref | ParamMode::Out => {
                        self.borrow_argument_place(place.clone(), BorrowKind::Unique, span)
                    }
                };
                ordered_args.insert(0, receiver_operand);
                arg_modes.insert(0, constructor_receiver_mode);
            }
        } else if call_info.is_constructor && has_receiver {
            // Initializers write into the receiver; treat the receiver as an `out` parameter so
            // definite-assignment analysis considers the constructed value initialized.
            if let Some(mode) = arg_modes.get_mut(0) {
                *mode = constructor_receiver_mode;
            }
            if !matches!(constructor_receiver_mode, ParamMode::Value) {
                if let Some(receiver_arg) = ordered_args.get_mut(0) {
                    if let Operand::Copy(place) | Operand::Move(place) = receiver_arg {
                        let borrow_kind = match constructor_receiver_mode {
                            ParamMode::In => BorrowKind::Shared,
                            ParamMode::Ref | ParamMode::Out => BorrowKind::Unique,
                            ParamMode::Value => BorrowKind::Unique,
                        };
                        *receiver_arg = self.borrow_argument_place(place.clone(), borrow_kind, span);
                    }
                }
            }
        }

        let continue_block = self.new_block(span);
        let call_destination = if call_info.is_constructor {
            None
        } else {
            destination_place.clone()
        };

        if !call_info.is_constructor {
            if let Some(destination) = destination_place.as_ref() {
                let ret_ty = if let Some(CallDispatch::Trait(dispatch)) = call_dispatch.as_ref() {
                    self.trait_registry
                        .get(dispatch.trait_name.as_str())
                        .and_then(|trait_info| {
                            trait_info
                                .methods
                                .iter()
                                .find(|method| method.name == dispatch.method)
                        })
                        .map(|method_info| Ty::from_type_expr(&method_info.signature.return_type))
                } else {
                    call_info
                        .resolved_symbol
                        .as_ref()
                        .and_then(|symbol| {
                            self.instantiate_symbol_signature(symbol, &call_info)
                                .map(|(_, ret, _)| (symbol, ret))
                        })
                        .map(|(symbol, ret)| {
                            if has_receiver
                                && matches!(&ret, Ty::Named(named) if named.name == "Self")
                            {
                                self.align_params_with_receiver(
                                    has_receiver,
                                    &ordered_args,
                                    vec![Ty::Unknown; ordered_args.len().saturating_sub(1)],
                                    symbol
                                        .owner
                                        .as_deref()
                                        .or_else(|| call_info.receiver_owner.as_deref()),
                                )
                                .into_iter()
                                .next()
                                .unwrap_or(ret)
                            } else {
                                ret
                            }
                        })
                };
                if let Some(ret_ty) = ret_ty {
                    self.hint_local_ty(destination.local, ret_ty.clone());
                    self.ensure_ty_layout_for_ty(&ret_ty);
                }
            }
        }

        if self.unsafe_depth == 0
            && matches!(
                &func_operand,
                Operand::Copy(_) | Operand::Move(_) | Operand::Borrow(_)
            )
        {
            let fn_ty = self.operand_ty(&func_operand).and_then(|ty| match ty {
                Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                Ty::Nullable(inner) => match inner.as_ref() {
                    Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                    _ => None,
                },
                _ => None,
            });
            if let Some(fn_ty) = fn_ty
                && matches!(fn_ty.abi, crate::mir::Abi::Extern(_))
            {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "call to `{}` function pointer requires an `unsafe` block",
                        fn_ty.canonical_name()
                    ),
                    span,
                });
            }
        }

        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: func_operand,
                args: ordered_args,
                arg_modes,
                destination: call_destination,
                target: continue_block,
                unwind: unwind_target,
                dispatch: call_dispatch,
            },
        );
        self.switch_to_block(continue_block);

        if let Some((local, ty)) = result_type_hint {
            self.hint_local_ty(local, ty);
        }

        self.ffi_pointer_context = prior_ffi_pointer_context;

        if let Some(place) = destination_place {
            Some(Operand::Copy(place))
        } else {
            Some(Operand::Const(ConstOperand::new(ConstValue::Unit)))
        }
    }

    fn call_targets_ffi_abi(
        &self,
        info: &CallBindingInfo,
        func_operand: &Operand,
    ) -> bool {
        if let Some(symbol) = info.resolved_symbol.as_ref() {
            if matches!(symbol.signature.abi, crate::mir::Abi::Extern(_)) {
                return true;
            }
        }
        let fn_ty = self.operand_ty(func_operand);
        match fn_ty.as_ref() {
            Some(Ty::Fn(fn_ty)) => matches!(fn_ty.abi, crate::mir::Abi::Extern(_)),
            Some(Ty::Nullable(inner)) => match inner.as_ref() {
                Ty::Fn(fn_ty) => matches!(fn_ty.abi, crate::mir::Abi::Extern(_)),
                _ => false,
            },
            _ => false,
        }
    }

    fn align_params_with_receiver(
        &self,
        has_receiver: bool,
        ordered_args: &[Operand],
        params: Vec<Ty>,
        receiver_owner: Option<&str>,
    ) -> Vec<Ty> {
        if !has_receiver {
            return params;
        }
        if params.len() + 1 != ordered_args.len() {
            return params;
        }
        let operand_receiver_ty = ordered_args
            .first()
            .and_then(|op| self.operand_ty(op))
            .unwrap_or(Ty::Unknown);

        let receiver_ty = receiver_owner
            .and_then(|owner| {
                let primitive = Ty::named(owner.to_string());
                if matches!(primitive, Ty::String | Ty::Str) {
                    return Some(primitive);
                }
                crate::frontend::parser::parse_type_expression_text(owner)
                    .as_ref()
                    .map(Ty::from_type_expr)
                    .or_else(|| Some(primitive))
            })
            .map(|owner_ty| match owner_ty {
                Ty::String | Ty::Str => owner_ty,
                Ty::Named(owner_named) => match operand_receiver_ty.as_named() {
                    Some(actual_named)
                        if owner_named.args.is_empty()
                            && !actual_named.args.is_empty()
                            && owner_named.canonical_path() == actual_named.canonical_path() =>
                    {
                        Ty::Named(crate::mir::NamedTy::with_args(
                            owner_named.name,
                            actual_named.args.clone(),
                        ))
                    }
                    _ => operand_receiver_ty.clone(),
                },
                _ => operand_receiver_ty.clone(),
            })
            .unwrap_or(operand_receiver_ty);

        let mut aligned = Vec::with_capacity(params.len() + 1);
        aligned.push(receiver_ty);
        aligned.extend(params.into_iter());
        aligned
    }


    fn base_receiver_owner(&self) -> Option<String> {
        let current = self.current_self_type_name()?;
        let bases = self.class_bases.get(&current)?;
        bases.iter().find_map(|candidate| {
            self.lookup_class_layout_by_name(candidate)
                .map(|_| candidate.clone())
        })
    }

    fn receiver_owner_from_operand(&self, operand: &Operand) -> Option<String> {
        let ty = self.operand_ty(operand)?;
        let ty = Self::strip_nullable(&ty);
        let base_ty = if let Ty::Ref(reference) = ty {
            &reference.element
        } else {
            ty
        };
        let name = match base_ty {
            Ty::Named(named) => {
                if named.args.is_empty() {
                    self.resolve_ty_name(base_ty)
                        .unwrap_or_else(|| base_ty.canonical_name())
                } else {
                    named.canonical_path().to_string()
                }
            }
            _ => self
                .resolve_ty_name(base_ty)
                .unwrap_or_else(|| base_ty.canonical_name()),
        };
        let owner = name
            .split('<')
            .next()
            .unwrap_or(name.as_str())
            .trim_end_matches('?')
            .replace('.', "::");
        Some(owner)
    }

    fn try_lower_delegate_call(
        &mut self,
        func_operand: &Operand,
        args_meta: &[EvaluatedArg],
        destination_place: Option<Place>,
        span: Option<Span>,
    ) -> Option<Option<Operand>> {
        let place = match func_operand {
            Operand::Copy(place) | Operand::Move(place) => {
                let mut p = place.clone();
                self.normalise_place(&mut p);
                Some(p)
            }
            _ => None,
        }?;

        let initial_name = self.place_type_name(&place);
        if std::env::var("CHIC_DEBUG_DELEGATE_CALLS").is_ok() {
            eprintln!(
                "[delegate-call] operand={func_operand:?} place={place:?} ty={initial_name:?}"
            );
        }
        let Some(name_text) = initial_name else {
            return None;
        };
        let Some(type_expr) = crate::frontend::parser::parse_type_expression_text(&name_text) else {
            return None;
        };
        let mut ty = Ty::from_type_expr(&type_expr);
        let mut is_nullable = false;
        if let Ty::Nullable(inner) = ty {
            is_nullable = true;
            ty = *inner;
        }
        let (name, signature) = if let Some((delegate_name, sig)) =
            self.delegate_signature_for_ty(&ty)
        {
            (delegate_name, sig)
        } else {
            return None;
        };
        self.type_layouts.ensure_delegate_layout(&name);
        self.type_layouts.ensure_fn_layout(&signature);

        if args_meta.len() != signature.params.len() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "delegate `{name}` expects {} arguments but {} were provided",
                    signature.params.len(),
                    args_meta.len()
                ),
                span,
            });
        }

        let ordered_args = args_meta
            .iter()
            .map(|arg| arg.operand.clone())
            .collect::<Vec<_>>();
        let arg_modes = args_meta
            .iter()
            .map(|arg| match arg.modifier {
                Some(CallArgumentModifier::In) => ParamMode::In,
                Some(CallArgumentModifier::Ref) => ParamMode::Ref,
                Some(CallArgumentModifier::Out) => ParamMode::Out,
                None => ParamMode::Value,
            })
            .collect::<Vec<_>>();

        if is_nullable {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("invoking nullable delegate `{name}?` may panic at runtime"),
                span,
            });
        }

        let continue_block = self.new_block(span);
        let call_destination = if matches!(signature.ret.as_ref(), Ty::Unit) {
            None
        } else {
            destination_place.clone()
        };

        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = Ty::Fn(signature.clone());
            local.is_nullable = false;
        }

        let load_field = |index: u32| {
            let mut p = place.clone();
            p.projection.push(ProjectionElem::Field(index));
            Operand::Copy(p)
        };

        let value = Rvalue::Aggregate {
            kind: AggregateKind::Adt {
                name: signature.canonical_name(),
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

        let unwind_target = self.current_unwind_target();
        self.set_terminator(
            span,
            Terminator::Call {
                func: Operand::Copy(Place::new(temp)),
                args: ordered_args.clone(),
                arg_modes: arg_modes.clone(),
                destination: call_destination.clone(),
                target: continue_block,
                unwind: unwind_target,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);

        if let Some(place) = call_destination {
            self.hint_local_ty(place.local, (*signature.ret).clone());
            Some(Some(Operand::Copy(place)))
        } else {
            Some(Some(Operand::Const(ConstOperand::new(ConstValue::Unit))))
        }
    }

    pub(crate) fn make_self_operand(&mut self, _span: Option<Span>) -> Option<Operand> {
        let self_local = self.lookup_name("self")?;
        let mut place = Place::new(self_local);
        self.normalise_place(&mut place);
        Some(Operand::Copy(place))
    }

    fn should_elide_conditional_call(
        &mut self,
        call_info: &CallBindingInfo,
        _span: Option<Span>,
    ) -> bool {
        let candidates = self.gather_function_candidates(call_info);
        if candidates.is_empty() {
            return false;
        }

        let mut symbol: Option<String> = None;
        for candidate in candidates {
            if !matches!(candidate.signature.ret.as_ref(), Ty::Unit) {
                return false;
            }
            let Some(attr) = self.conditional_symbol_for(candidate) else {
                return false;
            };
            if self.conditional_symbol_enabled(&attr.symbol) {
                return false;
            }
            if let Some(existing) = &symbol {
                if existing != &attr.symbol {
                    return false;
                }
            } else {
                symbol = Some(attr.symbol);
            }
        }

        symbol.is_some()
    }

    fn conditional_symbol_for(&self, symbol: &FunctionSymbol) -> Option<ConditionalAttribute> {
        let decls = self.symbol_index.function_decls(&symbol.qualified)?;
        for decl in decls {
            if decl.internal_name == symbol.internal_name {
                let (attr, _) = crate::frontend::attributes::extract_conditional_attribute(
                    &decl.function.attributes,
                );
                if let Some(found) = attr {
                    return Some(found);
                }
            }
        }
        None
    }

    fn conditional_symbol_enabled(&self, symbol: &str) -> bool {
        match self.conditional_defines.get(symbol) {
            Some(conditional::DefineValue::Bool(flag)) => *flag,
            Some(conditional::DefineValue::String(_)) => true,
            None => false,
        }
    }
}
