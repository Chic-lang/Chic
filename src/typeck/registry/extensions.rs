use super::*;
use crate::frontend::ast::ExtensionDecl;

pub(crate) fn normalize_extension_conditions(
    checker: &mut TypeChecker<'_>,
    ext: &ExtensionDecl,
    namespace: Option<&str>,
) -> Option<Vec<String>> {
    if ext.conditions.is_empty() {
        return Some(Vec::new());
    }
    let mut resolved = Vec::new();
    for condition in &ext.conditions {
        if !condition.target.name.eq_ignore_ascii_case("Self")
            || !condition.target.suffixes.is_empty()
        {
            checker.emit_error(
                codes::DEFAULT_CONDITION_INVALID,
                condition.span,
                "`when` clauses must use the form `Self : InterfaceName`",
            );
            return None;
        }
        match checker.resolve_type_for_expr(&condition.constraint, namespace, None) {
            ImportResolution::Found(candidate) => {
                if !checker.is_interface(&candidate) {
                    checker.emit_error(
                        codes::DEFAULT_CONDITION_INVALID,
                        condition.span,
                        format!(
                            "constraint `{}` must resolve to an interface",
                            condition.constraint.name
                        ),
                    );
                    return None;
                }
                resolved.push(candidate);
            }
            ImportResolution::Ambiguous(candidates) => {
                checker.emit_error(
                    codes::DEFAULT_CONDITION_INVALID,
                    condition.span,
                    format!(
                        "constraint `{}` resolves to multiple candidates: {}",
                        condition.constraint.name,
                        candidates.join(", ")
                    ),
                );
                return None;
            }
            ImportResolution::NotFound => {
                checker.emit_error(
                    codes::DEFAULT_CONDITION_INVALID,
                    condition.span,
                    format!(
                        "constraint `{}` is not a known interface",
                        condition.constraint.name
                    ),
                );
                return None;
            }
        }
    }
    Some(resolved)
}
