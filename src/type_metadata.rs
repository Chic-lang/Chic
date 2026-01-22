use std::collections::{HashMap, HashSet};

use crate::drop_glue::SynthesisedDropGlue;
use crate::frontend::ast::Variance as AstVariance;
use crate::mir::casts::short_type_name;
use crate::mir::table::{MIN_ALIGN, align_to};
use crate::mir::{MirModule, TypeLayout};
use crate::type_identity::type_identity_for_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeVariance {
    Invariant,
    Covariant,
    Contravariant,
}

impl TypeVariance {
    #[must_use]
    pub fn encode(self) -> u8 {
        match self {
            TypeVariance::Invariant => 0,
            TypeVariance::Covariant => 1,
            TypeVariance::Contravariant => 2,
        }
    }
}

impl From<AstVariance> for TypeVariance {
    fn from(value: AstVariance) -> Self {
        match value {
            AstVariance::Invariant => TypeVariance::Invariant,
            AstVariance::Covariant => TypeVariance::Covariant,
            AstVariance::Contravariant => TypeVariance::Contravariant,
        }
    }
}

/// Bitflags describing additional semantic properties of a type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TypeFlags {
    bits: u32,
}

impl TypeFlags {
    pub const FALLIBLE: TypeFlags = TypeFlags { bits: 1 << 0 };

    #[must_use]
    pub const fn empty() -> Self {
        TypeFlags { bits: 0 }
    }

    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        TypeFlags { bits }
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.bits
    }

    #[must_use]
    pub const fn contains(self, other: TypeFlags) -> bool {
        (self.bits & other.bits) == other.bits
    }

    pub fn insert(&mut self, other: TypeFlags) {
        self.bits |= other.bits;
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

impl std::ops::BitOr for TypeFlags {
    type Output = TypeFlags;

    fn bitor(self, rhs: TypeFlags) -> TypeFlags {
        TypeFlags {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitOrAssign for TypeFlags {
    fn bitor_assign(&mut self, rhs: TypeFlags) {
        self.bits |= rhs.bits;
    }
}

/// Metadata describing a concrete type required at runtime.
#[derive(Debug, Clone)]
pub struct SynthesisedTypeMetadata {
    pub type_name: String,
    pub type_identity: u64,
    pub size: usize,
    pub align: usize,
    pub drop_symbol: Option<String>,
    pub variances: Vec<TypeVariance>,
    pub flags: TypeFlags,
}

impl SynthesisedTypeMetadata {
    #[must_use]
    pub fn drop_required(&self) -> bool {
        self.drop_symbol.is_some()
    }
}

fn class_instance_size_and_align(
    module: &MirModule,
    type_name: &str,
    cache: &mut HashMap<String, (usize, usize)>,
    visiting: &mut HashSet<String>,
) -> Option<(usize, usize)> {
    let normalized = type_name.replace('.', "::");
    let resolved_key = module
        .type_layouts
        .resolve_type_key(&normalized)
        .unwrap_or(normalized.as_str())
        .to_string();

    if let Some(size_align) = cache.get(&resolved_key).copied() {
        return Some(size_align);
    }
    if !visiting.insert(resolved_key.clone()) {
        return None;
    }

    let layout = module.type_layouts.layout_for_name(&resolved_key)?;
    let (mut size, mut align) = match layout {
        TypeLayout::Struct(info) | TypeLayout::Class(info) => match (info.size, info.align) {
            (Some(size), Some(align)) => (size, align),
            _ => {
                visiting.remove(&resolved_key);
                return None;
            }
        },
        TypeLayout::Enum(info) => match (info.size, info.align) {
            (Some(size), Some(align)) => (size, align),
            _ => {
                visiting.remove(&resolved_key);
                return None;
            }
        },
        TypeLayout::Union(info) => match (info.size, info.align) {
            (Some(size), Some(align)) => (size, align),
            _ => {
                visiting.remove(&resolved_key);
                return None;
            }
        },
    };

    if let TypeLayout::Struct(info) | TypeLayout::Class(info) = layout {
        for field in &info.fields {
            let Some(field_offset) = field.offset else {
                continue;
            };
            if let Some((field_size, field_align)) =
                module.type_layouts.size_and_align_for_ty(&field.ty)
            {
                let effective_align = field_align.max(MIN_ALIGN);
                align = align.max(effective_align);
                let end = align_to(field_offset, effective_align).saturating_add(field_size);
                if end > size {
                    size = end;
                }
            }
        }
        size = align_to(size, align.max(MIN_ALIGN));
    }

    if let TypeLayout::Class(info) = layout {
        if let Some(class_info) = &info.class {
            for base in &class_info.bases {
                if let Some((base_size, base_align)) =
                    class_instance_size_and_align(module, base, cache, visiting)
                {
                    size = size.max(base_size);
                    align = align.max(base_align);
                }
            }
        }
        size = align_to(size, align.max(MIN_ALIGN));
    }

    visiting.remove(&resolved_key);
    cache.insert(resolved_key, (size, align));
    Some((size, align))
}

/// Build metadata entries for every concrete type in the module's layout table.
pub fn synthesise_type_metadata(
    module: &MirModule,
    drop_glue: &[SynthesisedDropGlue],
) -> Vec<SynthesisedTypeMetadata> {
    let mut entries = Vec::new();
    let mut class_instance_cache: HashMap<String, (usize, usize)> = HashMap::new();
    let mut class_instance_visiting: HashSet<String> = HashSet::new();
    let drop_symbol_map: HashMap<_, _> = drop_glue
        .iter()
        .map(|entry| (entry.type_name.as_str(), entry.symbol.as_str()))
        .collect();

    for (name, layout) in &module.type_layouts.types {
        if short_type_name(name) == "Self" {
            continue;
        }
        let (mut size, mut align) = match layout {
            TypeLayout::Struct(info) | TypeLayout::Class(info) => match (info.size, info.align) {
                (Some(size), Some(align)) => (size, align),
                _ => continue,
            },
            TypeLayout::Enum(info) => match (info.size, info.align) {
                (Some(size), Some(align)) => (size, align),
                _ => continue,
            },
            TypeLayout::Union(info) => match (info.size, info.align) {
                (Some(size), Some(align)) => (size, align),
                _ => continue,
            },
        };
        if let TypeLayout::Struct(info) | TypeLayout::Class(info) = layout {
            for field in &info.fields {
                let Some(field_offset) = field.offset else {
                    continue;
                };
                if let Some((field_size, field_align)) =
                    module.type_layouts.size_and_align_for_ty(&field.ty)
                {
                    let effective_align = field_align.max(MIN_ALIGN);
                    align = align.max(effective_align);
                    let end = align_to(field_offset, effective_align).saturating_add(field_size);
                    if end > size {
                        size = end;
                    }
                }
            }
            size = align_to(size, align.max(MIN_ALIGN));
        }
        if matches!(layout, TypeLayout::Class(_)) {
            if let Some((instance_size, instance_align)) = class_instance_size_and_align(
                module,
                name,
                &mut class_instance_cache,
                &mut class_instance_visiting,
            ) {
                size = size.max(instance_size);
                align = align.max(instance_align);
            }
        }

        let drop_symbol = drop_symbol_map
            .get(name.as_str())
            .map(|symbol| symbol.to_string());

        let flags = module.type_layouts.type_flags_for_name(name.clone());
        entries.push(SynthesisedTypeMetadata {
            type_name: name.clone(),
            type_identity: type_identity_for_name(&module.type_layouts, name),
            size,
            align,
            drop_symbol,
            variances: module.type_variance.get(name).cloned().unwrap_or_default(),
            flags,
        });
    }

    let pointer_size = crate::mir::pointer_size() as u32;
    let pointer_align = crate::mir::pointer_align() as u32;
    for desc in module.primitive_registry.descriptors() {
        let name = desc.primitive_name.as_str();
        if entries.iter().any(|entry| entry.type_name == name) {
            continue;
        }
        if let Some((size, align)) = module
            .primitive_registry
            .size_align_for_name(name, pointer_size, pointer_align)
            .map(|(size, align)| (size as usize, align as usize))
        {
            entries.push(SynthesisedTypeMetadata {
                type_name: name.to_string(),
                type_identity: type_identity_for_name(&module.type_layouts, name),
                size,
                align,
                drop_symbol: None,
                variances: Vec::new(),
                flags: module.type_layouts.type_flags_for_name(name.to_string()),
            });
        }
    }

    if std::env::var("CHIC_DEBUG_TYPE_METADATA").is_ok() {
        eprintln!(
            "[chic-debug] type_metadata entries generated: {}",
            entries.len()
        );
        for entry in entries.iter().filter(|e| {
            e.type_name.contains("Tests::Concurrency::Litmus")
                || e.type_name.contains("ThreadFunction")
                || e.type_name.contains("ThreadStart")
                || e.type_name.contains("Exception")
        }) {
            eprintln!(
                "[chic-debug] type_metadata litmus entry {} id={:#x} size={} align={}",
                entry.type_name, entry.type_identity, entry.size, entry.align
            );
        }
    }

    entries.sort_by_key(|entry| entry.type_identity);
    entries
}
