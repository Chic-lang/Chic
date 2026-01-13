use super::collect::{CaptureCollector, collect_expr_node};
use crate::syntax::expr::parse_expression;
use crate::syntax::pattern::{PatternBinaryOp, PatternNode, VariantPatternFieldsNode};

pub(super) fn collect_pattern_node(collector: &mut CaptureCollector<'_>, pattern: &PatternNode) {
    match pattern {
        PatternNode::Wildcard | PatternNode::Binding(_) | PatternNode::Literal(_) => {}
        PatternNode::Tuple(elements) | PatternNode::Positional { elements, .. } => {
            for element in elements {
                collect_pattern_node(collector, element);
            }
        }
        PatternNode::Struct { fields, .. } => {
            for field in fields {
                collect_pattern_node(collector, &field.pattern);
            }
        }
        PatternNode::Enum { fields, .. } => match fields {
            VariantPatternFieldsNode::Unit => {}
            VariantPatternFieldsNode::Tuple(elements) => {
                for element in elements {
                    collect_pattern_node(collector, element);
                }
            }
            VariantPatternFieldsNode::Struct(fields) => {
                for field in fields {
                    collect_pattern_node(collector, &field.pattern);
                }
            }
        },
        PatternNode::Type { subpattern, .. } => {
            if let Some(inner) = subpattern {
                collect_pattern_node(collector, inner);
            }
        }
        PatternNode::Relational { expr, .. } => {
            if let Ok(parsed) = parse_expression(&expr.text) {
                collect_expr_node(collector, &parsed);
            }
        }
        PatternNode::Binary {
            left,
            op: PatternBinaryOp::And | PatternBinaryOp::Or,
            right,
        } => {
            collect_pattern_node(collector, left);
            collect_pattern_node(collector, right);
        }
        PatternNode::Not(inner) => collect_pattern_node(collector, inner),
        PatternNode::List(list) => {
            for element in &list.prefix {
                collect_pattern_node(collector, element);
            }
            if let Some(slice) = &list.slice {
                collect_pattern_node(collector, slice);
            }
            for element in &list.suffix {
                collect_pattern_node(collector, element);
            }
        }
        PatternNode::Record(record) => {
            for field in &record.fields {
                collect_pattern_node(collector, &field.pattern);
            }
        }
    }
}
