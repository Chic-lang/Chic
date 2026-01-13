use super::auto_traits::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus};
use super::table::{
    FieldLayout, PositionalElement, StructLayout, TypeLayout, TypeLayoutTable, TypeRepr,
    pointer_align, pointer_size,
};
use crate::mir::data::{FnTy, PointerTy, Ty};

impl TypeLayoutTable {
    pub(crate) fn ensure_fn_layout(&mut self, fn_ty: &FnTy) {
        if matches!(fn_ty.abi, crate::mir::Abi::Extern(_)) {
            // Raw `fn @extern("C")` pointers are thin; they do not have a struct layout.
            return;
        }

        let name = fn_ty.canonical_name();
        if self.types.contains_key(&name) {
            return;
        }

        let pointer_ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
        let mut fields = Vec::with_capacity(6);
        let mut offset = 0usize;
        let push_field =
            |fields: &mut Vec<FieldLayout>, name: &str, ty: Ty, index: usize, offset: usize| {
                let idx = u32::try_from(index).unwrap_or(u32::MAX);
                fields.push(FieldLayout {
                    name: name.to_string(),
                    ty,
                    index: idx,
                    offset: Some(offset),
                    span: None,
                    mmio: None,
                    display_name: Some(name.to_string()),
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                });
            };

        push_field(&mut fields, "invoke", pointer_ty.clone(), 0, offset);
        offset += pointer_size();

        push_field(&mut fields, "context", pointer_ty.clone(), 1, offset);
        offset += pointer_size();

        push_field(&mut fields, "drop_glue", pointer_ty, 2, offset);
        offset += pointer_size();

        push_field(&mut fields, "type_id", Ty::named("u64"), 3, offset);
        offset += pointer_size();

        push_field(&mut fields, "env_size", Ty::named("usize"), 4, offset);
        offset += pointer_size();

        push_field(&mut fields, "env_align", Ty::named("usize"), 5, offset);
        offset += pointer_size();

        let positional = fields
            .iter()
            .map(|field| PositionalElement {
                field_index: field.index,
                name: Some(field.name.clone()),
                span: None,
            })
            .collect();

        let layout = StructLayout {
            name: name.clone(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional,
            list: None,
            size: Some(offset),
            align: Some(pointer_align()),
            is_readonly: false,
            is_intrinsic: true,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::new(
                AutoTraitStatus::Unknown,
                AutoTraitStatus::Unknown,
                AutoTraitStatus::No,
            ),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.types.insert(name, TypeLayout::Struct(layout));
    }

    pub(crate) fn ensure_delegate_layout(&mut self, name: &str) {
        if self.types.contains_key(name) {
            return;
        }

        let pointer_ty = Ty::Pointer(Box::new(PointerTy::new(Ty::Unit, true)));
        let mut fields = Vec::with_capacity(6);
        let mut offset = 0usize;
        let push_field =
            |fields: &mut Vec<FieldLayout>, name: &str, ty: Ty, index: usize, offset: usize| {
                let idx = u32::try_from(index).unwrap_or(u32::MAX);
                fields.push(FieldLayout {
                    name: name.to_string(),
                    ty,
                    index: idx,
                    offset: Some(offset),
                    span: None,
                    mmio: None,
                    display_name: Some(name.to_string()),
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                });
            };

        push_field(&mut fields, "invoke", pointer_ty.clone(), 0, offset);
        offset += pointer_size();

        push_field(&mut fields, "context", pointer_ty.clone(), 1, offset);
        offset += pointer_size();

        push_field(&mut fields, "drop_glue", pointer_ty, 2, offset);
        offset += pointer_size();

        push_field(&mut fields, "type_id", Ty::named("u64"), 3, offset);
        offset += pointer_size();

        push_field(&mut fields, "env_size", Ty::named("usize"), 4, offset);
        offset += pointer_size();

        push_field(&mut fields, "env_align", Ty::named("usize"), 5, offset);
        offset += pointer_size();

        let positional = fields
            .iter()
            .map(|field| PositionalElement {
                field_index: field.index,
                name: Some(field.name.clone()),
                span: None,
            })
            .collect();

        let layout = StructLayout {
            name: name.to_string(),
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional,
            list: None,
            size: Some(offset),
            align: Some(pointer_align()),
            is_readonly: false,
            is_intrinsic: true,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::new(
                AutoTraitStatus::Unknown,
                AutoTraitStatus::Unknown,
                AutoTraitStatus::No,
            ),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.types
            .insert(name.to_string(), TypeLayout::Struct(layout));
    }
}
