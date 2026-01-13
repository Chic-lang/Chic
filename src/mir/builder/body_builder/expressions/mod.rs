use super::symbol_index::{
    ConstSymbol, FieldMetadata, FunctionParamSymbol, FunctionSymbol, PropertySymbol,
};
use super::*;
use crate::drop_glue::drop_glue_symbol_for;
use crate::frontend::ast::PropertyAccessorKind;
use crate::frontend::metadata::reflection::TypeKind;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::builder::support::{resolve_type_layout_name, type_size_and_align_for_ty};
use crate::mir::data::{
    ArrayTy, BorrowKind, BorrowOperand, ConstOperand, ConstValue, InlineAsm, InlineAsmOperand,
    InlineAsmOperandKind, InlineAsmOptions, InlineAsmRegister, InlineAsmRegisterClass,
    InlineAsmTemplatePiece, ParamMode, PendingFunctionCandidate, PendingOperandInfo, Place,
    ReadOnlySpanTy, SpanTy, TupleTy, VecTy,
};
use crate::mir::layout::TypeLayout;
use crate::mir::operators::{OperatorMatch, OperatorOverload};
use crate::mir::{
    AggregateKind, InterpolatedStringSegment as MirInterpolatedStringSegment,
    class_vtable_symbol_name,
};
use crate::syntax::expr::{
    ArrayLiteralExpr, CallArgument, CallArgumentModifier, CastSyntax, InterpolatedExprSegment,
    InterpolatedStringExpr, InterpolatedStringSegment as ExprInterpolatedStringSegment, NewExpr,
    NewInitializer, ObjectInitializerField, RangeEndpoint, RangeExpr, SizeOfOperand,
};
use std::collections::{HashSet, VecDeque};
mod access;
mod aggregates;
mod call_support;
mod calls;
mod clone_glue;
mod control;
mod display;
mod drop_glue;
mod eq_glue;
mod hash_glue;
mod identifiers;
mod inline_asm;
mod intrinsics;
mod operators;
mod patterns;
mod shared;
mod type_id;
pub(crate) use call_support::CallBindingInfo;
pub(crate) use shared::OperatorResolution;
pub(crate) use shared::collect_path_segments;

#[derive(Clone, Copy)]
pub(crate) enum MmioIntent {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IndexableKind {
    Array(usize),
    Vec,
    Span,
    ReadOnlySpan,
}

pub(crate) enum GlueParseResult {
    NotMatch,
    MissingType,
    RuntimeArgs,
    Success { type_text: String },
}

pub(crate) enum GlueToken {
    Ident(String),
    Symbol(&'static str),
}

body_builder_impl! {
    const OBJECT_ALLOC_RUNTIME_FN: &'static str = "chic_rt_object_new";

    pub(super) fn expression_node(&mut self, expr: &Expression) -> Option<ExprNode> {
        if let Some(node) = expr.node.clone() {
            return Some(node);
        }
        match parse_expression(&expr.text) {
            Ok(node) => Some(node),
            Err(err) => {
                let span = Self::combine_expr_span(expr.span, err.span);
                self.diagnostics.push(LoweringDiagnostic {
                    message: err.message,
                    span,
                                    });
                None
            }
        }
    }

    fn combine_expr_span(outer: Option<Span>, inner: Option<Span>) -> Option<Span> {
        match (outer, inner) {
            (Some(outer), Some(inner)) => Some(Span::new(
                outer.start + inner.start,
                outer.start + inner.end,
            )),
            (Some(span), None) => Some(span),
            (None, Some(inner)) => Some(inner),
            _ => None,
        }
    }

    pub(super) fn lower_expression_operand(
        &mut self,
        expr: &crate::frontend::ast::Expression,
    ) -> Option<Operand> {
        self.validate_required_initializer(expr);
        let parsed = self.expression_node(expr)?;
        self.lower_expr_node(parsed, expr.span)
    }

    pub(super) fn lower_expression_statement(
        &mut self,
        expr: &crate::frontend::ast::Expression,
    ) -> bool {
        let Some(parsed) = self.expression_node(expr) else {
            return false;
        };

        match parsed {
            ExprNode::Assign { target, op, value } => {
                self.lower_assignment_statement(*target, op, *value, expr.span)
            }
            ExprNode::Call {
                callee,
                args,
                generics,
            } => self.lower_call_statement(*callee, args, generics, expr.span),
            await_expr @ ExprNode::Await { .. } => {
                self.lower_expr_node(await_expr, expr.span).is_some()
            }
            try_expr @ ExprNode::TryPropagate { .. } => {
                self.lower_expr_node(try_expr, expr.span).is_some()
            }
            inline_asm @ ExprNode::InlineAsm(_) => {
                self.lower_expr_node(inline_asm, expr.span).is_some()
            }
            _ => false,
        }
    }













    pub(crate) fn lower_expr_node(&mut self, expr: ExprNode, span: Option<Span>) -> Option<Operand> {
        if let Some(operand) = self.try_lower_clone_glue_expr(&expr, span) {
            return Some(operand);
        }
        if let Some(operand) = self.try_lower_drop_glue_expr(&expr, span) {
            return Some(operand);
        }
        if let Some(operand) = self.try_lower_hash_glue_expr(&expr, span) {
            return Some(operand);
        }
        if let Some(operand) = self.try_lower_eq_glue_expr(&expr, span) {
            return Some(operand);
        }

        match expr {
            ExprNode::Literal(literal) => {
                let meta = literal.numeric.clone();
                let value = self.normalise_const(literal.value.clone(), span);
                Some(Operand::Const(ConstOperand::with_literal(value, meta)))
            }
            ExprNode::Default(default_expr) => {
                if let Some(type_name) = default_expr.explicit_type.as_ref() {
                    if let Some(type_expr) = parse_type_expression_text(type_name) {
                        let ty = Ty::from_type_expr(&type_expr);
                        self.lower_default_operand(
                            Some(ty),
                            span.or(default_expr.keyword_span),
                            default_expr.type_span,
                        )
                    } else {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "unable to parse type `{type_name}` supplied to `default(...)`"
                            ),
                            span: default_expr.type_span.or(span),
                        });
                        None
                    }
                } else {
                    self.lower_default_operand(
                        None,
                        span.or(default_expr.keyword_span),
                        default_expr.type_span,
                    )
                }
            }
            ExprNode::Identifier(name) => self.lower_identifier_expr(&name, span),
            ExprNode::IndexFromEnd(from_end) => {
                self.lower_index_from_end_expr(*from_end.expr, from_end.span.or(span))
            }
            ExprNode::Range(range) => self.lower_range_value(range, span),
            ExprNode::Unary {
                op,
                expr,
                postfix,
            } => self.lower_unary_expr(op, *expr, postfix, span),
            ExprNode::Binary { op, left, right } => {
                if matches!(op, BinOp::NullCoalesce) {
                    self.lower_null_coalesce_expr(*left, *right, span)
                } else {
                    self.lower_binary_expr(op, *left, *right, span)
                }
            }
            ExprNode::Index {
                base,
                mut indices,
                null_conditional,
            } => {
                if null_conditional {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "null-conditional index access is only supported on assignment targets"
                            .into(),
                        span,
                    });
                    return None;
                }
                if indices.len() == 1 && matches!(indices[0], ExprNode::Range(_)) {
                    let range = indices.pop().expect("range element exists");
                    self.lower_range_index_expr(*base, range, span)
                } else {
                    self.lower_index_expr(*base, indices, span)
                }
            }
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.lower_conditional_expr(*condition, *then_branch, *else_branch, span),
            ExprNode::Switch(switch_expr) => self.lower_switch_expr(switch_expr, span),
            ExprNode::Cast { target, expr, syntax } => {
                self.lower_cast_expr(*expr, target, syntax, span)
            }
            ExprNode::Parenthesized(expr) => self.lower_expr_node(*expr, span),
            ExprNode::Tuple(elements) => self.lower_tuple_expr(elements, span),
            ExprNode::Ref { expr, readonly } => {
                self.lower_ref_expr(*expr, readonly, span)
            }
            ExprNode::Member {
                base,
                member,
                null_conditional,
            } => {
                if null_conditional {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "null-conditional member access is only supported on assignment targets"
                            .into(),
                        span,
                    });
                    return None;
                }
                self.lower_member_expr(*base, &member, span)
            }
            ExprNode::Call {
                callee,
                args,
                generics,
            } => self.lower_call(*callee, args, generics, span, true),
            ExprNode::New(new_expr) => self.lower_new_expr(new_expr, span),
            ExprNode::ArrayLiteral(array) => self.lower_array_literal_expr(array, span),
            ExprNode::Assign { .. } => self.report_assignment_expr(span),
            ExprNode::Lambda(lambda) => self.lower_lambda_expr(lambda, span),
            ExprNode::Await { expr } => self.lower_await_expr(*expr, span),
            ExprNode::TryPropagate {
                expr,
                question_span,
            } => self.lower_try_propagate_expr(*expr, question_span, span),
            ExprNode::IsPattern {
                value,
                pattern,
                guards,
            } => self.lower_is_pattern_expr(*value, &pattern, &guards, span),
            ExprNode::Throw { expr } => self.lower_throw_expr(expr, span),
            ExprNode::SizeOf(operand) => self.lower_sizeof_expr(operand, span),
            ExprNode::AlignOf(operand) => self.lower_alignof_expr(operand, span),
            ExprNode::NameOf(operand) => self.lower_nameof_expr(operand, span),
            ExprNode::InterpolatedString(interpolated) => {
                self.lower_interpolated_string(interpolated, span)
            }
            ExprNode::InlineAsm(asm) => self.lower_inline_asm_expr(asm, span),
            ExprNode::Quote(_) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "quote(expr) is only supported inside const expressions".into(),
                    span,
                });
                None
            }
        }
    }

    fn lower_default_operand(
        &mut self,
        explicit: Option<Ty>,
        span: Option<Span>,
        type_span: Option<Span>,
    ) -> Option<Operand> {
        if let Some(ty) = explicit {
            self.ensure_ty_layout_for_ty(&ty);
            return Some(self.zero_init_operand(ty, span));
        }
        Some(Operand::Pending(PendingOperand {
            category: ValueCategory::Pending,
            repr: "default".into(),
            span: span.or(type_span),
            info: None,
        }))
    }

    pub(super) fn zero_init_operand(&mut self, ty: Ty, span: Option<Span>) -> Operand {
        let temp = self.create_temp(span);
        self.hint_local_ty(temp, ty.clone());
        self.ensure_ty_layout_for_ty(&ty);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::ZeroInit {
                place: Place::new(temp),
            },
        });
        Operand::Copy(Place::new(temp))
    }

    fn lower_ref_expr(
        &mut self,
        expr: ExprNode,
        readonly: bool,
        span: Option<Span>,
    ) -> Option<Operand> {
        let place = self.lower_place_expr(expr, span)?;
        let kind = if readonly {
            BorrowKind::Shared
        } else {
            BorrowKind::Unique
        };
        Some(self.borrow_argument_place(place, kind, span))
    }

    pub(super) fn lower_new_expr(
        &mut self,
        new_expr: NewExpr,
        span: Option<Span>,
    ) -> Option<Operand> {
        let expr_span = new_expr.span;
        let new_span = span.or(expr_span);
        let (object_ty, canonical_name, kind) = self.resolve_new_type(&new_expr, new_span)?;
        let NewExpr {
            type_name,
            array_lengths,
            args,
            initializer,
            ..
        } = new_expr;
        if let Ty::Array(array_ty) = &object_ty {
            if array_ty.rank != 1 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "only single-dimensional arrays are supported".into(),
                    span: new_span,
                });
                return None;
            }
            if !args.is_empty() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "array creation does not accept constructor arguments; provide a length or initializer list instead".into(),
                    span: new_span,
                });
                return None;
            }
            let explicit_len_expr = array_lengths
                .as_ref()
                .and_then(|lengths| lengths.get(0).cloned());
            if let Some(lengths) = array_lengths.as_ref() {
                if lengths.len() > 1 {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: "multi-dimensional array lengths are not supported; use jagged arrays (`T[][]`) instead".into(),
                        span: new_span,
                    });
                    return None;
                }
            }

            let mut initializer_value = None;
            if let Some(init) = &initializer {
                match init {
                    NewInitializer::Object { .. } => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: "object initializers are not supported for arrays".into(),
                            span: new_span,
                        });
                        return None;
                    }
                    NewInitializer::Collection { elements, .. } => {
                        initializer_value = Some(elements.len());
                    }
                }
            }

            let length_operand = if let Some(len_expr) = explicit_len_expr {
                let Some(len_operand) = self.lower_expr_node(len_expr, new_span) else {
                    return None;
                };
                len_operand
            } else if let Some(count) = initializer_value {
                Operand::Const(ConstOperand::new(ConstValue::UInt(count as u128)))
            } else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "array length must be specified explicitly or via a collection initializer".into(),
                    span: new_span,
                });
                return None;
            };

            let (place, coerced_length, elem_size) =
                self.lower_new_array(array_ty, length_operand.clone(), new_span)?;

            self.zero_init_array_data(&place, elem_size.clone(), coerced_length.clone(), new_span);

            if let Some(init) = initializer {
                if !self.lower_array_initializer(&place, array_ty, init, new_span) {
                    return None;
                }
                self.set_array_len(&place, coerced_length.clone(), new_span);
            } else {
                self.set_array_len(&place, coerced_length.clone(), new_span);
            }
            return Some(Operand::Copy(place));
        }
        self.ensure_ty_layout_for_ty(&object_ty);

        let mut place = match kind {
            NewTargetKind::Reference => self.allocate_reference_object(&object_ty, new_span)?,
            NewTargetKind::Value => self.allocate_value_object(&object_ty, new_span),
        };

        if matches!(kind, NewTargetKind::Reference) {
            self.initialise_vtable_header(&place, &object_ty, new_span);
        }

        let needs_constructor = !args.is_empty()
            || !self
                .symbol_index
                .constructor_overloads(&canonical_name)
                .is_empty();

        if matches!(kind, NewTargetKind::Value) && !needs_constructor {
            self.push_statement(MirStatement {
                span: new_span,
                kind: MirStatementKind::DefaultInit {
                    place: place.clone(),
                },
            });
        }

        if needs_constructor {
            let (callee_name, callee_generics) =
                Self::split_new_constructor_callee_and_generics(&type_name);
            let callee = ExprNode::Identifier(callee_name);
            if self
                .lower_call_with_destination(
                    callee,
                    args,
                    callee_generics,
                    new_span,
                    true,
                    Some(place.clone()),
                )
                .is_none()
            {
                return None;
            }
        }

        self.normalise_place(&mut place);

        if let Some(initializer) = initializer {
            let init_span = match &initializer {
                NewInitializer::Object { span: init_span, .. }
                | NewInitializer::Collection { span: init_span, .. } => {
                    init_span.or(expr_span).or(span)
                }
            };
            if !self.lower_new_initializer(&place, initializer, init_span) {
                return None;
            }
        }

        Some(Operand::Copy(place))
    }

    fn split_new_constructor_callee_and_generics(type_name: &str) -> (String, Option<Vec<String>>) {
    let Some((base, args)) = Self::split_specialised_suffix(type_name) else {
        return (type_name.trim().to_string(), None);
    };
    let type_args = Self::split_top_level_type_args(args);
    if type_args.is_empty() {
        return (base, None);
    }
    (base, Some(type_args))
}

fn split_specialised_suffix(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    let mut chars = text.char_indices();
    let mut generic_start = None;
    while let Some((idx, ch)) = chars.next() {
        if ch == '<' {
            generic_start = Some(idx);
            break;
        }
    }
    let generic_start = generic_start?;

    let mut depth = 0u32;
    let mut generic_end = None;
    for (idx, ch) in text[generic_start..].char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    generic_end = Some(generic_start + idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let generic_end = generic_end?;
    if !text[generic_end + 1..].trim().is_empty() {
        return None;
    }

    let base = text[..generic_start].trim().to_string();
    let args = text[generic_start + 1..generic_end].trim().to_string();
    if base.is_empty() {
        return None;
    }
    Some((base, args))
}

fn split_top_level_type_args(args: String) -> Vec<String> {
    let args = args.trim();
    if args.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut start = 0usize;
    for (idx, ch) in args.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let part = args[start..idx].trim();
                if !part.is_empty() {
                    out.push(part.to_string());
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let last = args[start..].trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    out
}

    fn lower_new_array(
        &mut self,
        array_ty: &ArrayTy,
        length: Operand,
        span: Option<Span>,
    ) -> Option<(Place, Operand, Operand)> {
        let element_ty = array_ty.element.as_ref().clone();
        self.ensure_ty_layout_for_ty(&element_ty);
        self.type_layouts.ensure_array_layout(array_ty);

        let len_operand = self.coerce_operand_to_ty(length, &Ty::named("usize"), false, span);

        let Some(size_operand) = self.runtime_type_size_operand(&element_ty, span) else {
            return None;
        };
        let Some(align_operand) = self.runtime_type_align_operand(&element_ty, span) else {
            return None;
        };
        let Some(drop_operand) = self.runtime_type_drop_operand(&element_ty, span) else {
            return None;
        };

        let ret_ty = Ty::Array(array_ty.clone());
        let size_for_call = size_operand.clone();
        let array_operand = self.call_runtime_function(
            "chic_rt_vec_with_capacity",
            vec![size_for_call, align_operand, len_operand.clone(), drop_operand],
            ret_ty.clone(),
            span,
        );
        let mut place = self.operand_to_place(array_operand, span);
        self.normalise_place(&mut place);

        Some((place, len_operand, size_operand))
    }

    fn lower_new_vec(
        &mut self,
        vec_ty: &VecTy,
        length: Operand,
        span: Option<Span>,
    ) -> Option<(Place, Operand, Operand)> {
        let element_ty = vec_ty.element.as_ref().clone();
        self.ensure_ty_layout_for_ty(&element_ty);
        self.ensure_ty_layout_for_ty(&Ty::Vec(vec_ty.clone()));

        let len_operand = self.coerce_operand_to_ty(length, &Ty::named("usize"), false, span);

        let Some(size_operand) = self.runtime_type_size_operand(&element_ty, span) else {
            return None;
        };
        let Some(align_operand) = self.runtime_type_align_operand(&element_ty, span) else {
            return None;
        };
        let Some(drop_operand) = self.runtime_type_drop_operand(&element_ty, span) else {
            return None;
        };

        let ret_ty = Ty::Vec(vec_ty.clone());
        let size_for_call = size_operand.clone();
        let vec_operand = self.call_runtime_function(
            "chic_rt_vec_with_capacity",
            vec![size_for_call, align_operand, len_operand.clone(), drop_operand],
            ret_ty.clone(),
            span,
        );
        let mut place = self.operand_to_place(vec_operand, span);
        self.normalise_place(&mut place);

        Some((place, len_operand, size_operand))
    }

    fn set_array_len(&mut self, place: &Place, length: Operand, span: Option<Span>) {
        let mut len_place = place.clone();
        len_place
            .projection
            .push(ProjectionElem::FieldNamed("len".into()));
        self.normalise_place(&mut len_place);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: len_place,
                value: Rvalue::Use(length),
            },
        });
    }

    fn zero_init_array_data(
        &mut self,
        place: &Place,
        elem_size: Operand,
        length: Operand,
        span: Option<Span>,
    ) {
        let len_local = self.ensure_operand_local(length, span);
        let size_local = self.ensure_operand_local(elem_size, span);

        let byte_count = self.create_temp(span);
        self.hint_local_ty(byte_count, Ty::named("usize"));
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: Place::new(byte_count),
                value: Rvalue::Binary {
                    op: BinOp::Mul,
                    lhs: Operand::Copy(Place::new(len_local)),
                    rhs: Operand::Copy(Place::new(size_local)),
                    rounding: None,
                },
            },
        });

        let mut data_place = place.clone();
        data_place
            .projection
            .push(ProjectionElem::FieldNamed("ptr".into()));
        self.normalise_place(&mut data_place);

        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::ZeroInitRaw {
                pointer: Operand::Copy(data_place),
                length: Operand::Copy(Place::new(byte_count)),
            },
        });
    }

    fn lower_array_literal_expr(
        &mut self,
        literal: ArrayLiteralExpr,
        span: Option<Span>,
    ) -> Option<Operand> {
        #[derive(Clone)]
        enum ContainerKind {
            Array(ArrayTy),
            Vec(VecTy),
            Span(SpanTy),
            ReadOnlySpan(ReadOnlySpanTy),
        }

        let literal_span = literal.span.or(span);
        let mut operands = Vec::with_capacity(literal.elements.len());
        for (element, elem_span) in literal
            .elements
            .into_iter()
            .zip(literal.element_spans.into_iter())
        {
            let Some(op) = self.lower_expr_node(element, elem_span.or(literal_span)) else {
                return None;
            };
            operands.push((op, elem_span));
        }

        let mut container: Option<ContainerKind> = None;
        let mut element_ty: Option<Ty> = None;

        if let Some(type_name) = literal.explicit_type.as_ref() {
            if let Some(type_expr) = parse_type_expression_text(type_name) {
                let ty = Ty::from_type_expr(&type_expr);
                match ty {
                    Ty::Array(array) => {
                        if array.rank != 1 {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: "multi-dimensional array literals are not supported".into(),
                                span: literal_span.or(literal.explicit_type_span),
                            });
                            return None;
                        }
                        element_ty = Some(array.element.as_ref().clone());
                        container = Some(ContainerKind::Array(array));
                    }
                    Ty::Vec(vec) => {
                        element_ty = Some(vec.element.as_ref().clone());
                        container = Some(ContainerKind::Vec(vec));
                    }
                    Ty::Span(span_ty) => {
                        element_ty = Some(span_ty.element.as_ref().clone());
                        container = Some(ContainerKind::Span(span_ty));
                    }
                    Ty::ReadOnlySpan(span_ty) => {
                        element_ty = Some(span_ty.element.as_ref().clone());
                        container = Some(ContainerKind::ReadOnlySpan(span_ty));
                    }
                    other => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "type `{}` cannot be used as an array literal target",
                                other.canonical_name()
                            ),
                            span: literal_span.or(literal.explicit_type_span),
                        });
                        return None;
                    }
                }
            } else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`{}` is not a valid type expression for an array literal",
                        type_name
                    ),
                    span: literal_span.or(literal.explicit_type_span),
                });
                return None;
            }
        }

        if operands.is_empty() && element_ty.is_none() {
            self.diagnostics.push(LoweringDiagnostic {
                message: "cannot infer element type for empty array literal; annotate the literal with a target type".into(),
                span: literal_span,
            });
            return None;
        }

        if element_ty.is_none() {
            let (first_operand, _) = operands
                .first()
                .expect("non-empty operands when element type is unknown");
            element_ty = self.operand_ty(first_operand);
        }

        let Some(mut element_ty) = element_ty else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "array literal element type could not be determined".into(),
                span: literal_span,
            });
            return None;
        };

        for (operand, op_span) in operands.iter_mut() {
            if matches!(element_ty, Ty::Unknown) {
                if let Some(op_ty) = self.operand_ty(operand) {
                    element_ty = op_ty;
                }
            }
            *operand = self.coerce_operand_to_ty(
                operand.clone(),
                &element_ty,
                false,
                op_span.or(literal_span),
            );
            if let Some(op_ty) = self.operand_ty(operand) {
                if element_ty == Ty::Unknown {
                    element_ty = op_ty;
                } else if op_ty != element_ty && op_ty.canonical_name() != element_ty.canonical_name()
                {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "array literal elements must have a consistent type; expected `{}` but found `{}`",
                            element_ty.canonical_name(),
                            op_ty.canonical_name()
                        ),
                        span: op_span.or(literal_span),
                    });
                    return None;
                }
            }
        }

        if matches!(element_ty, Ty::Unknown) {
            self.diagnostics.push(LoweringDiagnostic {
                message: "array literal element type could not be determined".into(),
                span: literal_span,
            });
            return None;
        }

        let target_container = container.unwrap_or_else(|| {
            ContainerKind::Array(ArrayTy::new(Box::new(element_ty.clone()), 1))
        });

        let length_const =
            Operand::Const(ConstOperand::new(ConstValue::UInt(operands.len() as u128)));

        let (place, coerced_length, elem_size) = match &target_container {
            ContainerKind::Array(array) => {
                self.lower_new_array(array, length_const.clone(), literal_span)?
            }
            ContainerKind::Vec(vec) => {
                self.lower_new_vec(vec, length_const.clone(), literal_span)?
            }
            ContainerKind::Span(_) | ContainerKind::ReadOnlySpan(_) => {
                let backing = ArrayTy::new(Box::new(element_ty.clone()), 1);
                self.lower_new_array(&backing, length_const.clone(), literal_span)?
            }
        };

        self.zero_init_array_data(&place, elem_size.clone(), coerced_length.clone(), literal_span);

        for (index, (mut operand, op_span)) in operands.into_iter().enumerate() {
            operand = self.coerce_operand_to_ty(
                operand,
                &element_ty,
                false,
                op_span.or(literal_span),
            );
            let idx_const = Operand::Const(ConstOperand::new(ConstValue::UInt(index as u128)));
            let idx_local = self.ensure_operand_local(idx_const, op_span.or(literal_span));
            let mut element_place = place.clone();
            element_place
                .projection
                .push(ProjectionElem::Index(idx_local));
            self.normalise_place(&mut element_place);
            self.push_statement(MirStatement {
                span: op_span.or(literal_span),
                kind: MirStatementKind::Assign {
                    place: element_place,
                    value: Rvalue::Use(operand),
                },
            });
        }

        self.set_array_len(&place, coerced_length.clone(), literal_span);

        let array_operand = Operand::Copy(place.clone());
        match target_container {
            ContainerKind::Array(_) | ContainerKind::Vec(_) => Some(array_operand),
            ContainerKind::Span(span_ty) => self.emit_span_helper_call(
                "Std::Span::Span::FromArray",
                &[span_ty.element.as_ref().clone()],
                vec![array_operand],
                literal_span,
            ),
            ContainerKind::ReadOnlySpan(span_ty) => self.emit_span_helper_call(
                "Std::Span::ReadOnlySpan::FromArray",
                &[span_ty.element.as_ref().clone()],
                vec![array_operand],
                literal_span,
            ),
        }
    }

    fn lower_array_initializer(
        &mut self,
        place: &Place,
        array_ty: &ArrayTy,
        initializer: NewInitializer,
        span: Option<Span>,
    ) -> bool {
        match initializer {
            NewInitializer::Collection { elements, .. } => {
                for (index, element) in elements.into_iter().enumerate() {
                    let Some(mut value_operand) = self.lower_expr_node(element, span) else {
                        return false;
                    };
                    value_operand = self.coerce_operand_to_ty(
                        value_operand,
                        array_ty.element.as_ref(),
                        false,
                        span,
                    );
                    let index_const =
                        Operand::Const(ConstOperand::new(ConstValue::UInt(index as u128)));
                    let index_local = self.ensure_operand_local(index_const, span);
                    let mut element_place = place.clone();
                    element_place
                        .projection
                        .push(ProjectionElem::Index(index_local));
                    self.normalise_place(&mut element_place);
                    self.push_statement(MirStatement {
                        span,
                        kind: MirStatementKind::Assign {
                            place: element_place,
                            value: Rvalue::Use(value_operand),
                        },
                    });
                }
                true
            }
            NewInitializer::Object { .. } => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "object initializers are not supported for arrays".into(),
                    span,
                });
                false
            }
        }
    }

    fn lower_new_initializer(
        &mut self,
        place: &Place,
        initializer: NewInitializer,
        span: Option<Span>,
    ) -> bool {
        match initializer {
            NewInitializer::Object { fields, .. } => {
                self.apply_object_initializer(place, fields, span)
            }
            NewInitializer::Collection { elements, .. } => {
                self.apply_collection_initializer(place, elements, span)
            }
        }
    }

    fn apply_object_initializer(
        &mut self,
        place: &Place,
        fields: Vec<ObjectInitializerField>,
        span: Option<Span>,
    ) -> bool {
        let Some(type_name) = self.place_type_name(place) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: "object initializer target has unknown type".into(),
                span,
            });
            return false;
        };

        let mut success = true;
        for entry in fields {
            if !self.lower_object_initializer_entry(place, &type_name, entry, span) {
                success = false;
            }
        }
        success
    }

    fn lower_object_initializer_entry(
        &mut self,
        place: &Place,
        type_name: &str,
        entry: ObjectInitializerField,
        span: Option<Span>,
    ) -> bool {
        let ObjectInitializerField {
            name,
            name_span,
            value,
            value_span,
            span: field_span,
        } = entry;
        let member_span = field_span.or(name_span).or(span);
        if let Some((owner, symbol_ref)) = self.lookup_property_symbol(type_name, &name) {
            let symbol = symbol_ref.clone();
            let property_ty = symbol.ty.clone();
            let Some((metadata, _)) =
                self.property_setter_metadata(&symbol, &owner, &name, member_span)
            else {
                return false;
            };
            let mut args = Vec::new();
            args.push(Operand::Copy(place.clone()));
            let Some(mut value_operand) = self.lower_expr_node(value, value_span.or(member_span))
            else {
                return false;
            };
            let value_ty = Ty::named(property_ty);
            value_operand =
                self.coerce_operand_to_ty(value_operand, &value_ty, false, value_span);
            args.push(value_operand);
            if self
                .emit_property_call(&metadata.function, args, None, member_span)
                .is_none()
            {
                return false;
            }
            return true;
        }

        if let Some((owner, metadata)) =
            self.lookup_field_metadata(type_name, &name)
        {
            if metadata.is_static {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "field `{}.{}` is static and cannot be assigned via an initializer",
                        owner, name
                    ),
                    span: member_span,
                });
                return false;
            }

            let Some(mut value_operand) =
                self.lower_expr_node(value, value_span.or(member_span))
            else {
                return false;
            };
            let field_ty = Ty::from_type_expr(&metadata.ty);
            value_operand =
                self.coerce_operand_to_ty(value_operand, &field_ty, false, value_span);

            let mut field_place = place.clone();
            field_place
                .projection
                .push(ProjectionElem::FieldNamed(name.clone()));
            self.normalise_place(&mut field_place);

            self.push_statement(MirStatement {
                span: member_span,
                kind: MirStatementKind::Assign {
                    place: field_place,
                    value: Rvalue::Use(value_operand),
                },
            });
            return true;
        }

        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "type `{type_name}` does not declare a field or property named `{name}`"
            ),
            span: member_span,
        });
        false
    }
    fn apply_collection_initializer(
        &mut self,
        place: &Place,
        elements: Vec<ExprNode>,
        span: Option<Span>,
    ) -> bool {
        if elements.is_empty() {
            return true;
        }
        let binding_name = self.bind_place_as_identifier(place);
        let mut success = true;
        for element in elements {
            let callee = ExprNode::Member {
                base: Box::new(ExprNode::Identifier(binding_name.clone())),
                member: "Add".to_string(),
                null_conditional: false,
            };
            let arg = CallArgument::positional(element, span, span);
            if self.lower_call(callee.clone(), vec![arg], None, span, false).is_none() {
                success = false;
                break;
            }
        }
        self.pop_scope();
        success
    }

    fn bind_place_as_identifier(&mut self, place: &Place) -> String {
        let temp_name = format!("__object_init{}", self.temp_counter);
        self.temp_counter += 1;
        self.push_scope();
        self.bind_name(&temp_name, place.local);
        temp_name
    }

    fn lookup_property_symbol(
        &self,
        type_name: &str,
        member: &str,
    ) -> Option<(String, &PropertySymbol)> {
        for candidate in self.type_hierarchy(type_name) {
            if let Some(symbol) = self.symbol_index.property(&candidate, member) {
                return Some((candidate, symbol));
            }
        }
        None
    }

    fn lookup_field_metadata(
        &self,
        type_name: &str,
        member: &str,
    ) -> Option<(String, FieldMetadata)> {
        for candidate in self.type_hierarchy(type_name) {
            if let Some(metadata) = self.symbol_index.field_metadata(&candidate, member) {
                return Some((candidate, metadata));
            }
        }
        None
    }

    fn type_hierarchy(&self, root: &str) -> Vec<String> {
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        queue.push_back(root.to_string());
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            order.push(current.clone());
            if let Some(bases) = self.class_bases.get(&current) {
                for base in bases {
                    queue.push_back(base.clone());
                }
            }
        }
        order
    }

    pub(super) fn ensure_ty_layout_for_ty(&mut self, ty: &Ty) {
        match ty {
            Ty::Tuple(tuple) => {
                self.type_layouts.ensure_tuple_layout(tuple);
                for element in &tuple.elements {
                    self.ensure_ty_layout_for_ty(element);
                }
            }
            Ty::Fn(fn_ty) => {
                self.type_layouts.ensure_fn_layout(fn_ty);
            }
            Ty::Array(array) => {
                self.ensure_ty_layout_for_ty(array.element.as_ref());
                self.type_layouts.ensure_array_layout(array);
            }
            Ty::Vec(vec) => self.ensure_ty_layout_for_ty(vec.element.as_ref()),
            Ty::Span(span) => {
                self.ensure_ty_layout_for_ty(span.element.as_ref());
                self.type_layouts.ensure_span_layout(span);
            }
            Ty::ReadOnlySpan(span) => {
                self.ensure_ty_layout_for_ty(span.element.as_ref());
                self.type_layouts.ensure_readonly_span_layout(span);
            }
            Ty::Rc(rc) => self.ensure_ty_layout_for_ty(rc.element.as_ref()),
            Ty::Arc(arc) => self.ensure_ty_layout_for_ty(arc.element.as_ref()),
            Ty::Nullable(inner) => {
                self.ensure_ty_layout_for_ty(inner);
                self.type_layouts.ensure_nullable_layout(inner);
            }
            _ => {}
        }
    }





































































































}

enum NewTargetKind {
    Reference,
    Value,
}

impl<'a> BodyBuilder<'a> {
    fn resolve_new_type(
        &mut self,
        new_expr: &NewExpr,
        span: Option<Span>,
    ) -> Option<(Ty, String, NewTargetKind)> {
        if new_expr.type_name.is_empty() {
            return None;
        }
        let Some(type_expr) = parse_type_expression_text(&new_expr.type_name) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "`{}` is not a valid type expression for `new`",
                    new_expr.type_name
                ),
                span: span.or(new_expr.type_span).or(new_expr.span),
            });
            return None;
        };
        let ty = Ty::from_type_expr(&type_expr);
        if let Ty::Array(array) = &ty {
            let resolved = Ty::Array(array.clone()).canonical_name();
            return Some((ty, resolved, NewTargetKind::Value));
        }
        let mut canonical = ty.canonical_name();
        if let Some(self_type) = self.current_self_type_name() {
            if let Some(last_segment) = self_type.rsplit("::").next() {
                let duplicate = format!("{self_type}::{last_segment}");
                if canonical == duplicate {
                    canonical = self_type;
                }
            }
        }
        let base_name = strip_generic_arguments(&canonical);
        let mut resolved: Option<String> = None;
        let mut kind: Option<NewTargetKind> = None;

        for candidate in self.candidate_type_names(&base_name) {
            if let Some(layout) = self.type_layouts.layout_for_name(&candidate) {
                let canonical_key = self
                    .type_layouts
                    .resolve_type_key(&candidate)
                    .unwrap_or(&candidate);
                match layout {
                    TypeLayout::Class(_) => {
                        kind = Some(NewTargetKind::Reference);
                        resolved = Some(canonical_key.to_string());
                        break;
                    }
                    TypeLayout::Struct(_) | TypeLayout::Union(_) => {
                        kind = Some(NewTargetKind::Value);
                        resolved = Some(canonical_key.to_string());
                        break;
                    }
                    TypeLayout::Enum(_) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "enums cannot be constructed with `new` (`{candidate}`)"
                            ),
                            span: span.or(new_expr.type_span).or(new_expr.span),
                        });
                        return None;
                    }
                }
            } else if let Some(descriptor) = self.symbol_index.reflection_descriptor(&candidate) {
                match descriptor.kind {
                    TypeKind::Class => {
                        kind = Some(NewTargetKind::Reference);
                        resolved = Some(descriptor.name.clone());
                        break;
                    }
                    TypeKind::Struct | TypeKind::Record | TypeKind::Union => {
                        kind = Some(NewTargetKind::Value);
                        resolved = Some(descriptor.name.clone());
                        break;
                    }
                    TypeKind::Enum => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "enums cannot be constructed with `new` (`{candidate}`)"
                            ),
                            span: span.or(new_expr.type_span).or(new_expr.span),
                        });
                        return None;
                    }
                    _ => {
                        resolved.get_or_insert(candidate);
                    }
                }
            } else {
                resolved.get_or_insert(candidate);
            }
        }

        let Some(kind) = kind else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "unable to resolve type `{}` for `new` expression",
                    new_expr.type_name
                ),
                span: span.or(new_expr.type_span).or(new_expr.span),
            });
            return None;
        };

        let resolved = resolved.unwrap_or_else(|| base_name.clone());
        Some((ty, resolved, kind))
    }

    fn allocate_reference_object(&mut self, ty: &Ty, span: Option<Span>) -> Option<Place> {
        let type_id = self.type_id_operand_for_ty(ty, span)?;
        let operand = self.call_runtime_function(
            Self::OBJECT_ALLOC_RUNTIME_FN,
            vec![type_id],
            ty.clone(),
            span,
        );
        let place = self.operand_to_place(operand, span);
        Some(place)
    }

    fn allocate_value_object(&mut self, ty: &Ty, span: Option<Span>) -> Place {
        let temp = self.create_temp(span);
        self.hint_local_ty(temp, ty.clone());
        let place = Place::new(temp);
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::ZeroInit {
                place: place.clone(),
            },
        });
        place
    }

    fn initialise_vtable_header(&mut self, place: &Place, object_ty: &Ty, span: Option<Span>) {
        let Some(owner) = self.resolve_ty_name(object_ty) else {
            return;
        };
        if self.lookup_class_layout_by_name(&owner).is_none() {
            return;
        }
        let mut header_place = place.clone();
        header_place.projection.push(ProjectionElem::Deref);
        header_place
            .projection
            .push(ProjectionElem::FieldNamed("$vtable".into()));
        self.normalise_place(&mut header_place);
        let symbol = class_vtable_symbol_name(&owner);
        let operand = Operand::Const(ConstOperand::new(ConstValue::Symbol(symbol)));
        self.push_statement(MirStatement {
            span,
            kind: MirStatementKind::Assign {
                place: header_place,
                value: Rvalue::Use(operand),
            },
        });
    }

    fn candidate_type_names(&self, base: &str) -> Vec<String> {
        let trimmed = base.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        if trimmed.contains("::") {
            return vec![trimmed.to_string()];
        }

        let mut candidates = Vec::new();
        if let Some(namespace) = self.namespace.as_deref() {
            let mut current = Some(namespace);
            while let Some(prefix) = current {
                candidates.push(format!("{prefix}::{trimmed}"));
                current = prefix.rfind("::").map(|idx| &prefix[..idx]);
            }
        }
        candidates.push(trimmed.to_string());
        candidates
    }
}

fn strip_generic_arguments(text: &str) -> String {
    let mut result = String::new();
    let mut depth = 0i32;
    for ch in text.chars() {
        match ch {
            '<' => depth += 1,
            '>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ => {
                if depth == 0 {
                    result.push(ch);
                }
            }
        }
    }
    result.trim().to_string()
}
