//! Enum layout registration logic.

use std::collections::HashMap;

use super::super::super::{
    EnumDecl, EnumLayout, EnumVariantLayout, MIN_ALIGN, PositionalElement, TypeLayout, TypeRepr,
    align_to, qualify,
};
use super::super::driver::{
    LoweringDiagnostic, ModuleLowering, expect_u32_index, expr_path_segments, is_power_of_two,
};
use super::auto_traits;
use crate::frontend::ast::{EnumVariant, Expression};
use crate::frontend::attributes::{
    collect_layout_hints, extract_global_allocator, has_fallible_attr,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::builder::pointer_size;
use crate::mir::casts::{IntInfo, int_info};
use crate::mir::data::{BinOp, ConstValue, Ty, UnOp};
use crate::syntax::expr::{ExprNode, SizeOfOperand};
use crate::type_metadata::TypeFlags;

impl ModuleLowering {
    // --- layout::enums (planned extraction) ---
    // Depends on super::driver (discriminant helpers), expression parsing utilities, and shared field layout helpers.
    pub(crate) fn register_enum_layout(&mut self, enm: &EnumDecl, namespace: Option<&str>) {
        let name = qualify(namespace, &enm.name);
        if self.type_layouts.types.contains_key(&name) {
            return;
        }

        self.record_type_visibility(&name, enm.visibility, namespace, None);

        let (allocator_attr, errors) = extract_global_allocator(&enm.attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator_attr {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`@global_allocator` is only supported on struct or class declarations"
                    .to_string(),
                span: attr.span,
            });
        }

        let (layout_hints, errors) = collect_layout_hints(&enm.attributes);
        for error in errors {
            self.diagnostics.push(LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }

        let packing_limit = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1) as usize);
        let layout_packing = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1));

        let mut underlying_ty = enm
            .underlying_type
            .as_ref()
            .map(|expr| self.ty_from_type_expr(expr, namespace, Some(name.as_str())))
            .unwrap_or_else(|| Ty::named("int"));
        let underlying_display = underlying_ty.canonical_name();
        let explicit_underlying = enm.underlying_type.is_some();
        let ptr_size = pointer_size() as u32;
        let mut underlying_info =
            self.enum_underlying_info(&underlying_ty, &name, &underlying_display, ptr_size);
        if underlying_info.is_none() {
            underlying_ty = Ty::named("int");
            underlying_info = int_info(&self.primitive_registry, "int", ptr_size);
        }
        self.ensure_ty_layout(&underlying_ty);
        let effective_underlying = underlying_ty.canonical_name();
        let underlying_range = underlying_info.and_then(range_for_int_info);

        let mut variants = Vec::with_capacity(enm.variants.len());
        let mut max_variant_size = 0usize;
        let mut max_variant_align = MIN_ALIGN;
        let mut known_variant = false;
        let mut value_lookup: HashMap<String, i128> = HashMap::new();
        let mut last_value: Option<i128> = None;
        let mut known_flag_bits: u128 = 0;
        let mut next_flag_bit: u32 = 0;
        let mut assigned_values: HashMap<i128, String> = HashMap::new();

        for (index, variant) in enm.variants.iter().enumerate() {
            let discriminant = self.compute_enum_variant_value(
                &name,
                variant,
                index,
                enm.is_flags,
                underlying_range.as_ref(),
                &effective_underlying,
                &value_lookup,
                &mut last_value,
                &mut known_flag_bits,
                &mut next_flag_bit,
                &mut assigned_values,
            );

            self.insert_enum_value_aliases(
                &mut value_lookup,
                namespace,
                &enm.name,
                &name,
                &variant.name,
                discriminant,
            );

            let (fields, size, align) = self.compute_field_layouts(
                &variant.fields,
                namespace,
                Some(name.as_str()),
                packing_limit,
                0,
                0,
            );
            if let Some(size) = size {
                max_variant_size = max_variant_size.max(size);
                known_variant = true;
            }
            if let Some(align) = align {
                max_variant_align = max_variant_align.max(align);
            }
            let positional = fields
                .iter()
                .map(|field| PositionalElement {
                    field_index: field.index,
                    name: Some(field.name.clone()),
                    span: field.span,
                })
                .collect();
            let index_u32 = expect_u32_index(index, "enum variant index");
            variants.push(EnumVariantLayout {
                name: variant.name.clone(),
                index: index_u32,
                discriminant,
                fields,
                positional,
            });
        }

        let (discr_size, discr_align) = self
            .type_layouts
            .size_and_align_for_ty(&underlying_ty)
            .unwrap_or((4usize, 4usize));
        let mut final_align = max_variant_align.max(discr_align);
        if let Some(pack_limit) = packing_limit {
            final_align = final_align.min(pack_limit);
        }

        let mut size = if known_variant {
            Some(align_to(discr_size + max_variant_size, final_align))
        } else {
            Some(discr_size)
        };
        let mut align = Some(final_align);

        if let Some(align_hint) = layout_hints.align {
            let mut requested = align_hint.value as usize;
            if let Some(pack_limit) = packing_limit {
                if requested > pack_limit {
                    let pack_display = layout_packing.unwrap_or(1);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`@align({requested})` exceeds the `@repr(packed({pack_display}))` limit"),
                        span: align_hint.span.or(layout_hints.packing.and_then(|hint| hint.span)),
                                            });
                    requested = pack_limit;
                }
            }
            align = Some(align.map_or(requested, |current| current.max(requested)));
        }

        if let Some(alignment) = align {
            if let Some(current_size) = size {
                size = Some(align_to(current_size, alignment));
            }
        }

        let overrides = auto_traits::enum_overrides(enm);

        let layout = EnumLayout {
            name: name.clone(),
            repr: if layout_hints.repr_c {
                TypeRepr::C
            } else {
                TypeRepr::Default
            },
            packing: layout_packing,
            underlying: underlying_ty,
            underlying_info,
            explicit_underlying,
            variants,
            size,
            align,
            auto_traits: auto_traits::unknown_set(),
            overrides,
            is_flags: enm.is_flags,
        };
        self.type_layouts
            .types
            .insert(name.clone(), TypeLayout::Enum(layout));
        if has_fallible_attr(&enm.attributes) {
            self.type_layouts.add_type_flags(name, TypeFlags::FALLIBLE);
        }
    }

    fn insert_enum_value_aliases(
        &self,
        values: &mut HashMap<String, i128>,
        namespace: Option<&str>,
        simple_enum_name: &str,
        qualified_enum_name: &str,
        variant_name: &str,
        value: i128,
    ) {
        values.insert(variant_name.to_string(), value);
        values.insert(format!("{}::{}", simple_enum_name, variant_name), value);
        values.insert(format!("{}.{variant_name}", simple_enum_name), value);
        let qualified_dot = qualified_enum_name.replace("::", ".");
        values.insert(format!("{}::{}", qualified_enum_name, variant_name), value);
        values.insert(format!("{qualified_dot}.{variant_name}"), value);
        if let Some(ns) = namespace {
            let dotted_ns = ns.replace("::", ".");
            values.insert(
                format!("{}::{}::{}", ns, simple_enum_name, variant_name),
                value,
            );
            values.insert(
                format!("{dotted_ns}.{simple_enum_name}.{variant_name}"),
                value,
            );
        }
    }

    fn compute_enum_variant_value(
        &mut self,
        qualified_enum_name: &str,
        variant: &EnumVariant,
        index: usize,
        is_flags: bool,
        underlying: Option<&UnderlyingRange>,
        underlying_display: &str,
        value_lookup: &HashMap<String, i128>,
        last_value: &mut Option<i128>,
        known_flag_bits: &mut u128,
        next_flag_bit: &mut u32,
        assigned_values: &mut HashMap<i128, String>,
    ) -> i128 {
        if is_flags && !variant.fields.is_empty() {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "flags enum `{qualified_enum_name}` variant `{}` may not declare payload fields",
                    variant.name
                ),
                span: None,
            });
        }

        let span = variant.discriminant.as_ref().and_then(|expr| expr.span);
        let value = if variant.fields.is_empty() {
            if let Some(expr) = &variant.discriminant {
                match self.evaluate_enum_expression(
                    qualified_enum_name,
                    &variant.name,
                    expr,
                    value_lookup,
                ) {
                    Some(v) => v,
                    None => index as i128,
                }
            } else if is_flags {
                if index == 0 {
                    0
                } else {
                    let mut bit = *next_flag_bit;
                    loop {
                        if let Some(range) = underlying {
                            if bit >= u32::from(range.info.bits) {
                                self.diagnostics.push(LoweringDiagnostic {
                                    message: format!(
                                        "auto-assigned flag value for `{qualified_enum_name}` variant `{}` exceeds underlying type `{underlying_display}`",
                                        variant.name
                                    ),
                                    span,
                                });
                                *next_flag_bit = bit + 1;
                                break 0;
                            }
                        }
                        if bit >= i128::BITS {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "auto-assigned flag value for `{qualified_enum_name}` variant `{}` exceeds i128 bit width",
                                    variant.name
                                ),
                                span: None,
                            });
                            break 0;
                        }
                        let mask = 1u128 << bit;
                        if (*known_flag_bits & mask) == 0 {
                            let candidate = match (1i128).checked_shl(bit) {
                                Some(val) => val,
                                None => {
                                    self.diagnostics.push(LoweringDiagnostic {
                                        message: format!(
                                            "auto-assigned flag value for `{qualified_enum_name}` variant `{}` exceeds i128 range",
                                            variant.name
                                        ),
                                        span: None,
                                    });
                                    0
                                }
                            };
                            *next_flag_bit = bit + 1;
                            break candidate;
                        }
                        bit += 1;
                    }
                }
            } else {
                if let Some(prev) = *last_value {
                    match prev.checked_add(1) {
                        Some(next) => {
                            if let Some(range) = underlying {
                                if next > range.max {
                                    self.diagnostics.push(LoweringDiagnostic {
                                        message: format!(
                                            "auto-assigned discriminant for enum `{qualified_enum_name}` exceeds underlying type `{underlying_display}`"
                                        ),
                                        span,
                                    });
                                }
                            }
                            next
                        }
                        None => {
                            self.diagnostics.push(LoweringDiagnostic {
                                message: format!(
                                    "auto-assigned discriminant for enum `{qualified_enum_name}` overflowed i128 range"
                                ),
                                span,
                            });
                            prev
                        }
                    }
                } else {
                    0
                }
            }
        } else {
            if variant.discriminant.is_some() {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "enum `{qualified_enum_name}` variant `{}` with payload cannot declare an explicit discriminant",
                        variant.name
                    ),
                    span,
                });
            }
            index as i128
        };

        if is_flags && variant.fields.is_empty() {
            self.apply_flag_rules(
                qualified_enum_name,
                &variant.name,
                span,
                value,
                underlying.map(|range| range.info.bits),
                known_flag_bits,
                next_flag_bit,
            );
        } else if !is_flags {
            *last_value = Some(value);
        }

        self.validate_enum_value_range(
            qualified_enum_name,
            &variant.name,
            value,
            underlying,
            underlying_display,
            span,
        );

        if let Some(previous) = assigned_values.get(&value) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "enum `{qualified_enum_name}` variant `{}` reuses discriminant value {value} already assigned to `{previous}`",
                    variant.name
                ),
                span,
                            });
        } else {
            assigned_values.insert(value, variant.name.clone());
        }

        value
    }

    fn eval_const_expr(
        &mut self,
        node: &ExprNode,
        values: &HashMap<String, i128>,
        namespace: Option<&str>,
    ) -> Result<i128, String> {
        match node {
            ExprNode::ArrayLiteral(_) => {
                Err("array literals are not valid enum discriminants".to_string())
            }
            ExprNode::Literal(literal) => match &literal.value {
                ConstValue::Int(value) | ConstValue::Int32(value) => Ok(*value),
                ConstValue::UInt(value) => i128::try_from(*value)
                    .map_err(|_| format!("unsigned literal `{value}` exceeds i128 range")),
                ConstValue::Char(ch) => Ok((*ch) as i128),
                ConstValue::Bool(_) => {
                    Err("boolean literals are not valid enum discriminants".to_string())
                }
                ConstValue::Float(_) => {
                    Err("floating-point literals are not valid enum discriminants".to_string())
                }
                ConstValue::Decimal(_) => {
                    Err("decimal literals are not valid enum discriminants".to_string())
                }
                ConstValue::Str { .. } | ConstValue::RawStr(_) => {
                    Err("string literals are not valid enum discriminants".to_string())
                }
                ConstValue::Struct { .. } => {
                    Err("struct literals are not valid enum discriminants".to_string())
                }
                ConstValue::Symbol(_) => {
                    Err("symbol constants are not valid enum discriminants".to_string())
                }
                ConstValue::Null => {
                    Err("`null` literal is not a valid enum discriminant".to_string())
                }
                ConstValue::Unit => {
                    Err("unit literal is not a valid enum discriminant".to_string())
                }
                ConstValue::Unknown => {
                    Err("unparsed literal cannot be used as an enum discriminant".to_string())
                }
                ConstValue::Enum { discriminant, .. } => Ok(*discriminant),
            },
            ExprNode::Identifier(name) => values
                .get(name)
                .copied()
                .ok_or_else(|| format!("identifier `{name}` is not a known constant in this enum")),
            ExprNode::Member { .. } => {
                let segments = expr_path_segments(node)?;
                let mut candidates = Vec::with_capacity(3);
                if let Some(last) = segments.last() {
                    candidates.push(last.clone());
                }
                candidates.push(segments.join("::"));
                candidates.push(segments.join("."));
                for key in candidates {
                    if let Some(value) = values.get(&key) {
                        return Ok(*value);
                    }
                }
                let display_path = segments.join(".");
                Err(format!(
                    "path `{display_path}` is not a known enum constant"
                ))
            }
            ExprNode::IndexFromEnd(_) | ExprNode::Range(_) => {
                Err("range expressions are not valid enum discriminants".to_string())
            }
            ExprNode::Parenthesized(inner) => self.eval_const_expr(inner, values, namespace),
            ExprNode::Cast { .. } => {
                Err("cast expressions are not valid enum discriminants".to_string())
            }
            ExprNode::Conditional { .. } => {
                Err("conditional expressions are not valid enum discriminants".to_string())
            }
            ExprNode::Switch(_) => {
                Err("switch expressions are not valid enum discriminants".to_string())
            }
            ExprNode::Unary { op, expr, postfix } => {
                if *postfix && matches!(op, UnOp::Increment | UnOp::Decrement) {
                    return Err(
                        "postfix increment and decrement are not valid enum discriminants"
                            .to_string(),
                    );
                }
                let value = self.eval_const_expr(expr, values, namespace)?;
                match op {
                    UnOp::Neg => value
                        .checked_neg()
                        .ok_or_else(|| "negating discriminant overflowed i128 range".to_string()),
                    UnOp::Not | UnOp::BitNot => Ok(!value),
                    UnOp::UnaryPlus => Ok(value),
                    UnOp::Increment | UnOp::Decrement => {
                        Err("increment and decrement are not valid enum discriminants".to_string())
                    }
                    UnOp::Deref | UnOp::AddrOf | UnOp::AddrOfMut => {
                        Err("pointer operators are not valid enum discriminants".to_string())
                    }
                }
            }
            ExprNode::Binary { op, left, right } => {
                let lhs = self.eval_const_expr(left, values, namespace)?;
                let rhs = self.eval_const_expr(right, values, namespace)?;
                match op {
                    BinOp::Add => lhs.checked_add(rhs).ok_or_else(|| {
                        "enum discriminant addition overflowed i128 range".to_string()
                    }),
                    BinOp::Sub => lhs.checked_sub(rhs).ok_or_else(|| {
                        "enum discriminant subtraction overflowed i128 range".to_string()
                    }),
                    BinOp::Mul => lhs.checked_mul(rhs).ok_or_else(|| {
                        "enum discriminant multiplication overflowed i128 range".to_string()
                    }),
                    BinOp::Div => {
                        if rhs == 0 {
                            Err("division by zero in enum discriminant".to_string())
                        } else {
                            lhs.checked_div(rhs).ok_or_else(|| {
                                "enum discriminant division overflowed i128 range".to_string()
                            })
                        }
                    }
                    BinOp::Rem => {
                        if rhs == 0 {
                            Err("remainder by zero in enum discriminant".to_string())
                        } else {
                            lhs.checked_rem(rhs).ok_or_else(|| {
                                "enum discriminant remainder overflowed i128 range".to_string()
                            })
                        }
                    }
                    BinOp::BitAnd => Ok(lhs & rhs),
                    BinOp::BitOr => Ok(lhs | rhs),
                    BinOp::BitXor => Ok(lhs ^ rhs),
                    BinOp::Shl => {
                        if rhs < 0 {
                            return Err("shift amount must be non-negative in enum discriminant"
                                .to_string());
                        }
                        let shift = u32::try_from(rhs)
                            .map_err(|_| "shift amount does not fit in u32".to_string())?;
                        if shift >= i128::BITS {
                            return Err("shift amount exceeds i128 bit-width".to_string());
                        }
                        lhs.checked_shl(shift).ok_or_else(|| {
                            "enum discriminant shift overflowed i128 range".to_string()
                        })
                    }
                    BinOp::Shr => {
                        if rhs < 0 {
                            return Err("shift amount must be non-negative in enum discriminant"
                                .to_string());
                        }
                        let shift = u32::try_from(rhs)
                            .map_err(|_| "shift amount does not fit in u32".to_string())?;
                        if shift >= i128::BITS {
                            return Err("shift amount exceeds i128 bit-width".to_string());
                        }
                        lhs.checked_shr(shift).ok_or_else(|| {
                            "enum discriminant shift overflowed i128 range".to_string()
                        })
                    }
                    _ => {
                        Err("operator is not supported in enum discriminant expressions"
                            .to_string())
                    }
                }
            }
            ExprNode::SizeOf(operand) => {
                return self.eval_const_sizeof(operand, namespace);
            }
            ExprNode::AlignOf(operand) => {
                return self.eval_const_alignof(operand, namespace);
            }
            ExprNode::Default(_) => {
                return Err("`default` literal is not a valid enum discriminant".to_string());
            }
            ExprNode::NameOf(_) => {
                Err("`nameof` is not supported in enum discriminants".to_string())
            }
            ExprNode::IsPattern { .. }
            | ExprNode::Assign { .. }
            | ExprNode::Call { .. }
            | ExprNode::Index { .. }
            | ExprNode::Lambda(_)
            | ExprNode::TryPropagate { .. }
            | ExprNode::Await { .. }
            | ExprNode::Throw { .. }
            | ExprNode::New(_)
            | ExprNode::Tuple(_)
            | ExprNode::InterpolatedString(_)
            | ExprNode::Quote(_)
            | ExprNode::InlineAsm(_)
            | ExprNode::Ref { .. } => Err(
                "expression is not constant and cannot be used as an enum discriminant".to_string(),
            ),
        }
    }

    fn eval_const_sizeof(
        &mut self,
        operand: &SizeOfOperand,
        namespace: Option<&str>,
    ) -> Result<i128, String> {
        match operand {
            SizeOfOperand::Type(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err("`sizeof` requires a type operand".to_string());
                }
                let type_expr = parse_type_expression_text(trimmed)
                    .ok_or_else(|| format!("`{trimmed}` is not a valid type for `sizeof`"))?;
                let ty = self.ty_from_type_expr(&type_expr, namespace, None);
                let (size, _) = self.type_size_and_align(&ty, namespace).ok_or_else(|| {
                    format!("cannot determine size for type `{}`", ty.canonical_name())
                })?;
                i128::try_from(size)
                    .map_err(|_| "computed size does not fit in the i128 enum range".to_string())
            }
            SizeOfOperand::Value(_) => {
                Err("`sizeof` in enum discriminants must reference a type name".to_string())
            }
        }
    }

    fn eval_const_alignof(
        &mut self,
        operand: &SizeOfOperand,
        namespace: Option<&str>,
    ) -> Result<i128, String> {
        match operand {
            SizeOfOperand::Type(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err("`alignof` requires a type operand".to_string());
                }
                let type_expr = parse_type_expression_text(trimmed)
                    .ok_or_else(|| format!("`{trimmed}` is not a valid type for `alignof`"))?;
                let ty = self.ty_from_type_expr(&type_expr, namespace, None);
                let (_, align) = self.type_size_and_align(&ty, namespace).ok_or_else(|| {
                    format!(
                        "cannot determine alignment for type `{}`",
                        ty.canonical_name()
                    )
                })?;
                i128::try_from(align).map_err(|_| {
                    "computed alignment does not fit in the i128 enum range".to_string()
                })
            }
            SizeOfOperand::Value(_) => {
                Err("`alignof` in enum discriminants must reference a type name".to_string())
            }
        }
    }

    fn evaluate_enum_expression(
        &mut self,
        qualified_enum_name: &str,
        variant_name: &str,
        expr: &Expression,
        values: &HashMap<String, i128>,
    ) -> Option<i128> {
        let node = match expr.node.as_ref() {
            Some(node) => node,
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "enum `{qualified_enum_name}` variant `{variant_name}` discriminant is not a constant expression"
                    ),
                    span: expr.span,
                                    });
                return None;
            }
        };

        let namespace = qualified_enum_name.rsplit_once("::").map(|(ns, _)| ns);

        match self.eval_const_expr(node, values, namespace) {
            Ok(value) => Some(value),
            Err(message) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "enum `{qualified_enum_name}` variant `{variant_name}` uses invalid discriminant: {message}"
                    ),
                    span: expr.span,
                                    });
                None
            }
        }
    }

    fn apply_flag_rules(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        span: Option<Span>,
        value: i128,
        allowed_bits: Option<u16>,
        known_bits: &mut u128,
        next_flag_bit: &mut u32,
    ) {
        if value < 0 {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "flags enum `{enum_name}` variant `{variant_name}` must use non-negative discriminants"
                ),
                span,
                            });
            return;
        }

        let value_u128 = value as u128;
        if value_u128 == 0 {
            return;
        }

        if let Some(bits) = allowed_bits {
            let used_bits = 128 - value_u128.leading_zeros();
            if used_bits > u32::from(bits) {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "flags enum `{enum_name}` variant `{variant_name}` does not fit in the declared underlying type (uses bit {used_bits} of {bits})"
                    ),
                    span,
                });
            }
        }

        let new_bits = value_u128 & !*known_bits;
        if new_bits != 0 && !is_power_of_two(new_bits) {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!(
                    "flags enum `{enum_name}` variant `{variant_name}` introduces multiple undefined flag bits"
                ),
                span,
                            });
        }

        *known_bits |= value_u128;

        let highest_bit = 127 - value_u128.leading_zeros() as u32;
        if highest_bit + 1 > *next_flag_bit {
            *next_flag_bit = highest_bit + 1;
        }
    }

    fn validate_enum_value_range(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        value: i128,
        range: Option<&UnderlyingRange>,
        underlying_display: &str,
        span: Option<Span>,
    ) {
        if let Some(range) = range {
            if value < range.min || value > range.max {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "enum `{enum_name}` variant `{variant_name}` value {value} does not fit underlying type `{underlying_display}` (range {}..={})",
                        range.min, range.max
                    ),
                    span,
                });
            }
        }
    }

    fn enum_underlying_info(
        &mut self,
        ty: &Ty,
        enum_name: &str,
        display_name: &str,
        pointer_size: u32,
    ) -> Option<IntInfo> {
        let info =
            int_info(&self.primitive_registry, display_name, pointer_size).or_else(|| match ty {
                Ty::Named(named) => {
                    int_info(&self.primitive_registry, named.as_str(), pointer_size)
                }
                _ => None,
            });
        match info {
            Some(info) if info.bits <= 64 => Some(info),
            Some(info) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "enum `{enum_name}` underlying type `{display_name}` exceeds the supported 64-bit integral limit ({} bits)",
                        info.bits
                    ),
                    span: None,
                });
                None
            }
            None => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "Only integral numeric types may be used as enum underlying types; found `{display_name}` on enum `{enum_name}`"
                    ),
                    span: None,
                });
                None
            }
        }
    }
}

#[derive(Clone, Copy)]
struct UnderlyingRange {
    info: IntInfo,
    min: i128,
    max: i128,
}

fn range_for_int_info(info: IntInfo) -> Option<UnderlyingRange> {
    if info.bits == 0 {
        return None;
    }
    if info.signed {
        let shift = info.bits.saturating_sub(1) as u32;
        let max = (1i128 << shift).saturating_sub(1);
        let min = -(1i128 << shift);
        Some(UnderlyingRange { info, min, max })
    } else {
        if info.bits >= 127 {
            return None;
        }
        let max = (1i128 << info.bits) - 1;
        Some(UnderlyingRange { info, min: 0, max })
    }
}
