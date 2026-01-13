//! Auto-trait override helpers for struct/enum/union layouts.

use super::super::super::{AutoTraitOverride, AutoTraitSet, EnumDecl, StructDecl, UnionDecl};

/// Create an override from struct-level `@auto_trait` annotations.
pub(crate) fn struct_overrides(strct: &StructDecl) -> AutoTraitOverride {
    AutoTraitOverride {
        thread_safe: strct.thread_safe_override,
        shareable: strct.shareable_override,
        copy: strct.copy_override,
    }
}

/// Create an override from enum-level `@auto_trait` annotations.
pub(crate) fn enum_overrides(enm: &EnumDecl) -> AutoTraitOverride {
    AutoTraitOverride {
        thread_safe: enm.thread_safe_override,
        shareable: enm.shareable_override,
        copy: enm.copy_override,
    }
}

/// Create an override from union-level `@auto_trait` annotations.
pub(crate) fn union_overrides(union: &UnionDecl) -> AutoTraitOverride {
    AutoTraitOverride {
        thread_safe: union.thread_safe_override,
        shareable: union.shareable_override,
        copy: union.copy_override,
    }
}

/// Default auto-trait set prior to inference.
pub(crate) fn unknown_set() -> AutoTraitSet {
    AutoTraitSet::all_unknown()
}

/// Default override when declarations omit explicit auto-trait annotations.
pub(crate) fn default_override() -> AutoTraitOverride {
    AutoTraitOverride::default()
}
