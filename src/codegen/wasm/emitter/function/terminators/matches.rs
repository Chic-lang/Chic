use super::super::FunctionEmitter;
use super::super::ops::{Op, emit_instruction};
use crate::codegen::wasm::ValueType;
use crate::error::Error;
use crate::mir::{BlockId, MatchArm, Pattern, Place, Ty, TypeLayout, VariantPatternFields};

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_match(
        &mut self,
        buf: &mut Vec<u8>,
        value: &Place,
        arms: &[MatchArm],
        otherwise: BlockId,
    ) -> Result<(), Error> {
        wasm_debug!(
            "        lowering Match on {:?} with {} arms, otherwise {}",
            value,
            arms.len(),
            otherwise
        );
        if !value.projection.is_empty() {
            return Err(Error::Codegen(
                "WASM backend does not yet support projected match values".into(),
            ));
        }
        let value_ty = self.emit_place_value(buf, value)?;
        if !matches!(value_ty, ValueType::I32) {
            return Err(Error::Codegen(
                "match discriminant must lower to i32 in WASM backend".into(),
            ));
        }
        emit_instruction(buf, Op::LocalSet(self.temp_local));

        let enum_ty = self.local_tys.get(value.local.0).cloned();

        for arm in arms {
            if self.emit_match_arm(buf, arm, enum_ty.as_ref())? {
                return Ok(());
            }
        }

        self.emit_match_default(buf, otherwise);
        Ok(())
    }

    fn emit_match_arm(
        &mut self,
        buf: &mut Vec<u8>,
        arm: &MatchArm,
        enum_ty: Option<&Ty>,
    ) -> Result<bool, Error> {
        wasm_debug!(
            "          arm target {} pattern {:?}",
            arm.target,
            arm.pattern
        );
        if arm.guard.is_some() {
            wasm_debug!("            arm guard detected; guard block will handle predicate");
        }
        if !arm.bindings.is_empty() {
            wasm_debug!("            arm includes {} binding(s)", arm.bindings.len());
        }

        match &arm.pattern {
            Pattern::Wildcard | Pattern::Binding(_) => {
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(1));
                Ok(true)
            }
            Pattern::Struct { .. } | Pattern::Tuple(_) => {
                if Self::pattern_is_irrefutable(&arm.pattern) {
                    self.set_block(buf, arm.target);
                    emit_instruction(buf, Op::Br(1));
                    Ok(true)
                } else {
                    Err(Error::Codegen(
                        "complex destructuring patterns are not yet supported by the WASM backend"
                            .into(),
                    ))
                }
            }
            Pattern::Literal(literal) => {
                let literal_op = Self::const_to_op(literal)?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                emit_instruction(buf, literal_op);
                emit_instruction(buf, Op::I32Eq);
                emit_instruction(buf, Op::If);
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(2));
                emit_instruction(buf, Op::End);
                Ok(false)
            }
            Pattern::Enum {
                path,
                variant,
                fields,
                ..
            } => {
                let layout = enum_ty
                    .and_then(|ty| self.lookup_enum_layout(ty))
                    .or_else(|| {
                        let candidate = path.join("::");
                        self.layouts
                            .layout_for_name(&candidate)
                            .and_then(|layout| match layout {
                                TypeLayout::Enum(data) => Some(data),
                                _ => None,
                            })
                    });
                let Some(layout) = layout else {
                    if std::env::var_os("CHIC_DEBUG_WASM_MATCH").is_some() {
                        eprintln!(
                            "[wasm-match-missing-layout] func={} ty={} path={} variant={}",
                            self.function.name,
                            enum_ty
                                .map(|ty| ty.canonical_name())
                                .unwrap_or_else(|| "<unknown>".into()),
                            path.join("::"),
                            variant
                        );
                    }
                    self.set_block(buf, arm.target);
                    emit_instruction(buf, Op::Br(1));
                    return Ok(true);
                };
                if !matches!(fields, VariantPatternFields::Unit) {
                    return Err(Error::Codegen(
                        "enum patterns with payloads are not yet supported by the WASM backend"
                            .into(),
                    ));
                }
                let variant_layout = layout
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "enum `{}` does not define variant `{variant}`",
                            layout.name
                        ))
                    })?;
                emit_instruction(buf, Op::LocalGet(self.temp_local));
                let literal = Self::convert_switch_value(variant_layout.discriminant)?;
                emit_instruction(buf, Op::I32Const(literal));
                emit_instruction(buf, Op::I32Eq);
                emit_instruction(buf, Op::If);
                self.set_block(buf, arm.target);
                emit_instruction(buf, Op::Br(2));
                emit_instruction(buf, Op::End);
                Ok(false)
            }
        }
    }

    pub(super) fn emit_match_default(&self, buf: &mut Vec<u8>, otherwise: BlockId) {
        wasm_debug!("        match lowering: default branch {}", otherwise);
        self.set_block(buf, otherwise);
        emit_instruction(buf, Op::Br(1));
    }

    fn pattern_is_irrefutable(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Wildcard | Pattern::Binding(_) => true,
            Pattern::Tuple(items) => items.iter().all(Self::pattern_is_irrefutable),
            Pattern::Struct { fields, .. } => fields
                .iter()
                .all(|field| Self::pattern_is_irrefutable(&field.pattern)),
            Pattern::Literal(_) | Pattern::Enum { .. } => false,
        }
    }
}
