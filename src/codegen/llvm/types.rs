use crate::codegen::llvm::emitter::literals::LLVM_VEC_TYPE;
use crate::error::Error;
use crate::mir::casts::{is_pointer_type, pointer_depth, short_type_name};
use crate::mir::{
    ConstValue, FloatValue, FloatWidth, RoundingMode, StructLayout, Ty, TypeLayout, TypeLayoutTable,
};
use crate::syntax::numeric::{IntegerWidth, NumericLiteralMetadata, NumericLiteralType};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::OnceLock;

fn builtin_layouts() -> &'static TypeLayoutTable {
    static LAYOUTS: OnceLock<TypeLayoutTable> = OnceLock::new();
    LAYOUTS.get_or_init(TypeLayoutTable::default)
}

thread_local! {
    static LLVM_TYPE_VISITING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct LlvmTypeGuard {
    key: String,
    active: bool,
}

impl Drop for LlvmTypeGuard {
    fn drop(&mut self) {
        if self.active {
            LLVM_TYPE_VISITING.with(|visiting| {
                visiting.borrow_mut().remove(&self.key);
            });
        }
    }
}

fn builtin_type(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "f32x4" => Some("<4 x float>"),
        "f32x8" => Some("<8 x float>"),
        "f16" | "half" => Some("half"),
        "f16x8" => Some("<8 x half>"),
        "bf16" | "bfloat16" => Some("bfloat"),
        "bf16x8" => Some("<8 x bfloat>"),
        "f32" => Some("float"),
        "i32x16" => Some("<16 x i32>"),
        "i32x4" => Some("<4 x i32>"),
        "i8x64" => Some("<64 x i8>"),
        "i8x16" => Some("<16 x i8>"),
        "f64" => Some("double"),
        "float128" | "f128" | "quad" => Some("fp128"),
        "bool" | "boolean" | "byte" | "sbyte" | "i8" | "u8" => Some("i8"),
        "char" | "short" | "ushort" | "i16" | "u16" => Some("i16"),
        "int" | "uint" | "i32" | "u32" => Some("i32"),
        "long" | "ulong" | "usize" | "isize" | "nint" | "nuint" | "i64" | "u64" => Some("i64"),
        "i128" | "u128" | "int128" | "uint128" => Some("i128"),
        "float" => Some("float"),
        "double" => Some("double"),
        "decimal" => Some("i128"),
        _ => None,
    }
}

fn int_type_for_size(size: usize) -> Result<&'static str, Error> {
    match size {
        1 => Ok("i8"),
        2 => Ok("i16"),
        4 => Ok("i32"),
        8 => Ok("i64"),
        16 => Ok("i128"),
        other => Err(Error::Codegen(format!(
            "enum representation size {other} is not supported in LLVM backend"
        ))),
    }
}

pub(crate) fn map_type_owned(
    ty: &Ty,
    layouts: Option<&TypeLayoutTable>,
) -> Result<Option<String>, Error> {
    let debug_types = std::env::var("CHIC_DEBUG_TYPES").is_ok();
    let debug_async = std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok();
    let map_by_name = |name: &str| -> Result<Option<String>, Error> {
        let map_layout = |table: &TypeLayoutTable,
                          layout: &TypeLayout|
         -> Result<Option<String>, Error> {
            let mapped = match layout {
                TypeLayout::Enum(enum_layout) => {
                    let size = enum_layout.size.unwrap_or(4);
                    let repr = int_type_for_size(size)?;
                    Some(repr.to_string())
                }
                TypeLayout::Struct(struct_layout) => Some(map_struct_layout(struct_layout, table)?),
                TypeLayout::Class(class_layout) => {
                    let is_marker_interface = class_layout.fields.is_empty()
                        && class_layout
                            .class
                            .as_ref()
                            .and_then(|info| info.vtable_offset)
                            .is_some_and(|offset| offset == 0)
                        && class_layout.size == Some(crate::mir::pointer_size())
                        && class_layout.align == Some(crate::mir::pointer_align());
                    if is_marker_interface {
                        Some("{ ptr, ptr }".into())
                    } else {
                        Some("ptr".into())
                    }
                }
                TypeLayout::Union(_) => {
                    return Err(Error::Codegen(format!(
                        "union type `{name}` is not yet supported in LLVM backend"
                    )));
                }
            };
            Ok(mapped)
        };

        if let Some(table) = layouts {
            if let Some(layout) = table.layout_for_name(name) {
                let mapped = map_layout(table, layout)?;
                if debug_types || (debug_async && name.contains("Task")) {
                    let kind = match layout {
                        TypeLayout::Struct(_) => "struct",
                        TypeLayout::Class(_) => "class",
                        TypeLayout::Enum(_) => "enum",
                        TypeLayout::Union(_) => "union",
                    };
                    eprintln!("[chic-debug] map_type `{name}` via module => {mapped:?} ({kind})");
                }
                return Ok(mapped);
            }
            // Fall back to matching by short name when the layout table contains multiple
            // candidates (e.g., `Std::Datetime::Time` vs `Std::Platform::Time`). Prefer
            // non-class layouts when ambiguity remains so value types keep their concrete
            // representations.
            let short_matches: Vec<_> = table
                .types
                .iter()
                .filter(|(key, _)| key.rsplit("::").next() == Some(name))
                .collect();
            if !short_matches.is_empty() {
                let preferred = short_matches
                    .iter()
                    .find(|(key, layout)| {
                        key.starts_with("Std::") && !matches!(layout, TypeLayout::Class(_))
                    })
                    .copied()
                    .or_else(|| {
                        short_matches
                            .iter()
                            .find(|(key, _)| key.starts_with("Std::"))
                            .copied()
                    })
                    .or_else(|| {
                        short_matches
                            .iter()
                            .find(|(_, layout)| !matches!(layout, TypeLayout::Class(_)))
                            .copied()
                    })
                    .or_else(|| short_matches.first().copied());
                if let Some((key, layout)) = preferred {
                    let mapped = map_layout(table, layout)?;
                    if debug_types || (debug_async && name.contains("Task")) {
                        let kind = match layout {
                            TypeLayout::Struct(_) => "struct",
                            TypeLayout::Class(_) => "class",
                            TypeLayout::Enum(_) => "enum",
                            TypeLayout::Union(_) => "union",
                        };
                        eprintln!(
                            "[chic-debug] map_type `{name}` via short-name `{key}` => {mapped:?} ({kind})"
                        );
                    }
                    return Ok(mapped);
                }
            }
        }

        let builtins = builtin_layouts();
        if let Some(layout) = builtins.layout_for_name(name) {
            let mapped = map_layout(builtins, layout)?;
            if debug_types || (debug_async && name.contains("Task")) {
                let kind = match layout {
                    TypeLayout::Struct(_) => "struct",
                    TypeLayout::Class(_) => "class",
                    TypeLayout::Enum(_) => "enum",
                    TypeLayout::Union(_) => "union",
                };
                eprintln!("[chic-debug] map_type `{name}` via builtin => {mapped:?} ({kind})");
            }
            return Ok(mapped);
        }

        Ok(None)
    };

    match ty {
        Ty::Unit => Ok(Some("void".into())),
        Ty::Unknown => {
            if debug_types {
                eprintln!("[chic-debug] map_type encountered `Unknown`");
            }
            Err(Error::Codegen(
                "unknown type cannot be lowered to the LLVM backend".into(),
            ))
        }
        Ty::Array(array) => {
            let name = Ty::Array(array.clone()).canonical_name();
            if let Some(mapped) = map_by_name(&name)? {
                return Ok(Some(mapped));
            }
            // Arrays lower to vector storage; if metadata is missing fall back to the vector
            // representation so codegen stays usable.
            Ok(Some(LLVM_VEC_TYPE.into()))
        }
        Ty::Vec(vec) => {
            let name = Ty::Vec(vec.clone()).canonical_name();
            if let Some(mapped) = map_by_name(&name)? {
                return Ok(Some(mapped));
            }
            Ok(Some(LLVM_VEC_TYPE.into()))
        }
        Ty::Vector(vector) => {
            let element_ty = map_type_owned(&vector.element, layouts)?.ok_or_else(|| {
                Error::Codegen(format!(
                    "SIMD vector element type `{}` is missing LLVM lowering",
                    vector.element.canonical_name()
                ))
            })?;
            if element_ty.starts_with('{') || element_ty.starts_with('[') {
                return Err(Error::Codegen(format!(
                    "SIMD vectors require scalar element types; `{}` is not supported",
                    element_ty
                )));
            }
            Ok(Some(format!("<{} x {element_ty}>", vector.lanes)))
        }
        Ty::Span(span) => {
            let name = Ty::Span(span.clone()).canonical_name();
            if let Some(mapped) = map_by_name(&name)? {
                return Ok(Some(mapped));
            }
            Err(Error::Codegen(format!(
                "span type `{name}` missing layout metadata for LLVM backend"
            )))
        }
        Ty::ReadOnlySpan(span) => {
            let name = Ty::ReadOnlySpan(span.clone()).canonical_name();
            if let Some(mapped) = map_by_name(&name)? {
                return Ok(Some(mapped));
            }
            Err(Error::Codegen(format!(
                "readonly span type `{name}` missing layout metadata for LLVM backend"
            )))
        }
        Ty::Tuple(_) => Ok(Some("i8*".into())),
        Ty::Fn(fn_ty) => {
            if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                Ok(Some("ptr".into()))
            } else {
                // Chic function pointers lower to an intrinsic fat-pointer record:
                // `{ invoke, context, drop_glue, type_id, env_size, env_align }`.
                Ok(Some("{ ptr, ptr, ptr, i64, i64, i64 }".into()))
            }
        }
        Ty::Nullable(_) => Ok(Some("i8*".into())),
        Ty::Rc(_) | Ty::Arc(_) => Ok(Some("i8*".into())),
        Ty::Pointer(_) | Ty::Ref(_) => Ok(Some("ptr".into())),
        Ty::TraitObject(_) => Ok(Some("{ i8*, ptr }".into())),
        Ty::String => Ok(Some(
            crate::codegen::llvm::emitter::literals::LLVM_STRING_TYPE.into(),
        )),
        Ty::Str => Ok(Some("{ i8*, i64 }".into())),
        Ty::Named(_) => {
            let type_name = ty.canonical_name();
            let active = LLVM_TYPE_VISITING.with(|visiting| {
                let mut visiting = visiting.borrow_mut();
                if visiting.contains(&type_name) {
                    return false;
                }
                visiting.insert(type_name.clone());
                true
            });
            if !active {
                if std::env::var("CHIC_DEBUG_TYPES").is_ok() {
                    eprintln!("[chic-debug] map_type recursion fallback `{type_name}` -> ptr");
                }
                return Ok(Some("ptr".into()));
            }
            let _guard = LlvmTypeGuard {
                key: type_name.clone(),
                active,
            };

            let compact_name = if type_name.chars().any(|ch| ch.is_whitespace()) {
                Some(
                    type_name
                        .chars()
                        .filter(|ch| !ch.is_whitespace())
                        .collect::<String>(),
                )
            } else {
                None
            };
            if is_pointer_type(&type_name) {
                let depth = pointer_depth(&type_name);
                let mut ty = "i8".to_string();
                for _ in 0..depth {
                    ty.push('*');
                }
                return Ok(Some(ty));
            }
            if short_type_name(type_name.as_str()) == "Self" {
                return Ok(Some("ptr".into()));
            }
            let short = short_type_name(type_name.as_str());
            if let Some(mapped) = builtin_type(&type_name).or_else(|| builtin_type(short)) {
                return Ok(Some(mapped.to_string()));
            }
            if short.eq_ignore_ascii_case("array")
                || type_name.starts_with("Array<")
                || type_name.contains("::Array<")
                || type_name.ends_with("[]")
            {
                // Treat dynamic arrays the same as vectors when explicit layout metadata is
                // unavailable (e.g., generic instantiations like Task<byte[]>).
                return Ok(Some(LLVM_VEC_TYPE.into()));
            }
            if type_name.contains("SocketError") || short.eq_ignore_ascii_case("netsocketerror") {
                // Prefer the real socket error layout when present; otherwise treat as a 32-bit
                // error code to keep networking helpers compiling.
                if let Some(mapped) = map_by_name("Std::Net::Sockets::SocketError")? {
                    return Ok(Some(mapped));
                }
                return Ok(Some("i32".into()));
            }
            if short.eq_ignore_ascii_case("platformsocket")
                || type_name.contains("Platform::IO::Socket")
            {
                if let Some(mapped) = map_by_name("Std::Platform::IO::Socket")? {
                    return Ok(Some(mapped));
                }
                return Ok(Some("{ i32 }".into()));
            }
            if short.eq_ignore_ascii_case("ChicString") || type_name.contains("ChicString") {
                return Ok(Some(
                    crate::codegen::llvm::emitter::literals::LLVM_STRING_TYPE.into(),
                ));
            }
            if short.eq_ignore_ascii_case("ChicStr") || type_name.contains("ChicStr") {
                // Alias the runtime string slice handle to the standard `str` layout.
                if let Some(mapped) = map_by_name("str")? {
                    return Ok(Some(mapped));
                }
                return Ok(Some("{ i8*, i64 }".into()));
            }
            if let Some(mapped) = map_by_name(&type_name)? {
                return Ok(Some(mapped));
            }
            if let Some(compact) = compact_name.as_deref() {
                if let Some(mapped) = builtin_type(compact) {
                    return Ok(Some(mapped.to_string()));
                }
                if let Some(mapped) = map_by_name(compact)? {
                    return Ok(Some(mapped));
                }
            }
            if short.eq_ignore_ascii_case("object") || type_name.eq_ignore_ascii_case("object") {
                // Lower the root object alias to an opaque reference when layout metadata is
                // missing or only available under a different casing.
                return Ok(Some("ptr".into()));
            }
            if type_name.contains("DescriptorList")
                || type_name.starts_with("Std::Meta::")
                || type_name.starts_with("Foundation::Meta::")
            {
                if debug_types {
                    eprintln!(
                        "[chic-debug] map_type treating metadata type `{type_name}` as opaque ptr"
                    );
                }
                return Ok(Some("ptr".into()));
            }
            if short == "VecError" || type_name.ends_with("::VecError") {
                return Ok(Some("i32".into()));
            }
            if let Some(mapped) = map_by_name(&type_name)? {
                return Ok(Some(mapped));
            }
            if type_name.len() == 1 && type_name.as_bytes()[0].is_ascii_uppercase() {
                // Treat unbound generic type parameters as opaque pointers to keep stubbed async
                // layouts compiling without full monomorphization metadata.
                return Ok(Some("ptr".into()));
            }
            if type_name.starts_with('T')
                && !type_name.contains("::")
                && type_name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return Ok(Some("ptr".into()));
            }
            if short.starts_with('I') {
                // Treat interface/trait types as opaque pointers until explicit layouts exist.
                return Ok(Some("ptr".into()));
            }
            if type_name.contains('<') {
                let generic_section = type_name
                    .split_once('<')
                    .map(|(_, args)| args.trim_end_matches('>'))
                    .unwrap_or("");
                let has_unbound_generic = generic_section.split(',').any(|raw| {
                    let token = raw.trim();
                    let base = token.rsplit("::").next().unwrap_or(token);
                    if base.len() == 1 && base.as_bytes()[0].is_ascii_uppercase() {
                        return true;
                    }
                    if base.starts_with('T')
                        && !base.contains("::")
                        && base
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        return true;
                    }
                    false
                });
                if has_unbound_generic {
                    return Ok(Some("ptr".into()));
                }
            }
            if debug_types {
                eprintln!("[chic-debug] map_type fallback treating `{type_name}` as ptr");
            }
            let byte_dump = type_name
                .as_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            Err(Error::Codegen(format!(
                "unsupported type `{type_name}` in LLVM backend (short=`{short}` bytes=[{byte_dump}])"
            )))
        }
    }
}

fn map_struct_layout(layout: &StructLayout, layouts: &TypeLayoutTable) -> Result<String, Error> {
    let size = layout.size;
    let mut fields = layout.fields.clone();
    fields.sort_by_key(|field| field.offset.unwrap_or(0));
    if std::env::var("CHIC_DEBUG_TYPES").is_ok() {
        let field_info = fields
            .iter()
            .map(|field| {
                let offset = field
                    .offset
                    .map(|off| off.to_string())
                    .unwrap_or_else(|| "?".into());
                format!("{}:{}@{offset}", field.name, field.ty.canonical_name())
            })
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!(
            "[chic-debug] struct layout {} size={:?} -> [{}]",
            layout.name, size, field_info
        );
    }

    let mut parts = Vec::new();
    let mut offset = 0usize;
    let mut requires_packed = layout.packing.is_some();
    for field in &fields {
        let field_offset = field.offset.unwrap_or(offset);
        if let Some(struct_size) = size {
            if field_offset > struct_size {
                return Err(Error::Codegen(format!(
                    "field `{}` offset exceeds struct `{}` size",
                    field.name, layout.name
                )));
            }
        }
        if field_offset > offset {
            let padding = field_offset - offset;
            parts.push(padding_type(padding));
            offset = field_offset;
        }
        let field_ty = map_type_owned(&field.ty, Some(layouts))?.ok_or_else(|| {
            Error::Codegen(format!(
                "field `{}` on `{}` lowers to void type",
                field.name, layout.name
            ))
        })?;
        parts.push(field_ty);
        let (field_size, field_align) = layouts.size_and_align_for_ty(&field.ty).unwrap_or((0, 1));
        if !requires_packed && field_align > 1 && field_offset % field_align != 0 {
            requires_packed = true;
        }
        offset = offset
            .checked_add(field_size)
            .ok_or_else(|| Error::Codegen("struct layout exceeds addressable range".into()))?;
    }

    if let Some(struct_size) = size {
        if offset < struct_size {
            parts.push(padding_type(struct_size - offset));
        }
    }
    if parts.is_empty() {
        parts.push("[0 x i8]".into());
    }
    if requires_packed {
        Ok(format!("<{{ {} }}>", parts.join(", ")))
    } else {
        Ok(format!("{{ {} }}", parts.join(", ")))
    }
}

fn padding_type(bytes: usize) -> String {
    format!("[{bytes} x i8]")
}

pub(crate) fn const_repr(value: &ConstValue, ty: &str) -> Result<String, Error> {
    let trimmed_ty = ty.trim();
    let is_aggregate =
        trimmed_ty.starts_with('{') || trimmed_ty.starts_with('[') || trimmed_ty.starts_with('<');
    let is_pointer_context =
        trimmed_ty == "ptr" || trimmed_ty.starts_with("ptr ") || trimmed_ty.ends_with('*');
    let single_field_newtype = |ty: &str| -> Option<String> {
        let trimmed = ty.trim();
        let inner = if trimmed.starts_with("<{") && trimmed.ends_with("}>") {
            &trimmed[2..trimmed.len() - 2]
        } else if trimmed.starts_with('{') && trimmed.ends_with('}') {
            &trimmed[1..trimmed.len() - 1]
        } else {
            return None;
        };
        let inner = inner.trim();
        if inner.is_empty() || inner.contains(',') {
            return None;
        }
        let is_llvm_int = inner
            .strip_prefix('i')
            .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()));
        let scalarish =
            inner == "ptr" || is_llvm_int || matches!(inner, "half" | "float" | "double" | "fp128");
        if !scalarish {
            return None;
        }
        Some(inner.to_string())
    };
    let wants_float = is_float_ty(trimmed_ty);
    let float_width = match trimmed_ty {
        "half" => Some(FloatWidth::F16),
        "float" => Some(FloatWidth::F32),
        "double" => Some(FloatWidth::F64),
        _ => None,
    };
    let float_bits_for_const = |value: &ConstValue, width: FloatWidth| -> Option<u128> {
        let val = match value {
            ConstValue::Float(v) => {
                if v.width == width {
                    return Some(v.bits);
                }
                v.to_f64()
            }
            ConstValue::Int(v) | ConstValue::Int32(v) => *v as f64,
            ConstValue::UInt(v) => *v as f64,
            ConstValue::Bool(v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            }
            ConstValue::Enum { discriminant, .. } => *discriminant as f64,
            ConstValue::Char(c) => u32::from(*c) as f64,
            _ => return None,
        };
        Some(FloatValue::from_f64_as(val, width).bits)
    };
    if wants_float {
        // Use hex float literals to keep IR toolchain-compatible.
        // (Apple clang's `-x ir` parser rejects some decimal float formats, notably `e-XX`.)
        if let Some(FloatWidth::F16) = float_width {
            if let Some(bits) = float_bits_for_const(value, FloatWidth::F16) {
                return Ok(format!("bitcast (i16 {} to half)", bits as u16));
            }
        }
    }
    let float_width_for_context = if wants_float { float_width } else { None };
    let format_float_from_f64 = |f: f64| -> Result<String, Error> {
        let width = float_width_for_context.ok_or_else(|| {
            Error::Codegen("float constant requested without float width context".into())
        })?;
        let value = FloatValue::from_f64_as(f, width);
        match width {
            FloatWidth::F16 => Ok(format!("bitcast (i16 {} to half)", value.bits as u16)),
            FloatWidth::F32 => {
                let f32_value = f32::from_bits(value.bits as u32);
                Ok(format!("0x{:016X}", (f32_value as f64).to_bits()))
            }
            FloatWidth::F64 => Ok(format!("0x{:016X}", value.bits as u64)),
            FloatWidth::F128 => Ok(format!("0xL{:032X}", value.bits)),
        }
    };
    if is_pointer_context {
        match value {
            ConstValue::Int(v) | ConstValue::Int32(v) => {
                if *v == 0 {
                    return Ok("null".into());
                }
                if *v < i128::from(i64::MIN) || *v > i128::from(i64::MAX) {
                    return Err(Error::Codegen(format!(
                        "pointer constant out of range for i64: {v}"
                    )));
                }
                return Ok(format!("inttoptr (i64 {} to {trimmed_ty})", *v as i64));
            }
            ConstValue::UInt(v) => {
                if *v == 0 {
                    return Ok("null".into());
                }
                if *v > u128::from(u64::MAX) {
                    return Err(Error::Codegen(format!(
                        "pointer constant out of range for u64: {v}"
                    )));
                }
                return Ok(format!("inttoptr (i64 {v} to {trimmed_ty})"));
            }
            ConstValue::Float(v) => {
                if v.bits == 0 {
                    return Ok("null".into());
                }
                let bits = match v.width {
                    FloatWidth::F16 => u64::from(v.bits as u16),
                    FloatWidth::F32 => u64::from(v.bits as u32),
                    FloatWidth::F64 => v.bits as u64,
                    FloatWidth::F128 => {
                        return Err(Error::Codegen(
                            "cannot lower fp128 bits as pointer constant".into(),
                        ));
                    }
                };
                return Ok(format!("inttoptr (i64 {bits} to {trimmed_ty})"));
            }
            _ => {}
        }
    }
    match value {
        ConstValue::Int(v) | ConstValue::Int32(v) => {
            if is_aggregate && *v == 0 {
                return Ok("zeroinitializer".into());
            }
            if is_aggregate {
                if let Some(field_ty) = single_field_newtype(trimmed_ty) {
                    let inner = const_repr(value, &field_ty)?;
                    if trimmed_ty.starts_with("<{") {
                        return Ok(format!("<{{ {field_ty} {inner} }}>"));
                    }
                    return Ok(format!("{{ {field_ty} {inner} }}"));
                }
            }
            if wants_float {
                return format_float_from_f64(*v as f64);
            }
            Ok(v.to_string())
        }
        ConstValue::UInt(v) => {
            if is_aggregate && *v == 0 {
                return Ok("zeroinitializer".into());
            }
            if is_aggregate {
                if let Some(field_ty) = single_field_newtype(trimmed_ty) {
                    let inner = const_repr(value, &field_ty)?;
                    if trimmed_ty.starts_with("<{") {
                        return Ok(format!("<{{ {field_ty} {inner} }}>"));
                    }
                    return Ok(format!("{{ {field_ty} {inner} }}"));
                }
            }
            if wants_float {
                return format_float_from_f64(*v as f64);
            }
            Ok(v.to_string())
        }
        ConstValue::Bool(v) => {
            if is_aggregate && !*v {
                return Ok("zeroinitializer".into());
            }
            if is_aggregate {
                if let Some(field_ty) = single_field_newtype(trimmed_ty) {
                    let inner = const_repr(value, &field_ty)?;
                    if trimmed_ty.starts_with("<{") {
                        return Ok(format!("<{{ {field_ty} {inner} }}>"));
                    }
                    return Ok(format!("{{ {field_ty} {inner} }}"));
                }
            }
            if wants_float {
                return format_float_from_f64(if *v { 1.0 } else { 0.0 });
            }
            Ok(if *v { "1".into() } else { "0".into() })
        }
        ConstValue::Float(v) => {
            if is_aggregate && v.bits == 0 {
                return Ok("zeroinitializer".into());
            }
            match v.width {
                FloatWidth::F16 => Ok(format!("bitcast (i16 {} to half)", v.bits as u16)),
                FloatWidth::F32 => {
                    let f32_value = f32::from_bits(v.bits as u32);
                    Ok(format!("0x{:016X}", (f32_value as f64).to_bits()))
                }
                FloatWidth::F64 => Ok(format!("0x{:016X}", v.bits as u64)),
                FloatWidth::F128 => Ok(format!("0xL{:032X}", v.bits)),
            }
        }
        ConstValue::Decimal(decimal) => {
            let encoded = decimal.to_encoding();
            let signed = i128::from_le_bytes(encoded.to_le_bytes());
            Ok(signed.to_string())
        }
        ConstValue::Enum { discriminant, .. } => {
            if is_aggregate && *discriminant == 0 {
                return Ok("zeroinitializer".into());
            }
            if is_aggregate {
                if let Some(field_ty) = single_field_newtype(trimmed_ty) {
                    let inner = const_repr(value, &field_ty)?;
                    if trimmed_ty.starts_with("<{") {
                        return Ok(format!("<{{ {field_ty} {inner} }}>"));
                    }
                    return Ok(format!("{{ {field_ty} {inner} }}"));
                }
            }
            if wants_float {
                return format_float_from_f64(*discriminant as f64);
            }
            Ok(discriminant.to_string())
        }
        ConstValue::Char(c) => {
            let value = u32::from(*c);
            if is_aggregate && value == 0 {
                return Ok("zeroinitializer".into());
            }
            if is_aggregate {
                if let Some(field_ty) = single_field_newtype(trimmed_ty) {
                    let inner = const_repr(&ConstValue::UInt(value as u128), &field_ty)?;
                    if trimmed_ty.starts_with("<{") {
                        return Ok(format!("<{{ {field_ty} {inner} }}>"));
                    }
                    return Ok(format!("{{ {field_ty} {inner} }}"));
                }
            }
            if wants_float {
                return format_float_from_f64(value.into());
            }
            Ok(value.to_string())
        }
        ConstValue::Unit => {
            if wants_float {
                format_float_from_f64(0.0)
            } else if is_aggregate {
                Ok("zeroinitializer".into())
            } else {
                Ok("0".into())
            }
        }
        ConstValue::Str { id, value } => {
            let trimmed = ty.trim();
            let data_len = value.len();
            let array_len = data_len.max(1);
            let global = format!("@__chx_str_{}", id.index());
            let base =
                format!("getelementptr inbounds ([{array_len} x i8], ptr {global}, i32 0, i32 0)");
            if trimmed == "{ i8*, i64 }" || trimmed == "{ ptr, i64 }" {
                let ptr_ty = if trimmed.contains("i8*") {
                    "i8*"
                } else {
                    "ptr"
                };
                let ptr_expr = if ptr_ty == "ptr" {
                    base.clone()
                } else {
                    format!("bitcast (ptr {base} to {ptr_ty})")
                };
                return Ok(format!("{{ {ptr_ty} {ptr_expr}, i64 {data_len} }}"));
            }
            if pointer_depth(trimmed) > 0 || trimmed.starts_with("ptr") || trimmed.ends_with('*') {
                if trimmed == "ptr" {
                    return Ok(base);
                }
                return Ok(format!("bitcast (ptr {base} to {trimmed})"));
            }
            Err(Error::Codegen(format!(
                "string constant cannot be lowered to `{trimmed}`"
            )))
        }
        ConstValue::Symbol(symbol) => {
            let trimmed = ty.trim();
            if pointer_depth(trimmed) > 0 || trimmed.starts_with("ptr") || trimmed.ends_with('*') {
                if trimmed == "ptr" {
                    return Ok(format!("@{symbol}"));
                }
                return Ok(format!("bitcast (ptr @{symbol} to {trimmed})"));
            }
            Err(Error::Codegen(format!(
                "symbol constant `{symbol}` requires a pointer type (got `{trimmed}`)"
            )))
        }
        ConstValue::RawStr(_) => Err(Error::Codegen(
            "raw string constants must be interned before LLVM emission".into(),
        )),
        ConstValue::Struct { .. } => Err(Error::Codegen(
            "struct constants are not yet supported in LLVM backend".into(),
        )),
        ConstValue::Null => {
            if wants_float {
                return format_float_from_f64(0.0);
            }
            if pointer_depth(trimmed_ty) > 0
                || trimmed_ty.starts_with('%')
                || trimmed_ty.eq_ignore_ascii_case("ptr")
            {
                Ok("null".into())
            } else if trimmed_ty.starts_with('{')
                || trimmed_ty.starts_with('[')
                || trimmed_ty.starts_with('<')
            {
                Ok("zeroinitializer".into())
            } else {
                Ok("0".into())
            }
        }
        ConstValue::Unknown => Err(Error::Codegen(
            "unknown constant cannot be lowered to LLVM".into(),
        )),
    }
}

fn llvm_int_type_for_width(width: IntegerWidth) -> &'static str {
    match width {
        IntegerWidth::W8 => "i8",
        IntegerWidth::W16 => "i16",
        IntegerWidth::W32 => "i32",
        IntegerWidth::W64 => "i64",
        IntegerWidth::W128 => "i128",
        IntegerWidth::Size => "i64",
    }
}

pub(crate) fn infer_const_type(
    value: &ConstValue,
    literal: Option<&NumericLiteralMetadata>,
) -> Result<Option<String>, Error> {
    if let Some(meta) = literal {
        let ty = match meta.literal_type {
            NumericLiteralType::Signed(width) => llvm_int_type_for_width(width).to_string(),
            NumericLiteralType::Unsigned(width) => llvm_int_type_for_width(width).to_string(),
            NumericLiteralType::Float16 => "half".to_string(),
            NumericLiteralType::Float32 => "float".to_string(),
            NumericLiteralType::Float64 => "double".to_string(),
            NumericLiteralType::Float128 => "fp128".to_string(),
            NumericLiteralType::Decimal => "i128".to_string(),
        };
        return Ok(Some(ty));
    }
    match value {
        ConstValue::Int(_) | ConstValue::Int32(_) | ConstValue::UInt(_) => Ok(Some("i32".into())),
        ConstValue::Char(_) => Ok(Some("i16".into())),
        ConstValue::Bool(_) | ConstValue::Unit => Ok(Some("i8".into())),
        ConstValue::Float(value) => Ok(Some(match value.width {
            FloatWidth::F16 => "half".into(),
            FloatWidth::F32 => "float".into(),
            FloatWidth::F64 => "double".into(),
            FloatWidth::F128 => "fp128".into(),
        })),
        ConstValue::Decimal(_) => Ok(Some("i128".into())),
        ConstValue::Enum { .. } => Ok(None),
        ConstValue::Str { .. } | ConstValue::RawStr(_) => Ok(Some("{ i8*, i64 }".into())),
        ConstValue::Symbol(_) => Ok(Some("ptr".into())),
        ConstValue::Null => Ok(None),
        ConstValue::Struct { .. } => Ok(None),
        ConstValue::Unknown => Ok(None),
    }
}

/// Map a Chic rounding mode to LLVM constrained intrinsic string codes.
#[must_use]
#[allow(dead_code)]
pub(crate) fn constrained_rounding_string(mode: RoundingMode) -> &'static str {
    match mode {
        RoundingMode::NearestTiesToEven => "round.tonearest",
        RoundingMode::NearestTiesToAway => "round.tonearestaway",
        RoundingMode::TowardZero => "round.towardzero",
        RoundingMode::TowardPositive => "round.upward",
        RoundingMode::TowardNegative => "round.downward",
    }
}

/// Parse a canonical LLVM vector type string (`<lanes x elem>`) and return its lane count and
/// element type.
#[must_use]
pub(crate) fn parse_vector_type(ty: &str) -> Option<(u32, String)> {
    let trimmed = ty.trim();
    if !trimmed.starts_with('<') || !trimmed.ends_with('>') {
        return None;
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    let mut parts = inner.splitn(2, 'x');
    let lanes_part = parts.next()?.trim();
    let elem_part = parts.next()?.trim().trim_start_matches(char::is_whitespace);
    let lanes = lanes_part.parse::<u32>().ok()?;
    Some((lanes, elem_part.to_string()))
}

pub(crate) fn is_float_ty(ty: &str) -> bool {
    if matches!(ty, "half" | "float" | "double" | "fp128") {
        return true;
    }
    if let Some((_lanes, elem)) = parse_vector_type(ty) {
        return is_float_ty(elem.trim());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::llvm::emitter::literals::{LLVM_STRING_TYPE, LLVM_VEC_TYPE};
    use crate::mir::{
        ArrayTy, AutoTraitOverride, AutoTraitSet, EnumLayout, EnumVariantLayout, FloatValue,
        FloatWidth, TupleTy, TypeLayout, TypeLayoutTable, TypeRepr, VecTy, VectorTy,
    };

    #[test]
    fn map_type_handles_primitives() {
        assert_eq!(
            map_type_owned(&Ty::Unit, None).unwrap(),
            Some("void".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("bool"), None).unwrap(),
            Some("i8".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("int"), None).unwrap(),
            Some("i32".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("char"), None).unwrap(),
            Some("i16".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("ulong"), None).unwrap(),
            Some("i64".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("f32x4"), None).unwrap(),
            Some("<4 x float>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("f32x8"), None).unwrap(),
            Some("<8 x float>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("i32x16"), None).unwrap(),
            Some("<16 x i32>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("i32x4"), None).unwrap(),
            Some("<4 x i32>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("i8x64"), None).unwrap(),
            Some("<64 x i8>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("i8x16"), None).unwrap(),
            Some("<16 x i8>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("float"), None).unwrap(),
            Some("float".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("double"), None).unwrap(),
            Some("double".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("f16"), None).unwrap(),
            Some("half".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("f16x8"), None).unwrap(),
            Some("<8 x half>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("bf16"), None).unwrap(),
            Some("bfloat".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("bf16x8"), None).unwrap(),
            Some("<8 x bfloat>".to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::String, None).unwrap(),
            Some(crate::codegen::llvm::emitter::literals::LLVM_STRING_TYPE.to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::Str, None).unwrap(),
            Some("{ i8*, i64 }".to_string())
        );
    }

    #[test]
    fn map_type_handles_tuple_types() {
        let tuple_ty = Ty::Tuple(TupleTy::new(vec![Ty::named("int"), Ty::named("int")]));
        assert_eq!(
            map_type_owned(&tuple_ty, None).unwrap(),
            Some("i8*".to_string()),
            "tuple types should lower to pointer representations"
        );
    }

    #[test]
    fn map_type_handles_sequences_as_pointers() {
        let array = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
        assert_eq!(
            map_type_owned(&array, None).unwrap(),
            Some(LLVM_VEC_TYPE.into())
        );

        let vec = Ty::Vec(VecTy::new(Box::new(Ty::named("int"))));
        assert_eq!(
            map_type_owned(&vec, None).unwrap(),
            Some(LLVM_VEC_TYPE.into())
        );
    }

    #[test]
    fn map_type_handles_chic_string() {
        let layouts = TypeLayoutTable::default();
        assert_eq!(
            map_type_owned(&Ty::String, Some(&layouts)).unwrap(),
            Some(LLVM_STRING_TYPE.to_string())
        );
        assert_eq!(
            map_type_owned(
                &Ty::named("Std::Runtime::Native::ChicString"),
                Some(&layouts)
            )
            .unwrap(),
            Some(LLVM_STRING_TYPE.to_string())
        );
        assert_eq!(
            map_type_owned(&Ty::named("ChicString"), Some(&layouts)).unwrap(),
            Some(LLVM_STRING_TYPE.to_string())
        );
    }

    #[test]
    fn map_type_handles_vector_types() {
        let float_vector = Ty::Vector(VectorTy {
            element: Box::new(Ty::named("float")),
            lanes: 4,
        });
        assert_eq!(
            map_type_owned(&float_vector, None).unwrap(),
            Some("<4 x float>".to_string())
        );
        let int_vector = Ty::Vector(VectorTy {
            element: Box::new(Ty::named("int")),
            lanes: 8,
        });
        assert_eq!(
            map_type_owned(&int_vector, None).unwrap(),
            Some("<8 x i32>".to_string())
        );
    }

    #[test]
    fn is_float_ty_handles_vectors() {
        assert!(is_float_ty("<4 x float>"));
        assert!(is_float_ty("<8 x double>"));
        assert!(!is_float_ty("<4 x i32>"));
    }

    #[test]
    fn map_type_rejects_unknown_variants() {
        let err = map_type_owned(&Ty::Unknown, None).expect_err("expected unknown type error");
        match err {
            Error::Codegen { message, .. } => {
                assert!(
                    message.contains("unknown type cannot be lowered"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        let err = map_type_owned(&Ty::named("Custom"), None)
            .expect_err("expected unsupported named type error");
        match err {
            Error::Codegen { message, .. } => assert!(
                message.contains("unsupported type `Custom`"),
                "unexpected message: {message}"
            ),
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn map_type_owned_wraps_result() {
        let owned = map_type_owned(&Ty::named("uint"), None).unwrap();
        assert_eq!(owned, Some("i32".to_string()));

        let err =
            map_type_owned(&Ty::named("thing"), None).expect_err("expected unsupported type error");
        assert!(matches!(err, Error::Codegen { .. }));
    }

    #[test]
    fn map_type_handles_enums_via_layouts() {
        let mut layouts = TypeLayoutTable::default();
        layouts.types.insert(
            "Demo::Flags".into(),
            TypeLayout::Enum(EnumLayout {
                name: "Demo::Flags".into(),
                repr: TypeRepr::Default,
                packing: None,
                underlying: Ty::named("int"),
                underlying_info: Some(crate::mir::casts::IntInfo {
                    bits: 32,
                    signed: true,
                }),
                explicit_underlying: false,
                variants: vec![EnumVariantLayout {
                    name: "None".into(),
                    index: 0,
                    discriminant: 0,
                    fields: Vec::new(),
                    positional: Vec::new(),
                }],
                size: Some(4),
                align: Some(4),
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                is_flags: true,
            }),
        );

        let mapped = map_type_owned(&Ty::named("Demo::Flags"), Some(&layouts)).unwrap();
        assert_eq!(mapped.as_deref(), Some("i32"));
    }

    #[test]
    fn map_type_handles_decimal_intrinsic_result() {
        let layouts = TypeLayoutTable::default();
        assert!(
            layouts.resolve_type_key("DecimalIntrinsicResult").is_some(),
            "built-in layouts should register DecimalIntrinsicResult by short name"
        );
        let mapped = map_type_owned(
            &Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult"),
            Some(&layouts),
        )
        .expect("expected DecimalIntrinsicResult to map");
        assert!(
            mapped.as_deref().is_some(),
            "DecimalIntrinsicResult should lower to an LLVM struct"
        );
    }

    #[test]
    fn map_type_handles_async_task_with_result() {
        let layouts = TypeLayoutTable::default();
        let ty = Ty::named("Std::Async::Task<int>");
        let mapped = map_type_owned(&ty, Some(&layouts))
            .expect("mapping should succeed")
            .expect("task<int> should lower to an LLVM struct");
        assert_eq!(
            mapped,
            "{ { i64, i64, i64, i32, [4 x i8] }, i32, [4 x i8], { { i64, i64, i64, i32, [4 x i8] }, i8, [3 x i8], i32 } }"
        );
    }

    #[test]
    fn const_repr_formats_supported_constants() {
        assert_eq!(
            const_repr(&ConstValue::Int(42.into()), "i32").unwrap(),
            "42"
        );
        assert_eq!(const_repr(&ConstValue::UInt(27u128), "i32").unwrap(), "27");
        assert_eq!(const_repr(&ConstValue::Bool(true), "i1").unwrap(), "1");
        assert_eq!(const_repr(&ConstValue::Bool(false), "i1").unwrap(), "0");
        assert_eq!(
            const_repr(&ConstValue::Float(FloatValue::from_f64(3.14)), "double").unwrap(),
            format!("0x{:016X}", 3.14f64.to_bits())
        );
        assert_eq!(
            const_repr(&ConstValue::Char('A' as u16), "i16").unwrap(),
            format!("{}", 'A' as u32)
        );
        assert_eq!(const_repr(&ConstValue::Unit, "void").unwrap(), "0");
    }

    #[test]
    fn const_repr_converts_integers_in_float_contexts() {
        assert_eq!(
            const_repr(&ConstValue::Int((-128).into()), "double").unwrap(),
            format!("0x{:016X}", (-128.0f64).to_bits())
        );
        assert_eq!(
            const_repr(&ConstValue::UInt(5), "float").unwrap(),
            format!("0x{:016X}", (5.0f32 as f64).to_bits())
        );
        assert_eq!(
            const_repr(&ConstValue::Bool(true), "float").unwrap(),
            format!("0x{:016X}", (1.0f32 as f64).to_bits())
        );
        assert_eq!(
            const_repr(&ConstValue::Unit, "double").unwrap(),
            format!("0x{:016X}", 0.0f64.to_bits())
        );
    }

    #[test]
    fn const_repr_rejects_strings_and_unknown() {
        let err = const_repr(&ConstValue::RawStr("hi".into()), "ptr")
            .expect_err("expected string literal rejection");
        assert!(matches!(err, Error::Codegen { .. }));

        let err = const_repr(&ConstValue::Unknown, "i32")
            .expect_err("expected unknown constant rejection");
        assert!(matches!(err, Error::Codegen { .. }));
    }

    #[test]
    fn infer_const_type_returns_expected_types() {
        assert_eq!(
            infer_const_type(&ConstValue::Int(0.into()), None).unwrap(),
            Some("i32".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::UInt(0u128), None).unwrap(),
            Some("i32".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::Bool(true), None).unwrap(),
            Some("i8".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::Unit, None).unwrap(),
            Some("i8".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::Float(FloatValue::from_f64(2.71)), None).unwrap(),
            Some("double".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::Float(FloatValue::from_f16(1.0)), None).unwrap(),
            Some("half".into())
        );
        assert_eq!(
            infer_const_type(
                &ConstValue::Float(FloatValue::from_f64_as(1.0, FloatWidth::F128)),
                None
            )
            .unwrap(),
            Some("fp128".into())
        );
        assert_eq!(
            infer_const_type(&ConstValue::Char('x' as u16), None).unwrap(),
            Some("i16".into())
        );

        assert_eq!(
            infer_const_type(&ConstValue::RawStr("text".into()), None).unwrap(),
            Some("{ i8*, i64 }".into())
        );

        assert_eq!(
            infer_const_type(&ConstValue::Unknown, None).unwrap(),
            None,
            "unknown constant should not infer a type"
        );
    }

    #[test]
    fn infer_const_type_respects_literal_metadata() {
        let unsigned_meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Unsigned(IntegerWidth::W16),
            suffix_text: Some("u16".into()),
            explicit_suffix: true,
        };
        assert_eq!(
            infer_const_type(&ConstValue::UInt(42), Some(&unsigned_meta)).unwrap(),
            Some("i16".into())
        );

        let signed_meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Signed(IntegerWidth::W64),
            suffix_text: Some("i64".into()),
            explicit_suffix: true,
        };
        assert_eq!(
            infer_const_type(&ConstValue::Int(1), Some(&signed_meta)).unwrap(),
            Some("i64".into())
        );

        let float_meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Float32,
            suffix_text: Some("f32".into()),
            explicit_suffix: true,
        };
        assert_eq!(
            infer_const_type(
                &ConstValue::Float(FloatValue::from_f32(0.0)),
                Some(&float_meta)
            )
            .unwrap(),
            Some("float".into())
        );

        let half_meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Float16,
            suffix_text: Some("f16".into()),
            explicit_suffix: true,
        };
        assert_eq!(
            infer_const_type(
                &ConstValue::Float(FloatValue::from_f16(0.0)),
                Some(&half_meta)
            )
            .unwrap(),
            Some("half".into())
        );

        let quad_meta = NumericLiteralMetadata {
            literal_type: NumericLiteralType::Float128,
            suffix_text: Some("f128".into()),
            explicit_suffix: true,
        };
        assert_eq!(
            infer_const_type(
                &ConstValue::Float(FloatValue::from_f64_as(0.0, FloatWidth::F128)),
                Some(&quad_meta)
            )
            .unwrap(),
            Some("fp128".into())
        );
    }

    #[test]
    fn is_float_ty_matches_float_names() {
        assert!(is_float_ty("float"));
        assert!(is_float_ty("double"));
        assert!(is_float_ty("half"));
        assert!(is_float_ty("fp128"));
        assert!(!is_float_ty("i32"));
    }

    #[test]
    fn constrained_rounding_string_maps_all_modes() {
        assert_eq!(
            constrained_rounding_string(RoundingMode::NearestTiesToEven),
            "round.tonearest"
        );
        assert_eq!(
            constrained_rounding_string(RoundingMode::NearestTiesToAway),
            "round.tonearestaway"
        );
        assert_eq!(
            constrained_rounding_string(RoundingMode::TowardZero),
            "round.towardzero"
        );
        assert_eq!(
            constrained_rounding_string(RoundingMode::TowardPositive),
            "round.upward"
        );
        assert_eq!(
            constrained_rounding_string(RoundingMode::TowardNegative),
            "round.downward"
        );
    }
}
