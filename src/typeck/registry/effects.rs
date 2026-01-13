use super::*;
use crate::frontend::ast::{Parameter, Signature};
use crate::frontend::diagnostics::{Label, Span, Suggestion};

impl<'a> TypeChecker<'a> {
    pub(crate) fn record_declared_effects(
        &mut self,
        full_name: &str,
        signature: &Signature,
        namespace: Option<&str>,
        context_type: Option<&str>,
        clause_span: Option<Span>,
    ) {
        if let Some(clause) = &signature.throws {
            let mut effects = Vec::new();
            for effect in &clause.types {
                self.ensure_type_expr(effect, namespace, context_type, clause_span.or(clause.span));
                let resolved = match self.resolve_type_for_expr(effect, namespace, context_type) {
                    ImportResolution::Found(name) => name,
                    ImportResolution::Ambiguous(candidates) => candidates
                        .first()
                        .cloned()
                        .unwrap_or_else(|| effect.name.replace('.', "::")),
                    ImportResolution::NotFound => effect.name.replace('.', "::"),
                };
                if !effects.contains(&resolved) {
                    effects.push(resolved);
                }
            }
            self.declared_effects.insert(full_name.to_string(), effects);
        } else {
            self.declared_effects.remove(full_name);
        }
    }

    pub(crate) fn validate_parameter_defaults(
        &mut self,
        function_name: &str,
        parameters: &[Parameter],
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        let mut seen_default: Option<(String, Option<Span>, Option<Span>)> = None;
        for param in parameters {
            if let Some(default_expr) = &param.default {
                self.validate_expression(function_name, default_expr, namespace, context_type);
                self.check_numeric_literal_expression(default_expr, Some(&param.ty));
                if matches!(param.binding, BindingModifier::Ref | BindingModifier::Out) {
                    let modifier = match param.binding {
                        BindingModifier::Ref => "ref",
                        BindingModifier::Out => "out",
                        BindingModifier::In => "in",
                        BindingModifier::Value => "value",
                    };
                    self.emit_error(
                        codes::PARAMETER_DEFAULT_REF,
                        default_expr.span,
                        format!(
                            "parameter `{}` in `{function_name}` cannot declare a default because `{modifier}` parameters require explicit caller input",
                            param.name
                        ),
                    );
                }
                seen_default = Some((
                    param.name.clone(),
                    default_expr.span.or(param.default_span),
                    param.name_span,
                ));
            } else if let Some((prev_name, prev_default_span, prev_name_span)) = &seen_default {
                let mut diag = diagnostics::error(
                    codes::PARAMETER_DEFAULT_ORDER,
                    format!(
                        "parameters with default values must appear at the end of `{function_name}`"
                    ),
                    param.name_span.or(param.ty.span),
                );
                if let Some(primary) = param.name_span.or(param.ty.span) {
                    diag.primary_label = Some(Label::primary(
                        primary,
                        format!("`{}` is missing a default value", param.name),
                    ));
                }
                if let Some(secondary) = prev_default_span.or(*prev_name_span) {
                    diag.secondary_labels.push(Label::secondary(
                        secondary,
                        format!("`{}` declares a default value here", prev_name),
                    ));
                }
                diag.add_suggestion(Suggestion::new(
                    format!(
                        "move `{}` before `{}` or supply a default for `{}`",
                        param.name, prev_name, param.name
                    ),
                    param.name_span.or(param.ty.span),
                    None,
                ));
                self.diagnostics.push(diag);
            }
        }
    }

    pub(crate) fn validate_lends_return_clause(
        &mut self,
        function_name: &str,
        signature: &Signature,
    ) {
        let Some(clause) = &signature.lends_to_return else {
            return;
        };

        if !signature.return_type.is_view {
            self.emit_error(
                codes::LENDS_RETURN_REQUIRES_VIEW,
                clause.span,
                format!(
                    "`lends({})` requires the return type of `{function_name}` to be declared as a `view`",
                    clause.targets.join(", ")
                ),
            );
        }

        for target in &clause.targets {
            let Some(param) = signature
                .parameters
                .iter()
                .find(|param| param.name == *target)
            else {
                self.emit_error(
                    codes::LENDS_UNKNOWN_TARGET,
                    clause.span,
                    format!("`lends` references unknown parameter `{target}` in `{function_name}`"),
                );
                continue;
            };

            if !matches!(param.binding, BindingModifier::In | BindingModifier::Ref) {
                let binding = match param.binding {
                    BindingModifier::In => "in",
                    BindingModifier::Ref => "ref",
                    BindingModifier::Out => "out",
                    BindingModifier::Value => "value",
                };
                self.emit_error(
                    codes::LENDS_TARGET_NOT_BORROWED,
                    clause.span,
                    format!(
                        "`lends({target})` requires `{target}` to be an `in` or `ref` parameter, found `{binding}` in `{function_name}`"
                    ),
                );
            }

            if !param.ty.is_view {
                self.emit_error(
                    codes::LENDS_TARGET_NOT_VIEW,
                    clause.span,
                    format!(
                        "`lends({target})` requires `{target}` to be declared as a `view` type in `{function_name}`"
                    ),
                );
            }
        }
    }
}
