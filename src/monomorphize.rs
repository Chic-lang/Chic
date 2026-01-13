use std::collections::BTreeSet;

use crate::mir::{MirModule, TypeLayout};

/// Summary of concrete types encountered during monomorphisation analysis.
#[derive(Debug, Clone, Default)]
pub struct MonomorphizationSummary {
    pub drop_candidates: Vec<String>,
    pub clone_candidates: Vec<String>,
    pub hash_candidates: Vec<String>,
    pub eq_candidates: Vec<String>,
}

/// Analyse the MIR module to collect concrete aggregate types that require drop glue.
#[must_use]
pub fn analyse_module(module: &MirModule) -> MonomorphizationSummary {
    let mut droppable = BTreeSet::new();
    let mut cloneable = BTreeSet::new();
    let mut hashable = BTreeSet::new();
    let mut equatable = BTreeSet::new();
    for (name, layout) in &module.type_layouts.types {
        if module.type_layouts.type_requires_drop(name) {
            droppable.insert(name.clone());
        }
        if module.type_layouts.type_requires_clone(name) && has_clone_impl_for(module, name) {
            cloneable.insert(name.clone());
        }
        if has_hash_impl_for(module, name) || is_intrinsic_primitive(module, name, layout) {
            hashable.insert(name.clone());
        }
        if matches!(layout, TypeLayout::Enum(_))
            || has_eq_impl_for(module, name)
            || is_intrinsic_primitive(module, name, layout)
        {
            equatable.insert(name.clone());
        }
    }

    MonomorphizationSummary {
        drop_candidates: droppable.into_iter().collect(),
        clone_candidates: cloneable.into_iter().collect(),
        hash_candidates: hashable.into_iter().collect(),
        eq_candidates: equatable.into_iter().collect(),
    }
}

fn has_clone_impl_for(module: &MirModule, ty_name: &str) -> bool {
    let trait_label = "Clone";
    let method_symbol = format!("{ty_name}::{trait_label}::Clone");
    module
        .functions
        .iter()
        .any(|func| func.name == method_symbol)
}

fn has_hash_impl_for(module: &MirModule, ty_name: &str) -> bool {
    let method_symbol = format!("{ty_name}::GetHashCode");
    module
        .functions
        .iter()
        .any(|func| func.name == method_symbol)
}

fn has_eq_impl_for(module: &MirModule, ty_name: &str) -> bool {
    let method_symbol = format!("{ty_name}::op_Equality");
    module
        .functions
        .iter()
        .any(|func| func.name == method_symbol)
}

fn is_intrinsic_primitive(module: &MirModule, ty_name: &str, layout: &TypeLayout) -> bool {
    let is_intrinsic = match layout {
        TypeLayout::Struct(info) | TypeLayout::Class(info) => info.is_intrinsic,
        TypeLayout::Enum(_) | TypeLayout::Union(_) => false,
    };
    if !is_intrinsic {
        return false;
    }
    module
        .type_layouts
        .primitive_registry
        .lookup_by_name(ty_name)
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, Ty, TypeLayout, TypeRepr,
    };

    #[test]
    fn analyse_module_collects_droppable_structs() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::NeedsDrop".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::NeedsDrop".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: Vec::new(),
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: Some("Demo::NeedsDrop::dispose".into()),
                class: None,
            }),
        );
        module.type_layouts.types.insert(
            "Demo::Plain".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::Plain".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: Vec::new(),
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );

        let summary = analyse_module(&module);
        assert_eq!(summary.drop_candidates, vec!["Demo::NeedsDrop".to_string()]);
    }

    #[test]
    fn analyse_module_accounts_for_field_drops() {
        let mut module = MirModule::default();
        module.type_layouts.types.insert(
            "Demo::StringHolder".into(),
            TypeLayout::Struct(StructLayout {
                name: "Demo::StringHolder".into(),
                repr: TypeRepr::Default,
                packing: None,
                fields: vec![FieldLayout {
                    name: "Value".into(),
                    ty: Ty::String,
                    index: 0,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,

                    view_of: None,
                }],
                positional: Vec::new(),
                list: None,
                size: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                allow_cross_inline: false,
                auto_traits: AutoTraitSet::all_unknown(),
                overrides: AutoTraitOverride::default(),
                mmio: None,
                dispose: None,
                class: None,
            }),
        );

        let summary = analyse_module(&module);
        assert_eq!(
            summary.drop_candidates,
            vec!["Demo::StringHolder".to_string()]
        );
    }
}
