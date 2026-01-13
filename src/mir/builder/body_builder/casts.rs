use super::*;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::CastKind;
use crate::mir::Ty;
use crate::mir::casts::{
    float_info, int_cast_may_truncate, int_info, is_pointer_type, short_type_name,
};
use crate::mir::layout::table::TypeLayout;
use crate::mir::operators::ConversionResolution;
use crate::syntax::expr::CastSyntax;
use std::collections::{HashSet, VecDeque};

body_builder_impl! {
    pub(super) fn lower_cast_expr(
        &mut self,
        expr: ExprNode,
        target_text: String,
        syntax: CastSyntax,
        span: Option<Span>,
            ) -> Option<Operand> {
        if std::env::var("CHIC_DEBUG_DELEGATE_SIG").is_ok() && target_text.contains("Converter") {
            eprintln!("[cast] lowering cast to {target_text}");
        }
        let operand = self.lower_expr_node(expr, span)?;
        let (source_ty, source_name) = if let Some(ty) = self.operand_ty(&operand) {
            let name = match &ty {
                Ty::Named(named) => named.as_str().to_string(),
                other => other.canonical_name(),
            };
            (ty, name)
        } else {
            let local = self.ensure_operand_local(operand.clone(), span);
            let Some(decl) = self.locals.get(local.0) else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "cannot determine the source type for cast".into(),
                    span,
                });
                return None;
            };
            let ty = decl.ty.clone();
            let name = match &ty {
                Ty::Named(named) => named.as_str().to_string(),
                other => other.canonical_name(),
            };
            (ty, name)
        };

        let Some(target_expr) = parse_type_expression_text(&target_text) else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("`{target_text}` is not a valid cast target type"),
                span,
                            });
            return None;
        };

        let target_name = target_expr.name.clone();
        let target_ty = Ty::from_type_expr(&target_expr);
        self.ensure_ty_layout_for_ty(&target_ty);

        let span_pair = self.span_conversion_pair_exists(&source_ty, &target_ty);
        if span_pair {
            if let Some(converted) =
                self.try_span_conversion(operand.clone(), &target_ty, true, span)
            {
                return Some(converted);
            }
        }

        if short_type_name(&source_name) == short_type_name(&target_name) {
            return Some(operand);
        }

        if let (Ty::Fn(fn_ty), Ty::Named(_)) = (&source_ty, &target_ty) {
            // If the target resolves to a delegate type and the source is a function,
            // force a delegate conversion even if we cannot resolve the delegate signature here.
            let delegate_name = target_ty.canonical_name();
            let coerced = self.coerce_operand_to_delegate(
                operand.clone(),
                &delegate_name,
                fn_ty,
                span,
                Some(target_ty.clone()),
            );
            return Some(coerced);
        }

        if let Ty::Named(_) = &target_ty {
            if let Some((delegate_name, signature)) = self
                .delegate_signature_for_ty(&target_ty)
                .or_else(|| {
                    self.symbol_index
                        .delegate_signature(&target_expr.name)
                        .map(|sig| {
                            (
                                target_ty.canonical_name(),
                                self.instantiate_delegate_signature_from_ty(&target_ty, sig),
                            )
                        })
                })
            {
                let coerced = self.coerce_operand_to_delegate(
                    operand,
                    &delegate_name,
                    &signature,
                    span,
                    Some(target_ty.clone()),
                );
                return Some(coerced);
            }
        }

        if !span_pair {
            match self
                .operator_registry
                .resolve_conversion(&source_name, &target_name, true)
            {
                ConversionResolution::Found(overload) => {
                    return self
                        .emit_operator_call(overload.clone(), vec![operand], span);
                }
                ConversionResolution::Ambiguous(candidates) => {
                    let names = candidates
                        .iter()
                        .map(|candidate| candidate.function.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "ambiguous explicit conversion from `{source_name}` to `{target_name}`; \
                             candidates: {names}"
                        ),
                        span,
                                        });
                    return None;
                }
                ConversionResolution::None { .. } => {}
            }
        }

        if self.is_class_upcast(&source_ty, &target_ty) {
            return Some(self.emit_cast_rvalue(
                CastKind::Unknown,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if self.is_class_downcast(&source_ty, &target_ty) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "downcasting from `{source_name}` to `{target_name}` is not supported; \
consider pattern matching or a dedicated conversion helper"
                ),
                span,
            });
            return Some(self.emit_cast_rvalue(
                CastKind::Unknown,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if self.enum_has_payload(&source_ty) && !self.enum_numeric_info(&target_ty).is_some() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "enum `{source_name}` carries payload data and cannot be cast directly; \
match on the variants instead"
                ),
                span,
            });
        } else if self.enum_has_payload(&target_ty) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "enum `{target_name}` carries payload data and cannot be constructed from a cast"
                ),
                span,
            });
        }

        let source_int = self.int_metadata(&source_ty, &source_name);
        let target_int = self.int_metadata(&target_ty, &target_name);
        let source_is_enum = self.enum_numeric_info(&source_ty).is_some();
        let target_is_enum = self.enum_numeric_info(&target_ty).is_some();

        if let (Some(source_int), Some(target_int)) = (source_int, target_int) {
            self.maybe_emit_numeric_cast_warning(
                source_int,
                target_int,
                &source_name,
                &target_name,
                syntax,
                span,
                source_is_enum || target_is_enum,
            );
            return Some(self.emit_cast_rvalue(
                CastKind::IntToInt,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if let (Some(_source_int), Some(_target_float)) =
            (
                self.int_metadata(&source_ty, &source_name),
                float_info(self.primitive_registry, &target_name),
            )
        {
            if self.unchecked_depth == 0 {
                self.warn_lossy_cast(
                    &source_name,
                    &target_name,
                    "may lose precision when converting to float",
                    syntax,
                    span,
                );
            }
            return Some(self.emit_cast_rvalue(
                CastKind::IntToFloat,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if let (Some(_source_float), Some(_target_int)) = (
            float_info(self.primitive_registry, &source_name),
            self.int_metadata(&target_ty, &target_name),
        )
        {
            if self.unchecked_depth == 0 {
                self.warn_lossy_cast(
                    &source_name,
                    &target_name,
                    "may lose fractional information when converting to integer",
                    syntax,
                    span,
                );
            }
            return Some(self.emit_cast_rvalue(
                CastKind::FloatToInt,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if let (Some(source_float), Some(target_float)) = (
            float_info(self.primitive_registry, &source_name),
            float_info(self.primitive_registry, &target_name),
        )
        {
            if source_float.bits > target_float.bits {
                if self.unchecked_depth == 0 {
                    self.warn_lossy_cast(
                        &source_name,
                        &target_name,
                        "reduces floating-point precision",
                        syntax,
                        span,
                    );
                }
            } else if matches!(syntax, CastSyntax::As) && !source_is_enum && !target_is_enum {
                self.warn_infallible_cast(&source_name, &target_name, span);
            }
            return Some(self.emit_cast_rvalue(
                CastKind::FloatToFloat,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if let Ty::Nullable(inner) = &target_ty
            && let Ty::Pointer(target_ptr) = inner.as_ref()
        {
            let is_pointer_source = matches!(&source_ty, Ty::Pointer(_))
                || matches!(&source_ty, Ty::Nullable(inner) if matches!(inner.as_ref(), Ty::Pointer(_)));
            if is_pointer_source {
                let coerced =
                    self.coerce_pointer_operand(operand.clone(), target_ptr, true, span);
                let temp = self.create_temp(span);
                if let Some(local) = self.locals.get_mut(temp.0) {
                    local.ty = target_ty.clone();
                    local.is_nullable = true;
                }
                self.push_statement(MirStatement {
                    span,
                    kind: MirStatementKind::Assign {
                        place: Place::new(temp),
                        value: Rvalue::Use(coerced),
                    },
                });
                return Some(Operand::Copy(Place::new(temp)));
            }
        }

        if let Ty::Pointer(target_ptr) = &target_ty {
            let is_pointer_source = matches!(&source_ty, Ty::Pointer(_))
                || matches!(&source_ty, Ty::Nullable(inner) if matches!(inner.as_ref(), Ty::Pointer(_)));
            if is_pointer_source {
                let coerced =
                    self.coerce_pointer_operand(operand.clone(), target_ptr, true, span);
                return Some(coerced);
            }
        }

        if is_pointer_type(&source_name) && self.int_metadata(&target_ty, &target_name).is_some() {
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires an `unsafe` block"
                    ),
                    span,
                });
            }
            if !Self::pointer_has_expose_address(&source_ty) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires the pointer type to include `@expose_address`"
                    ),
                    span,
                });
            }
            self.warn_pointer_cast(&source_name, &target_name, syntax, span);
            return Some(self.emit_cast_rvalue(
                CastKind::PointerToInt,
                operand,
                source_ty,
                target_ty,
                span,
                            ));
        }

        if self.int_metadata(&source_ty, &source_name).is_some() && is_pointer_type(&target_name) {
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires an `unsafe` block"
                    ),
                    span,
                });
            }
            if !Self::pointer_has_expose_address(&target_ty) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires the target pointer type to include `@expose_address`"
                    ),
                    span,
                });
            }
            self.warn_pointer_cast(&source_name, &target_name, syntax, span);
            return Some(self.emit_cast_rvalue(
                CastKind::IntToPointer,
                operand,
                source_ty,
                target_ty,
                span,
                            ));
        }

        let is_fn_ptr = |ty: &Ty| match ty {
            Ty::Fn(_) => true,
            Ty::Nullable(inner) => matches!(inner.as_ref(), Ty::Fn(_)),
            _ => false,
        };
        let is_pointer_like = |ty: &Ty| match ty {
            Ty::Pointer(_) => true,
            Ty::Nullable(inner) => matches!(inner.as_ref(), Ty::Pointer(_)),
            _ => false,
        };

        if is_fn_ptr(&target_ty) && is_pointer_like(&source_ty) {
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires an `unsafe` block"
                    ),
                    span,
                });
            }
            if !Self::pointer_has_expose_address(&source_ty) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires the pointer type to include `@expose_address`"
                    ),
                    span,
                });
            }
            self.warn_pointer_cast(&source_name, &target_name, syntax, span);
            return Some(self.emit_cast_rvalue(
                CastKind::Unknown,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        if is_fn_ptr(&source_ty) && is_pointer_like(&target_ty) {
            if self.unsafe_depth == 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires an `unsafe` block"
                    ),
                    span,
                });
            }
            if !Self::pointer_has_expose_address(&target_ty) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "cast from `{source_name}` to `{target_name}` requires the target pointer type to include `@expose_address`"
                    ),
                    span,
                });
            }
            self.warn_pointer_cast(&source_name, &target_name, syntax, span);
            return Some(self.emit_cast_rvalue(
                CastKind::Unknown,
                operand,
                source_ty,
                target_ty,
                span,
            ));
        }

        // If we reach this point, treat the cast as an opaque bitcast to keep lowering moving.
        let source_unknown = matches!(source_ty, Ty::Unknown) || source_name.contains("<unknown>");
        let target_unknown = matches!(target_ty, Ty::Unknown) || target_name.contains("<unknown>");
        if source_unknown || target_unknown {
            // If either side is unknown, skip emitting a cast to avoid spurious errors.
            return Some(operand);
        }
        let message = match syntax {
            CastSyntax::As => Some(format!(
                "no explicit conversion from `{source_name}` to `{target_name}` is defined"
            )),
            CastSyntax::Paren => Some(format!(
                "no C-style cast from `{source_name}` to `{target_name}` is defined"
            )),
        };
        if let Some(message) = message {
            self.diagnostics.push(LoweringDiagnostic { message, span });
        }
        Some(self.emit_cast_rvalue(
            CastKind::Unknown,
            operand,
            source_ty,
            target_ty,
            span,
                        ))
    }

    fn emit_cast_rvalue(
        &mut self,
        kind: CastKind,
        operand: Operand,
        source_ty: Ty,
        target_ty: Ty,
        span: Option<Span>,
            ) -> Operand {
        if std::env::var("CHIC_DEBUG_DELEGATE_SIG")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            eprintln!(
                "[cast] emit_cast_rvalue source={} target={}",
                source_ty.canonical_name(),
                target_ty.canonical_name()
            );
        }
        if let Ty::Named(named) = &target_ty {
            let signature = match &source_ty {
                Ty::Fn(fn_ty) => Some(fn_ty.clone()),
                _ => self
                    .delegate_signature_for_ty(&target_ty)
                    .map(|(_, sig)| sig)
                    .or_else(|| self.delegate_signature_for_ty(&source_ty).map(|(_, sig)| sig))
                    .or_else(|| {
                        self.symbol_index
                            .delegate_signature(named.name.as_str())
                            .map(|sig| self.instantiate_delegate_signature_from_ty(&target_ty, sig))
                    }),
            };
            if let Some(signature) = signature {
                let delegate_name = target_ty.canonical_name();
                return self.coerce_operand_to_delegate(
                    operand,
                    &delegate_name,
                    &signature,
                    span,
                    Some(target_ty),
                );
            }
        }
        let temp = self.create_temp(span);
        if let Some(local) = self.locals.get_mut(temp.0) {
            local.ty = target_ty.clone();
            local.is_nullable = matches!(target_ty, Ty::Nullable(_));
        }
        self.push_statement(MirStatement {
            span,
                        kind: MirStatementKind::Assign {
                place: Place::new(temp),
                value: Rvalue::Cast {
                    kind,
                    operand,
                    source: source_ty,
                    target: target_ty,
                    rounding: None,
                },
            },
        });
        Operand::Copy(Place::new(temp))
    }

    fn maybe_emit_numeric_cast_warning(
        &mut self,
        source: crate::mir::casts::IntInfo,
        target: crate::mir::casts::IntInfo,
        source_name: &str,
        target_name: &str,
        syntax: CastSyntax,
        span: Option<Span>,
        skip_infallible: bool,
    ) {
        if self.should_suppress_numeric_cast_diagnostic() {
            return;
        }
        if self.unchecked_depth > 0 {
            return;
        }
        if int_cast_may_truncate(source, target) {
            self.warn_lossy_cast(
                source_name,
                target_name,
                "may truncate or wrap the value",
                syntax,
                span,
                            );
        } else if matches!(syntax, CastSyntax::As) && !skip_infallible {
            self.warn_infallible_cast(source_name, target_name, span);
        }
    }

    fn should_suppress_numeric_cast_diagnostic(&self) -> bool {
        let name = self.function_name.as_str();
        if name.starts_with("Std::")
            || name.starts_with("Std.")
            || name.starts_with("System::")
            || name.starts_with("System.")
            || name.starts_with("std::")
            || name.starts_with("std.")
        {
            return true;
        }
        self.namespace.as_deref().is_some_and(|ns| {
            let normalized = ns.replace("::", ".");
            normalized.starts_with("Std.")
                || normalized.starts_with("System.")
                || normalized.starts_with("std.")
        })
    }

    fn warn_lossy_cast(
        &mut self,
        source_name: &str,
        target_name: &str,
        reason: &str,
        syntax: CastSyntax,
        span: Option<Span>,
            ) {
        if self.should_suppress_numeric_cast_diagnostic() {
            return;
        }
        let prefix = match syntax {
            CastSyntax::As => "explicit conversion",
            CastSyntax::Paren => "C-style cast",
        };
        let message = format!(
            "warning: {prefix} from `{source_name}` to `{target_name}` {reason}"
        );
        self.diagnostics.push(LoweringDiagnostic { message, span });
    }

    fn warn_infallible_cast(&mut self, source_name: &str, target_name: &str, span: Option<Span>) {
        if self.should_suppress_numeric_cast_diagnostic() {
            return;
        }
        let message = format!(
            "warning: explicit conversion from `{source_name}` to `{target_name}` is infallible; prefer `From`/`Into`"
        );
        self.diagnostics.push(LoweringDiagnostic { message, span });
    }

    fn warn_pointer_cast(
        &mut self,
        source_name: &str,
        target_name: &str,
        syntax: CastSyntax,
        span: Option<Span>,
    ) {
        if self.should_suppress_pointer_cast_diagnostic() {
            return;
        }
        let verb = match syntax {
            CastSyntax::As => "pointer cast using `as`",
            CastSyntax::Paren => "C-style pointer cast",
        };
        let message = format!(
            "warning: {verb} from `{source_name}` to `{target_name}` may be unsafe; prefer dedicated pointer APIs"
        );
        self.diagnostics.push(LoweringDiagnostic { message, span });
    }

    fn int_metadata(&self, ty: &Ty, name: &str) -> Option<crate::mir::casts::IntInfo> {
        let ptr_size = pointer_size() as u32;
        if let Some(info) = int_info(self.primitive_registry, short_type_name(name), ptr_size) {
            return Some(info);
        }
        if let Some(info) = int_info(self.primitive_registry, name, ptr_size) {
            return Some(info);
        }
        match ty {
            Ty::Nullable(inner) => self.int_metadata(inner, &inner.canonical_name()),
            _ => self.enum_numeric_info(ty).map(|(info, _)| info),
        }
    }

    fn enum_has_payload(&self, ty: &Ty) -> bool {
        self.enum_numeric_info(ty)
            .is_none()
            && self
                .enum_layout_for_ty(ty)
                .map(|layout| {
                    layout
                        .variants
                        .iter()
                        .any(|variant| !variant.fields.is_empty())
                })
                .unwrap_or(false)
    }

    fn enum_layout_for_ty(&self, ty: &Ty) -> Option<&crate::mir::layout::table::EnumLayout> {
        let name = self.non_nullable_name(ty);
        let layout = self.type_layouts.layout_for_name(name.as_str())?;
        match layout {
            TypeLayout::Enum(layout) => Some(layout),
            _ => None,
        }
    }

    fn should_suppress_pointer_cast_diagnostic(&self) -> bool {
        self.should_suppress_numeric_cast_diagnostic()
    }

    fn enum_numeric_info(
        &self,
        ty: &Ty,
    ) -> Option<(crate::mir::casts::IntInfo, bool)> {
        let layout = self.enum_layout_for_ty(ty)?;
        if layout
            .variants
            .iter()
            .any(|variant| !variant.fields.is_empty())
        {
            return None;
        }
        let info = layout.underlying_info.unwrap_or_else(|| {
            let size = layout.size.unwrap_or(4);
            let bits = (size.saturating_mul(8)).min(u16::MAX as usize) as u16;
            crate::mir::casts::IntInfo {
                bits,
                signed: true,
            }
        });
        Some((info, layout.is_flags))
    }

    fn non_nullable_name(&self, ty: &Ty) -> String {
        match ty {
            Ty::Nullable(inner) => inner.canonical_name().trim_end_matches('?').to_string(),
            other => other.canonical_name(),
        }
    }

    pub(super) fn is_class_upcast(&self, source: &Ty, target: &Ty) -> bool {
        let source_name = self.non_nullable_name(source);
        let target_name = self.non_nullable_name(target);
        if source_name == target_name {
            return false;
        }
        if !self.is_class_type(&source_name) || !self.is_class_type(&target_name) {
            return false;
        }
        self.class_hierarchy_from(&source_name)
            .iter()
            .any(|candidate| self.type_names_equivalent_str(candidate, &target_name))
    }

    pub(super) fn is_class_downcast(&self, source: &Ty, target: &Ty) -> bool {
        let source_name = self.non_nullable_name(source);
        let target_name = self.non_nullable_name(target);
        if source_name == target_name {
            return false;
        }
        if !self.is_class_type(&source_name) || !self.is_class_type(&target_name) {
            return false;
        }
        self.class_hierarchy_from(&target_name)
            .iter()
            .any(|candidate| self.type_names_equivalent_str(candidate, &source_name))
    }

    fn is_class_type(&self, name: &str) -> bool {
        let Some(layout) = self.type_layouts.layout_for_name(name) else {
            return false;
        };
        matches!(layout, TypeLayout::Class(_))
    }

    fn type_names_equivalent_str(&self, a: &str, b: &str) -> bool {
        let a_short = short_type_name(a);
        let b_short = short_type_name(b);
        a_short == b_short
            || a.trim_end_matches('?') == b.trim_end_matches('?')
            || a == b
    }

    fn class_hierarchy_from(&self, root: &str) -> Vec<String> {
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        queue.push_back(root.to_string());
        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            order.push(current.clone());
            let mut bases = Vec::new();
            if let Some(known) = self.class_bases.get(&current) {
                bases.extend_from_slice(known);
            }
            if let Some(layout) = self.type_layouts.class_layout_info(current.as_str()) {
                bases.extend(layout.bases);
            }
            for base in bases {
                queue.push_back(base);
            }
        }
        order
    }
}
