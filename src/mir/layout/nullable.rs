use super::auto_traits::{AutoTraitOverride, AutoTraitSet};
use super::table::{
    MIN_ALIGN, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr, align_to, pointer_align,
    pointer_size,
};
use crate::mir::data::Ty;
use crate::type_metadata::TypeFlags;

impl TypeLayoutTable {
    pub fn ensure_nullable_layout(&mut self, inner: &Ty) {
        let name = nullable_type_name(inner);
        if self.types.contains_key(&name) {
            return;
        }

        let (flag_size, flag_align) = self
            .primitive_registry
            .size_align_for_name("bool", pointer_size() as u32, pointer_align() as u32)
            .map(|(size, align)| (size as usize, align as usize))
            .unwrap_or((1, 1));
        let value_layout = self.size_and_align_for_ty(inner);

        let payload_align = value_layout.map(|(_, align)| align).unwrap_or(MIN_ALIGN);
        let mut struct_align = flag_align.max(payload_align).max(MIN_ALIGN);

        let payload_offset = value_layout.map(|(_, align)| align_to(flag_size, align));
        let size = value_layout.map(|(size, align)| {
            let start = align_to(flag_size, align);
            align_to(start + size, struct_align)
        });

        if size.is_none() {
            struct_align = struct_align.max(MIN_ALIGN);
        }

        let has_value_field = super::table::FieldLayout {
            name: "HasValue".into(),
            ty: Ty::named("bool"),
            index: 0,
            offset: Some(0),
            span: None,
            mmio: None,
            display_name: Some("HasValue".into()),
            is_required: false,
            is_nullable: false,
            is_readonly: false,
            view_of: None,
        };

        let value_field = super::table::FieldLayout {
            name: "Value".into(),
            ty: inner.clone(),
            index: 1,
            offset: payload_offset,
            span: None,
            mmio: None,
            display_name: Some("Value".into()),
            is_required: false,
            is_nullable: matches!(inner, Ty::Nullable(_)),
            is_readonly: false,
            view_of: None,
        };

        let positional = vec![
            super::table::PositionalElement {
                field_index: has_value_field.index,
                name: Some("HasValue".into()),
                span: None,
            },
            super::table::PositionalElement {
                field_index: value_field.index,
                name: Some("Value".into()),
                span: None,
            },
        ];

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![has_value_field, value_field],
            positional,
            list: None,
            size,
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };

        if self.ty_is_fallible(inner) {
            self.add_type_flags(name.clone(), TypeFlags::FALLIBLE);
        }

        self.types.insert(name, TypeLayout::Struct(layout));
    }
}

pub(crate) fn nullable_type_name(inner: &Ty) -> String {
    format!("{}?", inner.canonical_name())
}
