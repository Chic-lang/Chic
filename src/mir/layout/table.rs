//! Type layout metadata and auto-trait computation for MIR.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

use crate::frontend::diagnostics::Span;
use crate::mir::casts::{IntInfo, pointer_depth};
use crate::mir::{ArrayTy, FnTy, GenericArg, ReadOnlySpanTy, SpanTy, Ty};
use crate::type_metadata::TypeFlags;

use super::auto_traits::{self, AutoTraitOverride, AutoTraitSet, AutoTraitStatus};
use super::nullable::nullable_type_name;
use crate::primitives::PrimitiveRegistry;

pub(crate) const MIN_ALIGN: usize = 1;

#[derive(Clone, Copy)]
struct PointerInfo {
    size: usize,
    align: usize,
}

fn pointer_info() -> PointerInfo {
    let raw = POINTER_INFO.with(|info| info.get());
    PointerInfo {
        size: (raw & 0xFFFF_FFFF) as usize,
        align: (raw >> 32) as usize,
    }
}

thread_local! {
    static POINTER_INFO: Cell<u64> = Cell::new((8u64 << 32) | 8u64);
    static SIZE_ALIGN_VISITING: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

struct SizeAlignGuard {
    key: String,
    active: bool,
}

impl Drop for SizeAlignGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        SIZE_ALIGN_VISITING.with(|visiting| {
            visiting.borrow_mut().remove(&self.key);
        });
    }
}

pub fn configure_pointer_width(size: usize, align: usize) {
    let size = u64::try_from(size.max(1)).unwrap_or(u64::from(u32::MAX));
    let align = u64::try_from(align.max(1)).unwrap_or(u64::from(u32::MAX));
    POINTER_INFO.with(|info| info.set((align << 32) | (size & 0xFFFF_FFFF)));
}

pub fn pointer_size() -> usize {
    pointer_info().size
}

pub fn pointer_align() -> usize {
    pointer_info().align
}

pub(crate) fn align_to(value: usize, align: usize) -> usize {
    if align <= 1 {
        value
    } else {
        value.div_ceil(align) * align
    }
}

/// Registry with layout/type metadata referenced by MIR backends.
#[derive(Debug, Clone)]
pub struct TypeLayoutTable {
    pub types: HashMap<String, TypeLayout>,
    type_flags: HashMap<String, TypeFlags>,
    delegate_signatures: HashMap<String, FnTy>,
    delegate_auto_traits: HashMap<String, AutoTraitSet>,
    type_generic_params: HashMap<String, Vec<String>>,
    pub primitive_registry: PrimitiveRegistry,
}

impl Default for TypeLayoutTable {
    fn default() -> Self {
        let mut table = TypeLayoutTable {
            types: HashMap::new(),
            type_flags: HashMap::new(),
            delegate_signatures: HashMap::new(),
            delegate_auto_traits: HashMap::new(),
            type_generic_params: HashMap::new(),
            primitive_registry: PrimitiveRegistry::with_builtins(),
        };
        table.insert_builtin_string_layouts();
        table.insert_builtin_collections_layouts();
        table.insert_builtin_shared_layouts();
        table.insert_builtin_startup_layouts();
        table.insert_builtin_async_layouts();
        table.insert_builtin_decimal_layouts();
        table.insert_builtin_span_layouts();
        table.insert_builtin_memory_layouts();
        table.insert_builtin_accelerator_layouts();
        table
    }
}

impl TypeLayoutTable {
    pub(crate) fn record_type_generic_params(
        &mut self,
        name: impl Into<String>,
        params: Vec<String>,
    ) {
        let key = name.into();
        if params.is_empty() {
            return;
        }
        self.type_generic_params.entry(key).or_insert(params);
    }

    pub(crate) fn type_generic_params_for(&self, name: &str) -> Option<&[String]> {
        let key = self.resolve_type_key(name).unwrap_or(name);
        self.type_generic_params
            .get(key)
            .map(|params| params.as_slice())
    }

    #[must_use]
    pub fn type_flags_for_ty(&self, ty: &Ty) -> TypeFlags {
        match ty {
            Ty::Named(name) => self.type_flags_for_name(name.canonical_path()),
            Ty::Nullable(inner) => {
                let key = nullable_type_name(inner);
                self.type_flags_for_name(key)
            }
            _ => TypeFlags::empty(),
        }
    }

    #[must_use]
    pub fn ty_is_fallible(&self, ty: &Ty) -> bool {
        self.type_flags_for_ty(ty).contains(TypeFlags::FALLIBLE)
    }

    #[must_use]
    pub fn type_flags_for_name(&self, name: impl Into<String>) -> TypeFlags {
        let candidate = name.into();
        if let Some(key) = self.resolve_type_key(&candidate) {
            if let Some(flags) = self.type_flags.get(key) {
                return *flags;
            }
        }
        if Self::matches_exception_name(&candidate) || Self::matches_result_name(&candidate) {
            return TypeFlags::FALLIBLE;
        }
        TypeFlags::empty()
    }

    pub fn record_delegate_signature(&mut self, name: impl Into<String>, signature: FnTy) {
        let key = name.into();
        self.ensure_fn_layout(&signature);
        self.delegate_signatures.insert(key, signature);
    }

    #[must_use]
    pub fn delegate_signature(&self, name: &str) -> Option<&FnTy> {
        if let Some(sig) = self.delegate_signatures.get(name) {
            return Some(sig);
        }
        self.delegate_signatures
            .iter()
            .find_map(|(qualified, sig)| {
                qualified
                    .rsplit("::")
                    .next()
                    .is_some_and(|short| short == name)
                    .then_some(sig)
            })
    }

    pub fn record_delegate_auto_traits(&mut self, name: impl Into<String>, traits: AutoTraitSet) {
        let key = name.into();
        self.delegate_auto_traits
            .entry(key)
            .and_modify(|existing| {
                existing.thread_safe = existing.thread_safe.combine(traits.thread_safe);
                existing.shareable = existing.shareable.combine(traits.shareable);
                existing.copy = existing.copy.combine(traits.copy);
            })
            .or_insert(traits);
    }

    pub(crate) fn delegate_auto_traits_for_key(&self, key: &str) -> Option<AutoTraitSet> {
        if let Some(traits) = self.delegate_auto_traits.get(key) {
            return Some(*traits);
        }
        self.delegate_auto_traits
            .iter()
            .find_map(|(qualified, traits)| {
                qualified
                    .rsplit("::")
                    .next()
                    .is_some_and(|short| short == key)
                    .then_some(*traits)
            })
    }

    pub fn add_type_flags(&mut self, name: impl Into<String>, flags: TypeFlags) {
        if flags.is_empty() {
            return;
        }
        let key = name.into();
        self.type_flags
            .entry(key)
            .and_modify(|current| current.insert(flags))
            .or_insert(flags);
    }

    pub fn finalize_type_flags(&mut self) {
        let keys: Vec<String> = self.types.keys().cloned().collect();
        for name in keys {
            let mut visited = HashSet::new();
            let _ = self.mark_exception_hierarchy(&name, &mut visited);
            if Self::matches_result_name(&name) {
                self.add_type_flags(name.clone(), TypeFlags::FALLIBLE);
            }
        }
    }

    pub(crate) fn ensure_span_layout(&mut self, span: &SpanTy) {
        let name = Ty::Span(span.clone()).canonical_name();
        if self.types.contains_key(&name) {
            return;
        }
        let word_size = pointer_size();
        let word_align = pointer_align();

        let mut fields = Vec::new();
        let mut offset = 0usize;

        fields.push(make_field(
            "data",
            Ty::named("Std::Runtime::Collections::ValueMutPtr"),
            0,
            offset,
        ));
        offset += align_to(word_size * 3, word_align);

        offset = align_to(offset, word_align);
        fields.push(make_field("len", Ty::named("usize"), 1, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_size", Ty::named("usize"), 2, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_align", Ty::named("usize"), 3, offset));
        offset += word_size;

        let align = word_align.max(1);
        let size = align_to(offset, align);

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: Some(ListLayout {
                element_index: Some(0),
                length_index: Some(1),
                span: None,
            }),
            size: Some(size),
            align: Some(align),
            is_readonly: false,
            is_intrinsic: true,
            allow_cross_inline: true,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };
        self.types.insert(name, TypeLayout::Struct(layout));
    }

    pub(crate) fn ensure_array_layout(&mut self, array: &ArrayTy) {
        let name = Ty::Array(array.clone()).canonical_name();
        if self.types.contains_key(&name) {
            return;
        }

        let word_size = pointer_size();
        let word_align = pointer_align();

        let mut fields = Vec::new();
        let mut offset = 0usize;

        fields.push(make_field("ptr", Ty::named("byte*"), 0, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("len", Ty::named("usize"), 1, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("cap", Ty::named("usize"), 2, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_size", Ty::named("usize"), 3, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_align", Ty::named("usize"), 4, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("drop_fn", Ty::named("isize"), 5, offset));
        offset += word_size;

        // Arrays are currently backed by the Chic runtime Vec representation (see
        // `chic_rt_vec_with_capacity`), so their intrinsic layout must match `VecPtr`'s ABI.
        offset = align_to(offset, word_align);
        fields.push(make_field("region_ptr", Ty::named("byte*"), 6, offset));
        offset += word_size;

        offset = align_to(offset, 1);
        fields.push(make_field("uses_inline", Ty::named("byte"), 7, offset));
        offset += 1;

        offset = align_to(offset, 1);
        fields.push(make_field(
            "inline_pad",
            Ty::named("Std::Runtime::Collections::InlinePadding7"),
            8,
            offset,
        ));
        offset += 7;

        offset = align_to(offset, 1);
        fields.push(make_field(
            "inline_storage",
            Ty::named("Std::Runtime::Collections::InlineBytes64"),
            9,
            offset,
        ));
        offset += 64;

        let align = word_align.max(1);
        let size = align_to(offset, align);

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: Some(ListLayout {
                element_index: Some(0),
                length_index: Some(1),
                span: None,
            }),
            size: Some(size),
            align: Some(align),
            is_readonly: false,
            is_intrinsic: true,
            allow_cross_inline: true,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };
        self.types.insert(name, TypeLayout::Struct(layout));
    }

    pub(crate) fn ensure_readonly_span_layout(&mut self, span: &ReadOnlySpanTy) {
        let name = Ty::ReadOnlySpan(span.clone()).canonical_name();
        if self.types.contains_key(&name) {
            return;
        }
        let word_size = pointer_size();
        let word_align = pointer_align();

        let mut fields = Vec::new();
        let mut offset = 0usize;

        fields.push(make_field(
            "data",
            Ty::named("Std::Runtime::Collections::ValueConstPtr"),
            0,
            offset,
        ));
        offset += align_to(word_size * 3, word_align);

        offset = align_to(offset, word_align);
        fields.push(make_field("len", Ty::named("usize"), 1, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_size", Ty::named("usize"), 2, offset));
        offset += word_size;

        offset = align_to(offset, word_align);
        fields.push(make_field("elem_align", Ty::named("usize"), 3, offset));
        offset += word_size;

        let align = word_align.max(1);
        let size = align_to(offset, align);

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: Some(ListLayout {
                element_index: Some(0),
                length_index: Some(1),
                span: None,
            }),
            size: Some(size),
            align: Some(align),
            is_readonly: false,
            is_intrinsic: true,
            allow_cross_inline: true,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };
        self.types.insert(name, TypeLayout::Struct(layout));
    }

    pub(crate) fn ensure_interface_layout(&mut self, name: &str) {
        if self.types.contains_key(name) {
            return;
        }

        let layout = StructLayout {
            name: name.to_string(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(pointer_size()),
            align: Some(pointer_align()),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: Some(ClassLayoutInfo {
                kind: ClassLayoutKind::Class,
                bases: Vec::new(),
                vtable_offset: Some(0),
            }),
        };

        self.types
            .insert(name.to_string(), TypeLayout::Class(layout));
    }

    pub fn backfill_missing_offsets(&mut self) {
        let max_passes = self.types.len().max(1);
        for _ in 0..max_passes {
            let mut changed = false;
            let keys: Vec<String> = self.types.keys().cloned().collect();
            for key in keys {
                let Some(layout) = self.types.remove(&key) else {
                    continue;
                };
                let (updated, did_change) = self.backfill_layout_once(layout);
                changed |= did_change;
                self.types.insert(key, updated);
            }
            if !changed {
                break;
            }
        }
    }

    fn backfill_layout_once(&self, layout: TypeLayout) -> (TypeLayout, bool) {
        match layout {
            TypeLayout::Struct(mut struct_layout) => {
                let changed = backfill_struct_like(self, &mut struct_layout);
                (TypeLayout::Struct(struct_layout), changed)
            }
            TypeLayout::Class(mut struct_layout) => {
                let changed = backfill_struct_like(self, &mut struct_layout);
                (TypeLayout::Class(struct_layout), changed)
            }
            other => (other, false),
        }
    }

    pub(crate) fn size_and_align_for_ty(&self, ty: &Ty) -> Option<(usize, usize)> {
        let key = ty.canonical_name();
        let active = SIZE_ALIGN_VISITING.with(|visiting| {
            let mut visiting = visiting.borrow_mut();
            if visiting.contains(&key) {
                return false;
            }
            visiting.insert(key.clone());
            true
        });
        if !active {
            return Some((pointer_size(), pointer_align()));
        }
        let _guard = SizeAlignGuard { key, active };
        match ty {
            Ty::Unit => Some((0, MIN_ALIGN)),
            Ty::Unknown => Some((pointer_size(), pointer_align())),
            Ty::Fn(fn_ty) => {
                if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
                    return Some((pointer_size(), pointer_align()));
                }
                let name = fn_ty.canonical_name();
                if let Some(layout) = self.layout_for_name(&name) {
                    match layout {
                        TypeLayout::Struct(data) | TypeLayout::Class(data) => {
                            return data.size.zip(data.align);
                        }
                        TypeLayout::Enum(enum_layout) => {
                            return enum_layout.size.zip(enum_layout.align);
                        }
                        TypeLayout::Union(union_layout) => {
                            return union_layout.size.zip(union_layout.align);
                        }
                    }
                }
                // Fallback to the intrinsic function-pointer layout shape.
                return Some((pointer_size() * 6, pointer_align()));
            }
            Ty::Rc(_) | Ty::Arc(_) | Ty::Pointer(_) | Ty::Ref(_) => {
                Some((pointer_size(), pointer_align()))
            }
            Ty::TraitObject(_) => Some((pointer_size() * 2, pointer_align())),
            Ty::Array(array) => {
                let name = Ty::Array(array.clone()).canonical_name();
                self.layout_for_name(&name).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
            Ty::Vec(vec) => {
                let name = Ty::Vec(vec.clone()).canonical_name();
                self.layout_for_name(&name).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
            Ty::Span(span) => {
                let name = Ty::Span(span.clone()).canonical_name();
                self.layout_for_name(&name).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
            Ty::ReadOnlySpan(span) => {
                let name = Ty::ReadOnlySpan(span.clone()).canonical_name();
                self.layout_for_name(&name).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
            Ty::Vector(vector) => {
                let element_layout = self.size_and_align_for_ty(&vector.element)?;
                let width_bits = (element_layout.0 * 8).checked_mul(vector.lanes as usize)?;
                let size = width_bits / 8;
                let vector_align = if width_bits >= 256 {
                    32
                } else if width_bits >= 128 {
                    16
                } else {
                    element_layout.1
                };
                let align = std::cmp::max(element_layout.1, vector_align);
                Some((size, align))
            }
            Ty::Tuple(tuple) => {
                let name = tuple.canonical_name();
                self.layout_for_name(&name).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
            Ty::String => self.size_and_align_for_named_type("string"),
            Ty::Str => self.size_and_align_for_named_type("str"),
            Ty::Named(name) => {
                if name.args().is_empty() {
                    self.size_and_align_for_named_type(name.as_str())
                } else {
                    let canonical = ty.canonical_name();
                    self.size_and_align_for_named_type(&canonical)
                }
            }
            Ty::Nullable(inner) => {
                let key = nullable_type_name(inner);
                self.layout_for_name(&key).and_then(|layout| match layout {
                    TypeLayout::Struct(data) | TypeLayout::Class(data) => data.size.zip(data.align),
                    _ => None,
                })
            }
        }
    }

    fn size_and_align_for_named_type(&self, name: &str) -> Option<(usize, usize)> {
        if pointer_depth(name) > 0 {
            return Some((pointer_size(), pointer_align()));
        }
        if let Some((size, align)) = self
            .primitive_registry
            .size_align_for_name(name, pointer_size() as u32, pointer_align() as u32)
            .map(|(size, align)| (size as usize, align as usize))
        {
            return Some((size, align));
        }
        if let Some(layout) = self.layout_for_name(name) {
            return match layout {
                TypeLayout::Struct(layout) => layout.size.zip(layout.align),
                TypeLayout::Class(layout) => {
                    let is_marker_interface = layout.fields.is_empty()
                        && layout
                            .class
                            .as_ref()
                            .and_then(|info| info.vtable_offset)
                            .is_some_and(|offset| offset == 0)
                        && layout.size == Some(pointer_size())
                        && layout.align == Some(pointer_align());
                    if is_marker_interface {
                        Some((pointer_size() * 2, pointer_align()))
                    } else {
                        Some((pointer_size(), pointer_align()))
                    }
                }
                TypeLayout::Enum(layout) => layout.size.zip(layout.align),
                TypeLayout::Union(layout) => layout.size.zip(layout.align),
            };
        }

        // If resolution failed due to ambiguous short names, fall back to the same heuristic as
        // LLVM type lowering (prefer `Std::*` value types, then any non-class layout).
        let canonical = name.replace('.', "::");
        let short = strip_generics(canonical.rsplit("::").next().unwrap_or(&canonical));
        let short_matches: Vec<_> = self
            .types
            .iter()
            .filter(|(key, _)| strip_generics(key.rsplit("::").next().unwrap_or(key)) == short)
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
            if let Some((_, layout)) = preferred {
                return match layout {
                    TypeLayout::Struct(layout) | TypeLayout::Class(layout) => {
                        layout.size.zip(layout.align)
                    }
                    TypeLayout::Enum(layout) => layout.size.zip(layout.align),
                    TypeLayout::Union(layout) => layout.size.zip(layout.align),
                };
            }
        }

        Some((pointer_size(), pointer_align()))
    }

    pub fn layout_for_name(&self, name: &str) -> Option<&TypeLayout> {
        if let Some(layout) = self.types.get(name) {
            if matches!(layout, TypeLayout::Struct(_)) && name.contains('<') {
                let base = strip_generics(name);
                if let Some(base_key) = self.resolve_type_key(base) {
                    if let Some(TypeLayout::Class(_)) = self.types.get(base_key) {
                        return self.types.get(base_key);
                    }
                }
            }
            return Some(layout);
        }
        if let Some(layout) = self.generated_async_layout(name) {
            return Some(layout);
        }
        if let Some(layout) = self.generated_span_layout(name) {
            return Some(layout);
        }
        let key = self.resolve_type_key(name)?;
        self.types
            .get(key)
            .or_else(|| self.generated_async_layout(key))
            .or_else(|| self.generated_span_layout(key))
    }

    pub(crate) fn resolve_type_key<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        // Prefer native runtime decimal pointer layouts when the short name is ambiguous with
        // other Std.Numeric variants. This keeps the native runtime as the default target surface
        // even when type names are referenced without a namespace.
        if matches!(
            name,
            "DecimalConstPtr" | "Std::Runtime::Native::DecimalConstPtr"
        ) || name.ends_with("::Decimal::DecimalConstPtr")
        {
            if let Some(entry) = self
                .types
                .get_key_value("Std::Runtime::Native::DecimalConstPtr")
            {
                return Some(entry.0.as_str());
            }
        }
        if matches!(
            name,
            "DecimalMutPtr" | "Std::Runtime::Native::DecimalMutPtr"
        ) || name.ends_with("::Decimal::DecimalMutPtr")
        {
            if let Some(entry) = self
                .types
                .get_key_value("Std::Runtime::Native::DecimalMutPtr")
            {
                return Some(entry.0.as_str());
            }
        }

        if self.types.contains_key(name) {
            return Some(name);
        }
        let stripped_name = strip_generics(name);
        if stripped_name != name && self.types.contains_key(stripped_name) {
            return Some(stripped_name);
        }
        if name.contains('.') {
            let canonical = name.replace('.', "::");
            if let Some(entry) = self.types.get_key_value(canonical.as_str()) {
                return Some(entry.0.as_str());
            }
            let canonical_base = strip_generics(&canonical).to_string();
            if let Some(entry) = self.types.get_key_value(canonical_base.as_str()) {
                return Some(entry.0.as_str());
            }
        }
        if let Some(alias_key) = self.async_alias_key(name) {
            return Some(alias_key);
        }

        if name.contains('<') {
            let tail = top_level_tail(name);
            let mut candidate: Option<&String> = None;
            for key in self.types.keys() {
                if !type_fragment_eq(top_level_tail(key), tail) {
                    continue;
                }
                if let Some(existing) = candidate {
                    let existing_qualified = existing.contains("::");
                    let key_qualified = key.contains("::");
                    if existing_qualified && !key_qualified {
                        continue;
                    }
                    if key_qualified && !existing_qualified {
                        candidate = Some(key);
                        continue;
                    }
                    match (
                        existing.contains("Std::Runtime::Startup::"),
                        key.contains("Std::Runtime::Startup::"),
                    ) {
                        (true, false) => continue,
                        (false, true) => {
                            candidate = Some(key);
                            continue;
                        }
                        _ => {
                            let existing_is_class =
                                matches!(self.types.get(existing), Some(TypeLayout::Class(_)));
                            let key_is_class =
                                matches!(self.types.get(key), Some(TypeLayout::Class(_)));
                            if existing_is_class && !key_is_class {
                                candidate = Some(key);
                                continue;
                            }
                            if key_is_class && !existing_is_class {
                                continue;
                            }
                            candidate = Some(if key.as_str() < existing.as_str() {
                                key
                            } else {
                                existing
                            });
                            continue;
                        }
                    }
                }
                candidate = Some(key);
            }
            if let Some(candidate) = candidate {
                return Some(candidate.as_str());
            }
        }

        let stripped = strip_generics(name);
        let short = stripped.rsplit("::").next().unwrap_or(stripped);
        let mut candidate: Option<&String> = None;
        for key in self.types.keys() {
            let key_stripped = strip_generics(key);
            let key_short = key_stripped.rsplit("::").next().unwrap_or(key_stripped);
            if key_stripped == stripped || key_short == short {
                if let Some(existing) = candidate {
                    let existing_qualified = existing.contains("::");
                    let key_qualified = key.contains("::");
                    if existing_qualified && !key_qualified {
                        continue;
                    }
                    if key_qualified && !existing_qualified {
                        candidate = Some(key);
                        continue;
                    }
                    match (
                        existing.contains("Std::Runtime::Startup::"),
                        key.contains("Std::Runtime::Startup::"),
                    ) {
                        (true, false) => continue,
                        (false, true) => {
                            candidate = Some(key);
                            continue;
                        }
                        _ => {
                            let existing_is_class =
                                matches!(self.types.get(existing), Some(TypeLayout::Class(_)));
                            let key_is_class =
                                matches!(self.types.get(key), Some(TypeLayout::Class(_)));
                            if existing_is_class && !key_is_class {
                                candidate = Some(key);
                                continue;
                            }
                            if key_is_class && !existing_is_class {
                                continue;
                            }
                            candidate = Some(if key.as_str() < existing.as_str() {
                                key
                            } else {
                                existing
                            });
                            continue;
                        }
                    }
                }
                candidate = Some(key);
            }
        }
        candidate.map(String::as_str)
    }

    fn async_alias_key(&self, name: &str) -> Option<&str> {
        let alias = canonical_async_alias(name)?;
        self.types
            .get_key_value(alias.as_str())
            .map(|(key, _)| key.as_str())
    }

    fn generated_async_layout(&self, name: &str) -> Option<&TypeLayout> {
        if !is_async_generic_name(name) {
            return None;
        }
        let key = canonical_async_key(name);
        if let Some(existing) = generated_async_layouts().read().ok()?.get(&key) {
            return Some(unsafe { std::mem::transmute::<&TypeLayout, &TypeLayout>(existing) });
        }
        let layout = synthesize_async_layout(self, &key)?;
        let mut cache = generated_async_layouts().write().ok()?;
        let entry = cache.entry(key).or_insert(layout);
        Some(unsafe { std::mem::transmute::<&TypeLayout, &TypeLayout>(entry) })
    }

    fn generated_span_layout(&self, name: &str) -> Option<&TypeLayout> {
        let canonical = name.replace('.', "::");
        let base = strip_generics(&canonical);
        let short = base.rsplit("::").next().unwrap_or(base);
        let is_readonly = short == "ReadOnlySpan";
        let is_span = is_readonly || short == "Span";
        if !is_span {
            return None;
        }
        let key = canonical;
        if let Some(existing) = generated_span_layouts().read().ok()?.get(&key) {
            return Some(unsafe { std::mem::transmute::<&TypeLayout, &TypeLayout>(existing) });
        }
        let layout = synthesize_span_layout(&key, is_readonly)?;
        let mut cache = generated_span_layouts().write().ok()?;
        let entry = cache.entry(key).or_insert(layout);
        Some(unsafe { std::mem::transmute::<&TypeLayout, &TypeLayout>(entry) })
    }

    #[must_use]
    pub fn class_layout_info(&self, name: &str) -> Option<ClassLayoutInfo> {
        let base = auto_traits::strip_generics(name);
        if matches!(base, "Exception" | "System::Exception" | "Std::Exception") {
            return Some(ClassLayoutInfo {
                kind: ClassLayoutKind::Error,
                bases: Vec::new(),
                vtable_offset: None,
            });
        }
        let Some(key) = self.resolve_type_key(base) else {
            return None;
        };
        match self.types.get(key) {
            Some(TypeLayout::Class(layout)) => layout.class.clone(),
            _ => None,
        }
    }

    #[must_use]
    pub fn ty_requires_drop(&self, ty: &Ty) -> bool {
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.ty_requires_drop_inner(ty, &mut cache, &mut visiting)
    }

    #[must_use]
    pub fn type_requires_drop(&self, name: &str) -> bool {
        let mut cache = HashMap::new();
        let mut visiting = HashSet::new();
        self.type_requires_drop_cached(name, &mut cache, &mut visiting)
    }

    #[must_use]
    pub fn type_requires_clone(&self, name: &str) -> bool {
        let traits = self.resolve_auto_traits(name);
        !matches!(traits.copy, AutoTraitStatus::Yes)
    }

    fn ty_requires_drop_inner(
        &self,
        ty: &Ty,
        cache: &mut HashMap<String, bool>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        match ty {
            Ty::String => true,
            Ty::Vec(_) => true,
            Ty::Array(array) => self.ty_requires_drop_inner(&array.element, cache, visiting),
            Ty::Rc(_) | Ty::Arc(_) => true,
            Ty::Span(_) | Ty::ReadOnlySpan(_) => false,
            Ty::Tuple(tuple) => tuple
                .elements
                .iter()
                .any(|element| self.ty_requires_drop_inner(element, cache, visiting)),
            Ty::Fn(_) => false,
            Ty::Pointer(_) => false,
            Ty::Ref(_) => false,
            Ty::Nullable(inner) => self.ty_requires_drop_inner(inner, cache, visiting),
            Ty::TraitObject(_) => true,
            Ty::Named(name) => {
                let trimmed = name.trim_end();
                if trimmed.ends_with('*') && pointer_depth(trimmed) > 0 {
                    return false;
                }
                self.type_requires_drop_cached(trimmed, cache, visiting)
            }
            _ => false,
        }
    }

    fn type_requires_drop_cached(
        &self,
        name: &str,
        cache: &mut HashMap<String, bool>,
        visiting: &mut HashSet<String>,
    ) -> bool {
        let canonical = name.to_string();
        if let Some(result) = cache.get(&canonical) {
            return *result;
        }
        if self.delegate_signature(name).is_some() {
            cache.insert(canonical.clone(), true);
            return true;
        }
        if canonical.trim_start().starts_with("fn") {
            cache.insert(canonical.clone(), false);
            return false;
        }
        if !visiting.insert(canonical.clone()) {
            return false;
        }

        let result =
            self.resolve_type_key(name)
                .and_then(|key| self.types.get(key))
                .map_or(false, |layout| match layout {
                    TypeLayout::Struct(struct_layout) | TypeLayout::Class(struct_layout) => {
                        if struct_layout.dispose.is_some() {
                            true
                        } else {
                            struct_layout.fields.iter().any(|field| {
                                self.ty_requires_drop_inner(&field.ty, cache, visiting)
                            })
                        }
                    }
                    TypeLayout::Enum(enum_layout) => enum_layout.variants.iter().any(|variant| {
                        variant
                            .fields
                            .iter()
                            .any(|field| self.ty_requires_drop_inner(&field.ty, cache, visiting))
                    }),
                    TypeLayout::Union(union_layout) => union_layout
                        .views
                        .iter()
                        .any(|view| self.ty_requires_drop_inner(&view.ty, cache, visiting)),
                });

        visiting.remove(&canonical);
        cache.insert(canonical, result);
        result
    }
    fn mark_exception_hierarchy(&mut self, name: &str, visited: &mut HashSet<String>) -> bool {
        let key = self.resolve_type_key(name).unwrap_or(name);
        let canonical = key.replace('.', "::");
        if visited.contains(&canonical) {
            return false;
        }
        if self
            .type_flags
            .get(canonical.as_str())
            .is_some_and(|flags| flags.contains(TypeFlags::FALLIBLE))
        {
            return true;
        }
        visited.insert(canonical.clone());
        if Self::matches_exception_name(&canonical) {
            self.type_flags
                .entry(canonical.clone())
                .and_modify(|flags| flags.insert(TypeFlags::FALLIBLE))
                .or_insert(TypeFlags::FALLIBLE);
            visited.remove(&canonical);
            return true;
        }

        let inherits = self
            .types
            .get(canonical.as_str())
            .and_then(|layout| match layout {
                TypeLayout::Class(layout) => layout.class.as_ref().map(|info| info.bases.clone()),
                _ => None,
            })
            .map_or(false, |bases| {
                bases.into_iter().any(|base| {
                    let normalized = base.replace('.', "::");
                    self.mark_exception_hierarchy(&normalized, visited)
                })
            });
        if inherits {
            self.type_flags
                .entry(canonical.clone())
                .and_modify(|flags| flags.insert(TypeFlags::FALLIBLE))
                .or_insert(TypeFlags::FALLIBLE);
        }
        visited.remove(&canonical);
        inherits
    }

    fn matches_exception_name(name: &str) -> bool {
        let stripped = strip_generics(name.trim());
        let trimmed = stripped.trim_end_matches('?');
        let canonical = trimmed.replace('.', "::");
        let short = canonical.rsplit("::").next().unwrap_or(&canonical);
        short.ends_with("Exception")
    }

    fn matches_result_name(name: &str) -> bool {
        let stripped = strip_generics(name.trim());
        let canonical = stripped.replace('.', "::");
        let short = canonical.rsplit("::").next().unwrap_or(&canonical);
        short.eq_ignore_ascii_case("Result")
    }
}

fn backfill_struct_like(table: &TypeLayoutTable, layout: &mut StructLayout) -> bool {
    if layout.fields.is_empty() {
        return false;
    }
    if layout.mmio.is_some() {
        return false;
    }

    let pack = layout.packing.and_then(|pack| usize::try_from(pack).ok());
    let mut derived_align = MIN_ALIGN;
    let mut changed = false;
    let mut can_compute_layout = true;
    let mut max_end = 0usize;

    for field in &mut layout.fields {
        let Some((size, natural_align)) = table.size_and_align_for_ty(&field.ty) else {
            can_compute_layout = false;
            continue;
        };
        let mut field_align = natural_align.max(MIN_ALIGN);
        if let Some(pack) = pack {
            field_align = field_align.min(pack.max(MIN_ALIGN));
        }

        derived_align = derived_align.max(field_align);

        let field_offset = if let Some(existing) = field.offset {
            existing
        } else if can_compute_layout {
            let aligned = align_to(max_end, field_align);
            field.offset = Some(aligned);
            changed = true;
            aligned
        } else {
            continue;
        };
        max_end = max_end.max(field_offset.saturating_add(size));
    }

    let requested_align = layout.align.unwrap_or(MIN_ALIGN);
    let computed_align = derived_align.max(requested_align).max(MIN_ALIGN);
    if layout.align != Some(computed_align) {
        layout.align = Some(computed_align);
        changed = true;
    }

    if can_compute_layout {
        let computed_size = align_to(max_end, computed_align);
        if layout.size != Some(computed_size) {
            layout.size = Some(computed_size);
            changed = true;
        }
    }

    changed
}

fn generated_async_layouts() -> &'static RwLock<HashMap<String, TypeLayout>> {
    static GENERATED_ASYNC_LAYOUTS: OnceLock<RwLock<HashMap<String, TypeLayout>>> = OnceLock::new();
    GENERATED_ASYNC_LAYOUTS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn generated_span_layouts() -> &'static RwLock<HashMap<String, TypeLayout>> {
    static GENERATED_SPAN_LAYOUTS: OnceLock<RwLock<HashMap<String, TypeLayout>>> = OnceLock::new();
    GENERATED_SPAN_LAYOUTS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn canonical_async_key(name: &str) -> String {
    name.replace('.', "::")
}

fn is_async_generic_name(name: &str) -> bool {
    let canonical = canonical_async_key(name);
    canonical.starts_with("Std::Async::Future<")
        || canonical.starts_with("Std::Async::Task<")
        || canonical.starts_with("Future<")
        || canonical.starts_with("Task<")
}

fn synthesize_span_layout(name: &str, readonly: bool) -> Option<TypeLayout> {
    let data_ty = if readonly {
        Ty::named("Std::Runtime::Collections::ValueConstPtr")
    } else {
        Ty::named("Std::Runtime::Collections::ValueMutPtr")
    };
    let mut fields = Vec::new();
    let mut offset = 0usize;

    fields.push(make_field("data", data_ty, 0, offset));
    offset += align_to(pointer_size() * 3, pointer_align());

    offset = align_to(offset, pointer_align());
    fields.push(make_field("len", Ty::named("usize"), 1, offset));
    offset += pointer_size();

    offset = align_to(offset, pointer_align());
    fields.push(make_field("elem_size", Ty::named("usize"), 2, offset));
    offset += pointer_size();

    offset = align_to(offset, pointer_align());
    fields.push(make_field("elem_align", Ty::named("usize"), 3, offset));
    offset += pointer_size();

    let align = pointer_align().max(1);
    let size = align_to(offset, align);

    Some(TypeLayout::Struct(StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: Some(ListLayout {
            element_index: Some(0),
            length_index: Some(1),
            span: None,
        }),
        size: Some(size),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }))
}

fn synthesize_async_layout(layouts: &TypeLayoutTable, name: &str) -> Option<TypeLayout> {
    let canonical = name.replace('.', "::");
    if let Some(inner) = canonical
        .strip_prefix("Std::Async::Future<")
        .or_else(|| canonical.strip_prefix("Future<"))
    {
        let inner_ty = parse_single_generic(inner)?;
        return Some(TypeLayout::Struct(generate_future_layout(
            layouts, &inner_ty,
        )?));
    }
    if let Some(inner) = canonical
        .strip_prefix("Std::Async::Task<")
        .or_else(|| canonical.strip_prefix("Task<"))
    {
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
    Some(Ty::named(trimmed.replace('.', "::")))
}

fn generate_future_layout(layouts: &TypeLayoutTable, result_ty: &Ty) -> Option<StructLayout> {
    let header_ty = Ty::named("Std::Async::FutureHeader");
    let (header_size, header_align) = layouts.size_and_align_for_ty(&header_ty)?;
    let (bool_size, bool_align) = layouts.size_and_align_for_ty(&Ty::named("bool"))?;
    let (result_size, result_align) = layouts.size_and_align_for_ty(result_ty)?;
    let mut offset = align_to(0, header_align);
    let header_offset = offset;
    offset = align_to(offset + header_size, bool_align);
    let completed_offset = offset;
    offset = align_to(offset + bool_size, result_align);
    let result_offset = offset;
    let align = header_align.max(result_align).max(bool_align);
    let total_size = align_to(result_offset + result_size, align);
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
        align: Some(align),
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
        vec![GenericArg::Type(inner_ty.clone())],
    );
    let (future_size, future_align) = layouts.size_and_align_for_ty(&future_ty).or_else(|| {
        generate_future_layout(layouts, inner_ty).and_then(|layout| layout.size.zip(layout.align))
    })?;
    let pointer_align = layouts
        .size_and_align_for_ty(&Ty::named("isize"))
        .map(|(_, a)| a)
        .unwrap_or(header_align.max(future_align));
    let base_size = align_to(header_size + flags_size, pointer_align);
    let inner_offset = align_to(base_size, future_align);
    let align = pointer_align
        .max(future_align)
        .max(header_align)
        .max(flags_align);
    let total_size = align_to(inner_offset + future_size, align);
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
        align: Some(align),
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

/// Captured layout for a nominal Chic type.
#[derive(Debug, Clone)]
pub enum TypeLayout {
    Struct(StructLayout),
    Class(StructLayout),
    Enum(EnumLayout),
    Union(UnionLayout),
}

impl TypeLayout {
    #[must_use]
    pub fn auto_traits(&self) -> AutoTraitSet {
        match self {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout.auto_traits,
            TypeLayout::Enum(layout) => layout.auto_traits,
            TypeLayout::Union(layout) => layout.auto_traits,
        }
    }

    pub fn auto_traits_mut(&mut self) -> &mut AutoTraitSet {
        match self {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => &mut layout.auto_traits,
            TypeLayout::Enum(layout) => &mut layout.auto_traits,
            TypeLayout::Union(layout) => &mut layout.auto_traits,
        }
    }

    #[must_use]
    pub fn overrides(&self) -> AutoTraitOverride {
        match self {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => layout.overrides,
            TypeLayout::Enum(layout) => layout.overrides,
            TypeLayout::Union(layout) => layout.overrides,
        }
    }

    pub fn overrides_mut(&mut self) -> &mut AutoTraitOverride {
        match self {
            TypeLayout::Struct(layout) | TypeLayout::Class(layout) => &mut layout.overrides,
            TypeLayout::Enum(layout) => &mut layout.overrides,
            TypeLayout::Union(layout) => &mut layout.overrides,
        }
    }
}

/// Field-oriented layout for structs and classes.
#[derive(Debug, Clone)]
pub struct StructLayout {
    pub name: String,
    pub repr: TypeRepr,
    pub packing: Option<u32>,
    pub fields: Vec<FieldLayout>,
    pub positional: Vec<PositionalElement>,
    pub list: Option<ListLayout>,
    pub size: Option<usize>,
    pub align: Option<usize>,
    pub is_readonly: bool,
    pub is_intrinsic: bool,
    pub allow_cross_inline: bool,
    pub auto_traits: AutoTraitSet,
    pub overrides: AutoTraitOverride,
    pub mmio: Option<MmioStructLayout>,
    pub dispose: Option<String>,
    pub class: Option<ClassLayoutInfo>,
}

/// Variant-oriented layout for enums.
#[derive(Debug, Clone)]
pub struct EnumLayout {
    pub name: String,
    pub repr: TypeRepr,
    pub packing: Option<u32>,
    pub underlying: Ty,
    pub underlying_info: Option<IntInfo>,
    pub explicit_underlying: bool,
    pub variants: Vec<EnumVariantLayout>,
    pub size: Option<usize>,
    pub align: Option<usize>,
    pub auto_traits: AutoTraitSet,
    pub overrides: AutoTraitOverride,
    pub is_flags: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassLayoutKind {
    Class,
    Error,
}

#[derive(Debug, Clone)]
pub struct ClassLayoutInfo {
    pub kind: ClassLayoutKind,
    pub bases: Vec<String>,
    pub vtable_offset: Option<usize>,
}

/// View-oriented layout for unions.
#[derive(Debug, Clone)]
pub struct UnionLayout {
    pub name: String,
    pub repr: TypeRepr,
    pub packing: Option<u32>,
    pub views: Vec<UnionFieldLayout>,
    pub size: Option<usize>,
    pub align: Option<usize>,
    pub auto_traits: AutoTraitSet,
    pub overrides: AutoTraitOverride,
}

/// Field metadata captured during lowering.
#[derive(Debug, Clone)]
pub struct FieldLayout {
    pub name: String,
    pub ty: Ty,
    pub index: u32,
    pub offset: Option<usize>,
    pub span: Option<Span>,
    pub mmio: Option<MmioFieldLayout>,
    pub display_name: Option<String>,
    pub is_required: bool,
    pub is_nullable: bool,
    pub is_readonly: bool,
    pub view_of: Option<String>,
}

impl FieldLayout {
    #[must_use]
    pub fn matches_name(&self, candidate: &str) -> bool {
        if self.name == candidate {
            return true;
        }
        if let Some(display) = &self.display_name {
            if display == candidate {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct MmioStructLayout {
    pub base_address: u64,
    pub size: Option<u64>,
    pub address_space: Option<String>,
    pub endianness: MmioEndianness,
    pub requires_unsafe: bool,
}

#[derive(Debug, Clone)]
pub struct MmioFieldLayout {
    pub offset: u32,
    pub width_bits: u16,
    pub access: MmioAccess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioAccess {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmioEndianness {
    Little,
    Big,
}

/// Enum variant metadata.
#[derive(Debug, Clone)]
pub struct EnumVariantLayout {
    pub name: String,
    pub index: u32,
    pub discriminant: i128,
    pub fields: Vec<FieldLayout>,
    pub positional: Vec<PositionalElement>,
}

/// Metadata describing positional accessors for tuple-like patterns.
#[derive(Debug, Clone)]
pub struct PositionalElement {
    pub field_index: u32,
    pub name: Option<String>,
    pub span: Option<Span>,
}

/// Metadata describing list-like layout for pattern matching.
#[derive(Debug, Clone)]
pub struct ListLayout {
    pub element_index: Option<u32>,
    pub length_index: Option<u32>,
    pub span: Option<Span>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_generic_async_future_layout_offsets() {
        configure_pointer_width(8, 8);
        let layouts = TypeLayoutTable::default();
        let layout = layouts
            .layout_for_name("Std::Async::Future<int>")
            .and_then(|layout| match layout {
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data.clone()),
                _ => None,
            })
            .expect("expected Std.Async.Future<int> layout");
        let header = layout
            .fields
            .iter()
            .find(|f| f.name == "Header")
            .and_then(|f| f.offset)
            .expect("header offset");
        let completed = layout
            .fields
            .iter()
            .find(|f| f.name == "Completed")
            .and_then(|f| f.offset)
            .expect("completed offset");
        let result = layout
            .fields
            .iter()
            .find(|f| f.name == "Result")
            .and_then(|f| f.offset)
            .expect("result offset");
        assert_eq!(header, 0);
        assert_eq!(completed, 32);
        assert_eq!(result, 36);
    }

    #[test]
    fn generates_generic_async_task_layout_offsets() {
        configure_pointer_width(8, 8);
        let layouts = TypeLayoutTable::default();
        let layout = layouts
            .layout_for_name("Std::Async::Task<int>")
            .and_then(|layout| match layout {
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data.clone()),
                _ => None,
            })
            .expect("expected Std.Async.Task<int> layout");
        let header = layout
            .fields
            .iter()
            .find(|f| f.name == "Header")
            .and_then(|f| f.offset)
            .expect("header offset");
        let flags = layout
            .fields
            .iter()
            .find(|f| f.name == "Flags")
            .and_then(|f| f.offset)
            .expect("flags offset");
        let inner_future = layout
            .fields
            .iter()
            .find(|f| f.name == "InnerFuture")
            .and_then(|f| f.offset)
            .expect("inner future offset");
        assert_eq!(header, 0);
        assert_eq!(flags, 32);
        assert_eq!(inner_future, 40);
    }

    #[test]
    fn accepts_dotted_async_names() {
        let layouts = TypeLayoutTable::default();
        let layout = layouts
            .layout_for_name("Std.Async.Task<bool>")
            .and_then(|layout| match layout {
                TypeLayout::Struct(data) | TypeLayout::Class(data) => Some(data),
                _ => None,
            })
            .expect("expected Std.Async.Task<bool> layout");
        assert!(
            layout.fields.iter().any(|f| f.name == "InnerFuture"),
            "task layout should expose InnerFuture field"
        );
    }
}

/// Union view metadata.
#[derive(Debug, Clone)]
pub struct UnionFieldLayout {
    pub name: String,
    pub ty: Ty,
    pub index: u32,
    pub mode: UnionFieldMode,
    pub span: Option<Span>,
    pub is_nullable: bool,
}

/// Access semantics for a union view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnionFieldMode {
    Value,
    Readonly,
}

impl UnionFieldMode {
    #[must_use]
    pub fn from_readonly(is_readonly: bool) -> Self {
        if is_readonly {
            UnionFieldMode::Readonly
        } else {
            UnionFieldMode::Value
        }
    }
}

/// Representation hints propagated from attributes (bootstrap currently defaults everything).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeRepr {
    Default,
    C,
}

#[cfg(test)]
fn cross_inline_override_map() -> &'static RwLock<HashMap<String, bool>> {
    static OVERRIDES: OnceLock<RwLock<HashMap<String, bool>>> = OnceLock::new();
    OVERRIDES.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(test)]
pub fn test_set_cross_inline_override(name: &str, allow: bool) {
    let _ = cross_inline_override_map()
        .write()
        .map(|mut map| map.insert(name.to_string(), allow));
}

#[cfg(test)]
pub fn test_clear_cross_inline_overrides() {
    if let Ok(mut map) = cross_inline_override_map().write() {
        map.clear();
    }
}

#[cfg(test)]
pub fn cross_inline_override(name: &str) -> Option<bool> {
    cross_inline_override_map()
        .read()
        .ok()
        .and_then(|map| map.get(name).copied())
}

#[cfg(not(test))]
#[inline]
pub fn cross_inline_override(_: &str) -> Option<bool> {
    None
}

pub(crate) fn make_field(name: &str, ty: Ty, index: u32, offset: usize) -> FieldLayout {
    FieldLayout {
        name: name.to_string(),
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

fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

fn type_fragment_eq(left: &str, right: &str) -> bool {
    let mut left = left.chars().filter(|ch| !ch.is_whitespace());
    let mut right = right.chars().filter(|ch| !ch.is_whitespace());
    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(l), Some(r)) if l == r => {}
            _ => return false,
        }
    }
}

fn top_level_tail(name: &str) -> &str {
    let bytes = name.as_bytes();
    let mut depth = 0u32;
    let mut last = 0usize;
    let mut idx = 0usize;
    while idx + 1 < bytes.len() {
        match bytes[idx] {
            b'<' => {
                depth = depth.saturating_add(1);
                idx += 1;
            }
            b'>' => {
                depth = depth.saturating_sub(1);
                idx += 1;
            }
            b':' if depth == 0 && bytes[idx + 1] == b':' => {
                last = idx + 2;
                idx += 2;
            }
            _ => idx += 1,
        }
    }
    &name[last..]
}

fn canonical_async_alias(name: &str) -> Option<String> {
    let canonical = name.replace('.', "::");
    let tail = canonical.rsplit("::").next().unwrap_or(&canonical);
    let head = tail.split('<').next().unwrap_or(tail);
    match head {
        "Task" | "Future" | "FutureHeader" | "FutureVTable" | "RuntimeContext" => {
            Some(format!("Std::Async::{tail}"))
        }
        _ => None,
    }
}
