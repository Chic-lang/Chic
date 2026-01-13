use super::{ExprError, ExprNode, parse_expression};
use crate::frontend::diagnostics::Span;
use crate::frontend::literals::StringSegment;
use crate::syntax::expr::builders::{
    InterpolatedExprSegment, InterpolatedStringExpr, InterpolatedStringSegment,
};

pub(super) fn parse_interpolated_string(
    segments: Vec<StringSegment>,
    literal_span: Span,
    content_start: usize,
) -> Result<ExprNode, ExprError> {
    let mut parsed_segments = Vec::with_capacity(segments.len());
    for segment in segments {
        match segment {
            StringSegment::Text(text) => {
                parsed_segments.push(InterpolatedStringSegment::Text(text));
            }
            StringSegment::Interpolation(segment) => {
                let expr_base = content_start.saturating_add(segment.expression_offset);
                let expr_span = if segment.expression_len == 0 {
                    None
                } else {
                    Some(Span::new(
                        expr_base,
                        expr_base.saturating_add(segment.expression_len),
                    ))
                };
                let expr = match parse_expression(segment.expression.as_str()) {
                    Ok(expr) => expr,
                    Err(mut err) => {
                        let offset_span = err.span.take().map(|inner| {
                            Span::new(
                                expr_base.saturating_add(inner.start),
                                expr_base.saturating_add(inner.end),
                            )
                        });
                        let span = offset_span.or(expr_span);
                        return Err(ExprError::new(err.message, span));
                    }
                };
                parsed_segments.push(InterpolatedStringSegment::Expr(InterpolatedExprSegment {
                    expr,
                    expr_text: segment.expression,
                    alignment: segment.alignment,
                    format: segment.format,
                    span: expr_span,
                }));
            }
        }
    }
    Ok(ExprNode::InterpolatedString(InterpolatedStringExpr {
        segments: parsed_segments,
        span: Some(literal_span),
    }))
}
