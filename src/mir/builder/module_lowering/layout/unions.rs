//! Union layout registration logic.

use super::super::super::{
    MIN_ALIGN, PositionalElement, StructLayout, TypeLayout, TypeRepr, UnionDecl, UnionFieldLayout,
    UnionFieldMode, UnionLayout, UnionMember, align_to, qualify,
};
use super::super::driver::{LoweringDiagnostic, ModuleLowering, expect_u32_index};
use super::auto_traits;
use crate::frontend::attributes::{
    collect_layout_hints, extract_global_allocator, has_fallible_attr,
};
use crate::mir::data::Ty;
use crate::type_metadata::TypeFlags;

impl ModuleLowering {
    // --- layout::unions (planned extraction) ---
    // Depends on shared field layout computation, visibility checks from super::driver, and auto-trait overrides.
    pub(crate) fn register_union_layout(&mut self, union: &UnionDecl, namespace: Option<&str>) {
        let name = qualify(namespace, &union.name);
        if self.type_layouts.types.contains_key(&name) {
            return;
        }

        self.record_type_visibility(&name, union.visibility, namespace, None);

        let (allocator_attr, errors) = extract_global_allocator(&union.attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator_attr {
            self.diagnostics.push(LoweringDiagnostic {
                message: "`@global_allocator` is only supported on struct or class declarations"
                    .to_string(),
                span: attr.span,
            });
        }

        let (layout_hints, errors) = collect_layout_hints(&union.attributes);
        for error in errors {
            self.diagnostics.push(LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }

        let packing_limit = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1) as usize);
        let layout_packing = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1));

        for member in &union.members {
            if let UnionMember::View(view) = member {
                let qualified = format!("{name}::{}", view.name);
                if self.type_layouts.types.contains_key(&qualified) {
                    continue;
                }
                self.record_type_visibility(
                    &qualified,
                    view.visibility,
                    namespace,
                    Some(name.as_str()),
                );
                let (fields, size, align) = self.compute_field_layouts(
                    &view.fields,
                    namespace,
                    Some(qualified.as_str()),
                    packing_limit,
                    0,
                );
                let positional = fields
                    .iter()
                    .map(|field| PositionalElement {
                        field_index: field.index,
                        name: Some(field.name.clone()),
                        span: field.span,
                    })
                    .collect();
                let list = Self::infer_list_layout(&fields);
                let overrides = auto_traits::default_override();
                let layout = StructLayout {
                    name: qualified.clone(),
                    repr: if layout_hints.repr_c {
                        TypeRepr::C
                    } else {
                        TypeRepr::Default
                    },
                    packing: layout_packing,
                    fields,
                    positional,
                    list,
                    size,
                    align,
                    is_readonly: false,
                    is_intrinsic: false,
                    allow_cross_inline: false,
                    auto_traits: auto_traits::unknown_set(),
                    overrides,
                    mmio: None,
                    dispose: None,
                    class: None,
                };
                self.type_layouts
                    .types
                    .insert(qualified, TypeLayout::Struct(layout));
            }
        }

        let mut views = Vec::new();
        let mut max_size = 0usize;
        let mut max_align = MIN_ALIGN;
        let mut any_known = false;

        for (index, member) in union.members.iter().enumerate() {
            match member {
                UnionMember::Field(field) => {
                    self.ensure_type_expr_accessible(
                        &field.ty,
                        namespace,
                        Some(name.as_str()),
                        &format!("union field `{}`", field.name),
                        None,
                    );
                    let ty = self.ty_from_type_expr(&field.ty, namespace, Some(name.as_str()));
                    if let Some((size, align)) = self.type_size_and_align(&ty, namespace) {
                        max_size = max_size.max(size);
                        max_align = max_align.max(align);
                        any_known = true;
                    }
                    let index_u32 = expect_u32_index(index, "union member index");
                    views.push(UnionFieldLayout {
                        name: field.name.clone(),
                        ty,
                        index: index_u32,
                        mode: UnionFieldMode::from_readonly(field.is_readonly),
                        span: None,
                        is_nullable: field.ty.is_nullable(),
                    });
                }
                UnionMember::View(view) => {
                    let ty_name = format!("{name}::{}", view.name);
                    let ty = Ty::named(ty_name.clone());
                    if let Some((size, align)) = self.type_size_and_align(&ty, Some(name.as_str()))
                    {
                        max_size = max_size.max(size);
                        max_align = max_align.max(align);
                        any_known = true;
                    }
                    let index_u32 = expect_u32_index(index, "union member index");
                    views.push(UnionFieldLayout {
                        name: view.name.clone(),
                        ty,
                        index: index_u32,
                        mode: UnionFieldMode::from_readonly(view.is_readonly),
                        span: None,
                        is_nullable: false,
                    });
                }
            }
        }

        let mut align = if any_known {
            Some(max_align.max(MIN_ALIGN))
        } else {
            Some(MIN_ALIGN)
        };
        if let Some(pack_limit) = packing_limit {
            align = Some(align.unwrap_or(pack_limit).min(pack_limit));
        }
        let mut size = if any_known {
            Some(align_to(max_size, align.unwrap_or(MIN_ALIGN)))
        } else {
            None
        };

        if let Some(align_hint) = layout_hints.align {
            let mut requested = align_hint.value as usize;
            if let Some(pack_limit) = packing_limit {
                if requested > pack_limit {
                    let pack_display = layout_packing.unwrap_or(1);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!("`@align({requested})` exceeds the `@repr(packed({pack_display}))` limit"),
                        span: align_hint.span.or(layout_hints.packing.and_then(|hint| hint.span)),
                                            });
                    requested = pack_limit;
                }
            }
            align = Some(align.map_or(requested, |current| current.max(requested)));
        }

        if let Some(alignment) = align {
            if let Some(current_size) = size {
                size = Some(align_to(current_size, alignment));
            }
        }

        let overrides = auto_traits::union_overrides(union);
        let layout = UnionLayout {
            name: name.clone(),
            repr: if layout_hints.repr_c {
                TypeRepr::C
            } else {
                TypeRepr::Default
            },
            packing: layout_packing,
            views,
            size,
            align,
            auto_traits: auto_traits::unknown_set(),
            overrides,
        };
        self.type_layouts
            .types
            .insert(name.clone(), TypeLayout::Union(layout));
        if has_fallible_attr(&union.attributes) {
            self.type_layouts.add_type_flags(name, TypeFlags::FALLIBLE);
        }
    }
}
