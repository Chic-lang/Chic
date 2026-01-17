use crate::mir::{
    AutoTraitOverride, AutoTraitSet, ClassLayoutInfo, ClassLayoutKind, EnumLayout, FieldLayout,
    StructLayout, Ty, TypeLayout, TypeLayoutTable, TypeRepr,
};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use super::ensure_u32;

#[derive(Clone, Copy, Debug)]
pub(crate) struct AggregateAllocation {
    pub(crate) offset: u32,
    pub(crate) size: u32,
    pub(crate) align: u32,
}

pub(crate) fn is_scalar_named(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "bool"
            | "int"
            | "uint"
            | "i32"
            | "u32"
            | "int32"
            | "uint32"
            | "short"
            | "ushort"
            | "int16"
            | "uint16"
            | "char"
            | "byte"
            | "sbyte"
            | "int8"
            | "uint8"
            | "long"
            | "ulong"
            | "i64"
            | "u64"
            | "int64"
            | "uint64"
            | "float"
            | "f32"
            | "float32"
            | "double"
            | "f64"
            | "float64"
            | "usize"
            | "isize"
            | "nint"
            | "nuint"
            | "intptr"
            | "uintptr"
            | "system::int32"
            | "std::int32"
            | "system::int64"
            | "std::int64"
            | "system::uint32"
            | "std::uint32"
            | "system::uint64"
            | "std::uint64"
            | "std::numeric::int8"
            | "std::numeric::uint8"
            | "std::numeric::int16"
            | "std::numeric::uint16"
            | "std::numeric::int32"
            | "std::numeric::uint32"
            | "std::numeric::int64"
            | "std::numeric::uint64"
            | "std::numeric::float32"
            | "std::numeric::float64"
            | "std::numeric::intptr"
            | "std::numeric::uintptr"
            | "system::single"
            | "std::single"
            | "system::double"
            | "std::double"
            | "system::boolean"
            | "std::boolean"
            | "system::int16"
            | "std::int16"
            | "system::uint16"
            | "std::uint16"
            | "system::sbyte"
            | "std::sbyte"
            | "system::byte"
            | "std::byte"
    )
}

fn is_128_bit_integer_name(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "i128"
            | "int128"
            | "u128"
            | "uint128"
            | "system::int128"
            | "system::uint128"
            | "std::int128"
            | "std::uint128"
    )
}

fn enum_layout_is_scalar(enum_layout: &EnumLayout) -> bool {
    let size = enum_layout.size.unwrap_or(0);
    if size == 0 || size > 8 {
        return false;
    }
    if enum_layout
        .variants
        .iter()
        .any(|variant| !variant.fields.is_empty() || !variant.positional.is_empty())
    {
        return false;
    }
    true
}

pub(crate) fn local_requires_memory(ty: &Ty, layouts: &TypeLayoutTable) -> bool {
    match ty {
        Ty::Unit | Ty::Unknown => false,
        Ty::Array(_) | Ty::Vec(_) => lookup_layout(layouts, ty).is_some(),
        Ty::Vector(_) => panic!(
            "[TYPE0704] WASM backend does not yet support SIMD vectors; enable wasm_simd128 or use the LLVM backend until scalar fallback is implemented",
        ),
        Ty::Span(_) | Ty::ReadOnlySpan(_) => lookup_layout(layouts, ty).is_some(),
        Ty::Pointer(_) | Ty::Ref(_) | Ty::Rc(_) | Ty::Arc(_) => false,
        Ty::Fn(fn_ty) => !matches!(fn_ty.abi, crate::mir::Abi::Extern(_)),
        Ty::String => true,
        Ty::Str => false,
        Ty::Tuple(_) => lookup_layout(layouts, ty).is_some(),
        Ty::Nullable(_) => lookup_layout(layouts, ty).is_some(),
        Ty::Named(name) => {
            let canonical = ty.canonical_name();
            if canonical.starts_with("Std::Sync::Atomic") || name.as_str().contains("Atomic") {
                return true;
            }
            if is_128_bit_integer_name(name.as_str()) || is_128_bit_integer_name(&canonical) {
                return true;
            }
            if is_scalar_named(name) {
                return false;
            }
            if layouts.class_layout_info(name.as_str()).is_some()
                || layouts.class_layout_info(&canonical).is_some()
            {
                return false;
            }
            match lookup_layout(layouts, ty) {
                Some(TypeLayout::Class(_)) => false,
                Some(TypeLayout::Struct(layout)) if layout.class.is_some() => false,
                Some(TypeLayout::Enum(enum_layout)) if enum_layout_is_scalar(enum_layout) => false,
                Some(_) => true,
                None => false,
            }
        }
        Ty::TraitObject(_) => true,
    }
}

pub(crate) fn compute_aggregate_allocation(
    ty: &Ty,
    layouts: &TypeLayoutTable,
) -> Option<AggregateAllocation> {
    if matches!(ty, Ty::Vector(_)) {
        panic!(
            "[TYPE0704] WASM backend does not yet support SIMD vectors; enable wasm_simd128 or use the LLVM backend until scalar fallback is implemented",
        )
    }
    if matches!(ty, Ty::TraitObject(_)) {
        return Some(AggregateAllocation {
            offset: 0,
            size: 16,
            align: 8,
        });
    }
    if let Ty::Named(name) = ty {
        if is_128_bit_integer_name(name.as_str()) {
            return Some(AggregateAllocation {
                offset: 0,
                size: 16,
                align: 16,
            });
        }
    }
    if let Ty::Fn(fn_ty) = ty {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            return None;
        }
        let canonical = fn_ty.canonical_name();
        let layout = layouts.types.get(&canonical)?;
        match layout {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => {
                let size =
                    ensure_u32(data.size?, "fn pointer layout size exceeds wasm32 limits").ok()?;
                let align = ensure_u32(
                    data.align?.max(1),
                    "fn pointer layout alignment exceeds wasm32 limits",
                )
                .ok()?;
                return Some(AggregateAllocation {
                    offset: 0,
                    size,
                    align,
                });
            }
            _ => return None,
        }
    }
    let layout = lookup_layout(layouts, ty)?;
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => {
            let size = ensure_u32(
                data.size?,
                "aggregate layout size exceeds WebAssembly limits",
            )
            .ok()?;
            let align = ensure_u32(
                data.align?.max(1),
                "aggregate layout alignment exceeds WebAssembly limits",
            )
            .ok()?;
            Some(AggregateAllocation {
                offset: 0,
                size,
                align,
            })
        }
        TypeLayout::Enum(data) => {
            let size =
                ensure_u32(data.size?, "enum layout size exceeds WebAssembly limits").ok()?;
            let align = ensure_u32(
                data.align?.max(1),
                "enum layout alignment exceeds WebAssembly limits",
            )
            .ok()?;
            Some(AggregateAllocation {
                offset: 0,
                size,
                align,
            })
        }
        TypeLayout::Union(data) => {
            let size =
                ensure_u32(data.size?, "union layout size exceeds WebAssembly limits").ok()?;
            let align = ensure_u32(
                data.align?.max(1),
                "union layout alignment exceeds WebAssembly limits",
            )
            .ok()?;
            Some(AggregateAllocation {
                offset: 0,
                size,
                align,
            })
        }
    }
}

pub(crate) fn lookup_layout<'a>(layouts: &'a TypeLayoutTable, ty: &Ty) -> Option<&'a TypeLayout> {
    match ty {
        Ty::Named(name) => {
            let canonical = ty.canonical_name();
            let canonical_key = canonical.replace('.', "::");
            if let Some(layout) = layouts.types.get(&canonical) {
                return Some(layout);
            }
            if let Some(layout) = lookup_generated_async_layout(layouts, &canonical_key) {
                return Some(layout);
            }
            if let Some(layout) = layouts.types.get(name.as_str()) {
                return Some(layout);
            }
            if canonical_key != canonical {
                if let Some(layout) = layouts.types.get(&canonical_key) {
                    return Some(layout);
                }
            }
            let name_key = name.as_str().replace('.', "::");
            if name_key != name.as_str() {
                if let Some(layout) = layouts.types.get(&name_key) {
                    return Some(layout);
                }
            }
            lookup_layout_by_suffix(layouts, &name_key)
                .or_else(|| lookup_generated_async_layout(layouts, &name_key))
        }
        Ty::Tuple(tuple) => {
            let canonical = tuple.canonical_name();
            layouts.types.get(&canonical)
        }
        Ty::Fn(fn_ty) => {
            let canonical = fn_ty.canonical_name();
            layouts.types.get(&canonical)
        }
        Ty::Vec(vec_ty) => {
            let canonical = Ty::Vec(vec_ty.clone()).canonical_name();
            layouts.types.get(&canonical)
        }
        Ty::Array(array_ty) => {
            let canonical = Ty::Array(array_ty.clone()).canonical_name();
            layouts.types.get(&canonical)
        }
        Ty::Span(span_ty) => {
            let canonical = Ty::Span(span_ty.clone()).canonical_name();
            layouts
                .types
                .get(&canonical)
                .or_else(|| layouts.types.get("Std::Span::SpanPtr"))
                .or_else(|| layouts.types.get("Std::Runtime::Collections::SpanPtr"))
        }
        Ty::ReadOnlySpan(span_ty) => {
            let canonical = Ty::ReadOnlySpan(span_ty.clone()).canonical_name();
            layouts
                .types
                .get(&canonical)
                .or_else(|| layouts.types.get("Std::Span::ReadOnlySpanPtr"))
                .or_else(|| {
                    layouts
                        .types
                        .get("Std::Runtime::Collections::ReadOnlySpanPtr")
                })
        }
        Ty::String => layouts
            .types
            .get("string")
            .or_else(|| layouts.types.get("Std::String"))
            .or_else(|| layouts.types.get("Std.String"))
            .or_else(|| layouts.types.get("System::String"))
            .or_else(|| layouts.types.get("System.String"))
            .or_else(|| layouts.types.get("String")),
        Ty::Str => layouts
            .types
            .get("str")
            .or_else(|| layouts.types.get("Std::Str"))
            .or_else(|| layouts.types.get("Std.Str"))
            .or_else(|| layouts.types.get("System::Str"))
            .or_else(|| layouts.types.get("System.Str"))
            .or_else(|| layouts.types.get("Str")),
        Ty::Nullable(inner) => {
            let canonical = format!("{}?", inner.canonical_name());
            layouts.types.get(&canonical)
        }
        _ => None,
    }
}

pub(crate) fn align_to(offset: u32, align: u32) -> u32 {
    if align <= 1 {
        return offset;
    }
    let mask = align - 1;
    (offset + mask) & !mask
}

fn lookup_layout_by_suffix<'a>(
    layouts: &'a TypeLayoutTable,
    suffix: &str,
) -> Option<&'a TypeLayout> {
    let suffix_normalized = suffix.replace('.', "::");
    let suffix_segment = suffix_normalized
        .rsplit("::")
        .next()
        .unwrap_or(suffix_normalized.as_str());
    let suffix = suffix_segment
        .split_once('<')
        .map(|(base, _)| base)
        .unwrap_or(suffix_segment);

    let mut candidate: Option<(&TypeLayout, &str)> = None;
    for (key, layout) in &layouts.types {
        let key_segment = key.rsplit("::").next().unwrap_or(key.as_str());
        let key_base = key_segment
            .split_once('<')
            .map(|(base, _)| base)
            .unwrap_or(key_segment);
        if key_base == suffix {
            let key_is_generic = key_segment.contains('<');
            match candidate {
                None => candidate = Some((layout, key.as_str())),
                Some((_existing_layout, existing_name)) => {
                    let existing_segment =
                        existing_name.rsplit("::").next().unwrap_or(existing_name);
                    let existing_is_generic = existing_segment.contains('<');
                    if existing_is_generic && !key_is_generic {
                        candidate = Some((layout, key.as_str()));
                        continue;
                    }
                    if !existing_is_generic && key_is_generic {
                        continue;
                    }
                    if suffix == "Time" {
                        let existing_datetime = existing_name.contains("::Datetime::");
                        let new_datetime = key.contains("::Datetime::");
                        if new_datetime && !existing_datetime {
                            candidate = Some((layout, key.as_str()));
                            continue;
                        }
                        if existing_datetime && !new_datetime {
                            continue;
                        }
                    }
                    let existing_std = existing_name.contains("Std::");
                    let new_std = key.contains("Std::");
                    if new_std && !existing_std {
                        candidate = Some((layout, key.as_str()));
                        continue;
                    }
                    if existing_std && !new_std {
                        continue;
                    }
                    if suffix == "SpanPtr" || suffix == "ReadOnlySpanPtr" {
                        let existing_runtime = existing_name.contains("Runtime::Collections::");
                        let new_runtime = key.contains("Runtime::Collections::");
                        if existing_runtime && !new_runtime {
                            continue;
                        }
                        if new_runtime && !existing_runtime {
                            candidate = Some((layout, key.as_str()));
                            continue;
                        }
                    }
                    let existing_startup = existing_name.contains("Std::Runtime::Startup::");
                    let new_startup = key.contains("Std::Runtime::Startup::");
                    if existing_startup && !new_startup {
                        continue;
                    }
                    if !existing_startup && new_startup {
                        candidate = Some((layout, key.as_str()));
                        continue;
                    }
                    let existing_async = existing_name.starts_with("Std::Async::");
                    let new_async = key.starts_with("Std::Async::");
                    if new_async && !existing_async {
                        candidate = Some((layout, key.as_str()));
                        continue;
                    }
                    if existing_async && !new_async {
                        continue;
                    }
                    return None;
                }
            }
        }
    }
    candidate.map(|(layout, _)| layout)
}

static GENERATED_LAYOUTS: OnceLock<RwLock<HashMap<String, &'static TypeLayout>>> = OnceLock::new();

fn generated_layouts() -> &'static RwLock<HashMap<String, &'static TypeLayout>> {
    GENERATED_LAYOUTS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn lookup_generated_async_layout<'a>(
    layouts: &'a TypeLayoutTable,
    name: &str,
) -> Option<&'static TypeLayout> {
    if !name.starts_with("Std::Async::Future<") && !name.starts_with("Std::Async::Task<") {
        return None;
    }
    let mut cache = generated_layouts().write().ok()?;
    if let Some(existing) = cache.get(name).copied() {
        return Some(existing);
    }

    let layout = synthesize_async_layout(layouts, name)?;
    let leaked = Box::leak(Box::new(layout));
    cache.insert(name.to_string(), leaked);
    Some(leaked)
}

fn synthesize_async_layout(layouts: &TypeLayoutTable, name: &str) -> Option<TypeLayout> {
    if let Some(inner) = name.strip_prefix("Std::Async::Future<") {
        let inner_ty = parse_single_generic(inner)?;
        return Some(TypeLayout::Struct(generate_future_layout(
            layouts, &inner_ty,
        )?));
    }
    if let Some(inner) = name.strip_prefix("Std::Async::Task<") {
        let inner_ty = parse_single_generic(inner)?;
        return Some(TypeLayout::Struct(generate_task_layout(
            layouts, &inner_ty,
        )?));
    }
    None
}

fn parse_single_generic(name_with_suffix: &str) -> Option<Ty> {
    let mut depth = 0usize;
    let mut end = name_with_suffix.len();
    for (i, ch) in name_with_suffix.char_indices().rev() {
        if ch == '>' {
            depth += 1;
            end = i;
            continue;
        }
        if depth > 0 {
            break;
        }
    }
    let trimmed = name_with_suffix[..end].trim_end_matches('>');
    if trimmed.is_empty() {
        return None;
    }
    Some(Ty::named(trimmed))
}

fn make_field(name: &str, ty: Ty, index: u32, offset: usize) -> FieldLayout {
    FieldLayout {
        name: name.into(),
        ty,
        index,
        offset: Some(offset),
        span: None,
        mmio: None,
        display_name: None,
        is_required: false,
        is_nullable: false,
        is_readonly: false,
        view_of: None,
    }
}

fn generate_future_layout(layouts: &TypeLayoutTable, result_ty: &Ty) -> Option<StructLayout> {
    let header_ty = Ty::named("Std::Async::FutureHeader");
    let (header_size, header_align) = layouts.size_and_align_for_ty(&header_ty)?;
    let (bool_size, bool_align) = layouts.size_and_align_for_ty(&Ty::named("bool"))?;
    let (result_size, result_align) = layouts.size_and_align_for_ty(result_ty)?;
    let mut offset = align_to_usize(0, header_align);
    let header_offset = offset;
    offset = align_to_usize(offset + header_size, bool_align);
    let completed_offset = offset;
    offset = align_to_usize(offset + bool_size, result_align);
    let result_offset = offset;
    let total_size = align_to_usize(result_offset + result_size, header_align.max(result_align));
    Some(StructLayout {
        name: format!("Std::Async::Future<{}>", result_ty.canonical_name()),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![
            make_field("Header", header_ty, 0, header_offset),
            make_field("Completed", Ty::named("bool"), 1, completed_offset),
            make_field("Result", result_ty.clone(), 2, result_offset),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(total_size),
        align: Some(header_align.max(result_align).max(bool_align)),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    })
}

fn generate_task_layout(layouts: &TypeLayoutTable, inner_ty: &Ty) -> Option<StructLayout> {
    let header_ty = Ty::named("Std::Async::FutureHeader");
    let (header_size, header_align) = layouts.size_and_align_for_ty(&header_ty)?;
    let (flags_size, flags_align) = layouts.size_and_align_for_ty(&Ty::named("uint"))?;
    let future_ty = Ty::named_generic(
        "Std::Async::Future",
        vec![crate::mir::GenericArg::Type(inner_ty.clone())],
    );
    let (future_size, future_align) = layouts.size_and_align_for_ty(&future_ty).or_else(|| {
        generate_future_layout(layouts, inner_ty)
            .and_then(|layout| Some((layout.size?, layout.align?)))
    })?;
    let pointer_align = layouts
        .size_and_align_for_ty(&Ty::named("isize"))
        .map(|(_, a)| a)
        .unwrap_or(header_align.max(future_align));
    let base_size = align_to_usize(header_size + flags_size, pointer_align);
    let inner_offset = align_to_usize(base_size, future_align);
    let total_size = align_to_usize(inner_offset + future_size, pointer_align.max(future_align));
    Some(StructLayout {
        name: format!("Std::Async::Task<{}>", inner_ty.canonical_name()),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![
            make_field("Header", header_ty, 0, 0),
            make_field("Flags", Ty::named("uint"), 1, header_size),
            make_field("InnerFuture", future_ty, 2, inner_offset),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(total_size),
        align: Some(
            pointer_align
                .max(future_align)
                .max(header_align)
                .max(flags_align),
        ),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases: vec!["Std::Async::Task".into()],
            vtable_offset: Some(0),
        }),
    })
}

fn align_to_usize(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        (value + align - 1) / align * align
    }
}
