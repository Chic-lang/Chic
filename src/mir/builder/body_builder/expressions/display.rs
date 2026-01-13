use super::*;
use crate::syntax::expr::builders::NewInitializer;
use std::char;

body_builder_impl! {
    pub(crate) fn expr_to_string(expr: &ExprNode) -> String {
        match expr {
            ExprNode::Identifier(name) => name.clone(),
            ExprNode::Literal(literal) => match literal.value.clone() {
                ConstValue::RawStr(s) => format!("\"{s}\""),
                ConstValue::Str { value, .. } => format!("\"{value}\""),
                ConstValue::Int(i) | ConstValue::Int32(i) => format!("{i}"),
                ConstValue::UInt(u) => format!("{u}"),
                ConstValue::Float(f) => f.display(),
                ConstValue::Decimal(value) => format!("{}", value.into_decimal()),
                ConstValue::Bool(b) => format!("{b}"),
                ConstValue::Char(c) => {
                    let mut repr = String::with_capacity(4);
                    repr.push('\'');
                    if let Some(scalar) = char::from_u32(u32::from(c)) {
                        repr.extend(scalar.escape_default());
                    } else {
                        repr.push_str(&format!("\\u{c:04X}"));
                    }
                    repr.push('\'');
                    repr
                }
                ConstValue::Null => String::from("null"),
                ConstValue::Symbol(sym) => sym,
                ConstValue::Enum {
                    type_name,
                    variant,
                    ..
                } => {
                    let short = type_name.rsplit("::").next().unwrap_or(&type_name);
                    format!("{short}::{variant}")
                }
                ConstValue::Struct { type_name, .. } => {
                    let short = type_name.rsplit("::").next().unwrap_or(&type_name);
                    format!("{short} {{ .. }}")
                }
                ConstValue::Unit => String::from("()"),
                ConstValue::Unknown => String::from("<unknown>"),
            },
            ExprNode::Default(default_expr) => {
                if let Some(explicit) = &default_expr.explicit_type {
                    format!("default({explicit})")
                } else {
                    "default".to_string()
                }
            }
            ExprNode::Member {
                base,
                member,
                null_conditional,
            } => {
                let separator = if *null_conditional { "?." } else { "." };
                format!("{}{}{}", Self::expr_to_string(base), separator, member)
            }
            ExprNode::Cast { target, expr, syntax } => match syntax {
                CastSyntax::Paren => format!("({target}){}", Self::expr_to_string(expr)),
                CastSyntax::As => format!("{} as {target}", Self::expr_to_string(expr)),
            },
            ExprNode::Call { callee, args, generics } => {
                let generic_text = generics.as_ref().map(|list| {
                    if list.is_empty() {
                        String::new()
                    } else {
                        let joined = list.join(", ");
                        format!("<{joined}>")
                    }
                }).unwrap_or_default();
                let arg_text = args
                    .iter()
                    .map(|arg| {
                        let value = Self::expr_to_string(&arg.value);
                        let modifier = arg
                            .modifier
                            .map(|modifier| match modifier {
                                CallArgumentModifier::In => "in ",
                                CallArgumentModifier::Ref => "ref ",
                                CallArgumentModifier::Out => "out ",
                            })
                            .unwrap_or("");
                        if let Some(name) = &arg.name {
                            format!("{}: {}{}", name.text, modifier, value)
                        } else {
                            format!("{modifier}{value}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{}{}({})",
                    Self::expr_to_string(callee),
                    generic_text,
                    arg_text
                )
            }
            ExprNode::Lambda(lambda) => {
                let mut text = String::new();
                if lambda.is_async {
                    text.push_str("async ");
                }
                let params = lambda
                    .params
                    .iter()
                    .map(|param| {
                        let mut part = String::new();
                        if let Some(modifier) = param.modifier {
                            let keyword = match modifier {
                                LambdaParamModifier::In => "in",
                                LambdaParamModifier::Ref => "ref",
                                LambdaParamModifier::Out => "out",
                            };
                            part.push_str(keyword);
                            part.push(' ');
                        }
                        if let Some(ty) = &param.ty {
                            if !ty.is_empty() {
                                part.push_str(ty);
                                part.push(' ');
                            }
                        }
                        part.push_str(&param.name);
                        part
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                text.push('(');
                text.push_str(&params);
                text.push(')');
                text.push_str(" => ");
                match &lambda.body {
                    LambdaBody::Expression(body) => text.push_str(&Self::expr_to_string(body)),
                    LambdaBody::Block(block) => text.push_str(block.text.as_str()),
                }
                text
            }
            ExprNode::Index {
                base,
                indices,
                null_conditional,
            } => {
                let parts = indices
                    .iter()
                    .map(Self::expr_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let prefix = if *null_conditional { "?[" } else { "[" };
                format!("{}{prefix}{parts}]", Self::expr_to_string(base))
            }
            ExprNode::IndexFromEnd(from_end) => {
                format!("^{}", Self::expr_to_string(&from_end.expr))
            }
            ExprNode::Range(range) => {
                let start = range.start.as_ref().map(|endpoint| {
                    let prefix = if endpoint.from_end { "^" } else { "" };
                    format!("{prefix}{}", Self::expr_to_string(&endpoint.expr))
                });
                let end = range.end.as_ref().map(|endpoint| {
                    let prefix = if endpoint.from_end { "^" } else { "" };
                    format!("{prefix}{}", Self::expr_to_string(&endpoint.expr))
                });
                let mut text = String::new();
                if let Some(start) = start {
                    text.push_str(&start);
                }
                text.push_str(if range.inclusive { "..=" } else { ".." });
                if let Some(end) = end {
                    text.push_str(&end);
                }
                text
            }
            ExprNode::Parenthesized(inner) => {
                format!("({})", Self::expr_to_string(inner))
            }
            ExprNode::Tuple(elements) => {
                let parts = elements
                    .iter()
                    .map(Self::expr_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({parts})")
            }
            ExprNode::ArrayLiteral(array) => {
                let elems = array
                    .elements
                    .iter()
                    .map(Self::expr_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut text = String::new();
                if let Some(explicit) = &array.explicit_type {
                    text.push_str(explicit);
                    text.push(' ');
                }
                text.push('[');
                text.push_str(&elems);
                if array.trailing_comma && !elems.is_empty() {
                    text.push(',');
                }
                text.push(']');
                text
            }
            ExprNode::Unary { op, expr, postfix } => {
                let op_str = match op {
                    UnOp::Neg => "-",
                    UnOp::UnaryPlus => "+",
                    UnOp::Not => "!",
                    UnOp::BitNot => "~",
                    UnOp::Increment => "++",
                    UnOp::Decrement => "--",
                    UnOp::Deref => "*",
                    UnOp::AddrOf => "&",
                    UnOp::AddrOfMut => "&mut",
                };
                if *postfix {
                    format!("{}{}", Self::expr_to_string(expr), op_str)
                } else {
                    format!("{}{}", op_str, Self::expr_to_string(expr))
                }
            }
            ExprNode::Ref { expr, readonly } => {
                let prefix = if *readonly { "ref readonly" } else { "ref" };
                format!("{prefix} {}", Self::expr_to_string(expr))
            }
            ExprNode::Binary { op, left, right } => {
                let op_str = format!("{op:?}");
                format!(
                    "({} {} {})",
                    Self::expr_to_string(left),
                    op_str.to_lowercase(),
                    Self::expr_to_string(right)
                )
            }
            ExprNode::Conditional {
                condition,
                then_branch,
                else_branch,
            } => format!(
                "{} ? {} : {}",
                Self::expr_to_string(condition),
                Self::expr_to_string(then_branch),
                Self::expr_to_string(else_branch)
            ),
            ExprNode::Switch(switch_expr) => {
                let discr = Self::expr_to_string(&switch_expr.value);
                let mut arms = Vec::new();
                for arm in &switch_expr.arms {
                    let mut head = Self::pattern_ast_to_string(&arm.pattern);
                    for guard in &arm.guards {
                        head.push_str(" when ");
                        head.push_str(&Self::expr_to_string(&guard.expr));
                    }
                    let value = Self::expr_to_string(&arm.expression);
                    arms.push(format!("{head} => {value}"));
                }
                format!("{discr} switch {{ {} }}", arms.join(", "))
            }
            ExprNode::Await { expr } => {
                format!("await {}", Self::expr_to_string(expr))
            }
            ExprNode::TryPropagate { expr, .. } => {
                format!("{}?", Self::expr_to_string(expr))
            }
            ExprNode::Throw { expr } => match expr {
                Some(value) => format!("throw {}", Self::expr_to_string(value)),
                None => "throw".to_string(),
            },
            ExprNode::SizeOf(operand) => match operand {
                SizeOfOperand::Type(name) => format!("sizeof({name})"),
                SizeOfOperand::Value(expr) => {
                    format!("sizeof({})", Self::expr_to_string(expr.as_ref()))
                }
            },
            ExprNode::AlignOf(operand) => match operand {
                SizeOfOperand::Type(name) => format!("alignof({name})"),
                SizeOfOperand::Value(expr) => {
                    format!("alignof({})", Self::expr_to_string(expr.as_ref()))
                }
            },
            ExprNode::New(new_expr) => {
                let mut text = format!("new {}", new_expr.type_name);
                let args_text = new_expr
                    .args
                    .iter()
                    .map(|arg| {
                        let value = Self::expr_to_string(&arg.value);
                        let modifier = arg
                            .modifier
                            .map(|modifier| match modifier {
                                CallArgumentModifier::In => "in ",
                                CallArgumentModifier::Ref => "ref ",
                                CallArgumentModifier::Out => "out ",
                            })
                            .unwrap_or("");
                        if let Some(name) = &arg.name {
                            format!("{}: {}{}", name.text, modifier, value)
                        } else {
                            format!("{modifier}{value}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                text.push('(');
                text.push_str(&args_text);
                text.push(')');
                if let Some(initializer) = &new_expr.initializer {
                    let init_text = match initializer {
                        NewInitializer::Object { fields, .. } => {
                            let field_text = fields
                                .iter()
                                .map(|field| {
                                    format!(
                                        "{} = {}",
                                        field.name,
                                        Self::expr_to_string(&field.value)
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!(" {{ {field_text} }}")
                        }
                        NewInitializer::Collection { elements, .. } => {
                            let elems_text = elements
                                .iter()
                                .map(Self::expr_to_string)
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!(" {{ {elems_text} }}")
                        }
                    };
                    text.push_str(&init_text);
                }
                text
            }
            ExprNode::NameOf(operand) => format!("nameof({})", operand.display()),
            ExprNode::InterpolatedString(_) => "$\"…\"".to_string(),
            ExprNode::InlineAsm(_) => "asm!".to_string(),
            ExprNode::Quote(literal) => format!("quote({})", literal.source),
            ExprNode::Assign { target, op, value } => {
                let op_str = format!("{op:?}");
                format!(
                    "{} {} {}",
                    Self::expr_to_string(target),
                    op_str.to_lowercase(),
                    Self::expr_to_string(value)
                )
            }
            ExprNode::IsPattern {
                value,
                pattern,
                guards,
            } => {
                let pattern_text = Self::pattern_ast_to_string(pattern);
                let mut text = format!("{} is {}", Self::expr_to_string(value), pattern_text);
                for guard in guards {
                    text.push_str(" when ");
                    text.push_str(&Self::expr_to_string(&guard.expr));
                }
                text
            }
        }
    }
    pub(crate) fn const_to_string(value: &ConstValue) -> String {
        match value {
            ConstValue::Str { value, .. } | ConstValue::RawStr(value) => {
                format!("\"{value}\"")
            }
            ConstValue::Int(i) | ConstValue::Int32(i) => i.to_string(),
            ConstValue::UInt(u) => u.to_string(),
            ConstValue::Float(f) => f.display(),
            ConstValue::Decimal(value) => value.into_decimal().to_string(),
            ConstValue::Bool(b) => b.to_string(),
            ConstValue::Char(c) => {
                let mut repr = String::with_capacity(4);
                repr.push('\'');
                if let Some(scalar) = char::from_u32(u32::from(*c)) {
                    repr.extend(scalar.escape_default());
                } else {
                    repr.push_str(&format!("\\u{c:04X}"));
                }
                repr.push('\'');
                repr
            }
            ConstValue::Null => "null".into(),
            ConstValue::Symbol(sym) => sym.clone(),
            ConstValue::Enum {
                type_name,
                variant,
                ..
            } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short}::{variant}")
            }
            ConstValue::Struct { type_name, .. } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short} {{ .. }}")
            }
            ConstValue::Unit => "()".into(),
            ConstValue::Unknown => "<unknown>".into(),
        }
    }
}
