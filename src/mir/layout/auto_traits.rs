use std::collections::{HashMap, HashSet};

use crate::frontend::ast::TypeExpr;
use crate::frontend::parser::parse_type_expression_text;
use crate::mir::data::Ty;

use super::table::{TypeLayout, TypeLayoutTable};

struct TraitStatusContext<'a> {
    process_thread_safe: bool,
    process_shareable: bool,
    process_copy: bool,
    thread_safe: &'a mut AutoTraitStatus,
    shareable: &'a mut AutoTraitStatus,
    copy: &'a mut AutoTraitStatus,
}

/// Auto trait fulfilment recorded for a Chic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoTraitStatus {
    Yes,
    No,
    Unknown,
}

impl AutoTraitStatus {
    #[must_use]
    pub fn from_bool(value: bool) -> Self {
        if value {
            AutoTraitStatus::Yes
        } else {
            AutoTraitStatus::No
        }
    }

    #[must_use]
    pub fn combine(self, other: AutoTraitStatus) -> AutoTraitStatus {
        match (self, other) {
            (AutoTraitStatus::No, _) | (_, AutoTraitStatus::No) => AutoTraitStatus::No,
            (AutoTraitStatus::Unknown, _) | (_, AutoTraitStatus::Unknown) => {
                AutoTraitStatus::Unknown
            }
            (AutoTraitStatus::Yes, AutoTraitStatus::Yes) => AutoTraitStatus::Yes,
        }
    }
}

/// Triple of auto trait statuses tracked per type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoTraitSet {
    pub thread_safe: AutoTraitStatus,
    pub shareable: AutoTraitStatus,
    pub copy: AutoTraitStatus,
}

impl AutoTraitSet {
    #[must_use]
    pub const fn new(
        thread_safe: AutoTraitStatus,
        shareable: AutoTraitStatus,
        copy: AutoTraitStatus,
    ) -> Self {
        Self {
            thread_safe,
            shareable,
            copy,
        }
    }

    #[must_use]
    pub const fn all_yes() -> Self {
        Self::new(
            AutoTraitStatus::Yes,
            AutoTraitStatus::Yes,
            AutoTraitStatus::Yes,
        )
    }

    #[must_use]
    pub const fn all_unknown() -> Self {
        Self::new(
            AutoTraitStatus::Unknown,
            AutoTraitStatus::Unknown,
            AutoTraitStatus::Unknown,
        )
    }

    #[must_use]
    pub const fn thread_share_yes_copy_no() -> Self {
        Self::new(
            AutoTraitStatus::Yes,
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
        )
    }
}

/// Optional overrides supplied via attributes.
#[derive(Debug, Clone, Copy, Default)]
pub struct AutoTraitOverride {
    pub thread_safe: Option<bool>,
    pub shareable: Option<bool>,
    pub copy: Option<bool>,
}

impl TypeLayoutTable {
    /// Resolve and record auto trait information for all known layouts.
    pub fn finalize_auto_traits(&mut self) {
        let keys: Vec<String> = self.types.keys().cloned().collect();
        let mut cache: HashMap<String, AutoTraitSet> = HashMap::new();
        for key in keys {
            if let Some(traits) = self.delegate_auto_traits_for_key(&key) {
                if let Some(layout) = self.types.get_mut(&key) {
                    *layout.auto_traits_mut() = traits;
                }
            }
            let set = self.compute_auto_traits_for_key(&key, &mut cache, &mut HashSet::new());
            if let Some(layout) = self.types.get_mut(&key) {
                *layout.auto_traits_mut() = set;
            }
        }
    }

    #[must_use]
    pub fn auto_traits_for_type(&self, ty: &Ty) -> AutoTraitSet {
        let mut cache: HashMap<String, AutoTraitSet> = HashMap::new();
        let mut stack: HashSet<String> = HashSet::new();
        self.auto_traits_for_ty(ty, &mut cache, &mut stack)
    }

    fn compute_auto_traits_for_key(
        &self,
        key: &str,
        cache: &mut HashMap<String, AutoTraitSet>,
        stack: &mut HashSet<String>,
    ) -> AutoTraitSet {
        if let Some(cached) = cache.get(key) {
            return *cached;
        }
        if !stack.insert(key.to_string()) {
            return AutoTraitSet::all_unknown();
        }

        let Some(layout) = self.types.get(key) else {
            stack.remove(key);
            return AutoTraitSet::all_unknown();
        };

        let overrides = layout.overrides();
        let preset = layout.auto_traits();

        let (mut thread_safe, process_thread_safe) =
            if preset.thread_safe != AutoTraitStatus::Unknown {
                (preset.thread_safe, false)
            } else {
                Self::initial_trait_status(overrides.thread_safe)
            };

        let (mut shareable, process_shareable) = if preset.shareable != AutoTraitStatus::Unknown {
            (preset.shareable, false)
        } else {
            Self::initial_trait_status(overrides.shareable)
        };

        let (mut copy, process_copy) = if preset.copy != AutoTraitStatus::Unknown {
            (preset.copy, false)
        } else {
            Self::initial_trait_status(overrides.copy)
        };

        if process_thread_safe || process_shareable || process_copy {
            let mut context = TraitStatusContext {
                process_thread_safe,
                process_shareable,
                process_copy,
                thread_safe: &mut thread_safe,
                shareable: &mut shareable,
                copy: &mut copy,
            };
            self.combine_layout_traits(layout, &mut context, cache, stack);
        }

        stack.remove(key);
        let result = AutoTraitSet::new(thread_safe, shareable, copy);
        cache.insert(key.to_string(), result);
        result
    }

    fn initial_trait_status(value: Option<bool>) -> (AutoTraitStatus, bool) {
        match value {
            Some(explicit) => (AutoTraitStatus::from_bool(explicit), false),
            None => (AutoTraitStatus::Yes, true),
        }
    }

    fn combine_layout_traits(
        &self,
        layout: &TypeLayout,
        ctx: &mut TraitStatusContext<'_>,
        cache: &mut HashMap<String, AutoTraitSet>,
        stack: &mut HashSet<String>,
    ) {
        if ctx.process_copy {
            match layout {
                TypeLayout::Struct(data) => {
                    if data.class.is_some() || data.dispose.is_some() {
                        *ctx.copy = AutoTraitStatus::No;
                    }
                }
                TypeLayout::Class(data) => {
                    if data.dispose.is_some() || data.class.is_some() {
                        *ctx.copy = AutoTraitStatus::No;
                    }
                }
                TypeLayout::Enum(_) => {}
                TypeLayout::Union(_) => {
                    *ctx.copy = AutoTraitStatus::No;
                }
            }
        }

        let mut merge_traits = |ty: &Ty| {
            let traits = self.auto_traits_for_ty(ty, cache, stack);
            if ctx.process_thread_safe {
                *ctx.thread_safe = (*ctx.thread_safe).combine(traits.thread_safe);
            }
            if ctx.process_shareable {
                *ctx.shareable = (*ctx.shareable).combine(traits.shareable);
            }
            if ctx.process_copy {
                *ctx.copy = (*ctx.copy).combine(traits.copy);
            }
        };

        match layout {
            TypeLayout::Struct(data) | TypeLayout::Class(data) => {
                for field in &data.fields {
                    merge_traits(&field.ty);
                }
                if let Some(list) = &data.list {
                    if let Some(index) = list.element_index {
                        if let Some(field) = data.fields.get(index as usize) {
                            merge_traits(&field.ty);
                        }
                    }
                }
            }
            TypeLayout::Enum(enum_layout) => {
                for variant in &enum_layout.variants {
                    for field in &variant.fields {
                        merge_traits(&field.ty);
                    }
                    for element in &variant.positional {
                        if let Some(field) = variant.fields.get(element.field_index as usize) {
                            merge_traits(&field.ty);
                        }
                    }
                }
            }
            TypeLayout::Union(data) => {
                for view in &data.views {
                    merge_traits(&view.ty);
                }
            }
        }
    }

    fn auto_traits_for_ty(
        &self,
        ty: &Ty,
        cache: &mut HashMap<String, AutoTraitSet>,
        stack: &mut HashSet<String>,
    ) -> AutoTraitSet {
        match ty {
            Ty::Unit => AutoTraitSet::all_yes(),
            Ty::Unknown => AutoTraitSet::all_unknown(),
            Ty::Array(array) => self.auto_traits_for_ty(array.element.as_ref(), cache, stack),
            Ty::Vec(vec) => self.auto_traits_for_ty(vec.element.as_ref(), cache, stack),
            Ty::Span(span) => self.auto_traits_for_ty(span.element.as_ref(), cache, stack),
            Ty::ReadOnlySpan(span) => self.auto_traits_for_ty(span.element.as_ref(), cache, stack),
            Ty::Rc(rc) => {
                let inner = self.auto_traits_for_ty(rc.element.as_ref(), cache, stack);
                AutoTraitSet::new(AutoTraitStatus::No, inner.shareable, AutoTraitStatus::No)
            }
            Ty::Arc(arc) => {
                let inner = self.auto_traits_for_ty(arc.element.as_ref(), cache, stack);
                AutoTraitSet::new(inner.thread_safe, inner.shareable, AutoTraitStatus::No)
            }
            Ty::Fn(_) => AutoTraitSet::all_yes(),
            Ty::Pointer(pointer) => {
                let inner = self.auto_traits_for_ty(&pointer.element, cache, stack);
                AutoTraitSet::new(inner.thread_safe, inner.shareable, AutoTraitStatus::Yes)
            }
            Ty::Ref(reference) => {
                let inner = self.auto_traits_for_ty(&reference.element, cache, stack);
                AutoTraitSet::new(inner.thread_safe, inner.shareable, AutoTraitStatus::Yes)
            }
            Ty::Tuple(tuple) => {
                let mut combined = AutoTraitSet::all_yes();
                for element in &tuple.elements {
                    let element_traits = self.auto_traits_for_ty(element, cache, stack);
                    combined = AutoTraitSet::new(
                        combined.thread_safe.combine(element_traits.thread_safe),
                        combined.shareable.combine(element_traits.shareable),
                        combined.copy.combine(element_traits.copy),
                    );
                }
                combined
            }
            Ty::Vector(vector) => self.auto_traits_for_ty(&vector.element, cache, stack),
            Ty::Nullable(inner) => self.auto_traits_for_ty(inner, cache, stack),
            Ty::String | Ty::Str => AutoTraitSet::all_yes(),
            Ty::TraitObject(_) => AutoTraitSet::all_unknown(),
            Ty::Named(name) => {
                let base = strip_generics(name);
                if let Some(traits) = Self::builtin_auto_traits(base) {
                    return traits;
                }
                if let Some(key) = self.resolve_type_key(base) {
                    return self.compute_auto_traits_for_key(key, cache, stack);
                }
                AutoTraitSet::all_unknown()
            }
        }
    }

    fn builtin_auto_traits(name: &str) -> Option<AutoTraitSet> {
        match name {
            "System::String" | "Std::String" => Some(AutoTraitSet::thread_share_yes_copy_no()),
            "str" | "System::Str" | "Std::Str" => Some(AutoTraitSet::thread_share_yes_copy_no()),
            "System::Boolean" | "Std::Boolean" | "bool" => Some(AutoTraitSet::all_yes()),
            "System::SByte" | "Std::SByte" | "System::Byte" | "Std::Byte" | "sbyte" | "byte" => {
                Some(AutoTraitSet::all_yes())
            }
            "System::Int16" | "Std::Int16" | "System::UInt16" | "Std::UInt16" | "short"
            | "ushort" | "char" => Some(AutoTraitSet::all_yes()),
            "System::Int32" | "Std::Int32" | "System::UInt32" | "Std::UInt32" | "int" | "uint" => {
                Some(AutoTraitSet::all_yes())
            }
            "System::Int64" | "Std::Int64" | "System::UInt64" | "Std::UInt64" | "long"
            | "ulong" => Some(AutoTraitSet::all_yes()),
            "System::IntPtr" | "Std::IntPtr" | "System::UIntPtr" | "Std::UIntPtr" | "nint"
            | "nuint" | "usize" | "isize" => Some(AutoTraitSet::all_yes()),
            "System::Single" | "Std::Single" | "System::Double" | "Std::Double" | "float"
            | "double" => Some(AutoTraitSet::all_yes()),
            "System::Threading::Mutex" | "std::sync::Mutex" | "Std::Sync::Mutex" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::MutexGuard" | "Std::Sync::MutexGuard" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::RwLock" | "Std::Sync::RwLock" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::RwLockReadGuard" | "Std::Sync::RwLockReadGuard" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::RwLockWriteGuard" | "Std::Sync::RwLockWriteGuard" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::Condvar" | "Std::Sync::Condvar" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::Once" | "Std::Sync::Once" => Some(AutoTraitSet::thread_share_yes_copy_no()),
            "std::sync::OnceCallback" | "Std::Sync::OnceCallback" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            "std::sync::AtomicBool"
            | "std::sync::AtomicI32"
            | "std::sync::AtomicI64"
            | "std::sync::AtomicU32"
            | "std::sync::AtomicU64"
            | "Std::Sync::AtomicBool"
            | "Std::Sync::AtomicI32"
            | "Std::Sync::AtomicI64"
            | "Std::Sync::AtomicU32"
            | "Std::Sync::AtomicU64" => Some(AutoTraitSet::all_yes()),
            "Std::Async::Task" | "Std::Async::Future" => {
                Some(AutoTraitSet::thread_share_yes_copy_no())
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn resolve_auto_traits(&self, name: &str) -> AutoTraitSet {
        if let Some(inner) = extract_single_generic(name, "Arc") {
            let inner_traits = self.resolve_auto_traits(inner);
            return AutoTraitSet::new(
                inner_traits.thread_safe,
                inner_traits.shareable,
                AutoTraitStatus::No,
            );
        }

        if let Some(inner) = extract_single_generic(name, "Rc") {
            let inner_traits = self.resolve_auto_traits(inner);
            return AutoTraitSet::new(
                AutoTraitStatus::No,
                inner_traits.shareable,
                AutoTraitStatus::No,
            );
        }

        if let Some(expr) = parse_type_text(name) {
            let ty = Ty::from_type_expr(&expr);
            let traits = self.auto_traits_for_type(&ty);
            if traits != AutoTraitSet::all_unknown() {
                return traits;
            }
        }
        let base = strip_generics(name);
        if let Some(traits) = Self::builtin_auto_traits(base) {
            return traits;
        }
        if let Some(key) = self.resolve_type_key(base)
            && let Some(layout) = self.types.get(key)
        {
            return layout.auto_traits();
        }
        AutoTraitSet::all_unknown()
    }
}

pub(crate) fn strip_generics(name: &str) -> &str {
    name.split('<').next().unwrap_or(name)
}

pub(crate) fn extract_single_generic<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    let trimmed = name.trim();
    if !trimmed.starts_with(prefix) {
        return None;
    }
    let rest = trimmed[prefix.len()..].trim_start();
    if !rest.starts_with('<') {
        return None;
    }
    let mut depth = 0i32;
    let mut end_index = None;
    let mut iter = rest.char_indices();
    let (_, first_char) = iter.next()?;
    if first_char != '<' {
        return None;
    }
    for (idx, ch) in iter {
        match ch {
            '<' => depth += 1,
            '>' => {
                if depth == 0 {
                    end_index = Some((idx, ch.len_utf8()));
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    let (end, len) = end_index?;
    if rest[end + len..].trim().is_empty() {
        let inner = rest[1..end].trim();
        Some(inner)
    } else {
        None
    }
}

fn parse_type_text(text: &str) -> Option<TypeExpr> {
    if let Some(expr) = parse_type_expression_text(text) {
        return Some(expr);
    }
    if text.contains("::") {
        let substituted = text.replace("::", ".");
        if let Some(mut expr) = parse_type_expression_text(&substituted) {
            expr.name = text.to_string();
            return Some(expr);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::data::{ArcTy, RcTy};
    use crate::mir::{StructLayout, TypeLayout, TypeLayoutTable, TypeRepr};

    #[test]
    fn rc_auto_traits_combine_inner_shareable() {
        let table = TypeLayoutTable::default();
        let ty = Ty::Rc(RcTy::new(Box::new(Ty::String)));
        let traits = table.auto_traits_for_type(&ty);
        assert_eq!(traits.thread_safe, AutoTraitStatus::No);
        assert_eq!(traits.shareable, AutoTraitStatus::Yes);
    }

    #[test]
    fn arc_auto_traits_follow_inner_traits() {
        let mut table = TypeLayoutTable::default();
        table.types.insert(
            "Demo::NotSafe".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::NotSafe".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: Vec::new(),
                positional: Vec::new(),
                list: None,
                size: Some(4),
                align: Some(4),
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::new(
                    AutoTraitStatus::No,
                    AutoTraitStatus::No,
                    AutoTraitStatus::No,
                ),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );
        table.finalize_auto_traits();
        let ty = Ty::Arc(ArcTy::new(Box::new(Ty::named("Demo::NotSafe"))));
        let traits = table.auto_traits_for_type(&ty);
        assert_eq!(traits.thread_safe, AutoTraitStatus::No);
        assert_eq!(traits.shareable, AutoTraitStatus::No);
    }

    #[test]
    fn arc_auto_traits_yes_for_safe_inner() {
        let table = TypeLayoutTable::default();
        let ty = Ty::Arc(ArcTy::new(Box::new(Ty::String)));
        let traits = table.auto_traits_for_type(&ty);
        assert_eq!(traits.thread_safe, AutoTraitStatus::Yes);
        assert_eq!(traits.shareable, AutoTraitStatus::Yes);
    }
}
