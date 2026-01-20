use std::fmt::Write;

use crate::codegen::llvm::emitter::literals::{LLVM_STR_TYPE, LLVM_STRING_TYPE};
use crate::codegen::llvm::signatures::resolve_function_name;
use crate::codegen::llvm::types::{const_repr, infer_const_type, map_type_owned};
use crate::error::Error;
use crate::mir::{ConstOperand, ConstValue, Operand, Place, ProjectionElem, Ty, pointer_align};

use super::super::builder::FunctionEmitter;
use super::value_ref::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_operand(
        &mut self,
        operand: &Operand,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if let Some(ty) = expected {
                    if ty == "double" {
                        if let Some(place_llvm_ty) = self.place_type(place)? {
                            if place_llvm_ty == "float" {
                                let ptr = self.place_ptr(place)?;
                                let align = self.place_alignment(place)?;
                                let load_tmp = self.new_temp();
                                writeln!(
                                    &mut self.builder,
                                    "  {load_tmp} = load float, ptr {ptr}, align {align}"
                                )
                                .ok();
                                let ext_tmp = self.new_temp();
                                writeln!(
                                    &mut self.builder,
                                    "  {ext_tmp} = fpext float {load_tmp} to double"
                                )
                                .ok();
                                return Ok(ValueRef::new(ext_tmp, ty));
                            }
                        }
                    }
                    if let Some(expected_width) = int_width(ty) {
                        if let Some(place_llvm_ty) = self.place_type(place)? {
                            if let Some(place_width) = int_width(&place_llvm_ty)
                                && place_width < expected_width
                            {
                                let mir_ty = self.mir_ty_of_place(place)?;
                                let signed = is_signed_int_ty(&mir_ty);
                                let ptr = self.place_ptr(place)?;
                                let align = self.place_alignment(place)?;
                                let load_tmp = self.new_temp();
                                writeln!(
                                    &mut self.builder,
                                    "  {load_tmp} = load {place_llvm_ty}, ptr {ptr}, align {align}"
                                )
                                .ok();
                                let ext_tmp = self.new_temp();
                                let op = if signed { "sext" } else { "zext" };
                                writeln!(
                                    &mut self.builder,
                                    "  {ext_tmp} = {op} {place_llvm_ty} {load_tmp} to {ty}"
                                )
                                .ok();
                                return Ok(ValueRef::new(ext_tmp, ty));
                            }
                        }
                    }
                }
                if place.projection.last().is_some_and(
                    |elem| matches!(elem, ProjectionElem::FieldNamed(name) if name == "IsEmpty"),
                ) {
                    let mut len_place = place.clone();
                    len_place.projection.pop();
                    len_place
                        .projection
                        .push(ProjectionElem::FieldNamed("Length".into()));
                    let len_val = self.emit_operand(&Operand::Copy(len_place), None)?;
                    let len_ty = len_val.ty().to_string();
                    let cmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {cmp} = icmp eq {len_ty} {}, 0",
                        len_val.repr()
                    )
                    .ok();
                    let bool_tmp = self.new_temp();
                    writeln!(&mut self.builder, "  {bool_tmp} = zext i1 {cmp} to i8").ok();
                    return Ok(ValueRef::new(bool_tmp, "i8"));
                }
                if let Some(ty) = expected {
                    if ty.starts_with("ptr")
                        && self.is_reference_param(place.local.0)
                        && place.projection.is_empty()
                    {
                        let ptr_ptr = self.place_ptr(place)?;
                        let tmp = self.new_temp();
                        let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = load ptr, ptr {ptr_ptr}{alias_suffix}"
                        )
                        .ok();
                        return Ok(ValueRef::new(tmp, ty));
                    }
                    if ty.starts_with("ptr") && self.function.name.contains("AsyncEntry::Chain") {
                        let place_ty = self.place_type(place)?;
                        panic!(
                            "debug chain operand {:?} expected {ty} place_ty={place_ty:?}",
                            place
                        );
                    }
                    if std::env::var("CHIC_DEBUG_OPERANDS").is_ok() && ty.starts_with("ptr") {
                        let place_ty = self.place_type(place)?;
                        eprintln!(
                            "[chic-debug] emit_operand {:?} expected {ty} place_ty={place_ty:?}",
                            place
                        );
                    }
                    if ty.starts_with("ptr") {
                        if let Some(place_ty) = self.place_type(place)? {
                            let place_ty_is_pointer = place_ty == "ptr"
                                || place_ty.starts_with("ptr ")
                                || place_ty.ends_with('*');
                            if !place_ty_is_pointer {
                                let ptr = self.place_ptr(place)?;
                                return Ok(ValueRef::new(ptr, ty));
                            }
                        }
                    }
                    self.load_place(place, ty)
                } else {
                    let owned = self
                        .place_type(place)?
                        .ok_or_else(|| Error::Codegen("place type unknown for load".into()))?;
                    self.load_place(place, &owned)
                }
            }
            Operand::Const(constant) => self.emit_const_operand(constant, expected),
            Operand::Mmio(spec) => self.emit_mmio_operand(spec, expected),
            Operand::Pending(_) => {
                let ty = expected.unwrap_or("i32");
                let repr = if ty.starts_with('{') || ty.starts_with('[') {
                    "zeroinitializer".to_string()
                } else if ty == "ptr" {
                    "null".to_string()
                } else {
                    "0".to_string()
                };
                Ok(ValueRef::new_literal(repr, ty))
            }
            Operand::Borrow(borrow) => {
                if let Some(ty) = expected {
                    if !ty.starts_with("ptr") && !ty.ends_with('*') {
                        return self.load_place(&borrow.place, ty);
                    }
                }
                let ptr = self.place_ptr(&borrow.place)?;
                if self.is_reference_param(borrow.place.local.0) {
                    let tmp = self.new_temp();
                    writeln!(&mut self.builder, "  {tmp} = load ptr, ptr {ptr}").ok();
                    let ty = expected.unwrap_or("ptr");
                    Ok(ValueRef::new(tmp, ty))
                } else {
                    let ty = expected.unwrap_or("ptr");
                    Ok(ValueRef::new(ptr, ty))
                }
            }
        }
    }

    fn emit_const_operand(
        &mut self,
        constant: &ConstOperand,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        match &constant.value {
            ConstValue::Int(v) | ConstValue::Int32(v)
                if expected.is_some_and(|ty| ty.starts_with("ptr") || ty.ends_with('*')) =>
            {
                let ty = expected.unwrap();
                let int_ty = format!("i{}", self.pointer_width_bits());
                let repr = format!("inttoptr ({int_ty} {v} to {ty})");
                return Ok(ValueRef::new_literal(repr, ty));
            }
            ConstValue::Bool(v)
                if expected.is_some_and(|ty| ty.starts_with("ptr") || ty.ends_with('*')) =>
            {
                let ty = expected.unwrap();
                let int_ty = format!("i{}", self.pointer_width_bits());
                let value = if *v { 1u128 } else { 0u128 };
                let repr = format!("inttoptr ({int_ty} {value} to {ty})");
                return Ok(ValueRef::new_literal(repr, ty));
            }
            ConstValue::UInt(v)
                if expected.is_some_and(|ty| ty.starts_with("ptr") || ty.ends_with('*')) =>
            {
                let ty = expected.unwrap();
                let int_ty = format!("i{}", self.pointer_width_bits());
                let repr = format!("inttoptr ({int_ty} {v} to {ty})");
                return Ok(ValueRef::new_literal(repr, ty));
            }
            ConstValue::Char(c)
                if expected.is_some_and(|ty| ty.starts_with("ptr") || ty.ends_with('*')) =>
            {
                let ty = expected.unwrap();
                let int_ty = format!("i{}", self.pointer_width_bits());
                let value = u32::from(*c) as u128;
                let repr = format!("inttoptr ({int_ty} {value} to {ty})");
                return Ok(ValueRef::new_literal(repr, ty));
            }
            ConstValue::Enum { discriminant, .. }
                if expected.is_some_and(|ty| ty.starts_with("ptr") || ty.ends_with('*')) =>
            {
                let ty = expected.unwrap();
                let int_ty = format!("i{}", self.pointer_width_bits());
                let value = *discriminant as i128;
                let repr = format!("inttoptr ({int_ty} {value} to {ty})");
                return Ok(ValueRef::new_literal(repr, ty));
            }
            ConstValue::Str { id, .. } => {
                let ty = expected.ok_or_else(|| {
                    Error::Codegen("string literals require an expected type".into())
                })?;
                if ty.starts_with("ptr") || ty.ends_with('*') {
                    let info = self.str_literals.get(id).ok_or_else(|| {
                        Error::Codegen(format!(
                            "missing interned string segment for literal {}",
                            id.index()
                        ))
                    })?;
                    let base = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {base} = getelementptr inbounds [{len} x i8], ptr {global}, i32 0, i32 0",
                        len = info.array_len,
                        global = info.global
                    )
                    .ok();
                    if ty == "ptr" {
                        return Ok(ValueRef::new(base, ty));
                    }
                    let cast = self.new_temp();
                    writeln!(&mut self.builder, "  {cast} = bitcast ptr {base} to {ty}").ok();
                    return Ok(ValueRef::new(cast, ty));
                }
                if ty == LLVM_STR_TYPE {
                    return self.emit_const_str(*id);
                }
                if ty == LLVM_STRING_TYPE {
                    let slice = self.emit_const_str(*id)?;
                    self.externals.insert("chic_rt_string_from_slice");
                    let tmp = self.new_temp();
                    writeln!(
                        &mut self.builder,
                        "  {tmp} = call {LLVM_STRING_TYPE} @chic_rt_string_from_slice({LLVM_STR_TYPE} {})",
                        slice.repr()
                    )
                    .ok();
                    return Ok(ValueRef::new(tmp, LLVM_STRING_TYPE));
                }
                if ty.starts_with('i') || ty.starts_with('u') {
                    return Ok(ValueRef::new_literal("0".to_string(), ty));
                }
                if ty.starts_with('{') || ty.starts_with('[') {
                    return Ok(ValueRef::new_literal("zeroinitializer".to_string(), ty));
                }
                Err(Error::Codegen(format!(
                    "string literal cannot be lowered to LLVM type `{ty}`"
                )))
            }
            ConstValue::Symbol(name) => {
                let ty = expected.unwrap_or("ptr");
                if let Some(canonical) =
                    resolve_function_name(self.signatures, name).or_else(|| {
                        if self.signatures.contains_key(name) {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                {
                    let signature = self.signatures.get(&canonical).ok_or_else(|| {
                        Error::Codegen(format!(
                            "unknown function `{}` referenced as constant",
                            name
                        ))
                    })?;
                    let ret_ty = signature.ret.clone().unwrap_or_else(|| "void".to_string());
                    let params = if signature.params.is_empty() {
                        String::new()
                    } else {
                        signature.params.join(", ")
                    };
                    let fn_ty = if params.is_empty() {
                        format!("{ret_ty} ()")
                    } else {
                        format!("{ret_ty} ({params})")
                    };
                    let source_ptr_ty = format!("{fn_ty}*");
                    let symbol = format!("@{}", signature.symbol);
                    if ty == source_ptr_ty {
                        Ok(ValueRef::new_literal(symbol, ty))
                    } else if ty.starts_with('i') {
                        let tmp = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = ptrtoint {source_ptr_ty} {symbol} to {ty}"
                        )
                        .ok();
                        Ok(ValueRef::new(tmp, ty))
                    } else {
                        let tmp = self.new_temp();
                        writeln!(
                            &mut self.builder,
                            "  {tmp} = bitcast {source_ptr_ty} {symbol} to {ty}"
                        )
                        .ok();
                        Ok(ValueRef::new(tmp, ty))
                    }
                } else if self.vtable_symbols.contains(name) {
                    let symbol = format!("@{name}");
                    if ty == "ptr" {
                        Ok(ValueRef::new_literal(symbol, ty))
                    } else if ty.starts_with('i') {
                        let tmp = self.new_temp();
                        writeln!(&mut self.builder, "  {tmp} = ptrtoint ptr {symbol} to {ty}").ok();
                        Ok(ValueRef::new(tmp, ty))
                    } else {
                        let tmp = self.new_temp();
                        writeln!(&mut self.builder, "  {tmp} = bitcast ptr {symbol} to {ty}").ok();
                        Ok(ValueRef::new(tmp, ty))
                    }
                } else {
                    // Fall back to a null/zero literal for unknown symbols so we can continue codegen
                    // in the presence of missing vtable globals (e.g., MemoryStream in tests).
                    if std::env::var("CHIC_DEBUG_CONST_SYMBOL").is_ok() {
                        eprintln!(
                            "[chic-debug] unknown const symbol `{name}` expected={expected:?}"
                        );
                    }
                    let ty = expected.unwrap_or("ptr");
                    let repr = if ty.starts_with('{') || ty.starts_with('[') {
                        "zeroinitializer".to_string()
                    } else if ty.starts_with("ptr") || ty.ends_with('*') || ty.starts_with('%') {
                        "null".to_string()
                    } else {
                        "0".to_string()
                    };
                    Ok(ValueRef::new_literal(repr, ty))
                }
            }
            ConstValue::Null => {
                let ty = expected.unwrap_or("ptr");
                let repr = if ty.starts_with('{') || ty.starts_with('[') {
                    "zeroinitializer".to_string()
                } else if ty.starts_with("ptr") || ty.ends_with('*') || ty.starts_with('%') {
                    "null".to_string()
                } else {
                    "0".to_string()
                };
                Ok(ValueRef::new(repr, ty))
            }
            ConstValue::Enum {
                type_name,
                variant: _,
                discriminant,
            } if expected.is_some_and(|ty| ty.starts_with('{'))
                && type_name == "Std::Numeric::Decimal::DecimalStatus" =>
            {
                let ty = expected.unwrap();
                let status_ty =
                    map_type_owned(&Ty::named(type_name.clone()), Some(self.type_layouts))?
                        .ok_or_else(|| {
                            Error::Codegen("enum constant lowered to void type".into())
                        })?;
                let decimal_ty = map_type_owned(&Ty::named("decimal"), Some(self.type_layouts))?
                    .ok_or_else(|| {
                        Error::Codegen("decimal constant lowered to void type".into())
                    })?;
                let variant_ty = map_type_owned(
                    &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicVariant"),
                    Some(self.type_layouts),
                )?
                .ok_or_else(|| Error::Codegen("variant constant lowered to void type".into()))?;
                let repr =
                    format!("{{ {status_ty} {discriminant}, {decimal_ty} 0, {variant_ty} 0 }}");
                Ok(ValueRef::new_literal(repr, ty))
            }
            ConstValue::Enum {
                type_name, variant, ..
            } => {
                let mapped = expected
                    .map(|ty| ty.to_string())
                    .or_else(|| {
                        map_type_owned(&Ty::named(type_name.clone()), Some(self.type_layouts))
                            .ok()
                            .flatten()
                    })
                    .unwrap_or_else(|| "i32".to_string());
                let discr = self
                    .type_layouts
                    .layout_for_name(type_name)
                    .and_then(|layout| match layout {
                        crate::mir::TypeLayout::Enum(enum_layout) => enum_layout
                            .variants
                            .iter()
                            .find(|v| v.name == *variant)
                            .map(|v| v.discriminant),
                        _ => None,
                    })
                    .unwrap_or(0);
                Ok(ValueRef::new_literal(discr.to_string(), &mapped))
            }
            ConstValue::Struct { type_name, fields } => {
                let ty = expected.ok_or_else(|| {
                    Error::Codegen("constant operands require an expected type".into())
                })?;
                let layout_key = self
                    .type_layouts
                    .layout_for_name(type_name)
                    .or_else(|| {
                        self.type_layouts
                            .resolve_type_key(type_name)
                            .and_then(|key| self.type_layouts.layout_for_name(key))
                    })
                    .ok_or_else(|| {
                        Error::Codegen(format!(
                            "type layout for constant `{type_name}` not recorded"
                        ))
                    })?;
                let struct_layout = match layout_key {
                    crate::mir::TypeLayout::Struct(layout)
                    | crate::mir::TypeLayout::Class(layout) => layout,
                    _ => {
                        return Err(Error::Codegen(format!(
                            "type `{type_name}` is not a struct/class for constant lowering"
                        )));
                    }
                };
                let mut parts = Vec::new();
                for field in &struct_layout.fields {
                    let value = fields
                        .iter()
                        .find(|(name, _)| name == &field.name)
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "constant for `{type_name}` missing field `{}`",
                                field.name
                            ))
                        })?;
                    let field_ty = map_type_owned(&field.ty, Some(&self.type_layouts))?
                        .ok_or_else(|| {
                            Error::Codegen(format!(
                                "field `{}` of `{type_name}` lowers to void type",
                                field.name
                            ))
                        })?;
                    let repr = const_repr(&value.1, &field_ty)?;
                    parts.push(format!("{field_ty} {repr}"));
                }
                let repr = format!("{{ {} }}", parts.join(", "));
                Ok(ValueRef::new_literal(repr, ty))
            }
            value => {
                let ty = expected.ok_or_else(|| {
                    Error::Codegen("constant operands require an expected type".into())
                })?;
                let repr = const_repr(value, ty)?;
                Ok(ValueRef::new_literal(repr, ty))
            }
        }
    }

    pub(crate) fn load_place(&mut self, place: &Place, ty: &str) -> Result<ValueRef, Error> {
        if self.is_reference_param(place.local.0) && place.projection.is_empty() {
            let ptr_ptr = self.place_ptr(place)?;
            let ptr_tmp = self.new_temp();
            let pointer_align = pointer_align();
            writeln!(
                &mut self.builder,
                "  {ptr_tmp} = load ptr, ptr {ptr_ptr}, align {pointer_align}"
            )
            .ok();
            let value_ty = self.param_value_type(place.local.0)?;
            let value_align = self.align_for_ty(&self.mir_ty_of_place(place)?).max(1);
            let tmp = self.new_temp();
            let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
            writeln!(
                &mut self.builder,
                "  {tmp} = load {value_ty}, ptr {ptr_tmp}, align {value_align}{alias_suffix}"
            )
            .ok();
            return Ok(ValueRef::new(tmp, &value_ty));
        }
        let ptr = self.place_ptr(place)?;
        let align = self.place_alignment(place)?;
        let tmp = self.new_temp();
        let alias_suffix = self.alias_suffix_for_place(place).unwrap_or_default();
        writeln!(
            &mut self.builder,
            "  {tmp} = load {ty}, ptr {ptr}, align {align}{alias_suffix}"
        )
        .ok();
        Ok(ValueRef::new(tmp, ty))
    }

    pub(crate) fn load_local(&mut self, index: usize, ty: &str) -> Result<ValueRef, Error> {
        let ptr = self
            .local_ptrs
            .get(index)
            .and_then(|opt| opt.as_ref())
            .cloned()
            .ok_or_else(|| Error::Codegen("local missing storage".into()))?;
        let mir_ty = self
            .function
            .body
            .locals
            .get(index)
            .map(|decl| decl.ty.clone())
            .unwrap_or_else(|| Ty::Unknown);
        let align = self.align_for_ty(&mir_ty);
        let tmp = self.new_temp();
        writeln!(
            &mut self.builder,
            "  {tmp} = load {ty}, ptr {ptr}, align {align}"
        )
        .ok();
        Ok(ValueRef::new(tmp, ty))
    }

    pub(crate) fn operand_type_hint(
        &self,
        operand: &Operand,
        locals: &[Option<String>],
    ) -> Result<Option<String>, Error> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                if place.projection.is_empty() {
                    Ok(locals.get(place.local.0).cloned().flatten())
                } else {
                    let ty = self.mir_ty_of_place(place)?;
                    map_type_owned(&ty, Some(self.type_layouts))
                }
            }
            Operand::Const(constant) => {
                infer_const_type(&constant.value, constant.literal.as_ref())
            }
            Operand::Mmio(spec) => {
                let ty = if spec.width_bits <= 32 { "i32" } else { "i64" };
                Ok(Some(ty.to_string()))
            }
            Operand::Borrow(_) | Operand::Pending(_) => Ok(None),
        }
    }
}

fn int_width(ty: &str) -> Option<u32> {
    ty.strip_prefix('i')
        .and_then(|bits| bits.parse::<u32>().ok())
}

fn is_signed_int_ty(ty: &Ty) -> bool {
    match ty {
        Ty::Named(named) => {
            let lower = named
                .name
                .rsplit("::")
                .next()
                .unwrap_or(named.name.as_str())
                .to_ascii_lowercase();
            matches!(
                lower.as_str(),
                "sbyte" | "i8" | "short" | "i16" | "int" | "i32" | "long" | "i64" | "nint"
            )
        }
        _ => false,
    }
}
