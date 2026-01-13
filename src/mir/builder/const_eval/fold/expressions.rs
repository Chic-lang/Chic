use std::collections::HashMap;

use crate::frontend::ast::Expression;
use crate::frontend::diagnostics::Span;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::ConstEvalContext;
use crate::mir::FloatValue;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};
use crate::mir::builder::support::resolve_type_layout_name;
use crate::mir::data::{BinOp, ConstValue, Ty, UnOp};
use crate::mir::layout::TypeLayout;
use crate::syntax::expr::{AssignOp, ExprNode, NameOfOperand, SizeOfOperand};

use super::super::ConstEvalResult;
use super::super::environment::{EvalEnv, ImmutableLocals};
use super::operations::DecimalBinaryKind;

impl<'a> ConstEvalContext<'a> {
    pub fn evaluate_expression(
        &mut self,
        expr: &Expression,
        namespace: Option<&str>,
        owner: Option<&str>,
        locals: Option<&HashMap<String, ConstValue>>,
        params: Option<&HashMap<String, ConstValue>>,
        target_ty: &Ty,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        self.ensure_ty_layout(target_ty);
        self.record_expression_request();
        let node = expr.node.as_ref().ok_or_else(|| ConstEvalError {
            message: "initializer is not a parsable constant expression".into(),
            span: expr.span.or(span),
        })?;
        let eval_span = expr.span.or(span);
        let use_memo =
            self.should_memoise() && locals.is_none() && params.is_none() && expr.node.is_some();
        let memo_key = if use_memo {
            Some(self.expression_key(expr, namespace, owner, target_ty))
        } else {
            None
        };

        if let Some(ref key) = memo_key {
            if let Some(cached) = self.expression_cache_lookup(key) {
                self.record_memo_hit();
                return Ok(cached);
            }
        }

        let mut local_scope = locals.map(ImmutableLocals::new);
        let mut env = EvalEnv {
            namespace,
            owner,
            span: eval_span,
            params,
            locals: local_scope
                .as_mut()
                .map(|scope| scope as &mut dyn super::super::environment::LocalResolver),
        };
        let value = self.evaluate_node(node, &mut env)?;
        self.record_expression_eval();
        if memo_key.is_some() {
            self.record_memo_miss();
        }
        let converted =
            self.convert_value_to_type(value.value, value.literal.clone(), target_ty, eval_span)?;
        let result = ConstEvalResult::with_literal(converted, None);
        if let Some(key) = memo_key {
            self.expression_cache_store(key, result.clone());
        }
        Ok(result)
    }

    pub(crate) fn evaluate_node(
        &mut self,
        node: &ExprNode,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        self.consume_fuel(env.span)?;
        match node {
            ExprNode::Literal(literal) => Ok(ConstEvalResult::with_literal(
                literal.value.clone(),
                literal.numeric.clone(),
            )),
            ExprNode::Identifier(name) => self.evaluate_identifier(name, env),
            ExprNode::Unary {
                op, expr, postfix, ..
            } => {
                let value = self.evaluate_node(expr, env)?;
                self.evaluate_unary(*op, *postfix, value, env.span)
            }
            ExprNode::Binary { op, left, right } => {
                let lhs = self.evaluate_node(left, env)?;
                let rhs = self.evaluate_node(right, env)?;
                self.evaluate_binary(*op, lhs, rhs, env.span)
            }
            ExprNode::Parenthesized(inner) => self.evaluate_node(inner, env),
            ExprNode::Cast { target, expr, .. } => {
                let eval = self.evaluate_node(expr, env)?;
                let ty_expr = parse_type_expression_text(target).ok_or_else(|| ConstEvalError {
                    message: format!("`{target}` is not a valid cast target"),
                    span: env.span,
                })?;
                let ty = Ty::from_type_expr(&ty_expr);
                self.ensure_ty_layout(&ty);
                self.convert_value_to_type(eval.value, eval.literal, &ty, env.span)
                    .map(ConstEvalResult::new)
            }
            ExprNode::Call {
                callee,
                args,
                generics,
            } => self.evaluate_call(callee, generics.as_deref(), args, env),
            ExprNode::Assign { target, op, value } => {
                self.evaluate_assignment(target, *op, value, env)
            }
            ExprNode::Member { base, member, .. } => {
                if let Ok(segments) = diagnostics::expr_path_segments(node) {
                    match self.evaluate_path(&segments, env) {
                        Ok(result) => return Ok(result),
                        Err(err) => {
                            if !err.message.contains("is not a constant value")
                                && !err.message.contains("does not resolve to a constant")
                            {
                                return Err(err);
                            }
                        }
                    }
                    if let Some(enum_const) = self.try_resolve_enum_variant(&segments) {
                        return Ok(ConstEvalResult::new(enum_const));
                    }
                }
                let base_value = self.evaluate_node(base, env)?;
                self.evaluate_struct_member(base_value.value, member, env.span)
            }
            ExprNode::SizeOf(operand) => self.evaluate_sizeof(operand, env),
            ExprNode::AlignOf(operand) => self.evaluate_alignof(operand, env),
            ExprNode::NameOf(operand) => self.evaluate_nameof(operand, env),
            ExprNode::Quote(literal) => self.evaluate_quote_literal(literal, env),
            _ => Err(ConstEvalError {
                message: "expression is not allowed in a constant initializer".into(),
                span: env.span,
            }),
        }
    }

    fn evaluate_struct_member(
        &mut self,
        value: ConstValue,
        member: &str,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match value {
            ConstValue::Struct { type_name, fields } => {
                let mut iter = fields.into_iter();
                if let Some((_, field_value)) = iter.find(|(name, _)| name.as_str() == member) {
                    Ok(ConstEvalResult::new(field_value))
                } else {
                    Err(ConstEvalError {
                        message: format!(
                            "struct `{type_name}` does not have constant field `{member}`"
                        ),
                        span,
                    })
                }
            }
            other => Err(ConstEvalError {
                message: format!("member `{member}` is not available on constant value {other:?}"),
                span,
            }),
        }
    }

    fn try_resolve_enum_variant(&self, segments: &[String]) -> Option<ConstValue> {
        if segments.len() < 2 {
            return None;
        }
        let variant = segments.last()?.as_str().to_string();
        let type_segments = &segments[..segments.len() - 1];
        let joined = type_segments.join("::");
        let mut candidates = vec![joined.clone()];
        candidates.push(joined.replace("::", "."));
        candidates.push(Ty::named(joined.clone()).canonical_name());
        candidates.dedup();

        for candidate in candidates {
            let Some(layout) = self.type_layouts.layout_for_name(&candidate) else {
                continue;
            };
            if let TypeLayout::Enum(enum_layout) = layout {
                if let Some(variant_layout) = enum_layout
                    .variants
                    .iter()
                    .find(|entry| entry.name == variant)
                {
                    return Some(ConstValue::Enum {
                        type_name: enum_layout.name.clone(),
                        variant,
                        discriminant: variant_layout.discriminant,
                    });
                }
            }
        }
        None
    }

    fn evaluate_identifier(
        &mut self,
        name: &str,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        if let Some(value) = env.resolve_identifier(name) {
            return Ok(ConstEvalResult::new(value));
        }

        if let Some(owner) = env.owner {
            if let Some(qualified) = self
                .symbol_index
                .type_const(owner, name)
                .map(|symbol| symbol.qualified.clone())
            {
                return self
                    .evaluate_const(&qualified, env.span)
                    .ok_or_else(|| ConstEvalError {
                        message: format!("could not evaluate constant `{qualified}`"),
                        span: env.span,
                    });
            }
        }

        if let Some(qualified) = self
            .symbol_index
            .namespace_const(env.namespace, name)
            .map(|symbol| symbol.qualified.clone())
        {
            return self
                .evaluate_const(&qualified, env.span)
                .ok_or_else(|| ConstEvalError {
                    message: format!("could not evaluate constant `{qualified}`"),
                    span: env.span,
                });
        }

        Err(ConstEvalError {
            message: format!("identifier `{name}` is not a constant value"),
            span: env.span,
        })
    }

    fn evaluate_path(
        &mut self,
        segments: &[String],
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        if segments.is_empty() {
            return Err(ConstEvalError {
                message: "invalid constant path".into(),
                span: env.span,
            });
        }
        let joined = segments.join("::");

        let mut candidates = Vec::new();
        candidates.push(joined.clone());
        if let Some(ns) = env.namespace {
            let mut current = Some(ns);
            while let Some(prefix) = current {
                candidates.push(format!("{prefix}::{joined}"));
                current = prefix.rfind("::").map(|idx| &prefix[..idx]);
            }
        }

        for candidate in candidates {
            if let Some(qualified) = self
                .symbol_index
                .const_symbol(&candidate)
                .map(|symbol| symbol.qualified.clone())
            {
                return self
                    .evaluate_const(&qualified, env.span)
                    .ok_or_else(|| ConstEvalError {
                        message: format!("could not evaluate constant `{qualified}`"),
                        span: env.span,
                    });
            }
        }
        if segments.len() >= 2 {
            let (owner_segments, const_name) = segments.split_at(segments.len() - 1);
            let const_name = const_name[0].as_str();
            let mut owner_candidates = Vec::new();
            owner_candidates.push(owner_segments.join("::"));
            if let Some(ns) = env.namespace {
                let mut current = Some(ns);
                while let Some(prefix) = current {
                    let candidate = format!("{prefix}::{}", owner_segments.join("::"));
                    owner_candidates.push(candidate);
                    current = prefix.rfind("::").map(|idx| &prefix[..idx]);
                }
            }
            for owner in owner_candidates {
                if let Some(symbol) = self.symbol_index.type_const(&owner, const_name) {
                    let qualified = symbol.qualified.clone();
                    return self.evaluate_const(&qualified, env.span).ok_or_else(|| {
                        ConstEvalError {
                            message: format!("could not evaluate constant `{qualified}`"),
                            span: env.span,
                        }
                    });
                }
            }
        }

        Err(ConstEvalError {
            message: format!("path `{joined}` does not resolve to a constant"),
            span: env.span,
        })
    }

    fn evaluate_unary(
        &self,
        op: UnOp,
        postfix: bool,
        operand: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        if postfix && matches!(op, UnOp::Increment | UnOp::Decrement) {
            return Err(ConstEvalError {
                message:
                    "postfix increment and decrement are not supported in constant expressions"
                        .into(),
                span,
            });
        }
        match op {
            UnOp::UnaryPlus => Ok(operand),
            UnOp::Neg => match operand.value {
                ConstValue::Int(value) => value
                    .checked_neg()
                    .map(ConstValue::Int)
                    .map(ConstEvalResult::new)
                    .ok_or_else(|| ConstEvalError {
                        message: "integer negation overflowed the supported range".into(),
                        span,
                    }),
                ConstValue::Int32(value) => value
                    .checked_neg()
                    .map(ConstValue::Int32)
                    .map(ConstEvalResult::new)
                    .ok_or_else(|| ConstEvalError {
                        message: "integer negation overflowed the supported range".into(),
                        span,
                    }),
                ConstValue::UInt(value) => {
                    let negated = if value == 0 {
                        Some(ConstValue::Int(0))
                    } else if value == (1u128 << 127) {
                        Some(ConstValue::Int(i128::MIN))
                    } else if let Ok(as_i128) = i128::try_from(value) {
                        as_i128.checked_neg().map(ConstValue::Int)
                    } else {
                        None
                    };
                    negated
                        .map(ConstEvalResult::new)
                        .ok_or_else(|| ConstEvalError {
                            message: "integer negation overflowed the supported range".into(),
                            span,
                        })
                }
                ConstValue::Float(value) => {
                    let negated = FloatValue::from_f64_as(-value.to_f64(), value.width);
                    Ok(ConstEvalResult::new(ConstValue::Float(negated)))
                }
                ConstValue::Decimal(value) => {
                    Ok(ConstEvalResult::new(ConstValue::Decimal(value.negate())))
                }
                other => Err(ConstEvalError {
                    message: format!("negation is not supported for {other:?}"),
                    span,
                }),
            },
            UnOp::Increment | UnOp::Decrement => Err(ConstEvalError {
                message: "increment and decrement are not supported in constant expressions".into(),
                span,
            }),
            UnOp::BitNot => match operand.value {
                ConstValue::Int(value) => Ok(ConstEvalResult::new(ConstValue::Int(!value))),
                ConstValue::Int32(value) => Ok(ConstEvalResult::new(ConstValue::Int32(!value))),
                ConstValue::UInt(value) => Ok(ConstEvalResult::new(ConstValue::UInt(!value))),
                other => Err(ConstEvalError {
                    message: format!("ones-complement is not supported for {other:?}"),
                    span,
                }),
            },
            UnOp::Not => match operand.value {
                ConstValue::Bool(value) => Ok(ConstEvalResult::new(ConstValue::Bool(!value))),
                ConstValue::Int(value) => Ok(ConstEvalResult::new(ConstValue::Int(!value))),
                ConstValue::Int32(value) => Ok(ConstEvalResult::new(ConstValue::Int32(!value))),
                ConstValue::UInt(value) => Ok(ConstEvalResult::new(ConstValue::UInt(!value))),
                other => Err(ConstEvalError {
                    message: format!("logical complement is not supported for {other:?}"),
                    span,
                }),
            },
            UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut => Err(ConstEvalError {
                message: "pointer operators are not supported in constant expressions".into(),
                span,
            }),
        }
    }

    fn evaluate_binary(
        &mut self,
        op: BinOp,
        left: ConstEvalResult,
        right: ConstEvalResult,
        span: Option<Span>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match op {
            BinOp::Add => {
                self.binary_numeric_op(left, right, span, DecimalBinaryKind::Add, |a, b| {
                    a.checked_add(b)
                })
            }
            BinOp::Sub => {
                self.binary_numeric_op(left, right, span, DecimalBinaryKind::Sub, |a, b| {
                    a.checked_sub(b)
                })
            }
            BinOp::Mul => {
                self.binary_numeric_op(left, right, span, DecimalBinaryKind::Mul, |a, b| {
                    a.checked_mul(b)
                })
            }
            BinOp::Div => self.binary_div(left, right, span),
            BinOp::Rem => self.binary_rem(left, right, span),
            BinOp::BitAnd => self.binary_integer(left, right, span, |a, b| (a & b, a & b)),
            BinOp::BitOr => self.binary_integer(left, right, span, |a, b| (a | b, a | b)),
            BinOp::BitXor => self.binary_integer(left, right, span, |a, b| (a ^ b, a ^ b)),
            BinOp::Shl => self.binary_shift(left, right, span, true),
            BinOp::Shr => self.binary_shift(left, right, span, false),
            BinOp::Eq => Ok(ConstEvalResult::new(ConstValue::Bool(
                self.const_equal(&left.value, &right.value),
            ))),
            BinOp::Ne => Ok(ConstEvalResult::new(ConstValue::Bool(
                !self.const_equal(&left.value, &right.value),
            ))),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.binary_compare(op, left, right, span)
            }
            BinOp::And | BinOp::Or => self.binary_bool(op, left, right, span),
            _ => Err(ConstEvalError {
                message: format!("operator `{op:?}` is not supported in constant expressions"),
                span,
            }),
        }
    }

    fn evaluate_assignment(
        &mut self,
        target: &ExprNode,
        op: AssignOp,
        value: &ExprNode,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let name = match target {
            ExprNode::Identifier(name) => name.clone(),
            _ => {
                return Err(ConstEvalError {
                    message: "compile-time function assignments must target local variables".into(),
                    span: env.span,
                });
            }
        };

        if matches!(op, AssignOp::NullCoalesceAssign) {
            let current = env
                .resolve_identifier(&name)
                .ok_or_else(|| ConstEvalError {
                    message: format!(
                        "identifier `{name}` is not declared in this scope of compile-time function"
                    ),
                    span: env.span,
                })?;
            if !matches!(current, ConstValue::Null) {
                return Ok(ConstEvalResult::new(current));
            }
            let rhs = self.evaluate_node(value, env)?;
            env.assign_identifier(&name, rhs.value.clone())?;
            return Ok(ConstEvalResult::new(rhs.value));
        }

        let rhs = self.evaluate_node(value, env)?;
        let new_value = match op {
            AssignOp::Assign => rhs.value.clone(),
            AssignOp::AddAssign
            | AssignOp::SubAssign
            | AssignOp::MulAssign
            | AssignOp::DivAssign
            | AssignOp::RemAssign
            | AssignOp::BitAndAssign
            | AssignOp::BitOrAssign
            | AssignOp::BitXorAssign
            | AssignOp::ShlAssign
            | AssignOp::ShrAssign => {
                let current = env.resolve_identifier(&name).ok_or_else(|| ConstEvalError {
                    message: format!(
                        "identifier `{name}` is not declared in this scope of compile-time function"
                    ),
                    span: env.span,
                                    })?;
                let bin_op = match op {
                    AssignOp::AddAssign => BinOp::Add,
                    AssignOp::SubAssign => BinOp::Sub,
                    AssignOp::MulAssign => BinOp::Mul,
                    AssignOp::DivAssign => BinOp::Div,
                    AssignOp::RemAssign => BinOp::Rem,
                    AssignOp::BitAndAssign => BinOp::BitAnd,
                    AssignOp::BitOrAssign => BinOp::BitOr,
                    AssignOp::BitXorAssign => BinOp::BitXor,
                    AssignOp::ShlAssign => BinOp::Shl,
                    AssignOp::ShrAssign => BinOp::Shr,
                    _ => unreachable!(),
                };
                self.evaluate_binary(bin_op, ConstEvalResult::new(current), rhs, env.span)?
                    .value
            }
            AssignOp::NullCoalesceAssign => unreachable!(),
        };
        env.assign_identifier(&name, new_value.clone())?;
        Ok(ConstEvalResult::new(new_value))
    }

    fn evaluate_sizeof(
        &mut self,
        operand: &SizeOfOperand,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match operand {
            SizeOfOperand::Type(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err(ConstEvalError {
                        message: "`sizeof` requires a type operand".into(),
                        span: env.span,
                    });
                }
                let type_expr =
                    parse_type_expression_text(trimmed).ok_or_else(|| ConstEvalError {
                        message: format!("`{trimmed}` is not a valid type for `sizeof`"),
                        span: env.span,
                    })?;
                let ty = Ty::from_type_expr(&type_expr);
                self.ensure_ty_layout(&ty);
                let (size, _) =
                    self.size_and_align_for_ty(&ty, env.namespace)
                        .ok_or_else(|| ConstEvalError {
                            message: format!(
                                "cannot determine size for type `{}`",
                                ty.canonical_name()
                            ),
                            span: env.span,
                        })?;
                Ok(ConstEvalResult::new(ConstValue::UInt(size as u128)))
            }
            SizeOfOperand::Value(_) => Err(ConstEvalError {
                message: "`sizeof` in constant expressions must reference a type name".into(),
                span: env.span,
            }),
        }
    }

    fn evaluate_alignof(
        &mut self,
        operand: &SizeOfOperand,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        match operand {
            SizeOfOperand::Type(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err(ConstEvalError {
                        message: "`alignof` requires a type operand".into(),
                        span: env.span,
                    });
                }
                let type_expr =
                    parse_type_expression_text(trimmed).ok_or_else(|| ConstEvalError {
                        message: format!("`{trimmed}` is not a valid type for `alignof`"),
                        span: env.span,
                    })?;
                let ty = Ty::from_type_expr(&type_expr);
                self.ensure_ty_layout(&ty);
                let (_, align) =
                    self.size_and_align_for_ty(&ty, env.namespace)
                        .ok_or_else(|| ConstEvalError {
                            message: format!(
                                "cannot determine alignment for type `{}`",
                                ty.canonical_name()
                            ),
                            span: env.span,
                        })?;
                Ok(ConstEvalResult::new(ConstValue::UInt(align as u128)))
            }
            SizeOfOperand::Value(_) => Err(ConstEvalError {
                message: "`alignof` in constant expressions must reference a type name".into(),
                span: env.span,
            }),
        }
    }

    fn evaluate_nameof(
        &mut self,
        operand: &NameOfOperand,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        if operand.segments.is_empty() {
            return Err(ConstEvalError {
                message: "`nameof` requires an operand".into(),
                span: env.span,
            });
        }

        let display = operand.display().to_string();
        let segments = operand
            .segments
            .iter()
            .map(String::from)
            .collect::<Vec<_>>();

        if segments.len() == 1 {
            let candidate = segments[0].clone();
            if env
                .params
                .is_some_and(|params| params.contains_key(&candidate))
            {
                return Ok(ConstEvalResult::new(ConstValue::RawStr(candidate)));
            }
        }

        if let Some(type_name) = resolve_type_layout_name(
            self.type_layouts,
            self.import_resolver(),
            env.namespace,
            env.owner,
            &display,
        ) {
            let simple = diagnostics::simple_name(&type_name).to_string();
            return Ok(ConstEvalResult::new(ConstValue::RawStr(simple)));
        }

        let segment_refs: Vec<&str> = segments.iter().map(String::as_str).collect();
        for candidate in crate::mir::builder::symbol_index::candidate_function_names(
            env.namespace,
            &segment_refs,
        ) {
            if let Some(count) = self.symbol_index.function_count(&candidate) {
                if count == 1 {
                    let simple = segments.last().cloned().unwrap_or_default();
                    return Ok(ConstEvalResult::new(ConstValue::RawStr(simple)));
                }
                return Err(ConstEvalError {
                    message: format!(
                        "`nameof` operand `{display}` resolves to {count} overloads of `{}`",
                        segments.last().cloned().unwrap_or_default()
                    ),
                    span: env.span,
                });
            }
        }

        Err(ConstEvalError {
            message: format!("cannot resolve symbol `{display}` for `nameof`"),
            span: env.span,
        })
    }
}
