use crate::mir::{PatternBindingMode, PatternBindingMutability};
use crate::syntax::pattern::{
    BindingPatternNode, ListPatternNode, PatternAst, PatternBinaryOp, PatternFieldNode,
    PatternNode, RelationalOp, VariantPatternFieldsNode,
};

use super::strings::format_const_value;

pub(super) fn format_pattern(pattern: &PatternAst) -> String {
    format_pattern_node(&pattern.node)
}

fn format_pattern_node(node: &PatternNode) -> String {
    match node {
        PatternNode::Wildcard => "_".to_string(),
        PatternNode::Literal(value) => format_const_value(value),
        PatternNode::Binding(binding) => format_binding(binding),
        PatternNode::Tuple(elements) => {
            let inner = elements
                .iter()
                .map(format_pattern_node)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({inner})")
        }
        PatternNode::Struct { path, fields } => {
            let prefix = join_path(path);
            if fields.is_empty() {
                format!("{prefix} {{}}")
            } else {
                let fields = format_pattern_fields(fields);
                format!("{prefix} {{ {fields} }}")
            }
        }
        PatternNode::Record(record) => {
            let fields = format_pattern_fields(&record.fields);
            if let Some(path) = &record.path {
                let prefix = join_path(path);
                format!("{prefix} {{ {fields} }}")
            } else {
                format!("{{ {fields} }}")
            }
        }
        PatternNode::Enum {
            path,
            variant,
            fields,
        } => {
            let prefix = join_path(path);
            let rendered_fields = format_variant_fields(fields);
            if prefix.is_empty() {
                format!("{variant}{rendered_fields}")
            } else {
                format!("{prefix}.{variant}{rendered_fields}")
            }
        }
        PatternNode::Positional { path, elements } => {
            let inner = elements
                .iter()
                .map(format_pattern_node)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({inner})", join_path(path))
        }
        PatternNode::Type { path, subpattern } => {
            if let Some(pattern) = subpattern {
                format!("{} {}", join_path(path), format_pattern_node(pattern))
            } else {
                join_path(path)
            }
        }
        PatternNode::Relational { op, expr } => {
            format!("{} {}", relational_op_symbol(*op), expr.text)
        }
        PatternNode::Binary { op, left, right } => format!(
            "{} {} {}",
            format_pattern_node(left),
            pattern_binary_symbol(*op),
            format_pattern_node(right)
        ),
        PatternNode::Not(inner) => format!("not {}", format_pattern_node(inner)),
        PatternNode::List(list) => format_list_pattern(list),
    }
}

fn format_binding(binding: &BindingPatternNode) -> String {
    let mut parts = Vec::new();
    match binding.mode {
        PatternBindingMode::In => parts.push("in".to_string()),
        PatternBindingMode::Ref => parts.push("ref".to_string()),
        PatternBindingMode::RefReadonly => {
            parts.push("ref".to_string());
            parts.push("readonly".to_string());
        }
        PatternBindingMode::Move => parts.push("move".to_string()),
        PatternBindingMode::Value => {}
    }
    match binding.mutability {
        PatternBindingMutability::Immutable => parts.push("let".to_string()),
        PatternBindingMutability::Mutable => parts.push("var".to_string()),
    }
    parts.push(binding.name.clone());
    parts.join(" ")
}

fn format_pattern_fields(fields: &[PatternFieldNode]) -> String {
    fields
        .iter()
        .map(|field| format!("{}: {}", field.name, format_pattern_node(&field.pattern)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_variant_fields(fields: &VariantPatternFieldsNode) -> String {
    match fields {
        VariantPatternFieldsNode::Unit => String::new(),
        VariantPatternFieldsNode::Tuple(elements) => {
            let inner = elements
                .iter()
                .map(format_pattern_node)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({inner})")
        }
        VariantPatternFieldsNode::Struct(fields) => {
            format!(" {{ {} }}", format_pattern_fields(fields))
        }
    }
}

fn format_list_pattern(list: &ListPatternNode) -> String {
    let mut parts = Vec::new();
    parts.extend(list.prefix.iter().map(format_pattern_node));
    if let Some(slice) = &list.slice {
        let slice_text = format!("..{}", format_pattern_node(slice));
        parts.push(slice_text);
    }
    parts.extend(list.suffix.iter().map(format_pattern_node));
    format!("[{}]", parts.join(", "))
}

fn join_path(path: &[String]) -> String {
    path.join(".")
}

fn relational_op_symbol(op: RelationalOp) -> &'static str {
    match op {
        RelationalOp::Less => "<",
        RelationalOp::LessEqual => "<=",
        RelationalOp::Greater => ">",
        RelationalOp::GreaterEqual => ">=",
    }
}

fn pattern_binary_symbol(op: PatternBinaryOp) -> &'static str {
    match op {
        PatternBinaryOp::And => "and",
        PatternBinaryOp::Or => "or",
    }
}
