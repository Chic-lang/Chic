use super::auto_traits::{AutoTraitOverride, AutoTraitSet};
use super::table::{
    FieldLayout, MIN_ALIGN, PositionalElement, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr,
    align_to,
};
use crate::mir::data::{TupleTy, Ty};

impl TypeLayoutTable {
    pub fn ensure_tuple_layout(&mut self, tuple: &TupleTy) {
        let name = tuple.canonical_name();
        if self.types.contains_key(&name) {
            return;
        }

        for element in &tuple.elements {
            if let Ty::Tuple(inner) = element {
                self.ensure_tuple_layout(inner);
            }
        }

        let mut fields = Vec::with_capacity(tuple.elements.len());
        let mut offset = 0usize;
        let mut struct_align = MIN_ALIGN;
        let mut known_any = true;

        for (index, element) in tuple.elements.iter().enumerate() {
            let info = self.size_and_align_for_ty(element);
            let field_offset = if let Some((size, align)) = info {
                let aligned = align_to(offset, align);
                offset = aligned + size;
                struct_align = struct_align.max(align);
                Some(aligned)
            } else {
                known_any = false;
                None
            };
            let idx = u32::try_from(index).unwrap_or(u32::MAX);
            let field_name = tuple_field_name(index);
            let alias = tuple
                .element_names
                .get(index)
                .cloned()
                .and_then(|name| name);
            fields.push(FieldLayout {
                name: field_name.clone(),
                ty: element.clone(),
                index: idx,
                offset: field_offset,
                span: None,
                mmio: None,
                display_name: alias.clone(),
                is_required: false,
                is_nullable: matches!(element, Ty::Nullable(_)),
                is_readonly: false,
                view_of: None,
            });
        }

        let align = if known_any {
            Some(struct_align.max(MIN_ALIGN))
        } else if tuple.elements.is_empty() {
            Some(MIN_ALIGN)
        } else {
            None
        };

        let size = align.map(|align| align_to(offset, align));

        let positional = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let alias = tuple
                    .element_names
                    .get(index)
                    .cloned()
                    .and_then(|name| name);
                PositionalElement {
                    field_index: field.index,
                    name: alias.or_else(|| Some(field.name.clone())),
                    span: field.span,
                }
            })
            .collect();

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional,
            list: None,
            size,
            align,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.types.insert(name, TypeLayout::Struct(layout));
    }
}

pub(crate) fn tuple_field_name(index: usize) -> String {
    format!("Item{}", index + 1)
}
