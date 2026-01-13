use super::expression::{
    Associativity, ChildPosition, ExpressionFormatter, PREC_CALL, PREC_LAMBDA, PREC_PRIMARY,
    PREC_UNARY, Rendered, format_lambda_param, wrap_child,
};
use super::strings::escape_interpolated_text;
use crate::syntax::expr::builders::{
    CallArgument, ExprNode, InlineBindingKind, InterpolatedExprSegment, InterpolatedStringExpr,
    InterpolatedStringSegment, LambdaBlock, LambdaBody, LambdaExpr, NameOfOperand, NewExpr,
    NewInitializer, QuoteLiteral, SizeOfOperand,
};

impl ExpressionFormatter {
    pub(super) fn render_lambda(&self, lambda: &LambdaExpr) -> Rendered {
        let params = lambda
            .params
            .iter()
            .map(format_lambda_param)
            .collect::<Vec<_>>()
            .join(", ");
        let body = match &lambda.body {
            LambdaBody::Expression(expr) => self.render_expr(expr).text,
            LambdaBody::Block(LambdaBlock { text, .. }) => text.trim().to_string(),
        };
        let prefix = if lambda.is_async { "async " } else { "" };
        Rendered {
            text: format!("{prefix}({params}) => {body}"),
            precedence: PREC_LAMBDA,
        }
    }

    pub(super) fn render_tuple(&self, elements: &[ExprNode]) -> Rendered {
        let inner = elements
            .iter()
            .map(|element| self.render_expr(element).text)
            .collect::<Vec<_>>()
            .join(", ");
        let text = if elements.len() == 1 {
            format!("({inner},)")
        } else {
            format!("({inner})")
        };
        Rendered {
            text,
            precedence: PREC_PRIMARY,
        }
    }

    pub(super) fn render_member_access(
        &self,
        base: &ExprNode,
        member: &str,
        null_conditional: bool,
    ) -> Rendered {
        let rendered = self.render_expr(base);
        let wrapped = wrap_child(
            rendered,
            PREC_CALL,
            ChildPosition::Left,
            Associativity::Left,
        );
        let separator = if null_conditional { "?." } else { "." };
        Rendered {
            text: format!("{wrapped}{separator}{member}"),
            precedence: PREC_CALL,
        }
    }

    pub(super) fn render_call(
        &self,
        callee: &ExprNode,
        generics: Option<&[String]>,
        args: &[CallArgument],
    ) -> Rendered {
        let rendered = self.render_expr(callee);
        let callee_text = wrap_child(
            rendered,
            PREC_CALL,
            ChildPosition::Left,
            Associativity::Left,
        );
        let generic_text = generics
            .filter(|items| !items.is_empty())
            .map(|items| format!("<{}>", items.join(", ")))
            .unwrap_or_default();
        let arguments = args
            .iter()
            .map(|arg| self.format_argument(arg))
            .collect::<Vec<_>>()
            .join(", ");
        Rendered {
            text: format!("{callee_text}{generic_text}({arguments})"),
            precedence: PREC_CALL,
        }
    }

    pub(super) fn render_new(&self, new_expr: &NewExpr) -> Rendered {
        let arguments = new_expr
            .args
            .iter()
            .map(|arg| self.format_argument(arg))
            .collect::<Vec<_>>()
            .join(", ");
        let mut text = format!("new {}({arguments})", new_expr.type_name);
        if let Some(initializer) = &new_expr.initializer {
            let init_text = match initializer {
                NewInitializer::Object { fields, .. } => {
                    let body = fields
                        .iter()
                        .map(|field| {
                            format!("{} = {}", field.name, self.render_expr(&field.value).text)
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    if body.is_empty() {
                        " { }".to_string()
                    } else {
                        format!(" {{ {body} }}")
                    }
                }
                NewInitializer::Collection { elements, .. } => {
                    let body = elements
                        .iter()
                        .map(|element| self.render_expr(element).text)
                        .collect::<Vec<_>>()
                        .join(", ");
                    if body.is_empty() {
                        " { }".to_string()
                    } else {
                        format!(" {{ {body} }}")
                    }
                }
            };
            text.push_str(&init_text);
        }
        Rendered {
            text,
            precedence: PREC_CALL,
        }
    }

    pub(super) fn render_index(
        &self,
        base: &ExprNode,
        indices: &[ExprNode],
        null_conditional: bool,
    ) -> Rendered {
        let rendered = self.render_expr(base);
        let base_text = wrap_child(
            rendered,
            PREC_CALL,
            ChildPosition::Left,
            Associativity::Left,
        );
        let entries = indices
            .iter()
            .map(|element| self.render_expr(element).text)
            .collect::<Vec<_>>()
            .join(", ");
        let prefix = if null_conditional { "?[" } else { "[" };
        Rendered {
            text: format!("{base_text}{prefix}{entries}]"),
            precedence: PREC_CALL,
        }
    }

    pub(super) fn render_try_propagate(&self, expr: &ExprNode) -> Rendered {
        let rendered = self.render_expr(expr);
        let base = wrap_child(
            rendered,
            PREC_CALL,
            ChildPosition::Left,
            Associativity::Left,
        );
        Rendered {
            text: format!("{base}?"),
            precedence: PREC_CALL,
        }
    }

    pub(super) fn render_await(&self, expr: &ExprNode) -> Rendered {
        let rendered = self.render_expr(expr);
        let body = if rendered.precedence < PREC_UNARY {
            format!("({})", rendered.text)
        } else {
            rendered.text
        };
        Rendered {
            text: format!("await {body}"),
            precedence: PREC_UNARY,
        }
    }

    pub(super) fn render_throw(&self, expr: Option<&ExprNode>) -> Rendered {
        let suffix = expr.map(|inner| {
            let rendered = self.render_expr(inner);
            if rendered.precedence < PREC_UNARY {
                format!(" ({})", rendered.text)
            } else {
                format!(" {}", rendered.text)
            }
        });
        Rendered {
            text: format!("throw{}", suffix.unwrap_or_default()),
            precedence: PREC_UNARY,
        }
    }

    pub(super) fn render_size_related(&self, keyword: &str, operand: &SizeOfOperand) -> Rendered {
        let inner = match operand {
            SizeOfOperand::Type(name) => name.to_string(),
            SizeOfOperand::Value(expr) => self.render_expr(expr).text,
        };
        Rendered {
            text: format!("{}({inner})", keyword),
            precedence: PREC_PRIMARY,
        }
    }

    pub(super) fn render_nameof(&self, operand: &NameOfOperand) -> Rendered {
        Rendered {
            text: format!("nameof({})", operand.display()),
            precedence: PREC_PRIMARY,
        }
    }

    pub(super) fn render_interpolated_string(
        &self,
        interpolated: &InterpolatedStringExpr,
    ) -> Rendered {
        let body = interpolated
            .segments
            .iter()
            .map(|segment| match segment {
                InterpolatedStringSegment::Text(raw) => escape_interpolated_text(raw),
                InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                    expr,
                    alignment,
                    format,
                    ..
                }) => {
                    let mut part = format!("{{{}", self.render_expr(expr).text);
                    if let Some(alignment) = alignment {
                        part.push_str(&format!(",{alignment}"));
                    }
                    if let Some(spec) = format {
                        part.push(':');
                        part.push_str(spec);
                    }
                    part.push('}');
                    part
                }
            })
            .collect::<String>();
        Rendered {
            text: format!("$\"{body}\""),
            precedence: PREC_PRIMARY,
        }
    }

    pub(super) fn render_quote(&self, quote: &QuoteLiteral) -> Rendered {
        Rendered {
            text: format!("quote({})", quote.source),
            precedence: PREC_PRIMARY,
        }
    }

    pub(super) fn format_argument(&self, argument: &CallArgument) -> String {
        let name = argument
            .name
            .as_ref()
            .map(|name| format!("{}: ", name.text))
            .unwrap_or_default();
        let modifier = argument
            .modifier
            .map(|modifier| format!("{} ", modifier.keyword()))
            .unwrap_or_default();
        let value = if let Some(binding) = argument.inline_binding.as_ref() {
            let keyword = match &binding.kind {
                InlineBindingKind::Var => "var".to_string(),
                InlineBindingKind::Typed { type_name, .. } => type_name.clone(),
            };
            let initializer = binding
                .initializer
                .as_ref()
                .map(|expr| {
                    let rendered = self.render_expr(expr);
                    format!(" = {}", rendered.text)
                })
                .unwrap_or_default();
            format!("{keyword} {}{initializer}", binding.name)
        } else {
            self.render_expr(&argument.value).text
        };
        format!("{name}{modifier}{value}")
    }
}
