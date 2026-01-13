use super::helpers::collect_type_param_names;
use super::*;

pub(crate) struct DefaultArgumentCtx<'a> {
    pub(crate) internal_name: &'a str,
    pub(crate) display_name: &'a str,
    pub(crate) owner: Option<&'a str>,
    pub(crate) namespace: Option<&'a str>,
    pub(crate) type_generics: Vec<String>,
    pub(crate) method_generics: Vec<String>,
}

impl ModuleLowering {
    pub(crate) fn build_function_default_arguments(
        &mut self,
        internal: String,
        decls: Vec<FunctionDeclSymbol>,
    ) {
        let Some(primary) = decls.first() else {
            return;
        };
        let Some(merged) = self.merge_function_parameter_defaults(&decls) else {
            return;
        };
        if merged.iter().all(|entry| entry.is_none()) {
            return;
        }
        let mut lowered: Vec<Option<DefaultArgumentValue>> = Vec::with_capacity(merged.len());
        for (index, expr) in merged.into_iter().enumerate() {
            if let Some(expression) = expr {
                let value = self.lower_function_default_value(primary, index, &expression);
                if let Some(ref entry) = value {
                    self.record_default_argument_metadata(
                        &primary.qualified,
                        &primary.internal_name,
                        &primary.function.signature.parameters[index].name,
                        index,
                        expression.span,
                        entry,
                    );
                }
                lowered.push(value);
            } else {
                lowered.push(None);
            }
        }
        if lowered.iter().any(|entry| entry.is_some()) {
            self.default_arguments
                .borrow_mut()
                .record(internal, lowered);
        }
    }

    pub(crate) fn build_constructor_default_arguments(
        &mut self,
        internal: String,
        decls: Vec<ConstructorDeclSymbol>,
    ) {
        let Some(primary) = decls.first() else {
            return;
        };
        let Some(merged) = self.merge_constructor_parameter_defaults(&decls) else {
            return;
        };
        if merged.iter().all(|entry| entry.is_none()) {
            return;
        }
        let mut lowered: Vec<Option<DefaultArgumentValue>> = Vec::with_capacity(merged.len());
        for (index, expr) in merged.into_iter().enumerate() {
            if let Some(expression) = expr {
                let value = self.lower_constructor_default_value(primary, index, &expression);
                if let Some(ref entry) = value {
                    self.record_default_argument_metadata(
                        &primary.qualified,
                        &primary.internal_name,
                        &primary.constructor.parameters[index].name,
                        index,
                        expression.span,
                        entry,
                    );
                }
                lowered.push(value);
            } else {
                lowered.push(None);
            }
        }
        if lowered.iter().any(|entry| entry.is_some()) {
            self.default_arguments
                .borrow_mut()
                .record(internal, lowered);
        }
    }

    fn merge_function_parameter_defaults(
        &mut self,
        decls: &[FunctionDeclSymbol],
    ) -> Option<Vec<Option<Expression>>> {
        let primary = decls.first()?;
        let param_len = primary.function.signature.parameters.len();
        let mut merged: Vec<Option<Expression>> = vec![None; param_len];
        let mut has_defaults = false;
        for decl in decls {
            if decl.function.signature.parameters.len() != param_len {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "declaration of `{}` does not match overload parameter count",
                        decl.qualified
                    ),
                    span: None,
                });
                continue;
            }
            for (index, param) in decl.function.signature.parameters.iter().enumerate() {
                if let Some(default_expr) = &param.default {
                    has_defaults = true;
                    if let Some(existing) = &merged[index] {
                        if existing.text.trim() != default_expr.text.trim() {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "conflicting default values for parameter `{}` on `{}`",
                                    param.name, decl.qualified
                                ),
                                span: default_expr.span,
                            });
                        }
                    } else {
                        merged[index] = Some(default_expr.clone());
                    }
                }
            }
        }
        if has_defaults { Some(merged) } else { None }
    }

    fn merge_constructor_parameter_defaults(
        &mut self,
        decls: &[ConstructorDeclSymbol],
    ) -> Option<Vec<Option<Expression>>> {
        let primary = decls.first()?;
        let param_len = primary.constructor.parameters.len();
        let mut merged: Vec<Option<Expression>> = vec![None; param_len];
        let mut has_defaults = false;
        for decl in decls {
            if decl.constructor.parameters.len() != param_len {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "constructor `{}` has inconsistent parameter counts across declarations",
                        decl.qualified
                    ),
                    span: decl.constructor.span,
                });
                continue;
            }
            for (index, param) in decl.constructor.parameters.iter().enumerate() {
                if let Some(default_expr) = &param.default {
                    has_defaults = true;
                    if let Some(existing) = &merged[index] {
                        if existing.text.trim() != default_expr.text.trim() {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "conflicting default values for parameter `{}` on `{}`",
                                    param.name, decl.qualified
                                ),
                                span: default_expr.span,
                            });
                        }
                    } else {
                        merged[index] = Some(default_expr.clone());
                    }
                }
            }
        }
        if has_defaults { Some(merged) } else { None }
    }

    fn lower_function_default_value(
        &mut self,
        decl: &FunctionDeclSymbol,
        index: usize,
        expr: &Expression,
    ) -> Option<DefaultArgumentValue> {
        self.lower_default_argument_value(
            DefaultArgumentCtx {
                internal_name: &decl.internal_name,
                display_name: &decl.qualified,
                owner: decl.owner.as_deref(),
                namespace: decl.namespace.as_deref(),
                type_generics: self.collect_owner_generic_names(decl.owner.as_deref()),
                method_generics: collect_type_param_names(decl.function.generics.as_ref()),
            },
            &decl.function.signature.parameters[index],
            index,
            expr,
        )
    }

    fn lower_constructor_default_value(
        &mut self,
        decl: &ConstructorDeclSymbol,
        index: usize,
        expr: &Expression,
    ) -> Option<DefaultArgumentValue> {
        self.lower_default_argument_value(
            DefaultArgumentCtx {
                internal_name: &decl.internal_name,
                display_name: &decl.qualified,
                owner: Some(decl.owner.as_str()),
                namespace: decl.namespace.as_deref(),
                type_generics: self.collect_owner_generic_names(Some(decl.owner.as_str())),
                method_generics: Vec::new(),
            },
            &decl.constructor.parameters[index],
            index,
            expr,
        )
    }

    fn lower_default_argument_value(
        &mut self,
        ctx: DefaultArgumentCtx<'_>,
        param: &Parameter,
        index: usize,
        expr: &Expression,
    ) -> Option<DefaultArgumentValue> {
        if matches!(param.binding, BindingModifier::Ref | BindingModifier::Out) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "parameter `{}` in `{}` cannot declare a default value because `{}` parameters require explicit caller input",
                    param.name,
                    ctx.display_name,
                    match param.binding {
                        BindingModifier::Ref => "ref",
                        BindingModifier::Out => "out",
                        BindingModifier::In => "in",
                        BindingModifier::Value => "value",
                    }
                ),
                span: expr.span,
            });
            return None;
        }
        if param.is_extension_this
            || param.name.eq_ignore_ascii_case("self")
            || param.name.eq_ignore_ascii_case("this")
        {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "implicit receiver `{}` in `{}` cannot specify a default value",
                    param.name, ctx.display_name
                ),
                span: expr.span,
            });
            return None;
        }
        let target_ty = Ty::from_type_expr(&param.ty);
        if let Some(value) = self.try_const_eval_default(expr, ctx.namespace, ctx.owner, &target_ty)
        {
            return Some(DefaultArgumentValue::Const(value));
        }
        let thunk_name = format!("{}::default_arg#{}", ctx.internal_name, index);
        let internal = self.allocate_internal_name(&thunk_name);
        self.lower_default_thunk(ctx, &internal, &target_ty, expr, index)
    }

    fn try_const_eval_default(
        &mut self,
        expr: &Expression,
        namespace: Option<&str>,
        owner: Option<&str>,
        target_ty: &Ty,
    ) -> Option<ConstValue> {
        let mut context = ConstEvalContext::new(
            &mut self.symbol_index,
            &mut self.type_layouts,
            Some(&self.import_resolver),
        );
        context
            .evaluate_expression(expr, namespace, owner, None, None, target_ty, expr.span)
            .ok()
            .map(|result| result.value)
    }

    pub(crate) fn lower_default_thunk(
        &mut self,
        ctx: DefaultArgumentCtx<'_>,
        internal: &str,
        target_ty: &Ty,
        expr: &Expression,
        param_index: usize,
    ) -> Option<DefaultArgumentValue> {
        let metadata_count = 0usize;
        let params = Vec::new();
        let sig = FnSig {
            params,
            ret: target_ty.clone(),
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        };
        let mut generic_param_names = ctx.type_generics.clone();
        generic_param_names.extend(ctx.method_generics.iter().cloned());
        let builder_kind = if ctx.owner.is_some() {
            FunctionKind::Method
        } else {
            FunctionKind::Function
        };
        let opaque_return = opaque_return_info_from_ty(target_ty, expr.span);
        let mut builder = BodyBuilder::new(
            &sig,
            expr.span,
            ctx.display_name,
            false,
            false,
            generic_param_names,
            &mut self.type_layouts,
            &self.type_visibilities,
            &self.primitive_registry,
            self.default_arguments.clone(),
            ctx.namespace,
            self.current_package.clone(),
            &self.function_packages,
            &self.operator_registry,
            &mut self.string_interner,
            &self.symbol_index,
            &self.import_resolver,
            &self.static_registry,
            &self.class_bases,
            &self.class_virtual_slots,
            &self.trait_decls,
            builder_kind,
            false,
            crate::threading::thread_runtime_mode(),
            None,
            opaque_return,
            self.generic_specializations.clone(),
        );
        let Some(node) = expr.node.clone() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "default value for parameter `{}` in `{}` is not a parsable expression",
                    param_index, ctx.display_name
                ),
                span: expr.span,
            });
            return None;
        };
        let value = match builder.lower_expr_node(node, expr.span) {
            Some(value) => value,
            None => {
                let (_body, mut diagnostics, mut constraints, nested_functions) = builder.finish();
                self.diagnostics.append(&mut diagnostics);
                self.constraints.append(&mut constraints);
                self.functions.extend(nested_functions);
                return None;
            }
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
        self.functions.extend(nested_functions);
        let function = MirFunction {
            name: internal.to_string(),
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
        };
        self.functions.push(function);
        Some(DefaultArgumentValue::Thunk {
            symbol: internal.to_string(),
            metadata_count,
            span: expr.span,
        })
    }

    fn record_default_argument_metadata(
        &mut self,
        display_name: &str,
        internal: &str,
        param_name: &str,
        index: usize,
        span: Option<Span>,
        value: &DefaultArgumentValue,
    ) {
        let kind = match value {
            DefaultArgumentValue::Const(constant) => DefaultArgumentKind::Const(constant.clone()),
            DefaultArgumentValue::Thunk {
                symbol,
                metadata_count,
                ..
            } => DefaultArgumentKind::Thunk {
                symbol: symbol.clone(),
                metadata_count: *metadata_count,
            },
        };
        self.default_argument_records.push(DefaultArgumentRecord {
            function: display_name.to_string(),
            internal: internal.to_string(),
            param_name: param_name.to_string(),
            param_index: index,
            span,
            value: kind,
        });
    }

    fn collect_owner_generic_names(&self, owner: Option<&str>) -> Vec<String> {
        owner
            .and_then(|name| self.symbol_index.type_generics(name))
            .map(|params| params.iter().map(|param| param.name.clone()).collect())
            .unwrap_or_default()
    }
}
