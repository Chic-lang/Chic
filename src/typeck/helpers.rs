use crate::frontend::ast::TypeExpr;

pub(super) fn base_type_name(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

pub(super) fn strip_receiver(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

pub(super) fn type_names_equivalent(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let a_short = strip_receiver(a);
    let b_short = strip_receiver(b);
    if a_short == b_short {
        return true;
    }
    if a.contains('<') || b.contains('<') {
        return false;
    }
    let a_base = base_type_name(a);
    let b_base = base_type_name(b);
    if a_base == b_base {
        return true;
    }
    strip_receiver(a_base) == strip_receiver(b_base)
}

pub(super) fn canonical_type_name(expr: &TypeExpr) -> String {
    base_type_name(expr.name.as_str()).replace('.', "::")
}

pub(super) fn type_expr_path(expr: &TypeExpr) -> Option<String> {
    if expr.base.is_empty() {
        None
    } else {
        Some(expr.base.join("::"))
    }
}
