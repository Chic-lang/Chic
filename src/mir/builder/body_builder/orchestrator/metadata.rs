use super::super::*;
use crate::mir::builder::symbol_index::FunctionDeclSymbol;
use crate::type_identity::type_identity_for_name;

body_builder_impl! {
    pub(crate) fn type_param_type_id_operand(&mut self, name: &str, span: Option<Span>) -> Option<Operand> {
        if !self.generic_param_index.contains_key(name) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("type parameter `{name}` is not available in this scope"),
                span,
            });
            return None;
        }
        let type_id = drop_type_identity(name);
        Some(Operand::Const(ConstOperand::new(ConstValue::UInt(
            u128::from(type_id),
        ))))
    }

    pub(crate) fn type_param_name_for_ty(&self, ty: &Ty) -> Option<String> {
        match ty {
            Ty::Named(named) => {
                if named.args().is_empty() && self.generic_param_index.contains_key(named.as_str())
                {
                    Some(named.as_str().to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn method_generic_param_names(&self, canonical: &str, required_len: usize) -> Vec<String> {
        fn base_name(name: &str) -> &str {
            name.split('<').next().unwrap_or(name).trim()
        }

        let canonical_base = base_name(canonical);
        let mut selected: Option<Vec<FunctionDeclSymbol>> =
            self.symbol_index.function_decls(canonical).map(|decls| decls.to_vec());
        if selected.is_none() {
            let mut best_len = 0usize;
            for decls in self.symbol_index.function_decl_groups() {
                let Some(first) = decls.first() else { continue };
                let qualified = &first.qualified;
                let qualified_base = base_name(qualified);
                if qualified == canonical
                    || qualified.ends_with(canonical)
                    || canonical.ends_with(qualified)
                    || qualified_base == canonical_base
                    || qualified_base.ends_with(canonical_base)
                    || canonical_base.ends_with(qualified_base)
                {
                    let score = qualified.len();
                    if score >= best_len {
                        best_len = score;
                        selected = Some(decls.clone());
                    }
                }
            }
        }
        let Some(decls) = selected else {
            return Vec::new();
        };

        fn type_param_names(function: &FunctionDecl) -> Vec<String> {
            function
                .generics
                .as_ref()
                .map(|params| {
                    params
                        .params
                        .iter()
                        .filter_map(|param| {
                            if matches!(param.kind, GenericParamKind::Type(_)) {
                                Some(param.name.clone())
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        if required_len > 0 {
            if let Some(symbol) = decls.iter().find(|symbol| {
                let names = type_param_names(&symbol.function);
                names.len() == required_len
            }) {
                return type_param_names(&symbol.function);
            }
        }

        let function_params = decls
            .iter()
            .find_map(|symbol| {
                let names = type_param_names(&symbol.function);
                if !names.is_empty() { Some(names) } else { None }
            })
            .or_else(|| {
                decls
                    .first()
                    .map(|symbol| type_param_names(&symbol.function))
            })
            .unwrap_or_default()
            ;

        if required_len > 0 && function_params.is_empty() {
            if let Some((owner, _)) = canonical.rsplit_once("::") {
                let owner = owner.split('<').next().unwrap_or(owner);
                if let Some(owner_key) = self.type_layouts.resolve_type_key(owner) {
                    if let Some(params) = self.type_layouts.type_generic_params_for(owner_key) {
                        if params.len() == required_len {
                            return params.to_vec();
                        }
                    }
                }
                if let Some(params) = self.type_layouts.type_generic_params_for(owner) {
                    if params.len() == required_len {
                        return params.to_vec();
                    }
                }
            }
        }

        function_params
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn parse_type_arguments(&mut self, text: &str, span: Option<Span>) -> Option<Vec<Ty>> {
        if !text.contains('<') {
            return Some(Vec::new());
        }
        let Some(expr) = parse_type_expression_text(text) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{text}` is not a valid type expression"),
                span,
            });
            return None;
        };
        let ty = Ty::from_type_expr(&expr);
        match ty {
            Ty::Named(named) => {
                let mut args = Vec::new();
                for arg in named.args() {
                    if let Some(inner) = arg.as_type() {
                        args.push(inner.clone());
                    }
                }
                Some(args)
            }
            _ => Some(Vec::new()),
        }
    }

    pub(crate) fn type_id_operand_for_ty(
        &mut self,
        ty: &Ty,
        span: Option<Span>,
    ) -> Option<Operand> {
        if let Some(name) = self.type_param_name_for_ty(ty) {
            return self.type_param_type_id_operand(&name, span);
        }
        let canonical = self.resolve_ty_name(ty).unwrap_or_else(|| ty.canonical_name());
        if canonical == "<unknown>" {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot determine type identity for `<unknown>`".into(),
                span,
            });
            return None;
        }
        let type_id = type_identity_for_name(&self.type_layouts, &canonical);
        Some(Operand::Const(ConstOperand::new(ConstValue::UInt(
            u128::from(type_id),
        ))))
    }

    pub(crate) fn call_runtime_function(
        &mut self,
        symbol: &str,
        args: Vec<Operand>,
        ret_ty: Ty,
        span: Option<Span>,
    ) -> Operand {
        let temp = self.create_temp(span);
        self.hint_local_ty(temp, ret_ty);
        let destination = Place::new(temp);
        let continue_block = self.new_block(span);
        let arg_modes = vec![ParamMode::Value; args.len()];
        self.set_terminator(
            span,
            Terminator::Call {
                func: Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol.to_string()))),
                args,
                arg_modes,
                destination: Some(destination.clone()),
                target: continue_block,
                unwind: None,
                dispatch: None,
            },
        );
        self.switch_to_block(continue_block);
        Operand::Copy(destination)
    }

    pub(crate) fn runtime_type_metadata_call(
        &mut self,
        symbol: &str,
        ty: &Ty,
        ret_ty: Ty,
        span: Option<Span>,
    ) -> Option<Operand> {
        let type_id = self.type_id_operand_for_ty(ty, span)?;
        let operand = self.call_runtime_function(symbol, vec![type_id], ret_ty, span);
        Some(operand)
    }

    pub(crate) fn runtime_type_size_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call("chic_rt_type_size", ty, Ty::named("usize"), span)
    }

    pub(crate) fn runtime_type_align_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call("chic_rt_type_align", ty, Ty::named("usize"), span)
    }

    pub(crate) fn runtime_type_drop_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call("chic_rt_type_drop_glue", ty, Ty::named("nint"), span)
    }

    pub(crate) fn runtime_type_clone_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call(
            "chic_rt_type_clone_glue",
            ty,
            Ty::named("nint"),
            span,
        )
    }

    pub(crate) fn runtime_type_hash_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call(
            "chic_rt_type_hash_glue",
            ty,
            Ty::named("nint"),
            span,
        )
    }

    pub(crate) fn runtime_type_eq_operand(&mut self, ty: &Ty, span: Option<Span>) -> Option<Operand> {
        self.runtime_type_metadata_call(
            "chic_rt_type_eq_glue",
            ty,
            Ty::named("nint"),
            span,
        )
    }
}
