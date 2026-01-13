use super::super::*;
use super::analysis::{
    CaptureCacheMetrics, CapturedLocal, LambdaLoweringBody, analyze_captures, capture_cache_key,
};
use super::environment::{
    LambdaParameterInfo, closure_temp_operand, convert_lambda_parameters, register_closure_layout,
};
use crate::drop_glue::{drop_glue_symbol_for, drop_type_identity};
use crate::frontend::ast::Expression;
use crate::frontend::parser::parse_block_text;
use crate::mir::builder::symbol_index::FunctionParamSymbol;
use crate::mir::layout::AutoTraitSet;
use crate::mir::{AggregateKind, PointerTy};
use crate::syntax::expr::{LambdaBody, LambdaExpr};
use tracing::trace;

#[derive(Clone, Debug)]
pub(crate) struct ClosureInfo {
    pub(crate) invoke_symbol: String,
    pub(crate) capture_fields: Vec<String>,
    pub(crate) fn_ty: FnTy,
    pub(crate) environment: Option<ClosureEnvironmentInfo>,
    pub(crate) params: Vec<FunctionParamSymbol>,
    pub(crate) capture_ty_name: Option<String>,
}

impl ClosureInfo {
    pub(crate) fn capture_count(&self) -> usize {
        self.capture_fields.len()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureEnvironmentInfo {
    pub(crate) drop_glue_symbol: Option<String>,
    pub(crate) env_size: Option<usize>,
    pub(crate) env_align: Option<usize>,
    pub(crate) env_ty_name: Option<String>,
}

pub(crate) fn convert_lambda_body(
    builder: &mut BodyBuilder<'_>,
    body: LambdaBody,
    span: Option<Span>,
) -> Option<LambdaLoweringBody> {
    match body {
        LambdaBody::Expression(expr) => Some(LambdaLoweringBody::Expression(expr)),
        LambdaBody::Block(block) => match parse_block_text(&block.text) {
            Ok(parsed) => Some(LambdaLoweringBody::Block(parsed)),
            Err(err) => {
                for diagnostic in err.diagnostics() {
                    let diag_span = diagnostic
                        .primary_label
                        .as_ref()
                        .map(|label| label.span)
                        .or(span);
                    builder.diagnostics.push(LoweringDiagnostic {
                        message: diagnostic.message.clone(),
                        span: diag_span,
                    });
                }
                None
            }
        },
    }
}

pub(crate) fn register_nested_lambda_function(
    builder: &mut BodyBuilder<'_>,
    capture_ty_name: &str,
    mut closure_sig: FnSig,
    invoke_symbol: String,
    lambda_span: Option<Span>,
    is_async: bool,
    captures: &[CapturedLocal],
    lambda_params: &[LambdaParameterInfo],
    body: LambdaLoweringBody,
) -> ClosureInfo {
    let body_span = lambda_span;
    let opaque_return = opaque_return_info_from_ty(&closure_sig.ret, lambda_span);
    let mut nested_builder = BodyBuilder::new(
        &closure_sig,
        lambda_span,
        &invoke_symbol,
        is_async,
        false,
        Vec::new(),
        builder.type_layouts,
        builder.type_visibilities,
        builder.primitive_registry,
        builder.default_arguments.clone(),
        builder.namespace.as_deref(),
        builder.current_package.clone(),
        builder.function_packages,
        builder.operator_registry,
        builder.string_interner,
        builder.symbol_index,
        builder.import_resolver,
        builder.static_registry,
        builder.class_bases,
        builder.class_virtual_slots,
        builder.trait_registry,
        FunctionKind::Function,
        false,
        builder.thread_runtime_mode,
        None,
        opaque_return,
        builder.generic_specializations.clone(),
    );

    for (index, capture) in captures.iter().enumerate() {
        nested_builder.ensure_ty_layout_for_ty(&capture.ty);
        let mut decl = LocalDecl::new(
            Some(capture.name.clone()),
            capture.ty.clone(),
            capture.is_mutable,
            lambda_span,
            LocalKind::Arg(index),
        )
        .with_param_mode(ParamMode::Value);
        if capture.is_nullable {
            decl.is_nullable = true;
        }
        let id = nested_builder.push_local(decl);
        nested_builder.bind_name(&capture.name, id);
    }

    for (parameter_index, param) in lambda_params.iter().enumerate() {
        nested_builder.ensure_ty_layout_for_ty(&param.ty);
        let mut decl = LocalDecl::new(
            Some(param.name.clone()),
            param.ty.clone(),
            param.mutable,
            lambda_span,
            LocalKind::Arg(captures.len() + parameter_index),
        )
        .with_param_mode(param.mode);
        if param.is_nullable {
            decl.is_nullable = true;
        }
        let id = nested_builder.push_local(decl);
        nested_builder.bind_name(&param.name, id);
    }

    match body {
        LambdaLoweringBody::Expression(expr) => {
            if let Some(value) = nested_builder.lower_expr_node(*expr, body_span) {
                let result_ty = nested_builder.operand_ty(&value);
                nested_builder.push_statement(MirStatement {
                    span: body_span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(LocalId(0)),
                        value: Rvalue::Use(value),
                    },
                });
                nested_builder.set_terminator(body_span, Terminator::Return);
                if let Some(ty) = result_ty {
                    closure_sig.ret = ty.clone();
                    nested_builder.return_type = ty;
                    if let Some(ret) = nested_builder.locals.get_mut(0) {
                        ret.ty = closure_sig.ret.clone();
                    }
                }
            }
        }
        LambdaLoweringBody::Block(block) => {
            nested_builder.lower_block(&block);
        }
    }

    let (body, mut diagnostics, mut constraints, nested_functions) = nested_builder.finish();
    let nested_function = MirFunction {
        name: invoke_symbol.clone(),
        kind: FunctionKind::Function,
        signature: closure_sig.clone(),
        body,
        is_async,
        async_result: None,
        is_generator: false,
        span: lambda_span,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    builder.register_nested_function(nested_function);
    builder.register_nested_functions(nested_functions);
    builder.diagnostics.append(&mut diagnostics);
    builder.constraints.append(&mut constraints);

    let fn_ty = FnTy::with_modes(
        lambda_params.iter().map(|param| param.ty.clone()).collect(),
        lambda_params.iter().map(|param| param.mode).collect(),
        closure_sig.ret.clone(),
        Abi::Chic,
        false,
    );
    let params = lambda_params
        .iter()
        .map(|param| FunctionParamSymbol {
            name: param.name.clone(),
            has_default: param.default.is_some(),
            mode: param.mode,
            is_extension_this: false,
        })
        .collect();

    let environment = if captures.is_empty() {
        None
    } else {
        let ty = Ty::named(capture_ty_name.to_string());
        let canonical = ty.canonical_name();
        let requires_drop = builder.type_layouts.type_requires_drop(&canonical);
        let drop_glue_symbol = if requires_drop {
            Some(drop_glue_symbol_for(&canonical))
        } else {
            None
        };
        let layout_info = builder.type_layouts.size_and_align_for_ty(&ty);
        Some(ClosureEnvironmentInfo {
            drop_glue_symbol,
            env_size: layout_info.map(|(size, _)| size),
            env_align: layout_info.map(|(_, align)| align),
            env_ty_name: Some(canonical),
        })
    };

    ClosureInfo {
        invoke_symbol,
        capture_fields: captures
            .iter()
            .map(|capture| capture.name.clone())
            .collect(),
        fn_ty,
        environment,
        params,
        capture_ty_name: Some(capture_ty_name.to_string()),
    }
}

fn ty_matches(actual: &Ty, expected: &Ty) -> bool {
    matches!(actual, Ty::Unknown) || actual == expected
}

fn fn_types_compatible(actual: &FnTy, expected: &FnTy) -> bool {
    actual.abi == expected.abi
        && actual.params.len() == expected.params.len()
        && actual.param_modes.len() == expected.param_modes.len()
        && actual
            .param_modes
            .iter()
            .zip(expected.param_modes.iter())
            .all(|(a, e)| a == e)
        && actual
            .params
            .iter()
            .zip(expected.params.iter())
            .all(|(a, e)| ty_matches(a, e))
        && ty_matches(&actual.ret, &expected.ret)
}

body_builder_impl! {
    pub(crate) fn build_fn_pointer_value(
        &mut self,
        fn_ty: &FnTy,
        invoke_symbol: String,
        context: Operand,
        drop_glue: Operand,
        type_id: Operand,
        env_size: Operand,
        env_align: Operand,
        span: Option<Span>,
    ) -> Operand {
        if matches!(fn_ty.abi, Abi::Extern(_)) {
            let temp = self.create_temp(span);
            if let Some(local) = self.locals.get_mut(temp.0) {
                local.ty = Ty::Fn(fn_ty.clone());
                local.is_nullable = false;
            }
            let value = Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Symbol(
                invoke_symbol,
            ))));
            self.push_statement(MirStatement {
                span,
                kind: MirStatementKind::Assign {
                    place: Place::new(temp),
                    value,
                },
            });
            return Operand::Copy(Place::new(temp));
        }
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = Ty::Fn(fn_ty.clone());
            local.is_nullable = false;
        }
        self.ensure_ty_layout_for_ty(&Ty::Fn(fn_ty.clone()));
        let value = Rvalue::Aggregate {
            kind: AggregateKind::Adt {
                name: fn_ty.canonical_name(),
                variant: None,
            },
            fields: vec![
                Operand::Const(ConstOperand::new(ConstValue::Symbol(invoke_symbol))),
                context,
                drop_glue,
                type_id,
                env_size,
                env_align,
            ],
        };
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value,
            },
        });
        Operand::Copy(Place::new(temp))
    }

    pub(crate) fn build_delegate_value(
        &mut self,
        delegate_name: &str,
        invoke_symbol: String,
        context: Operand,
        drop_glue: Operand,
        type_id: Operand,
        env_size: Operand,
        env_align: Operand,
        auto_traits: Option<AutoTraitSet>,
        span: Option<Span>,
        delegate_ty: Option<Ty>,
    ) -> Operand {
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = delegate_ty.unwrap_or_else(|| Ty::named(delegate_name.to_string()));
            local.is_nullable = false;
        }
        self.type_layouts.ensure_delegate_layout(delegate_name);
        if let Some(traits) = auto_traits {
            self.type_layouts
                .record_delegate_auto_traits(delegate_name.to_string(), traits);
        }
        let value = Rvalue::Aggregate {
            kind: AggregateKind::Adt {
                name: delegate_name.to_string(),
                variant: None,
            },
            fields: vec![
                Operand::Const(ConstOperand::new(ConstValue::Symbol(invoke_symbol))),
                context,
                drop_glue,
                type_id,
                env_size,
                env_align,
            ],
        };
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value,
            },
        });
        Operand::Copy(Place::new(temp))
    }

    pub(crate) fn synthesise_fn_adapter(
        &mut self,
        info: &ClosureInfo,
        capture_ty: Option<String>,
        target_symbol: String,
        span: Option<Span>,
    ) -> Option<String> {
        let base_name = self.function_name.clone();
        let adapter_symbol =
            format!("{base_name}::to_fn_ptr#{}", self.allocate_closure_id());
        let mut params: Vec<Ty> = Vec::with_capacity(info.fn_ty.params.len() + 1);
        let context_ty = capture_ty
            .as_ref()
            .map(|name| Ty::Pointer(Box::new(PointerTy::new(Ty::named(name.clone()), true))))
            .unwrap_or_else(|| Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true))));
        params.push(context_ty.clone());
        params.extend(info.fn_ty.params.iter().cloned());

        let sig = FnSig {
            params: params.clone(),
            ret: info.fn_ty.ret.as_ref().clone(),
            abi: info.fn_ty.abi.clone(),
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        };
        let mut body = MirBody::new(params.len(), span);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            sig.ret.clone(),
            false,
            span,
            LocalKind::Return,
        ));
        for (index, ty) in params.iter().enumerate() {
            let decl = LocalDecl::new(
                None,
                ty.clone(),
                matches!(index, 0),
                span,
                LocalKind::Arg(index),
            )
            .with_param_mode(if index == 0 {
                ParamMode::Value
            } else {
                info.params
                    .get(index.saturating_sub(1))
                    .map(|p| p.mode)
                    .unwrap_or(ParamMode::Value)
            });
            body.locals.push(decl);
        }

        let mut blocks = Vec::new();
        let mut block = BasicBlock::new(BlockId(0), span);
        let mut call_args = Vec::new();
        let mut arg_modes = Vec::new();

        if let Some(name) = capture_ty.as_ref()
            && let Some(layout) = self.type_layouts.layout_for_name(name)
        {
            if let TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) = layout {
                let mut fields = struct_layout.fields.clone();
                fields.sort_by_key(|f| f.index);
                for field in fields {
                    let place = Place {
                        local: LocalId(1),
                        projection: vec![ProjectionElem::Deref, ProjectionElem::Field(field.index)],
                    };
                    call_args.push(Operand::Copy(place));
                    arg_modes.push(ParamMode::Value);
                }
            }
        }

        for (idx, _param) in info.fn_ty.params.iter().enumerate() {
            let place = Place::new(LocalId(idx + 2));
            call_args.push(Operand::Copy(place));
            let mode = info
                .params
                .get(idx)
                .map(|p| p.mode)
                .unwrap_or(ParamMode::Value);
            arg_modes.push(mode);
        }

        let call_target = BlockId(1);
        block.terminator = Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(target_symbol))),
            args: call_args,
            arg_modes,
            destination: if matches!(sig.ret, Ty::Unit) {
                None
            } else {
                Some(Place::new(LocalId(0)))
            },
            target: call_target,
            unwind: None,
            dispatch: None,
        });
        blocks.push(block);

        let mut ret_block = BasicBlock::new(call_target, span);
        ret_block.terminator = Some(Terminator::Return);
        blocks.push(ret_block);

        body.blocks = blocks;
        let function = MirFunction {
            name: adapter_symbol.clone(),
            kind: FunctionKind::Function,
            signature: sig,
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        self.register_nested_function(function);
        Some(adapter_symbol)
    }

    pub(crate) fn build_plain_fn_adapter(
        &mut self,
        fn_ty: &FnTy,
        target_symbol: &str,
        span: Option<Span>,
        context_param_ty: Option<Ty>,
        include_context_arg: bool,
    ) -> String {
        let base_name = self.function_name.clone();
        let adapter_symbol =
            format!("{base_name}::fn_ptr_adapter#{}", self.allocate_closure_id());
        let mut params: Vec<Ty> = Vec::with_capacity(fn_ty.params.len() + 1);
        let context_ty = context_param_ty.unwrap_or_else(|| {
            Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)))
        });
        params.push(context_ty);
        params.extend(fn_ty.params.iter().cloned());
        let sig = FnSig {
            params: params.clone(),
            ret: fn_ty.ret.as_ref().clone(),
            abi: fn_ty.abi.clone(),
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        };
        let mut body = MirBody::new(params.len(), span);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            sig.ret.clone(),
            false,
            span,
            LocalKind::Return,
        ));
        for (index, ty) in params.iter().enumerate() {
            body.locals.push(
                LocalDecl::new(None, ty.clone(), false, span, LocalKind::Arg(index))
                    .with_param_mode(ParamMode::Value),
            );
        }
        let mut blocks = Vec::new();
        let mut entry = BasicBlock::new(BlockId(0), span);
        let mut args = Vec::new();
        let mut arg_modes = Vec::new();
        if include_context_arg {
            args.push(Operand::Copy(Place::new(LocalId(1))));
            arg_modes.push(ParamMode::Value);
        }
        for idx in 0..fn_ty.params.len() {
            let place = Place::new(LocalId(idx + 2));
            args.push(Operand::Copy(place));
            arg_modes.push(ParamMode::Value);
        }
        entry.terminator = Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                target_symbol.to_string(),
            ))),
            args,
            arg_modes,
            destination: if matches!(sig.ret, Ty::Unit) {
                None
            } else {
                Some(Place::new(LocalId(0)))
            },
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        });
        blocks.push(entry);
        let mut ret_block = BasicBlock::new(BlockId(1), span);
        ret_block.terminator = Some(Terminator::Return);
        blocks.push(ret_block);
        body.blocks = blocks;
        let function = MirFunction {
            name: adapter_symbol.clone(),
            kind: FunctionKind::Function,
            signature: sig,
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        self.register_nested_function(function);
        adapter_symbol
    }

    pub(crate) fn capture_cache_metrics(&self) -> CaptureCacheMetrics {
        self.capture_cache.metrics()
    }

    pub(crate) fn convert_closure_operand_to_fn_ptr(
        &mut self,
        operand: Operand,
        info: &ClosureInfo,
        span: Option<Span>,
            ) -> Option<Operand> {
        let type_id_operand = Operand::Const(ConstOperand::new(ConstValue::UInt(
            drop_type_identity(&info.fn_ty.canonical_name()).into(),
        )));
        let adapter_symbol = self.synthesise_fn_adapter(
            info,
            info.capture_ty_name
                .clone()
                .or_else(|| info.environment.as_ref().and_then(|env| env.env_ty_name.clone())),
            info.invoke_symbol.clone(),
            span,
        )?;

        let (context_operand, env_size_operand, env_align_operand, drop_operand) =
            if info.capture_count() == 0 {
                (
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                )
            } else {
                let Some(env_info) = &info.environment else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment metadata missing for `.to_fn_ptr()`".into(),
                        span,
                    });
                    return None;
                };
                let Some(size) = env_info.env_size else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment size unavailable for `.to_fn_ptr()`".into(),
                        span,
                    });
                    return None;
                };
                let Some(align) = env_info.env_align else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment alignment unavailable for `.to_fn_ptr()`".into(),
                        span,
                    });
                    return None;
                };
                let env_place = match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        let mut place = place.clone();
                        self.normalise_place(&mut place);
                        place
                    }
                    Operand::Borrow(borrow) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "captured closures cannot be converted to function pointers through a borrow"
                                .into(),
                            span: borrow.span.or(span),
                        });
                        return None;
                    }
                    Operand::Const(_) | Operand::Pending(_) | Operand::Mmio(_) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "unsupported closure operand for `.to_fn_ptr()` conversion".into(),
                            span,
                        });
                        return None;
                    }
                };

                let src_ptr_temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(src_ptr_temp.0) {
                    if let Some(name) = env_info.env_ty_name.clone() {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named(name), true)));
                    } else {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
                    }
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(src_ptr_temp),
                        value: Rvalue::AddressOf {
                            mutability: Mutability::Mutable,
                            place: env_place,
                        },
                    },
                });

                let clone_temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(clone_temp.0) {
                    if let Some(name) = env_info.env_ty_name.clone() {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named(name), true)));
                    } else {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
                    }
                    local.is_nullable = false;
                }
                let continue_block = self.new_block(span);
                let unwind_target = self.current_unwind_target();
                self.set_terminator(
                    span,
                    Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "chic_rt_closure_env_clone".to_string(),
                        ))),
                        args: vec![
                            Operand::Copy(Place::new(src_ptr_temp)),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(
                                u64::try_from(size).unwrap_or(0).into(),
                            ))),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(
                                u64::try_from(align).unwrap_or(0).into(),
                            ))),
                        ],
                        arg_modes: vec![ParamMode::Value; 3],
                        destination: Some(Place::new(clone_temp)),
                        target: continue_block,
                        unwind: unwind_target,
                        dispatch: None,
                    },
                );
                self.switch_to_block(continue_block);

                let drop_operand = env_info
                    .drop_glue_symbol
                    .clone()
                    .map_or(Operand::Const(ConstOperand::new(ConstValue::Null)), |symbol| {
                        Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)))
                    });

                (
                    Operand::Copy(Place::new(clone_temp)),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(
                        u64::try_from(size).unwrap_or(0).into(),
                    ))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(
                        u64::try_from(align).unwrap_or(0).into(),
                    ))),
                    drop_operand,
                )
            };

        Some(self.build_fn_pointer_value(
            &info.fn_ty,
            adapter_symbol,
            context_operand,
            drop_operand,
            type_id_operand,
            env_size_operand,
            env_align_operand,
            span,
        ))
    }

    fn cached_captures(
        &mut self,
        body: &LambdaLoweringBody,
    ) -> (Vec<CapturedLocal>, bool) {
        let key = capture_cache_key(body);
        let (captures, hit) = if let Some(existing) = self.capture_cache.get(&key) {
            (existing, true)
        } else {
            let computed = analyze_captures(self, body);
            self.capture_cache.insert(key.clone(), computed.clone());
            (computed, false)
        };
        let metrics = self.capture_cache_metrics();
        if hit {
            trace!(
                target = "mir::closures",
                event = "capture_cache.hit",
                hits = metrics.hits,
                misses = metrics.misses
            );
        } else {
            trace!(
                target = "mir::closures",
                event = "capture_cache.miss",
                hits = metrics.hits,
                misses = metrics.misses
            );
        }
        (captures, hit)
    }

    pub(crate) fn allocate_closure_id(&mut self) -> usize {
        let id = self.next_closure_id;
        self.next_closure_id += 1;
        id
    }

    pub(crate) fn register_nested_function(&mut self, function: MirFunction) {
        self.nested_functions.push(function);
    }

    pub(crate) fn register_nested_functions(&mut self, functions: Vec<MirFunction>) {
        self.nested_functions.extend(functions);
    }

    pub(crate) fn specialise_closure_signature(
        &mut self,
        type_name: &str,
        invoke_symbol: &str,
        target_sig: &FnTy,
    ) {
        let mut capture_count = 0usize;
        if let Some(info) = self.closure_registry.get_mut(type_name) {
            info.fn_ty = target_sig.clone();
            capture_count = info.capture_count();
        }
        let mut invoke_params = None;
        for function in self.nested_functions.iter_mut() {
            if function.name == invoke_symbol {
                let new_params = if capture_count == 0 {
                    target_sig.params.clone()
                } else {
                    let mut params = function.signature.params.clone();
                    if params.len() >= capture_count {
                        params.truncate(capture_count);
                        params.extend(target_sig.params.iter().cloned());
                    } else {
                        params = target_sig.params.clone();
                    }
                    params
                };
                function.signature.params = new_params;
                function.signature.ret = (*target_sig.ret).clone();
                if let Some(ret_local) = function.body.locals.get_mut(0) {
                    ret_local.ty = (*target_sig.ret).clone();
                }
                let mut arg_index = 0usize;
                for local in function.body.locals.iter_mut() {
                    if matches!(local.kind, LocalKind::Arg(_)) {
                        if capture_count == 0 {
                            if let Some(ty) = target_sig.params.get(arg_index) {
                                local.ty = ty.clone();
                            }
                        } else if arg_index >= capture_count {
                            if let Some(ty) = target_sig.params.get(arg_index - capture_count) {
                                local.ty = ty.clone();
                            }
                        }
                        arg_index += 1;
                    }
                }
                invoke_params = Some(function.signature.params.clone());
                break;
            }
        }
        let fn_ty = if capture_count == 0 {
            target_sig.clone()
        } else if let Some(params) = invoke_params {
            let mut param_modes = Vec::with_capacity(params.len());
            param_modes.extend(std::iter::repeat(ParamMode::Value).take(capture_count));
            param_modes.extend(target_sig.param_modes.iter().cloned());
            while param_modes.len() < params.len() {
                param_modes.push(ParamMode::Value);
            }
            FnTy::with_modes(
                params,
                param_modes,
                (*target_sig.ret).clone(),
                target_sig.abi.clone(),
                target_sig.variadic,
            )
        } else {
            target_sig.clone()
        };
        self.register_closure_fn_signature(invoke_symbol.to_string(), fn_ty);
    }

    pub(crate) fn register_closure_info(&mut self, name: String, info: ClosureInfo) {
        let invoke_symbol = info.invoke_symbol.clone();
        let fn_ty = info.fn_ty.clone();
        self.closure_registry.insert(name, info);
        self.register_closure_fn_signature(invoke_symbol, fn_ty);
    }

    pub(crate) fn register_closure_fn_signature(&mut self, symbol: String, fn_ty: FnTy) {
        self.closure_fn_signatures.insert(symbol, fn_ty);
    }

    pub(crate) fn register_lambda_default_arguments(
        &mut self,
        symbol: &str,
        lambda_params: &[LambdaParameterInfo],
    ) {
        if lambda_params.iter().all(|param| param.default.is_none()) {
            return;
        }
        let mut entries = Vec::with_capacity(lambda_params.len());
        for (index, param) in lambda_params.iter().enumerate() {
            if let Some(expr) = &param.default {
                let thunk_symbol = format!("{symbol}::default_arg#{index}");
                let value = self.build_lambda_default_thunk(thunk_symbol, expr, &param.ty);
                entries.push(value);
            } else {
                entries.push(None);
            }
        }
        if entries.iter().any(|entry| entry.is_some()) {
            self.default_arguments
                .borrow_mut()
                .record(symbol.to_string(), entries);
        }
    }

    pub(crate) fn convert_closure_operand_to_delegate(
        &mut self,
        operand: Operand,
        info: &ClosureInfo,
        delegate_name: &str,
        _delegate_sig: &FnTy,
        span: Option<Span>,
    ) -> Option<Operand> {
        let type_id_operand =
            Operand::Const(ConstOperand::new(ConstValue::UInt(drop_type_identity(delegate_name).into())));
        let adapter_symbol = self.synthesise_fn_adapter(
            info,
            info.capture_ty_name
                .clone()
                .or_else(|| info.environment.as_ref().and_then(|env| env.env_ty_name.clone())),
            info.invoke_symbol.clone(),
            span,
        )?;

        let mut auto_traits = AutoTraitSet::all_yes();
        let (context_operand, env_size_operand, env_align_operand, drop_operand) =
            if info.capture_count() == 0 {
                (
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(0))),
                    Operand::Const(ConstOperand::new(ConstValue::Null)),
                )
            } else {
                let Some(env_info) = &info.environment else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment metadata missing for delegate conversion".into(),
                        span,
                    });
                    return None;
                };
                let Some(size) = env_info.env_size else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment size unavailable for delegate conversion".into(),
                        span,
                    });
                    return None;
                };
                let Some(align) = env_info.env_align else {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "closure environment alignment unavailable for delegate conversion".into(),
                        span,
                    });
                    return None;
                };
                let env_place = match operand {
                    Operand::Copy(place) | Operand::Move(place) => {
                        let mut place = place.clone();
                        self.normalise_place(&mut place);
                        place
                    }
                    Operand::Borrow(borrow) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message:
                                "captured closures cannot be converted to delegates through a borrow"
                                    .into(),
                            span: borrow.span.or(span),
                        });
                        return None;
                    }
                    Operand::Const(_) | Operand::Pending(_) | Operand::Mmio(_) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "unsupported closure operand for delegate conversion".into(),
                            span,
                        });
                        return None;
                    }
                };

                let src_ptr_temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(src_ptr_temp.0) {
                    if let Some(name) = env_info.env_ty_name.clone() {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named(name), true)));
                    } else {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
                    }
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(src_ptr_temp),
                        value: Rvalue::AddressOf {
                            mutability: Mutability::Mutable,
                            place: env_place,
                        },
                    },
                });

                let clone_temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(clone_temp.0) {
                    if let Some(name) = env_info.env_ty_name.clone() {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::named(name), true)));
                    } else {
                        local.ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
                    }
                    local.is_nullable = false;
                }
                let continue_block = self.new_block(span);
                let unwind_target = self.current_unwind_target();
                self.set_terminator(
                    span,
                    Terminator::Call {
                        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                            "chic_rt_closure_env_clone".to_string(),
                        ))),
                        args: vec![
                            Operand::Copy(Place::new(src_ptr_temp)),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(
                                u64::try_from(size).unwrap_or(0).into(),
                            ))),
                            Operand::Const(ConstOperand::new(ConstValue::UInt(
                                u64::try_from(align).unwrap_or(0).into(),
                            ))),
                        ],
                        arg_modes: vec![ParamMode::Value; 3],
                        destination: Some(Place::new(clone_temp)),
                        target: continue_block,
                        unwind: unwind_target,
                        dispatch: None,
                    },
                );
                self.switch_to_block(continue_block);

                let drop_operand = env_info
                    .drop_glue_symbol
                    .clone()
                    .map_or(Operand::Const(ConstOperand::new(ConstValue::Null)), |symbol| {
                        Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)))
                    });

                auto_traits = env_info
                    .env_ty_name
                    .as_ref()
                    .map(|name| self.type_layouts.auto_traits_for_type(&Ty::named(name.clone())))
                    .unwrap_or_else(AutoTraitSet::all_yes);
                (
                    Operand::Copy(Place::new(clone_temp)),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(size as u128))),
                    Operand::Const(ConstOperand::new(ConstValue::UInt(align as u128))),
                    drop_operand,
                )
            };

        Some(self.build_delegate_value(
            delegate_name,
            adapter_symbol,
            context_operand,
            drop_operand,
            type_id_operand,
            env_size_operand,
            env_align_operand,
            Some(auto_traits),
            span,
            Some(Ty::named(delegate_name.to_string())),
        ))
    }

    fn build_lambda_default_thunk(
        &mut self,
        thunk_symbol: String,
        expr: &Expression,
        target_ty: &Ty,
    ) -> Option<DefaultArgumentValue> {
        let sig = FnSig {
            params: Vec::new(),
            ret: target_ty.clone(),
            abi: Abi::Chic,
            effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };
        let opaque_return = opaque_return_info_from_ty(target_ty, expr.span);
        let mut builder = BodyBuilder::new(
            &sig,
            expr.span,
            &thunk_symbol,
            false,
            false,
            Vec::new(),
            self.type_layouts,
            self.type_visibilities,
            self.primitive_registry,
            self.default_arguments.clone(),
            self.namespace.as_deref(),
            self.current_package.clone(),
            self.function_packages,
            self.operator_registry,
            self.string_interner,
            self.symbol_index,
            self.import_resolver,
            self.static_registry,
            self.class_bases,
            self.class_virtual_slots,
            self.trait_registry,
            FunctionKind::Function,
            false,
            self.thread_runtime_mode,
            None,
            opaque_return,
            self.generic_specializations.clone(),
        );
        let Some(node) = expr.node.clone() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "default value for parameter `{thunk_symbol}` is not a parsable expression"
                ),
                span: expr.span,
            });
            return None;
        };
        let Some(value) = builder.lower_expr_node(node, expr.span) else {
            return None;
        };
        builder.push_statement(MirStatement {
            span: expr.span,
            kind: MirStatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(value),
            },
        });
        builder.set_terminator(expr.span, Terminator::Return);
        let (body, mut diagnostics, mut constraints, nested_functions) = builder.finish();
        self.diagnostics.append(&mut diagnostics);
        self.constraints.append(&mut constraints);
        self.register_nested_function(MirFunction {
            name: thunk_symbol.clone(),
            kind: FunctionKind::Function,
            signature: sig,
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: expr.span,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        });
        self.register_nested_functions(nested_functions);
        Some(DefaultArgumentValue::Thunk {
            symbol: thunk_symbol,
            metadata_count: 0,
            span: expr.span,
        })
    }

    pub(crate) fn try_closure_to_fn_pointer(
        &mut self,
        operand: &Operand,
        expected: &FnTy,
        span: Option<Span>,
            ) -> Option<Result<Operand, ()>> {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            Operand::Borrow(borrow) => borrow.place.clone(),
            _ => return None,
        };
        let type_name = self.place_type_name(&place)?;
        let mut info = self.closure_registry.get(&type_name)?.clone();
        if info.capture_count() > 0 {
            let Some(converted) =
                self.convert_closure_operand_to_fn_ptr(operand.clone(), &info, span) else {
                    return Some(Err(()));
                };
            return Some(Ok(converted));
        }

        if fn_types_compatible(&info.fn_ty, expected) {
            self.specialise_closure_signature(&type_name, &info.invoke_symbol, expected);
            info = self
                .closure_registry
                .get(&type_name)
                .cloned()
                .unwrap_or(info);
            let Some(converted) =
                self.convert_closure_operand_to_fn_ptr(operand.clone(), &info, span) else {
                    return Some(Err(()));
                };
            return Some(Ok(converted));
        }

        let actual_sig = info.fn_ty.canonical_name();
        let expected_sig = expected.canonical_name();
        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "closure `{}` has signature `{actual_sig}` but `{expected_sig}` is required",
                info.invoke_symbol
            ),
            span,
                    });
        Some(Err(()))
    }

    pub(crate) fn lower_lambda_expr(
        &mut self,
        lambda: LambdaExpr,
        span: Option<Span>,
            ) -> Option<Operand> {
        let LambdaExpr {
            params,
            body,
            is_async,
            span: lambda_span,
                        ..
        } = lambda;

        let lowering_body = convert_lambda_body(self, body, span)?;
        let (captures, _cache_hit) = self.cached_captures(&lowering_body);

        let closure_id = self.allocate_closure_id();
        let capture_ty_name = format!("{}::lambda#{}", self.function_name, closure_id);
        register_closure_layout(self, &capture_ty_name, &captures);

        let lambda_params = convert_lambda_parameters(self, &params, span);

        let closure_operand = closure_temp_operand(self, span, &capture_ty_name, &captures);

        let mut param_types = Vec::with_capacity(captures.len() + lambda_params.len());
        for capture in &captures {
            param_types.push(capture.ty.clone());
        }
        for param in &lambda_params {
            param_types.push(param.ty.clone());
        }

        let closure_sig = FnSig {
            params: param_types,
            ret: Ty::Unknown,
            abi: Abi::Chic,
            effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };

        let invoke_symbol = format!("{capture_ty_name}::Invoke");
        let info = register_nested_lambda_function(
            self,
            &capture_ty_name,
            closure_sig,
            invoke_symbol.clone(),
            lambda_span.or(span),
            is_async,
            &captures,
            &lambda_params,
            lowering_body,
        );

        self.register_lambda_default_arguments(&invoke_symbol, &lambda_params);
        self.register_closure_info(capture_ty_name, info);
        Some(closure_operand)
    }

    pub(crate) fn prepare_closure_call(
        &mut self,
        operand: &Operand,
    ) -> Option<(String, Vec<Operand>)> {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => place.clone(),
            _ => return None,
        };
        let type_name = self.place_type_name(&place)?;
        let info = self.closure_registry.get(&type_name)?;
        let mut captures = Vec::with_capacity(info.capture_count());
        for field in &info.capture_fields {
            let mut projection = place.clone();
            projection
                .projection
                .push(ProjectionElem::FieldNamed(field.clone()));
            self.normalise_place(&mut projection);
            captures.push(Operand::Copy(projection));
        }
        Some((info.invoke_symbol.clone(), captures))
    }
}
