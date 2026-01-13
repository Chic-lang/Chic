use super::AutoTraitKind;
use super::arena::{TypeChecker, TypeInfo, TypeKind};
use super::diagnostics::codes;
use super::helpers::{base_type_name, type_names_equivalent};
use crate::frontend::ast::{
    AutoTraitConstraint, ConstParamData, GenericArgument, GenericConstraintKind, GenericParam,
    GenericParamKind, GenericParams, TypeExpr, TypeParamData, Visibility, expressions::Expression,
};
use crate::frontend::diagnostics::Span;
use crate::mir::AutoTraitStatus;
use crate::mir::{ConstEvalContext, ConstValue, Ty};
use crate::syntax::expr::{ExprError, parse_expression};
use std::char;
use std::collections::{HashMap, HashSet};

impl<'a> TypeChecker<'a> {
    pub(super) fn validate_generics(&mut self, owner: &str, generics: Option<&GenericParams>) {
        let Some(params) = generics else {
            return;
        };
        if params.params.is_empty() {
            return;
        }

        let mut declared = Vec::new();
        let mut seen = HashSet::new();
        let all_names: Vec<String> = params
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect();

        for param in &params.params {
            if !seen.insert(param.name.clone()) {
                self.emit_error(
                    codes::DUPLICATE_GENERIC_PARAMETER,
                    param.span,
                    format!(
                        "type parameter `{}` appears multiple times on `{}`",
                        param.name, owner
                    ),
                );
                continue;
            }
            match &param.kind {
                GenericParamKind::Type(data) => {
                    self.validate_type_param_constraints(owner, param, data, &declared, &all_names);
                }
                GenericParamKind::Const(data) => {
                    self.validate_const_param(owner, param, data);
                }
            }
            declared.push(param.name.clone());
        }
    }

    pub(super) fn validate_generic_arguments(
        &mut self,
        usage: &str,
        info: &TypeInfo,
        args: &[GenericArgument],
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        let Some(def) = info.generics.as_ref() else {
            if !args.is_empty() {
                let base = base_type_name(usage);
                if !base.ends_with("Task") {
                    self.emit_error(
                        codes::TYPE_NOT_GENERIC,
                        span,
                        format!("type `{}` is not generic", usage),
                    );
                }
            }
            return;
        };

        let expected = def.params.len();
        if expected != args.len() {
            self.emit_error(
                codes::GENERIC_ARGUMENT_MISMATCH,
                span,
                format!(
                    "type `{}` expects {} type argument{}, but {} {} supplied",
                    usage,
                    expected,
                    if expected == 1 { "" } else { "s" },
                    args.len(),
                    if args.len() == 1 { "was" } else { "were" }
                ),
            );
            return;
        }

        let mut type_arg_map: HashMap<&str, &TypeExpr> = HashMap::new();
        let mut const_values: HashMap<String, ConstValue> = HashMap::new();
        let mut had_error = false;

        for (param, arg) in def.params.iter().zip(args.iter()) {
            match &param.kind {
                GenericParamKind::Type(_) => {
                    let Some(ty_arg) = arg.ty() else {
                        self.emit_error(
                            codes::GENERIC_ARGUMENT_MISMATCH,
                            span,
                            format!(
                                "type `{}` expects a type argument for `{}`, but a const expression was supplied",
                                usage, param.name
                            ),
                        );
                        had_error = true;
                        continue;
                    };
                    type_arg_map.insert(param.name.as_str(), ty_arg);
                    self.ensure_type_expr(ty_arg, None, context_type, span);
                }
                GenericParamKind::Const(data) => {
                    let Some(value) = self.evaluate_const_generic_argument(
                        usage,
                        param,
                        data,
                        arg,
                        arg.expression(),
                        &const_values,
                        span,
                    ) else {
                        had_error = true;
                        continue;
                    };
                    const_values.insert(param.name.clone(), value);
                }
            }
        }

        if had_error {
            return;
        }

        for (param, arg) in def.params.iter().zip(args.iter()) {
            if let (Some(data), Some(ty_arg)) = (param.as_type(), arg.ty()) {
                self.enforce_argument_constraints(
                    usage,
                    param,
                    data,
                    ty_arg,
                    &type_arg_map,
                    context_type,
                    span,
                );
            }
        }

        if Self::is_atomic_type(usage) {
            if let Some(first_arg) = args.get(0)
                && let Some(arg_ty) = first_arg.ty()
                && matches!(
                    def.params.first().map(|p| &p.kind),
                    Some(GenericParamKind::Type(_))
                )
            {
                self.validate_atomic_type_argument(usage, arg_ty, span);
            }
        }

        self.enforce_const_predicates(usage, def, &const_values, span);
    }

    fn validate_type_param_constraints(
        &mut self,
        owner: &str,
        param: &GenericParam,
        data: &TypeParamData,
        declared: &[String],
        all_names: &[String],
    ) {
        let mut has_struct = false;
        let mut has_class = false;
        let mut class_type: Option<String> = None;
        let mut has_new = false;
        let mut thread_safe_seen = false;
        let mut shareable_seen = false;

        for constraint in &data.constraints {
            let span = constraint.span.or(param.span);
            match &constraint.kind {
                GenericConstraintKind::Struct => {
                    if has_struct {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` repeats the `struct` constraint",
                                param.name, owner
                            ),
                        );
                    }
                    if has_class || class_type.is_some() {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                                                        format!("type parameter `{}` on `{}` cannot combine `struct` with reference-type constraints",
                                param.name, owner
                            ),
                        );
                    }
                    has_struct = true;
                }
                GenericConstraintKind::Class => {
                    if has_class {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` repeats the `class` constraint",
                                param.name, owner
                            ),
                        );
                    }
                    if has_struct {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` cannot combine `struct` and `class`",
                                param.name, owner
                            ),
                        );
                    }
                    has_class = true;
                }
                GenericConstraintKind::NotNull => {
                    if has_struct {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "`notnull` constraint on `{}` in `{}` is redundant with `struct`",
                                param.name, owner
                            ),
                        );
                    }
                }
                GenericConstraintKind::DefaultConstructor => {
                    if has_new {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` repeats the `new()` constraint",
                                param.name, owner
                            ),
                        );
                    }
                    if has_struct {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` cannot combine `new()` with `struct`",
                                param.name, owner
                            ),
                        );
                    }
                    has_new = true;
                }
                GenericConstraintKind::Type(ty_expr) => {
                    let target_name = ty_expr.name.as_str();
                    if target_name == param.name {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                            format!(
                                "type parameter `{}` on `{}` cannot reference itself",
                                param.name, owner
                            ),
                        );
                        continue;
                    }

                    if declared.iter().any(|name| name == target_name) {
                        continue;
                    }

                    if all_names.iter().any(|name| name == target_name) {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            span,
                                                        format!("type parameter `{}` on `{}` cannot reference `{}` before it is declared",
                                param.name, owner, target_name
                            ),
                        );
                        continue;
                    }

                    self.ensure_type_expr(ty_expr, None, Some(owner), span);

                    if self.is_builtin_reference_type(target_name) {
                        if has_struct {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                span,
                                                                format!("type parameter `{}` on `{}` cannot require both `struct` and `{}`",
                                    param.name, owner, target_name
                                ),
                            );
                        }
                        if let Some(existing) = &class_type {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                span,
                                                                format!("type parameter `{}` on `{}` specifies multiple base classes (`{}` and `{}`)",
                                    param.name, owner, existing, target_name
                                ),
                            );
                        }
                        if has_class {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                span,
                                                                format!("type parameter `{}` on `{}` cannot combine `class` with explicit base class `{}`",
                                    param.name, owner, target_name
                                ),
                            );
                        }
                        class_type = Some(target_name.to_string());
                        continue;
                    }

                    if let Some(info) = self.resolve_type_info(target_name) {
                        match info.kind {
                            TypeKind::Class { .. } => {
                                if has_struct {
                                    self.emit_error(
                                        codes::GENERIC_CONSTRAINT_VIOLATION,
                                        span,
                                                                                format!("type parameter `{}` on `{}` cannot combine `struct` with base class `{}`",
                                            param.name, owner, target_name
                                        ),
                                    );
                                }
                                if let Some(existing) = &class_type {
                                    self.emit_error(
                                        codes::GENERIC_CONSTRAINT_VIOLATION,
                                        span,
                                                                                format!("type parameter `{}` on `{}` specifies multiple base classes (`{}` and `{}`)",
                                            param.name, owner, existing, target_name
                                        ),
                                    );
                                }
                                if has_class {
                                    self.emit_error(
                                        codes::GENERIC_CONSTRAINT_VIOLATION,
                                        span,
                                                                                format!("type parameter `{}` on `{}` cannot combine `class` with explicit base class `{}`",
                                            param.name, owner, target_name
                                        ),
                                    );
                                }
                                class_type = Some(target_name.to_string());
                            }
                            TypeKind::Struct { .. } | TypeKind::Union | TypeKind::Enum => {
                                if has_class || class_type.is_some() {
                                    self.emit_error(
                                        codes::GENERIC_CONSTRAINT_VIOLATION,
                                        span,
                                                                                format!("type parameter `{}` on `{}` mixes value-type and reference-type constraints",
                                            param.name, owner
                                        ),
                                    );
                                }
                            }
                            TypeKind::Interface { .. }
                            | TypeKind::Trait
                            | TypeKind::Delegate { .. } => {}
                        }
                    }
                }
                GenericConstraintKind::AutoTrait(kind) => match kind {
                    AutoTraitConstraint::ThreadSafe => {
                        if thread_safe_seen {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                span,
                                format!(
                                    "type parameter `{}` on `{}` repeats the `@thread_safe` constraint",
                                    param.name, owner
                                ),
                            );
                        }
                        thread_safe_seen = true;
                    }
                    AutoTraitConstraint::Shareable => {
                        if shareable_seen {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                span,
                                format!(
                                    "type parameter `{}` on `{}` repeats the `@shareable` constraint",
                                    param.name, owner
                                ),
                            );
                        }
                        shareable_seen = true;
                    }
                },
            }
        }
    }

    fn validate_const_param(&mut self, _owner: &str, param: &GenericParam, data: &ConstParamData) {
        self.ensure_type_expr(&data.ty, None, None, param.span);
    }

    fn enforce_argument_constraints(
        &mut self,
        owner: &str,
        param: &GenericParam,
        data: &TypeParamData,
        arg: &TypeExpr,
        arg_map: &HashMap<&str, &TypeExpr>,
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        for constraint in &data.constraints {
            let constraint_span = constraint.span.or(span);
            match &constraint.kind {
                GenericConstraintKind::Struct => {
                    if !self.type_is_value_type(arg, context_type) {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            constraint_span,
                            format!("type argument `{}` for `{}` in `{}` must be a value type due to `struct` constraint",
                                arg.name, param.name, owner
                            ),
                        );
                    }
                }
                GenericConstraintKind::Class => {
                    if !self.type_is_reference_type(arg, context_type) {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            constraint_span,
                            format!("type argument `{}` for `{}` in `{}` must be a reference type due to `class` constraint",
                                arg.name, param.name, owner
                            ),
                        );
                    }
                }
                GenericConstraintKind::NotNull => {
                    if arg.is_nullable() {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            constraint_span,
                            format!("type argument `{}` for `{}` in `{}` cannot be nullable because of `notnull` constraint",
                                arg.name, param.name, owner
                            ),
                        );
                    }
                }
                GenericConstraintKind::DefaultConstructor => {
                    if !self.type_has_public_default_constructor(arg, context_type) {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            constraint_span,
                            format!("type argument `{}` for `{}` in `{}` must provide a public parameterless constructor",
                                arg.name, param.name, owner
                            ),
                        );
                    }
                }
                GenericConstraintKind::Type(required) => {
                    if let Some(message) = self.enforce_type_requirement(
                        owner,
                        param,
                        arg,
                        required,
                        arg_map,
                        context_type,
                    ) {
                        self.emit_error(
                            codes::GENERIC_CONSTRAINT_VIOLATION,
                            constraint_span,
                            message,
                        );
                    }
                }
                GenericConstraintKind::AutoTrait(trait_req) => {
                    self.enforce_auto_trait_requirement(
                        owner,
                        param,
                        arg,
                        *trait_req,
                        context_type,
                        constraint_span,
                    );
                }
            }
        }
    }

    fn enforce_auto_trait_requirement(
        &mut self,
        owner: &str,
        param: &GenericParam,
        arg: &TypeExpr,
        required: AutoTraitConstraint,
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        if self.generic_argument_has_context_auto_trait(context_type, arg, required) {
            return;
        }
        let trait_kind = match required {
            AutoTraitConstraint::ThreadSafe => AutoTraitKind::ThreadSafe,
            AutoTraitConstraint::Shareable => AutoTraitKind::Shareable,
        };
        let ty = Ty::from_type_expr(arg);
        let traits = self.type_layouts.auto_traits_for_type(&ty);
        let status = match trait_kind {
            AutoTraitKind::ThreadSafe => traits.thread_safe,
            AutoTraitKind::Shareable => traits.shareable,
            AutoTraitKind::Copy => traits.copy,
        };
        let message = format!(
            "type argument `{}` for `{}` in `{}` must satisfy `{}`",
            arg.name,
            param.name,
            owner,
            required.attribute_name()
        );
        match status {
            AutoTraitStatus::Yes => {}
            AutoTraitStatus::No => {
                self.emit_error(codes::AUTO_TRAIT_REQUIRED, span, message);
            }
            AutoTraitStatus::Unknown => {
                self.emit_error(codes::AUTO_TRAIT_UNPROVEN, span, message);
            }
        }
    }

    fn generic_argument_has_context_auto_trait(
        &self,
        context_type: Option<&str>,
        arg: &TypeExpr,
        required: AutoTraitConstraint,
    ) -> bool {
        let Some(param) = self.lookup_context_generic_param(context_type, arg.name.as_str()) else {
            return false;
        };
        let Some(data) = param.as_type() else {
            return false;
        };
        data.constraints.iter().any(|constraint| {
            matches!(
                constraint.kind,
                GenericConstraintKind::AutoTrait(kind) if kind == required
            )
        })
    }

    fn evaluate_const_generic_argument(
        &mut self,
        usage: &str,
        param: &GenericParam,
        data: &ConstParamData,
        argument_meta: &GenericArgument,
        argument: &Expression,
        resolved: &HashMap<String, ConstValue>,
        span: Option<Span>,
    ) -> Option<ConstValue> {
        let prepared = match self.prepare_const_expression(argument) {
            Ok(expr) => expr,
            Err(err) => {
                self.emit_error(
                    codes::CONST_EVAL_FAILURE,
                    err.span.or(argument.span).or(span),
                    format!(
                        "const argument `{}` for `{}` is not a valid expression: {}",
                        param.name, usage, err.message
                    ),
                );
                return None;
            }
        };
        let mut layouts = self.type_layouts.clone();
        let mut eval_ctx = ConstEvalContext::new(
            &mut self.symbol_index,
            &mut layouts,
            Some(&self.import_resolver),
        );
        let target_ty = Ty::from_type_expr(&data.ty);
        match eval_ctx.evaluate_expression(
            &prepared,
            None,
            None,
            None,
            Some(resolved),
            &target_ty,
            argument.span.or(span),
        ) {
            Ok(result) => {
                argument_meta.set_evaluated_value(self.format_const_value(&result.value));
                Some(result.value)
            }
            Err(err) => {
                self.emit_error(
                    codes::CONST_EVAL_FAILURE,
                    err.span.or(argument.span).or(span),
                    format!(
                        "failed to evaluate const argument `{}` for `{}`: {}",
                        param.name, usage, err.message
                    ),
                );
                None
            }
        }
    }

    fn enforce_const_predicates(
        &mut self,
        usage: &str,
        generics: &GenericParams,
        values: &HashMap<String, ConstValue>,
        span: Option<Span>,
    ) {
        if values.is_empty() {
            return;
        }
        for param in &generics.params {
            let Some(data) = param.as_const() else {
                continue;
            };
            for predicate in &data.constraints {
                let prepared = match self.prepare_const_expression(&predicate.expr) {
                    Ok(expr) => expr,
                    Err(err) => {
                        self.emit_error(
                            codes::CONST_EVAL_FAILURE,
                            err.span.or(predicate.span).or(span),
                            format!(
                                "const constraint on `{}` for `{}` is not a valid expression: {}",
                                param.name, usage, err.message
                            ),
                        );
                        continue;
                    }
                };
                let mut layouts = self.type_layouts.clone();
                let mut eval_ctx = ConstEvalContext::new(
                    &mut self.symbol_index,
                    &mut layouts,
                    Some(&self.import_resolver),
                );
                let bool_ty = Ty::named("bool");
                match eval_ctx.evaluate_expression(
                    &prepared,
                    None,
                    None,
                    None,
                    Some(values),
                    &bool_ty,
                    predicate.span.or(span),
                ) {
                    Ok(result) => match result.value {
                        ConstValue::Bool(true) => {}
                        ConstValue::Bool(false) => {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                predicate.span.or(span),
                                format!(
                                    "const constraint on `{}` for `{}` evaluated to false",
                                    param.name, usage
                                ),
                            );
                        }
                        other => {
                            self.emit_error(
                                codes::GENERIC_CONSTRAINT_VIOLATION,
                                predicate.span.or(span),
                                format!(
                                    "const constraint on `{}` for `{}` produced `{other:?}` instead of `bool`",
                                    param.name, usage
                                ),
                            );
                        }
                    },
                    Err(err) => {
                        self.emit_error(
                            codes::CONST_EVAL_FAILURE,
                            err.span.or(predicate.span).or(span),
                            format!(
                                "failed to evaluate const constraint on `{}` for `{}`: {}",
                                param.name, usage, err.message
                            ),
                        );
                    }
                }
            }
        }
    }

    fn prepare_const_expression(&self, expr: &Expression) -> Result<Expression, ExprError> {
        if expr.node.is_some() {
            return Ok(expr.clone());
        }
        let parsed = parse_expression(expr.text.as_str())?;
        Ok(Expression::with_node(expr.text.clone(), expr.span, parsed))
    }

    fn format_char_value(&self, value: u16) -> String {
        match char::from_u32(u32::from(value)) {
            Some(scalar) => scalar.escape_default().collect(),
            None => format!("\\u{:04X}", value),
        }
    }

    fn format_const_value(&self, value: &ConstValue) -> String {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => v.to_string(),
            ConstValue::UInt(v) => format!("{v}u"),
            ConstValue::Float(v) => v.display(),
            ConstValue::Decimal(v) => v.into_decimal().to_string(),
            ConstValue::Bool(v) => v.to_string(),
            ConstValue::Char(ch) => format!("'{}'", self.format_char_value(*ch)),
            ConstValue::Str { value, .. } | ConstValue::RawStr(value) => {
                format!("\"{value}\"")
            }
            ConstValue::Symbol(sym) => sym.clone(),
            ConstValue::Enum {
                type_name, variant, ..
            } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short}::{variant}")
            }
            ConstValue::Struct { type_name, .. } => {
                let short = type_name.rsplit("::").next().unwrap_or(type_name);
                format!("{short} {{ .. }}")
            }
            ConstValue::Null => "null".into(),
            ConstValue::Unit => "()".into(),
            ConstValue::Unknown => "<unknown>".into(),
        }
    }

    fn enforce_type_requirement(
        &mut self,
        owner: &str,
        param: &GenericParam,
        arg: &TypeExpr,
        requirement: &TypeExpr,
        arg_map: &HashMap<&str, &TypeExpr>,
        context_type: Option<&str>,
    ) -> Option<String> {
        let arg_rendered = format_type_expr(arg);
        let requirement_rendered = format_type_expr(requirement);
        let arg_name = arg.name.as_str();
        let requirement_name = requirement.name.as_str();

        if self.generic_argument_has_context_type_requirement(context_type, arg_name, requirement) {
            return None;
        }

        // When validating generic constraints during the registry pass, referenced types may not
        // have been registered into `self.types` yet (file-order forward refs). If the symbol
        // index knows about the type, defer emitting a violation until a later phase.
        if (self.resolve_type_info(arg_name).is_none()
            && self.symbol_index_has_short_type(arg_name))
            || (self.resolve_type_info(requirement_name).is_none()
                && self.symbol_index_has_short_type(requirement_name))
        {
            return None;
        }

        if let Some(other_arg) = arg_map.get(requirement_name) {
            let other_name = other_arg.name.as_str();
            if type_names_equivalent(arg_name, other_name) {
                return None;
            }
            if self.type_is_subclass_of_name(arg_name, other_name)
                || self.type_implements_interface_by_name(arg_name, other_name)
            {
                return None;
            }
            return Some(format!(
                "type argument `{}` for `{}` in `{}` must be compatible with `{}`",
                arg.name, param.name, owner, other_arg.name
            ));
        }

        if let Some(other_arg) = arg_map.get(arg_name) {
            let other_name = other_arg.name.as_str();
            if self.type_is_subclass_of_name(other_name, requirement_name)
                || self.type_implements_interface_by_name(other_name, requirement_name)
            {
                return None;
            }
        }

        if self.type_is_subclass_of_name(&arg_rendered, &requirement_rendered)
            || self.type_implements_interface_by_name(&arg_rendered, &requirement_rendered)
        {
            return None;
        }

        if let Some(expr) = arg_map.get(requirement_name) {
            return Some(format!(
                "type argument `{}` for `{}` in `{}` must satisfy constraint `{}`",
                arg.name, param.name, owner, expr.name
            ));
        }

        Some(format!(
            "type argument `{}` for `{}` in `{}` must satisfy constraint `{}`",
            arg.name, param.name, owner, requirement.name
        ))
    }

    fn generic_argument_has_context_type_requirement(
        &self,
        context_type: Option<&str>,
        arg_name: &str,
        requirement: &TypeExpr,
    ) -> bool {
        let Some(param) = self.lookup_context_generic_param(context_type, arg_name) else {
            return false;
        };
        let Some(data) = param.as_type() else {
            return false;
        };
        let requirement_name = requirement.name.as_str();
        data.constraints
            .iter()
            .any(|constraint| match &constraint.kind {
                GenericConstraintKind::Type(bound) => {
                    let bound_name = bound.name.as_str();
                    type_names_equivalent(bound_name, requirement_name)
                        || self.type_is_subclass_of_name(bound_name, requirement_name)
                        || self.type_implements_interface_by_name(bound_name, requirement_name)
                }
                _ => false,
            })
    }

    fn symbol_index_has_short_type(&self, short: &str) -> bool {
        self.symbol_index
            .types()
            .any(|name| name.rsplit("::").next().is_some_and(|tail| tail == short))
    }

    fn summarize_param_constraints(
        &self,
        context_type: Option<&str>,
        name: &str,
    ) -> Option<ConstraintSummary> {
        let param = self.lookup_context_generic_param(context_type, name)?;
        let data = param.as_type()?;
        let mut summary = ConstraintSummary::default();
        for constraint in &data.constraints {
            match &constraint.kind {
                GenericConstraintKind::Struct => {
                    summary.is_struct = true;
                    summary.has_new = true;
                }
                GenericConstraintKind::Class => summary.is_class = true,
                GenericConstraintKind::DefaultConstructor => summary.has_new = true,
                GenericConstraintKind::Type(ty) => {
                    let base = base_type_name(&ty.name);
                    if let Some(info) = self.resolve_type_info(base) {
                        if matches!(info.kind, TypeKind::Class { .. }) {
                            summary.has_class_bound = true;
                        } else if matches!(
                            info.kind,
                            TypeKind::Struct { .. } | TypeKind::Union | TypeKind::Enum
                        ) {
                            summary.is_struct = true;
                        }
                    }
                }
                _ => {}
            }
        }
        Some(summary)
    }

    pub(super) fn type_is_value_type(&self, expr: &TypeExpr, context_type: Option<&str>) -> bool {
        if expr.array_ranks().next().is_some() {
            return true;
        }
        if expr.is_nullable() {
            return false;
        }
        if let Some(summary) = self.summarize_param_constraints(context_type, expr.name.as_str()) {
            if summary.is_struct {
                return true;
            }
            if summary.is_class || summary.has_class_bound {
                return false;
            }
        }
        let base = base_type_name(&expr.name);
        if self.is_builtin_value_type(base) {
            return true;
        }
        if let Some(info) = self.resolve_type_info(base) {
            matches!(
                info.kind,
                TypeKind::Struct { .. } | TypeKind::Union | TypeKind::Enum
            )
        } else {
            false
        }
    }

    pub(super) fn type_is_reference_type(
        &self,
        expr: &TypeExpr,
        context_type: Option<&str>,
    ) -> bool {
        if let Some(summary) = self.summarize_param_constraints(context_type, expr.name.as_str()) {
            if summary.is_class || summary.has_class_bound {
                return true;
            }
            if summary.is_struct {
                return false;
            }
        }
        let base = base_type_name(&expr.name);
        if self.is_builtin_reference_type(base) {
            return true;
        }
        if let Some(info) = self.resolve_type_info(base) {
            matches!(
                info.kind,
                TypeKind::Class { .. } | TypeKind::Interface { .. } | TypeKind::Trait
            )
        } else {
            false
        }
    }

    pub(super) fn type_has_public_default_constructor(
        &self,
        expr: &TypeExpr,
        context_type: Option<&str>,
    ) -> bool {
        if self.type_is_value_type(expr, context_type) {
            return true;
        }
        if let Some(summary) = self.summarize_param_constraints(context_type, expr.name.as_str()) {
            if summary.is_struct || summary.has_new {
                return true;
            }
            return false;
        }
        let base = base_type_name(&expr.name);
        if self.is_builtin_reference_type(base) {
            return true;
        }
        let Some(info) = self.resolve_type_info(base) else {
            return true;
        };
        match &info.kind {
            TypeKind::Class { constructors, .. } | TypeKind::Struct { constructors, .. } => {
                if constructors.is_empty() {
                    return true;
                }
                constructors.iter().any(|ctor| {
                    ctor.param_count == 0 && matches!(ctor.visibility, Visibility::Public)
                })
            }
            _ => false,
        }
    }

    pub(super) fn is_builtin_value_type(&self, name: &str) -> bool {
        matches!(
            base_type_name(name),
            "bool"
                | "int"
                | "long"
                | "float"
                | "double"
                | "usize"
                | "isize"
                | "decimal"
                | "vector"
                | "ChicStr"
                | "Std::Runtime::Native::ChicStr"
                | "Std.Runtime.Native.ChicStr"
                | "ChicCharSpan"
                | "Std::Runtime::Native::ChicCharSpan"
                | "Std.Runtime.Native.ChicCharSpan"
        )
    }

    pub(super) fn is_builtin_reference_type(&self, name: &str) -> bool {
        matches!(
            base_type_name(name),
            "string" | "System::String" | "Std::String" | "str" | "System::Str" | "Std::Str"
        )
    }
}

fn format_type_expr(expr: &TypeExpr) -> String {
    if let Some(args) = expr.generic_arguments() {
        let mut text = base_type_name(&expr.name).to_string();
        text.push('<');
        for (index, arg) in args.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            if let Some(arg_ty) = arg.ty() {
                text.push_str(&format_type_expr(arg_ty));
            } else {
                text.push_str(&arg.expression().text);
            }
        }
        text.push('>');
        return text;
    }

    expr.name.clone()
}

#[derive(Default)]
struct ConstraintSummary {
    is_struct: bool,
    is_class: bool,
    has_new: bool,
    has_class_bound: bool,
}
