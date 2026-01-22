use super::*;
use crate::eq_glue::eq_glue_symbol_for;

body_builder_impl! {
    pub(crate) fn try_lower_eq_glue_expr(
        &mut self,
        expr: &ExprNode,
        span: Option<Span>,
    ) -> Option<Operand> {
        let mut tokens = Vec::new();
        if Self::collect_eq_glue_tokens(expr, &mut tokens) {
            match Self::parse_eq_glue_tokens(&tokens) {
                GlueParseResult::Success { type_text } => {
                    let operand = self.lower_eq_glue_for_type(&type_text, span);
                    return Some(operand);
                }
                GlueParseResult::RuntimeArgs => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "`__eq_glue_of` does not accept runtime arguments".into(),
                        span,
                    });
                    return Some(Operand::Const(ConstOperand::new(ConstValue::Null)));
                }
                GlueParseResult::MissingType => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "`__eq_glue_of` requires a type argument (e.g. `__eq_glue_of<MyType>()`)".into(),
                        span,
                    });
                    return Some(Operand::Const(ConstOperand::new(ConstValue::Null)));
                }
                GlueParseResult::NotMatch => {
                    // fall through to call-based handling below
                }
            }
        }

        if let ExprNode::Call {
            callee,
            args,
            generics,
        } = expr
        {
            if let ExprNode::Identifier(name) = callee.as_ref()
                && name.trim() == "__eq_glue_of"
            {
                if let Some(generics) = generics
                    && let Some(first) = generics.first()
                {
                    if let Some(arg) = args.first() {
                        let arg_span = arg.span.or(arg.value_span).or(span);
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "`__eq_glue_of` does not accept runtime arguments".into(),
                            span: arg_span,
                        });
                    }
                    let operand = self.lower_eq_glue_for_type(first.trim(), span);
                    return Some(operand);
                }
            }
        }

        if let ExprNode::Call { callee, args, .. } = expr {
            return self.try_lower_eq_glue_intrinsic(callee, args, span);
        }

        None
    }

    pub(crate) fn try_lower_eq_glue_intrinsic(
        &mut self,
        callee: &ExprNode,
        args: &[CallArgument],
        span: Option<Span>,
    ) -> Option<Operand> {
        let ExprNode::Identifier(name) = callee else {
            return None;
        };

        const INTRINSIC_NAME: &str = "__eq_glue_of";

        if name.trim() != INTRINSIC_NAME {
            return None;
        }

        if args.first().is_some() {
            let arg_span = args[0].span.or(args[0].value_span).or(span);
            self.diagnostics.push(LoweringDiagnostic {
                message: "`__eq_glue_of` does not accept runtime arguments".into(),
                span: arg_span,
            });
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: "`__eq_glue_of` requires a type argument (e.g. `__eq_glue_of<MyType>()`)".into(),
            span,
        });
        Some(Operand::Const(ConstOperand::new(ConstValue::Null)))
    }

    pub(crate) fn lower_eq_glue_for_type(
        &mut self,
        type_text: &str,
        span: Option<Span>,
    ) -> Operand {
        let trimmed = type_text.trim();
        let Some(type_expr) = parse_type_expression_text(trimmed) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{trimmed}` is not a valid type for `__eq_glue_of`"),
                span,
            });
            return Operand::Const(ConstOperand::new(ConstValue::Null));
        };

        let ty = Ty::from_type_expr(&type_expr);
        self.ensure_ty_layout_for_ty(&ty);

        if self.type_param_name_for_ty(&ty).is_some() {
            if let Some(operand) = self.runtime_type_eq_operand(&ty, span) {
                return operand;
            }
            return Operand::Const(ConstOperand::new(ConstValue::Null));
        }

        let canonical = ty.canonical_name();
        let current_type = self.current_self_type_name();
        let layout_name = resolve_type_layout_name(
            self.type_layouts,
            Some(self.import_resolver),
            self.namespace.as_deref(),
            current_type.as_deref(),
            &canonical,
        )
        .unwrap_or(canonical.clone());

        let method_symbol = format!("{layout_name}::op_Equality");
        if self
            .symbol_index
            .function_overloads(&method_symbol)
            .is_none()
        {
            return Operand::Const(ConstOperand::new(ConstValue::Null));
        }

        let symbol = eq_glue_symbol_for(&layout_name);
        Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)))
    }

    pub(crate) fn collect_eq_glue_tokens(expr: &ExprNode, out: &mut Vec<GlueToken>) -> bool {
        match expr {
            ExprNode::Identifier(name) => {
                out.push(GlueToken::Ident(name.clone()));
                true
            }
            ExprNode::Literal(literal) => match &literal.value {
                ConstValue::Unit => {
                    out.push(GlueToken::Symbol("()"));
                    true
                }
                ConstValue::Symbol(sym) => {
                    out.push(GlueToken::Ident(sym.clone()));
                    true
                }
                _ => false,
            },
            ExprNode::Binary { op, left, right } => {
                let symbol = match op {
                    BinOp::Lt => "<",
                    BinOp::Gt => ">",
                    BinOp::Shr => ">>",
                    BinOp::Shl => "<<",
                    _ => return false,
                };
                if !Self::collect_eq_glue_tokens(left, out) {
                    return false;
                }
                match symbol {
                    ">>" => {
                        out.push(GlueToken::Symbol(">"));
                        out.push(GlueToken::Symbol(">"));
                    }
                    "<<" => {
                        out.push(GlueToken::Symbol("<"));
                        out.push(GlueToken::Symbol("<"));
                    }
                    _ => out.push(GlueToken::Symbol(symbol)),
                }
                Self::collect_eq_glue_tokens(right, out)
            }
            ExprNode::Parenthesized(inner) => {
                out.push(GlueToken::Symbol("("));
                if !Self::collect_eq_glue_tokens(inner, out) {
                    return false;
                }
                out.push(GlueToken::Symbol(")"));
                true
            }
            _ => false,
        }
    }

    pub(crate) fn parse_eq_glue_tokens(tokens: &[GlueToken]) -> GlueParseResult {
        if tokens.len() < 3 {
            return GlueParseResult::NotMatch;
        }

        match tokens.first() {
            Some(GlueToken::Ident(name)) if name == "__eq_glue_of" => {}
            _ => return GlueParseResult::NotMatch,
        }

        if !matches!(tokens.get(1), Some(GlueToken::Symbol("<"))) {
            return GlueParseResult::NotMatch;
        }

        let mut idx = 2usize;
        let mut depth = 1usize;
        let mut type_text = String::new();

        while idx < tokens.len() {
            match &tokens[idx] {
                GlueToken::Symbol("<") => {
                    depth += 1;
                    type_text.push('<');
                }
                GlueToken::Symbol(">") => {
                    if depth == 0 {
                        return GlueParseResult::NotMatch;
                    }
                    depth -= 1;
                    if depth == 0 {
                        idx += 1;
                        break;
                    }
                    type_text.push('>');
                }
                GlueToken::Symbol("()") => {
                    if depth == 0 {
                        break;
                    }
                    return GlueParseResult::NotMatch;
                }
                GlueToken::Symbol("(") | GlueToken::Symbol(")") => {
                    if depth == 0 {
                        break;
                    }
                    return GlueParseResult::NotMatch;
                }
                GlueToken::Ident(name) => {
                    if depth == 0 {
                        break;
                    }
                    type_text.push_str(name);
                }
                GlueToken::Symbol(sym) => {
                    if depth == 0 {
                        break;
                    }
                    type_text.push_str(sym);
                }
            }
            idx += 1;
        }

        if depth != 0 || type_text.trim().is_empty() {
            return GlueParseResult::MissingType;
        }

        let mut saw_runtime_args = false;
        while idx < tokens.len() {
            match &tokens[idx] {
                GlueToken::Symbol("()") => {
                    idx += 1;
                    continue;
                }
                GlueToken::Symbol("(") => {
                    saw_runtime_args = true;
                    break;
                }
                GlueToken::Symbol(")") => {
                    return GlueParseResult::NotMatch;
                }
                GlueToken::Ident(_) | GlueToken::Symbol(_) => {
                    saw_runtime_args = true;
                    break;
                }
            }
        }

        if saw_runtime_args {
            GlueParseResult::RuntimeArgs
        } else {
            GlueParseResult::Success { type_text }
        }
    }
}
