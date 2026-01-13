use std::collections::HashSet;

use crate::frontend::diagnostics::Span;
use crate::mir::ConstEvalContext;
use crate::mir::builder::const_eval::ConstEvalResult;
use crate::mir::builder::const_eval::diagnostics::{self, ConstEvalError};
use crate::mir::builder::const_eval::environment::EvalEnv;
use crate::mir::data::{BinOp, ConstValue, UnOp};
use crate::syntax::expr::builders::{
    AssignOp, InterpolatedExprSegment, InterpolatedStringSegment, LambdaBody, NewInitializer,
    QuoteLiteral, QuoteSourceSpan, SizeOfOperand,
};
use crate::syntax::expr::{CallArgument, ExprNode, format_expression};

const META_QUOTE: &str = "Std::Meta::Quote";
const META_QUOTE_SPAN: &str = "Std::Meta::QuoteSpan";
const META_QUOTE_HYGIENE: &str = "Std::Meta::QuoteHygiene";
const META_QUOTE_INTERPOLATION: &str = "Std::Meta::QuoteInterpolation";
const META_QUOTE_NODE: &str = "Std::Meta::QuoteNode";
const META_QUOTE_NODE_KIND: &str = "Std::Meta::QuoteNodeKind";

impl<'a> ConstEvalContext<'a> {
    pub(crate) fn evaluate_quote_literal(
        &mut self,
        literal: &QuoteLiteral,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstEvalResult, ConstEvalError> {
        let span_value = self.build_quote_span_value(literal.content_span, env.span);
        let hygiene_value = self.build_quote_hygiene_value(literal, env.span);
        let interpolation_list = self.build_quote_interpolations(literal, env)?;
        let captures = self.build_quote_capture_list(&literal.expression);
        let capture_list = self.reflect_string_list(&captures);
        let root = self.build_quote_node_value(&literal.expression);

        let quote_value = ConstValue::Struct {
            type_name: META_QUOTE.into(),
            fields: vec![
                ("Source".into(), ConstValue::RawStr(literal.source.clone())),
                (
                    "Sanitized".into(),
                    ConstValue::RawStr(literal.sanitized.clone()),
                ),
                ("Span".into(), span_value),
                ("Hygiene".into(), hygiene_value),
                ("Captures".into(), capture_list),
                ("Interpolations".into(), interpolation_list),
                ("Root".into(), root),
            ],
        };
        Ok(ConstEvalResult::new(quote_value))
    }

    fn build_quote_interpolations(
        &mut self,
        literal: &QuoteLiteral,
        env: &mut EvalEnv<'_, '_>,
    ) -> Result<ConstValue, ConstEvalError> {
        let mut values = Vec::with_capacity(literal.interpolations.len());
        for interpolation in &literal.interpolations {
            let result = self.evaluate_node(&interpolation.expression, env)?;
            if !self.is_quote_value(&result.value) {
                let span = self
                    .quote_span_to_span(interpolation.span, env.span)
                    .or(env.span);
                return Err(ConstEvalError {
                    message: format!(
                        "quote interpolation `{}` must evaluate to `Std.Meta.Quote`",
                        interpolation.placeholder
                    ),
                    span,
                });
            }
            let span_value = self.build_quote_span_value(interpolation.span, env.span);
            let fields = vec![
                (
                    "Placeholder".into(),
                    ConstValue::RawStr(interpolation.placeholder.clone()),
                ),
                ("Value".into(), result.value),
                ("Span".into(), span_value),
            ];
            values.push(ConstValue::Struct {
                type_name: META_QUOTE_INTERPOLATION.into(),
                fields,
            });
        }
        Ok(self.reflect_descriptor_list(META_QUOTE_INTERPOLATION, values))
    }

    fn is_quote_value(&self, value: &ConstValue) -> bool {
        match value {
            ConstValue::Struct { type_name, .. } => {
                diagnostics::simple_name(type_name) == diagnostics::simple_name(META_QUOTE)
            }
            _ => false,
        }
    }

    fn build_quote_span_value(
        &self,
        span: Option<QuoteSourceSpan>,
        outer: Option<Span>,
    ) -> ConstValue {
        let (start, end) = span
            .map(|relative| self.absolute_quote_span_values(relative, outer))
            .unwrap_or((0, 0));
        ConstValue::Struct {
            type_name: META_QUOTE_SPAN.into(),
            fields: vec![
                ("Start".into(), ConstValue::UInt(start as u128)),
                ("End".into(), ConstValue::UInt(end as u128)),
            ],
        }
    }

    fn quote_span_to_span(
        &self,
        span: Option<QuoteSourceSpan>,
        outer: Option<Span>,
    ) -> Option<Span> {
        span.map(|relative| {
            let (start, end) = self.absolute_quote_span_values(relative, outer);
            Span::new(start, end)
        })
    }

    fn absolute_quote_span_values(
        &self,
        span: QuoteSourceSpan,
        outer: Option<Span>,
    ) -> (usize, usize) {
        if let Some(base) = outer {
            (
                base.start.saturating_add(span.start),
                base.start.saturating_add(span.end),
            )
        } else {
            (span.start, span.end)
        }
    }

    fn build_quote_hygiene_value(&self, literal: &QuoteLiteral, outer: Option<Span>) -> ConstValue {
        let anchor = if let Some(span) = outer {
            span.start.saturating_add(literal.hygiene_anchor)
        } else {
            literal.hygiene_anchor
        } as u128;
        let seed = self.compute_quote_seed(literal, outer) as u128;
        ConstValue::Struct {
            type_name: META_QUOTE_HYGIENE.into(),
            fields: vec![
                ("Anchor".into(), ConstValue::UInt(anchor)),
                ("Seed".into(), ConstValue::UInt(seed)),
            ],
        }
    }

    fn compute_quote_seed(&self, literal: &QuoteLiteral, outer: Option<Span>) -> u64 {
        let anchor = if let Some(span) = outer {
            span.start.saturating_add(literal.hygiene_anchor)
        } else {
            literal.hygiene_anchor
        } as u64;
        let mut hash = anchor ^ (literal.sanitized.len() as u64).rotate_left(13);
        hash ^= (literal.interpolations.len() as u64).rotate_left(27);
        for byte in literal.sanitized.as_bytes() {
            hash = hash.wrapping_mul(1099511628211).wrapping_add(*byte as u64);
        }
        hash
    }

    pub(crate) fn build_quote_capture_list(&self, expr: &ExprNode) -> Vec<String> {
        let mut captures = Vec::new();
        let mut seen = HashSet::new();
        self.collect_quote_captures(expr, &mut captures, &mut seen);
        captures
    }

    fn collect_quote_captures(
        &self,
        node: &ExprNode,
        captures: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        match node {
            ExprNode::Identifier(name) => {
                if !name.starts_with("__chic_quote_slot") && seen.insert(name.clone()) {
                    captures.push(name.clone());
                }
            }
            ExprNode::Default(_) => {}
            ExprNode::Unary { expr, .. }
            | ExprNode::Parenthesized(expr)
            | ExprNode::Ref { expr, .. }
            | ExprNode::Await { expr }
            | ExprNode::TryPropagate { expr, .. }
            | ExprNode::Throw {
                expr: Some(expr), ..
            } => self.collect_quote_captures(expr, captures, seen),
            ExprNode::IndexFromEnd(index) => {
                self.collect_quote_captures(index.expr.as_ref(), captures, seen);
            }
            ExprNode::Range(range) => {
                if let Some(start) = range.start.as_ref() {
                    self.collect_quote_captures(start.expr.as_ref(), captures, seen);
                }
                if let Some(end) = range.end.as_ref() {
                    self.collect_quote_captures(end.expr.as_ref(), captures, seen);
                }
            }
            ExprNode::Binary { left, right, .. } => {
                self.collect_quote_captures(left, captures, seen);
                self.collect_quote_captures(right, captures, seen);
            }
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_quote_captures(condition, captures, seen);
                self.collect_quote_captures(then_branch, captures, seen);
                self.collect_quote_captures(else_branch, captures, seen);
            }
            ExprNode::Switch(switch_expr) => {
                self.collect_quote_captures(&switch_expr.value, captures, seen);
                for arm in &switch_expr.arms {
                    self.collect_quote_captures(&arm.expression, captures, seen);
                    for guard in &arm.guards {
                        self.collect_quote_captures(&guard.expr, captures, seen);
                    }
                }
            }
            ExprNode::Cast { expr, .. } | ExprNode::SizeOf(SizeOfOperand::Value(expr)) => {
                self.collect_quote_captures(expr, captures, seen);
            }
            ExprNode::AlignOf(SizeOfOperand::Value(expr)) => {
                self.collect_quote_captures(expr, captures, seen);
            }
            ExprNode::Assign { target, value, .. } => {
                self.collect_quote_captures(target, captures, seen);
                self.collect_quote_captures(value, captures, seen);
            }
            ExprNode::Member { base, .. } => {
                self.collect_quote_captures(base, captures, seen);
            }
            ExprNode::Index {
                base,
                indices,
                null_conditional: _,
            } => {
                self.collect_quote_captures(base, captures, seen);
                for element in indices {
                    self.collect_quote_captures(element, captures, seen);
                }
            }
            ExprNode::Tuple(indices) => {
                for element in indices {
                    self.collect_quote_captures(element, captures, seen);
                }
            }
            ExprNode::ArrayLiteral(array) => {
                for element in &array.elements {
                    self.collect_quote_captures(element, captures, seen);
                }
            }
            ExprNode::Call { callee, args, .. } => {
                self.collect_quote_captures(callee, captures, seen);
                for arg in args {
                    self.collect_quote_captures(&arg.value, captures, seen);
                    if let Some(binding) = arg.inline_binding.as_ref() {
                        if let Some(initializer) = binding.initializer.as_ref() {
                            self.collect_quote_captures(initializer, captures, seen);
                        }
                    }
                }
            }
            ExprNode::New(new_expr) => {
                for arg in &new_expr.args {
                    self.collect_quote_captures(&arg.value, captures, seen);
                }
                if let Some(initializer) = &new_expr.initializer {
                    match initializer {
                        NewInitializer::Object { fields, .. } => {
                            for field in fields {
                                self.collect_quote_captures(&field.value, captures, seen);
                            }
                        }
                        NewInitializer::Collection { elements, .. } => {
                            for element in elements {
                                self.collect_quote_captures(element, captures, seen);
                            }
                        }
                    }
                }
            }
            ExprNode::Lambda(lambda) => {
                if let LambdaBody::Expression(body) = &lambda.body {
                    self.collect_quote_captures(body, captures, seen);
                }
            }
            ExprNode::InterpolatedString(interpolated) => {
                for segment in &interpolated.segments {
                    if let InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                        expr, ..
                    }) = segment
                    {
                        self.collect_quote_captures(expr, captures, seen);
                    }
                }
            }
            ExprNode::Quote(_) | ExprNode::Literal(_) | ExprNode::Throw { expr: None, .. } => {}
            ExprNode::NameOf(_) => {}
            ExprNode::InlineAsm(_) => {}
            ExprNode::IsPattern { value, .. } => {
                self.collect_quote_captures(value, captures, seen);
            }
            ExprNode::SizeOf(SizeOfOperand::Type(_))
            | ExprNode::AlignOf(SizeOfOperand::Type(_)) => {}
        }
    }

    pub(crate) fn build_quote_node_value(&mut self, node: &ExprNode) -> ConstValue {
        let (kind, value, children) = match node {
            ExprNode::Literal(_) => (
                QuoteNodeKindValue::Literal,
                Some(self.render_expr_text(node)),
                Vec::new(),
            ),
            ExprNode::Default(default_expr) => (
                QuoteNodeKindValue::Literal,
                Some(
                    default_expr
                        .explicit_type
                        .as_ref()
                        .map(|ty| format!("default({ty})"))
                        .unwrap_or_else(|| "default".to_string()),
                ),
                Vec::new(),
            ),
            ExprNode::Identifier(name) => (
                QuoteNodeKindValue::Identifier,
                Some(name.clone()),
                Vec::new(),
            ),
            ExprNode::Unary { op, expr, postfix } => {
                let child = self.build_quote_node_value(expr);
                let symbol = self.unary_symbol(*op);
                let value = if *postfix && matches!(op, UnOp::Increment | UnOp::Decrement) {
                    format!("{symbol}(post)")
                } else {
                    symbol.to_string()
                };
                (QuoteNodeKindValue::Unary, Some(value), vec![child])
            }
            ExprNode::IndexFromEnd(index) => {
                let child = self.build_quote_node_value(&index.expr);
                (
                    QuoteNodeKindValue::Unary,
                    Some("^".to_string()),
                    vec![child],
                )
            }
            ExprNode::ArrayLiteral(array) => {
                let children = array
                    .elements
                    .iter()
                    .map(|elem| self.build_quote_node_value(elem))
                    .collect();
                (QuoteNodeKindValue::Tuple, Some("[]".to_string()), children)
            }
            ExprNode::Range(range) => {
                let mut parts = Vec::new();
                if let Some(start) = range.start.as_ref() {
                    parts.push(self.build_quote_node_value(&start.expr));
                }
                if let Some(end) = range.end.as_ref() {
                    parts.push(self.build_quote_node_value(&end.expr));
                }
                (
                    QuoteNodeKindValue::Binary,
                    Some(if range.inclusive { "..=" } else { ".." }.to_string()),
                    parts,
                )
            }
            ExprNode::Binary { op, left, right } => {
                let left_node = self.build_quote_node_value(left);
                let right_node = self.build_quote_node_value(right);
                (
                    QuoteNodeKindValue::Binary,
                    Some(self.binary_symbol(*op).to_string()),
                    vec![left_node, right_node],
                )
            }
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_node = self.build_quote_node_value(condition);
                let then_node = self.build_quote_node_value(then_branch);
                let else_node = self.build_quote_node_value(else_branch);
                (
                    QuoteNodeKindValue::Conditional,
                    Some(self.render_expr_text(node)),
                    vec![condition_node, then_node, else_node],
                )
            }
            ExprNode::Switch(switch_expr) => {
                let mut children = vec![self.build_quote_node_value(&switch_expr.value)];
                for arm in &switch_expr.arms {
                    children.push(self.build_quote_node_value(&arm.expression));
                }
                (
                    QuoteNodeKindValue::Unknown,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Cast { target, expr, .. } => {
                let child = self.build_quote_node_value(expr);
                (QuoteNodeKindValue::Cast, Some(target.clone()), vec![child])
            }
            ExprNode::IsPattern { value, .. } => {
                let child = self.build_quote_node_value(value);
                (
                    QuoteNodeKindValue::Pattern,
                    Some(self.render_expr_text(node)),
                    vec![child],
                )
            }
            ExprNode::Lambda(lambda) => {
                let mut children = Vec::new();
                for param in &lambda.params {
                    children.push(self.make_quote_node(
                        QuoteNodeKindValue::Argument,
                        Some(param.name.clone()),
                        Vec::new(),
                    ));
                }
                match &lambda.body {
                    LambdaBody::Expression(body) => {
                        children.push(self.build_quote_node_value(body));
                    }
                    LambdaBody::Block(block) => {
                        children.push(self.make_quote_node(
                            QuoteNodeKindValue::Unknown,
                            Some(block.text.trim().to_string()),
                            Vec::new(),
                        ));
                    }
                }
                (
                    QuoteNodeKindValue::Lambda,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Parenthesized(expr) => {
                let child = self.build_quote_node_value(expr);
                (
                    QuoteNodeKindValue::Tuple,
                    Some(self.render_expr_text(node)),
                    vec![child],
                )
            }
            ExprNode::Tuple(elements) => {
                let children = elements
                    .iter()
                    .map(|element| self.build_quote_node_value(element))
                    .collect::<Vec<_>>();
                (
                    QuoteNodeKindValue::Tuple,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Assign { op, target, value } => {
                let target_node = self.build_quote_node_value(target);
                let value_node = self.build_quote_node_value(value);
                (
                    QuoteNodeKindValue::Assign,
                    Some(self.assign_symbol(*op).to_string()),
                    vec![target_node, value_node],
                )
            }
            ExprNode::Member {
                base,
                member,
                null_conditional: _,
            } => {
                let child = self.build_quote_node_value(base);
                (
                    QuoteNodeKindValue::Member,
                    Some(member.clone()),
                    vec![child],
                )
            }
            ExprNode::Call { callee, args, .. } => {
                let mut children = Vec::with_capacity(args.len() + 1);
                children.push(self.build_quote_node_value(callee));
                for arg in args {
                    children.push(self.build_quote_argument_node(arg));
                }
                (
                    QuoteNodeKindValue::Call,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Ref { expr, readonly } => {
                let child = self.build_quote_node_value(expr);
                (
                    QuoteNodeKindValue::Ref,
                    Some(if *readonly {
                        "readonly".into()
                    } else {
                        "ref".into()
                    }),
                    vec![child],
                )
            }
            ExprNode::New(new_expr) => {
                let mut children = Vec::new();
                for arg in &new_expr.args {
                    children.push(self.build_quote_argument_node(arg));
                }
                if let Some(initializer) = &new_expr.initializer {
                    match initializer {
                        NewInitializer::Object { fields, .. } => {
                            for field in fields {
                                let value = self.build_quote_node_value(&field.value);
                                children.push(self.make_quote_node(
                                    QuoteNodeKindValue::Argument,
                                    Some(format!("field={}", field.name)),
                                    vec![value],
                                ));
                            }
                        }
                        NewInitializer::Collection { elements, .. } => {
                            for (index, element) in elements.iter().enumerate() {
                                let value = self.build_quote_node_value(element);
                                children.push(self.make_quote_node(
                                    QuoteNodeKindValue::Argument,
                                    Some(format!("element#{index}")),
                                    vec![value],
                                ));
                            }
                        }
                    }
                }
                (
                    QuoteNodeKindValue::New,
                    Some(new_expr.type_name.clone()),
                    children,
                )
            }
            ExprNode::Index {
                base,
                indices,
                null_conditional: _,
            } => {
                let mut children = Vec::with_capacity(indices.len() + 1);
                children.push(self.build_quote_node_value(base));
                for element in indices {
                    children.push(self.build_quote_node_value(element));
                }
                (
                    QuoteNodeKindValue::Index,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Await { expr } => {
                let child = self.build_quote_node_value(expr);
                (
                    QuoteNodeKindValue::Await,
                    Some(self.render_expr_text(node)),
                    vec![child],
                )
            }
            ExprNode::TryPropagate { expr, .. } => {
                let child = self.build_quote_node_value(expr);
                (
                    QuoteNodeKindValue::TryPropagate,
                    Some(self.render_expr_text(node)),
                    vec![child],
                )
            }
            ExprNode::Throw { expr } => {
                let children = expr
                    .iter()
                    .map(|inner| self.build_quote_node_value(inner))
                    .collect::<Vec<_>>();
                (
                    QuoteNodeKindValue::Throw,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::SizeOf(operand) => match operand {
                SizeOfOperand::Type(name) => {
                    (QuoteNodeKindValue::SizeOf, Some(name.clone()), Vec::new())
                }
                SizeOfOperand::Value(expr) => {
                    let child = self.build_quote_node_value(expr);
                    (
                        QuoteNodeKindValue::SizeOf,
                        Some(self.render_expr_text(node)),
                        vec![child],
                    )
                }
            },
            ExprNode::AlignOf(operand) => match operand {
                SizeOfOperand::Type(name) => {
                    (QuoteNodeKindValue::AlignOf, Some(name.clone()), Vec::new())
                }
                SizeOfOperand::Value(expr) => {
                    let child = self.build_quote_node_value(expr);
                    (
                        QuoteNodeKindValue::AlignOf,
                        Some(self.render_expr_text(node)),
                        vec![child],
                    )
                }
            },
            ExprNode::NameOf(operand) => (
                QuoteNodeKindValue::NameOf,
                Some(operand.text.clone()),
                Vec::new(),
            ),
            ExprNode::InlineAsm(_) => (
                QuoteNodeKindValue::Literal,
                Some("asm!".to_string()),
                Vec::new(),
            ),
            ExprNode::InterpolatedString(interpolated) => {
                let mut children = Vec::new();
                for segment in &interpolated.segments {
                    match segment {
                        InterpolatedStringSegment::Text(text) => {
                            children.push(self.make_quote_node(
                                QuoteNodeKindValue::Literal,
                                Some(text.clone()),
                                Vec::new(),
                            ));
                        }
                        InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                            expr, ..
                        }) => {
                            children.push(self.build_quote_node_value(expr));
                        }
                    }
                }
                (
                    QuoteNodeKindValue::InterpolatedString,
                    Some(self.render_expr_text(node)),
                    children,
                )
            }
            ExprNode::Quote(inner) => {
                let child = self.build_quote_node_value(&inner.expression);
                (
                    QuoteNodeKindValue::Quote,
                    Some(inner.source.clone()),
                    vec![child],
                )
            }
        };
        self.make_quote_node(kind, value, children)
    }

    fn render_expr_text(&self, node: &ExprNode) -> String {
        format_expression(node)
    }

    fn make_quote_node(
        &self,
        kind: QuoteNodeKindValue,
        value: Option<String>,
        children: Vec<ConstValue>,
    ) -> ConstValue {
        let (variant, discriminant) = kind.descriptor();
        ConstValue::Struct {
            type_name: META_QUOTE_NODE.into(),
            fields: vec![
                (
                    "Kind".into(),
                    self.enum_const(META_QUOTE_NODE_KIND, variant, discriminant),
                ),
                (
                    "Value".into(),
                    value.map(ConstValue::RawStr).unwrap_or(ConstValue::Null),
                ),
                (
                    "Children".into(),
                    self.reflect_descriptor_list(META_QUOTE_NODE, children),
                ),
            ],
        }
    }

    fn build_quote_argument_node(&mut self, argument: &CallArgument) -> ConstValue {
        let mut details = Vec::new();
        if let Some(name) = argument.name.as_ref() {
            details.push(format!("name={}", name.text));
        }
        if let Some(modifier) = argument.modifier {
            details.push(format!("modifier={}", modifier.keyword()));
        }
        if argument.inline_binding.is_some() {
            details.push("binding=true".to_string());
        }
        let summary = if details.is_empty() {
            None
        } else {
            Some(details.join(","))
        };
        let child = self.build_quote_node_value(&argument.value);
        self.make_quote_node(QuoteNodeKindValue::Argument, summary, vec![child])
    }

    fn unary_symbol(&self, op: UnOp) -> &'static str {
        match op {
            UnOp::Neg => "-",
            UnOp::UnaryPlus => "+",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
            UnOp::Increment => "++",
            UnOp::Decrement => "--",
            UnOp::Deref => "*",
            UnOp::AddrOf => "&",
            UnOp::AddrOfMut => "&mut",
        }
    }

    fn binary_symbol(&self, op: BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::NullCoalesce => "??",
        }
    }

    fn assign_symbol(&self, op: AssignOp) -> &'static str {
        match op {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
            AssignOp::RemAssign => "%=",
            AssignOp::BitAndAssign => "&=",
            AssignOp::BitOrAssign => "|=",
            AssignOp::BitXorAssign => "^=",
            AssignOp::ShlAssign => "<<=",
            AssignOp::ShrAssign => ">>=",
            AssignOp::NullCoalesceAssign => "??=",
        }
    }
}

enum QuoteNodeKindValue {
    Literal,
    Identifier,
    Unary,
    Binary,
    Conditional,
    Cast,
    Lambda,
    Tuple,
    Assign,
    Member,
    Call,
    Argument,
    Ref,
    New,
    Index,
    Await,
    TryPropagate,
    Throw,
    SizeOf,
    AlignOf,
    NameOf,
    InterpolatedString,
    Quote,
    Pattern,
    Unknown,
}

impl QuoteNodeKindValue {
    fn descriptor(&self) -> (&'static str, i128) {
        match self {
            Self::Literal => ("Literal", 0),
            Self::Identifier => ("Identifier", 1),
            Self::Unary => ("Unary", 2),
            Self::Binary => ("Binary", 3),
            Self::Conditional => ("Conditional", 4),
            Self::Cast => ("Cast", 5),
            Self::Lambda => ("Lambda", 6),
            Self::Tuple => ("Tuple", 7),
            Self::Assign => ("Assign", 8),
            Self::Member => ("Member", 9),
            Self::Call => ("Call", 10),
            Self::Argument => ("Argument", 11),
            Self::Ref => ("Ref", 12),
            Self::New => ("New", 13),
            Self::Index => ("Index", 14),
            Self::Await => ("Await", 15),
            Self::TryPropagate => ("TryPropagate", 16),
            Self::Throw => ("Throw", 17),
            Self::SizeOf => ("SizeOf", 18),
            Self::AlignOf => ("AlignOf", 19),
            Self::NameOf => ("NameOf", 20),
            Self::InterpolatedString => ("InterpolatedString", 21),
            Self::Quote => ("Quote", 22),
            Self::Pattern => ("Pattern", 23),
            Self::Unknown => ("Unknown", 24),
        }
    }
}
