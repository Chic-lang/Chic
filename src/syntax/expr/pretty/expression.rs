//! Expression pretty-printing utilities.

use super::pattern::format_pattern;
use super::strings::format_const_value;
use crate::mir::{BinOp, UnOp};
use crate::syntax::expr::builders::{
    ArrayLiteralExpr, AssignOp, CastSyntax, ExprNode, LambdaParam, PatternGuardExpr, RangeExpr,
    SwitchExpr,
};
use crate::syntax::expr::precedence::{
    assignment_operator_symbol, binary_operator_symbol, is_right_associative, precedence_for_bin_op,
};
use crate::syntax::pattern::PatternAst;

pub(super) const PREC_LAMBDA: u8 = 0;
pub(super) const PREC_ASSIGN: u8 = 1;
pub(super) const PREC_CONDITIONAL: u8 = 2;
pub(super) const PREC_BINARY_BASE: u8 = 5;
pub(super) const PREC_RELATIONAL: u8 = PREC_BINARY_BASE + 6;
pub(super) const PREC_UNARY: u8 = 30;
pub(super) const PREC_PAREN_CAST: u8 = 32;
pub(super) const PREC_CALL: u8 = 40;
pub(super) const PREC_PRIMARY: u8 = 50;

/// Render an expression tree into a canonical textual form.
#[must_use]
pub fn format_expression(expr: &ExprNode) -> String {
    ExpressionFormatter::default().format(expr)
}

#[derive(Default)]
pub(super) struct ExpressionFormatter;

pub(super) struct Rendered {
    pub(super) text: String,
    pub(super) precedence: u8,
}

#[derive(Clone, Copy)]
pub(super) enum Associativity {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub(super) enum ChildPosition {
    Left,
    Right,
}

impl ExpressionFormatter {
    fn format(&self, expr: &ExprNode) -> String {
        self.render_expr(expr).text
    }

    pub(super) fn render_expr(&self, expr: &ExprNode) -> Rendered {
        match expr {
            ExprNode::Literal(literal) => Rendered {
                text: format_const_value(&literal.value),
                precedence: PREC_PRIMARY,
            },
            ExprNode::Identifier(name) => Rendered {
                text: name.clone(),
                precedence: PREC_PRIMARY,
            },
            ExprNode::Unary { op, expr, postfix } => self.render_unary(*op, expr, *postfix),
            ExprNode::IndexFromEnd(index) => Rendered {
                text: format!("^{}", self.format(&index.expr)),
                precedence: PREC_UNARY,
            },
            ExprNode::Range(range) => self.render_range(range),
            ExprNode::Binary { op, left, right } => self.render_binary(*op, left, right),
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.render_conditional(condition, then_branch, else_branch),
            ExprNode::Cast {
                target,
                expr,
                syntax,
            } => self.render_cast(target, expr, *syntax),
            ExprNode::IsPattern {
                value,
                pattern,
                guards,
            } => self.render_is_pattern(value, pattern, guards),
            ExprNode::Lambda(lambda) => self.render_lambda(lambda),
            ExprNode::Parenthesized(inner) => {
                let rendered = self.render_expr(inner);
                Rendered {
                    text: format!("({})", rendered.text),
                    precedence: PREC_PRIMARY,
                }
            }
            ExprNode::Tuple(elements) => self.render_tuple(elements),
            ExprNode::ArrayLiteral(array) => self.render_array_literal(array),
            ExprNode::Assign { target, op, value } => self.render_assignment(target, *op, value),
            ExprNode::Member {
                base,
                member,
                null_conditional,
            } => self.render_member_access(base, member, *null_conditional),
            ExprNode::Call {
                callee,
                args,
                generics,
            } => self.render_call(callee, generics.as_deref(), args),
            ExprNode::Ref { expr, readonly } => {
                let rendered = self.render_expr(expr);
                let body = if rendered.precedence < PREC_UNARY {
                    format!("({})", rendered.text)
                } else {
                    rendered.text
                };
                let prefix = if *readonly { "ref readonly" } else { "ref" };
                Rendered {
                    text: format!("{prefix} {body}"),
                    precedence: PREC_UNARY,
                }
            }
            ExprNode::New(new_expr) => self.render_new(new_expr),
            ExprNode::Index {
                base,
                indices,
                null_conditional,
            } => self.render_index(base, indices, *null_conditional),
            ExprNode::Await { expr } => self.render_await(expr),
            ExprNode::TryPropagate { expr, .. } => self.render_try_propagate(expr),
            ExprNode::Throw { expr } => self.render_throw(expr.as_deref()),
            ExprNode::SizeOf(operand) => self.render_size_related("sizeof", operand),
            ExprNode::AlignOf(operand) => self.render_size_related("alignof", operand),
            ExprNode::NameOf(operand) => self.render_nameof(operand),
            ExprNode::Switch(switch_expr) => self.render_switch(switch_expr),
            ExprNode::InterpolatedString(interpolated) => {
                self.render_interpolated_string(interpolated)
            }
            ExprNode::InlineAsm(_) => Rendered {
                text: "asm!(...)".to_string(),
                precedence: PREC_PRIMARY,
            },
            ExprNode::Quote(literal) => self.render_quote(literal),
            ExprNode::Default(default_expr) => {
                let text = if let Some(explicit) = &default_expr.explicit_type {
                    format!("default({explicit})")
                } else {
                    "default".to_string()
                };
                Rendered {
                    text,
                    precedence: PREC_PRIMARY,
                }
            }
        }
    }

    fn render_range(&self, range: &RangeExpr) -> Rendered {
        let start_text = range
            .start
            .as_ref()
            .map(|endpoint| {
                let prefix = if endpoint.from_end { "^" } else { "" };
                format!("{prefix}{}", self.format(&endpoint.expr))
            })
            .unwrap_or_else(String::new);
        let end_text = range.end.as_ref().map(|endpoint| {
            let prefix = if endpoint.from_end { "^" } else { "" };
            format!("{prefix}{}", self.format(&endpoint.expr))
        });
        let mut text = start_text;
        text.push_str(if range.inclusive { "..=" } else { ".." });
        if let Some(end) = end_text {
            text.push_str(&end);
        }
        Rendered {
            text,
            precedence: PREC_RELATIONAL,
        }
    }

    fn render_array_literal(&self, array: &ArrayLiteralExpr) -> Rendered {
        let elements: Vec<String> = array
            .elements
            .iter()
            .map(|elem| self.format(elem))
            .collect();
        let mut text = String::new();
        if let Some(explicit) = &array.explicit_type {
            text.push_str(explicit);
            text.push(' ');
        }
        text.push('[');
        text.push_str(&elements.join(", "));
        if array.trailing_comma && !elements.is_empty() {
            text.push(',');
        }
        text.push(']');
        Rendered {
            text,
            precedence: PREC_PRIMARY,
        }
    }

    fn render_unary(&self, op: UnOp, expr: &ExprNode, postfix: bool) -> Rendered {
        let rendered = self.render_expr(expr);
        let body = if rendered.precedence < PREC_UNARY {
            format!("({})", rendered.text)
        } else {
            rendered.text
        };
        let text = if postfix {
            format!("{body}{}", unary_prefix(op))
        } else {
            format!("{}{}", unary_prefix(op), body)
        };
        Rendered {
            text,
            precedence: PREC_UNARY,
        }
    }

    fn render_binary(&self, op: BinOp, left: &ExprNode, right: &ExprNode) -> Rendered {
        let left_rendered = self.render_expr(left);
        let right_rendered = self.render_expr(right);
        let prec = binary_precedence(op);
        let assoc = if is_right_associative(op) {
            Associativity::Right
        } else {
            Associativity::Left
        };
        let left_text = wrap_child(left_rendered, prec, ChildPosition::Left, assoc);
        let right_text = wrap_child(right_rendered, prec, ChildPosition::Right, assoc);
        Rendered {
            text: format!("{left_text} {} {right_text}", binary_operator_symbol(op)),
            precedence: prec,
        }
    }

    fn render_conditional(
        &self,
        condition: &ExprNode,
        then_branch: &ExprNode,
        else_branch: &ExprNode,
    ) -> Rendered {
        let condition_text = wrap_child(
            self.render_expr(condition),
            PREC_CONDITIONAL,
            ChildPosition::Left,
            Associativity::Right,
        );
        let then_rendered = self.render_expr(then_branch);
        let then_text = if then_rendered.precedence <= PREC_CONDITIONAL {
            format!("({})", then_rendered.text)
        } else {
            then_rendered.text
        };
        let else_text = wrap_child(
            self.render_expr(else_branch),
            PREC_CONDITIONAL,
            ChildPosition::Right,
            Associativity::Right,
        );
        Rendered {
            text: format!("{condition_text} ? {then_text} : {else_text}"),
            precedence: PREC_CONDITIONAL,
        }
    }

    fn render_assignment(&self, target: &ExprNode, op: AssignOp, value: &ExprNode) -> Rendered {
        let left = wrap_child(
            self.render_expr(target),
            PREC_ASSIGN,
            ChildPosition::Left,
            Associativity::Right,
        );
        let right = wrap_child(
            self.render_expr(value),
            PREC_ASSIGN,
            ChildPosition::Right,
            Associativity::Right,
        );
        Rendered {
            text: format!("{left} {} {right}", assignment_operator_symbol(op)),
            precedence: PREC_ASSIGN,
        }
    }

    fn render_cast(&self, target: &str, expr: &ExprNode, syntax: CastSyntax) -> Rendered {
        match syntax {
            CastSyntax::Paren => {
                let rendered = self.render_expr(expr);
                let operand = if rendered.precedence < PREC_PAREN_CAST {
                    format!("({})", rendered.text)
                } else {
                    rendered.text
                };
                Rendered {
                    text: format!("({target}){operand}"),
                    precedence: PREC_PAREN_CAST,
                }
            }
            CastSyntax::As => {
                let rendered = self.render_expr(expr);
                let left = wrap_child(
                    rendered,
                    PREC_RELATIONAL,
                    ChildPosition::Left,
                    Associativity::Left,
                );
                Rendered {
                    text: format!("{left} as {target}"),
                    precedence: PREC_RELATIONAL,
                }
            }
        }
    }

    fn render_is_pattern(
        &self,
        value: &ExprNode,
        pattern: &PatternAst,
        guards: &[PatternGuardExpr],
    ) -> Rendered {
        let rendered = self.render_expr(value);
        let left = wrap_child(
            rendered,
            PREC_RELATIONAL,
            ChildPosition::Left,
            Associativity::Left,
        );
        let mut text = format!("{left} is {}", format_pattern(pattern));
        for guard in guards {
            text.push_str(" when ");
            text.push_str(&self.render_expr(&guard.expr).text);
        }
        Rendered {
            text,
            precedence: PREC_RELATIONAL,
        }
    }

    fn render_switch(&self, switch_expr: &SwitchExpr) -> Rendered {
        let discr = wrap_child(
            self.render_expr(&switch_expr.value),
            PREC_CONDITIONAL,
            ChildPosition::Left,
            Associativity::Right,
        );
        let mut rendered_arms = Vec::new();
        for arm in &switch_expr.arms {
            let mut arm_head = format_pattern(&arm.pattern);
            for guard in &arm.guards {
                arm_head.push_str(" when ");
                arm_head.push_str(&self.render_expr(&guard.expr).text);
            }
            let rendered_value = self.render_expr(&arm.expression);
            let value_text = wrap_child(
                rendered_value,
                PREC_ASSIGN,
                ChildPosition::Right,
                Associativity::Right,
            );
            rendered_arms.push(format!("{arm_head} => {value_text}"));
        }
        let arms = rendered_arms.join(", ");
        Rendered {
            text: format!("{discr} switch {{ {arms} }}"),
            precedence: PREC_CONDITIONAL,
        }
    }
}

pub(super) fn wrap_child(
    rendered: Rendered,
    parent_prec: u8,
    position: ChildPosition,
    associativity: Associativity,
) -> String {
    let needs_parens =
        child_needs_parens(rendered.precedence, parent_prec, position, associativity);
    if needs_parens {
        format!("({})", rendered.text)
    } else {
        rendered.text
    }
}

pub(super) fn child_needs_parens(
    child_prec: u8,
    parent_prec: u8,
    position: ChildPosition,
    associativity: Associativity,
) -> bool {
    if child_prec < parent_prec {
        return true;
    }
    if child_prec > parent_prec {
        return false;
    }
    match (position, associativity) {
        (ChildPosition::Left, Associativity::Right) => true,
        (ChildPosition::Right, Associativity::Left) => true,
        _ => false,
    }
}

pub(super) fn binary_precedence(op: BinOp) -> u8 {
    precedence_for_bin_op(op).saturating_add(PREC_BINARY_BASE)
}

pub(super) fn unary_prefix(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::UnaryPlus => "+",
        UnOp::Not => "!",
        UnOp::BitNot => "~",
        UnOp::Increment => "++",
        UnOp::Decrement => "--",
        UnOp::Deref => "*",
        UnOp::AddrOf => "&",
        UnOp::AddrOfMut => "&mut ",
    }
}

pub(super) fn format_lambda_param(param: &LambdaParam) -> String {
    let mut text = String::new();
    if let Some(modifier) = param.modifier {
        text.push_str(modifier.keyword());
        text.push(' ');
    }
    if let Some(ty) = &param.ty {
        text.push_str(ty);
        text.push(' ');
    }
    text.push_str(&param.name);
    text
}
