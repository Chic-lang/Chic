//! Struct layout registration and shared field layout helpers.

use super::super::super::functions::{lower_constructor, lower_function};
use super::super::super::{
    Abi, FieldDecl, FieldLayout, FunctionKind, ListLayout, MIN_ALIGN, PositionalElement,
    StructDecl, StructLayout, TypeLayout, TypeRepr, Visibility, align_to, pointer_align,
    pointer_size, qualify,
};
use super::super::driver::{LoweringDiagnostic, ModuleLowering, expect_u32_index};
use super::auto_traits;
use crate::frontend::ast::InlineAttr;
use crate::frontend::ast::TypeExpr;
use crate::frontend::attributes::{
    collect_layout_hints, extract_global_allocator, has_fallible_attr,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::type_utils::sequence_descriptor;
use crate::mir::builder::support::type_size_and_align_for_ty;
use crate::mir::data::{ArrayTy, ReadOnlySpanTy, SpanTy, Ty, VecTy};
use crate::mir::layout::make_field;
use crate::type_metadata::TypeFlags;

impl ModuleLowering {
    // --- layout::structs (planned extraction) ---
    // Depends on super::driver (visibility, diagnostics), shared `compute_field_layouts`, and layout::mmio helpers.
    pub(crate) fn register_struct_layout(&mut self, strct: &StructDecl, namespace: Option<&str>) {
        let name = qualify(namespace, &strct.name);
        self.register_primitive_attribute(&name, &strct.attributes);
        if let Some(generics) = strct.generics.as_ref() {
            let params = generics
                .params
                .iter()
                .filter_map(|param| match &param.kind {
                    crate::frontend::ast::GenericParamKind::Type(_) => Some(param.name.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.type_layouts
                .record_type_generic_params(name.clone(), params);
        }
        if self.type_layouts.types.contains_key(&name) {
            return;
        }

        for field in &strct.fields {
            self.register_static_field_decl(&name, namespace, strct.visibility, field);
        }
        for property in &strct.properties {
            self.register_static_property_backing(&name, namespace, property);
        }

        self.record_type_visibility(&name, strct.visibility, namespace, None);

        let resolved_bases: Vec<String> = strct
            .bases
            .iter()
            .filter_map(|base| {
                match self.resolve_type_for_expr(base, namespace, Some(name.as_str())) {
                    ImportResolution::Found(resolved) => Some(resolved),
                    ImportResolution::Ambiguous(candidates) => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "base type of `{name}` resolves to multiple types: {}",
                                candidates.join(", ")
                            ),
                            span: base.span,
                        });
                        None
                    }
                    ImportResolution::NotFound => {
                        self.diagnostics.push(LoweringDiagnostic {
                            message: format!(
                                "base type `{}` for `{name}` could not be resolved",
                                base.name
                            ),
                            span: base.span,
                        });
                        None
                    }
                }
            })
            .collect();
        if !resolved_bases.is_empty() {
            self.class_bases
                .insert(name.clone(), resolved_bases.clone());
        }

        let (allocator_attr, errors) = extract_global_allocator(&strct.attributes);
        self.push_attribute_errors(errors);
        if let Some(attr) = allocator_attr {
            self.record_global_allocator(name.clone(), attr);
        }

        let (mut layout_hints, errors) = collect_layout_hints(&strct.attributes);
        if let Some(struct_layout) = strct.layout {
            layout_hints.repr_c = struct_layout.repr_c;
            if struct_layout.packing.is_some() {
                layout_hints.packing = struct_layout.packing;
            }
            if struct_layout.align.is_some() {
                layout_hints.align = struct_layout.align;
            }
        }
        for error in errors {
            self.diagnostics.push(LoweringDiagnostic {
                message: error.message,
                span: error.span,
            });
        }

        let mut field_members = strct
            .fields
            .iter()
            .filter(|field| !field.is_static)
            .cloned()
            .collect::<Vec<_>>();
        field_members.extend(
            strct
                .properties
                .iter()
                .filter(|property| property.is_auto() && !property.is_static)
                .map(|property| FieldDecl {
                    visibility: Visibility::Private,
                    name: property.backing_field_name(),
                    ty: property.ty.clone(),
                    initializer: property.initializer.clone(),
                    mmio: None,
                    doc: None,
                    is_required: property.is_required,
                    display_name: Some(property.name.clone()),
                    attributes: Vec::new(),
                    is_readonly: false,
                    is_static: false,
                    view_of: None,
                }),
        );

        let mut packing_limit = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1) as usize);
        let mut layout_packing = layout_hints
            .packing
            .map(|hint| hint.value.unwrap_or(1).max(1));

        let (fields, mut size, mut align, mmio_layout) = if let Some(mmio_attr) = &strct.mmio {
            if let Some(packing_hint) = layout_hints.packing {
                self.diagnostics.push(LoweringDiagnostic {
                    message: "`@repr(packed)` is not supported on `@mmio` structs".to_string(),
                    span: packing_hint.span,
                });
            }
            layout_packing = None;
            packing_limit = None;
            let (fields, size, align, mmio_layout) =
                self.compute_mmio_struct_layout(strct, namespace, name.as_str(), mmio_attr);
            (fields, size, align, Some(mmio_layout))
        } else {
            let (fields, size, align) = self.compute_field_layouts(
                &field_members,
                namespace,
                Some(name.as_str()),
                packing_limit,
                0,
                0,
            );
            (fields, size, align, None)
        };

        if let Some(align_hint) = layout_hints.align {
            let mut requested = align_hint.value as usize;
            if let Some(pack_limit) = packing_limit {
                if requested > pack_limit {
                    let pack_display = layout_packing.unwrap_or(1);
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "`@align({requested})` exceeds the `@repr(packed({pack_display}))` limit"
                        ),
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

        let positional = if strct.is_record && !strct.record_positional_fields.is_empty() {
            let mut slots = Vec::new();
            for positional in &strct.record_positional_fields {
                if let Some(field) = fields.iter().find(|field| field.name == positional.name) {
                    slots.push(PositionalElement {
                        field_index: field.index,
                        name: Some(field.name.clone()),
                        span: positional.span.or(field.span),
                    });
                }
            }
            slots
        } else {
            fields
                .iter()
                .map(|field| PositionalElement {
                    field_index: field.index,
                    name: Some(field.name.clone()),
                    span: field.span,
                })
                .collect()
        };
        let list = Self::infer_list_layout(&fields);
        // layout::auto_traits centralises overrides + default auto-trait sets.
        let overrides = auto_traits::struct_overrides(strct);
        let mut layout = StructLayout {
            name: name.clone(),
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
            is_readonly: strct.is_readonly,
            is_intrinsic: strct.is_intrinsic,
            allow_cross_inline: matches!(strct.inline_attr, Some(InlineAttr::Cross)),
            auto_traits: auto_traits::unknown_set(),
            overrides,
            mmio: mmio_layout,
            dispose: None,
            class: None,
        };
        layout.dispose = self.dispose_symbol(&name);
        self.type_layouts
            .types
            .insert(name.clone(), TypeLayout::Struct(layout));
        self.mark_fallible_struct(&name, &strct.attributes);
    }

    fn mark_fallible_struct(&mut self, name: &str, attrs: &[crate::frontend::ast::Attribute]) {
        if has_fallible_attr(attrs) {
            self.type_layouts
                .add_type_flags(name.to_string(), TypeFlags::FALLIBLE);
        }
        if Self::is_result_struct(name) {
            self.type_layouts
                .add_type_flags(name.to_string(), TypeFlags::FALLIBLE);
        }
    }

    fn is_result_struct(name: &str) -> bool {
        let short = name.rsplit("::").next().unwrap_or(name);
        short.eq_ignore_ascii_case("Result")
    }

    pub(crate) fn dispose_symbol(&self, type_name: &str) -> Option<String> {
        let qualified = format!("{type_name}::dispose");
        let signature = self.symbol_index.function_signature(&qualified)?;
        if signature.abi != Abi::Chic {
            return None;
        }
        if signature.params.len() != 1 {
            return None;
        }
        if !matches!(*signature.ret, Ty::Unit) {
            return None;
        }
        let param = &signature.params[0];
        fn matches_type(param: &Ty, type_name: &str) -> bool {
            match param {
                Ty::Named(name) => {
                    if name.as_str() == type_name
                        || name.as_str() == "Self"
                        || name.as_str().eq_ignore_ascii_case("var")
                    {
                        true
                    } else {
                        let param_last = name.as_str().rsplit("::").next();
                        let ty_last = type_name.rsplit("::").next();
                        param_last == ty_last
                    }
                }
                Ty::Ref(inner) => matches_type(&inner.element, type_name),
                Ty::Pointer(pointer) => matches_type(&pointer.element, type_name),
                Ty::Nullable(inner) => matches_type(inner, type_name),
                _ => false,
            }
        }
        let matches_self = matches_type(param, type_name);
        if matches_self { Some(qualified) } else { None }
    }

    pub(crate) fn compute_field_layouts(
        &mut self,
        fields: &[FieldDecl],
        namespace: Option<&str>,
        context_type: Option<&str>,
        packing: Option<usize>,
        base_offset: usize,
        index_base: usize,
    ) -> (Vec<FieldLayout>, Option<usize>, Option<usize>) {
        let mut layouts = Vec::with_capacity(fields.len());
        let mut offset = base_offset;
        let mut offset_known = true;
        let mut struct_align = MIN_ALIGN;
        let mut known_any = base_offset != 0;
        let packing_limit = packing.unwrap_or(usize::MAX).max(MIN_ALIGN);

        for (index, field) in fields.iter().enumerate() {
            self.ensure_type_expr_accessible(
                &field.ty,
                namespace,
                context_type,
                &format!("field `{}`", field.name),
                None,
            );
            let ty = self.ty_from_type_expr(&field.ty, namespace, context_type);
            let size_align = self.type_size_and_align(&ty, namespace);

            if let Some((_size, align)) = size_align {
                let base_align = align.max(MIN_ALIGN);
                let effective_align = if packing_limit == usize::MAX {
                    base_align
                } else {
                    base_align.min(packing_limit)
                };
                struct_align = struct_align.max(effective_align);
                known_any = true;
            }

            let field_offset = if offset_known {
                if let Some((size, align)) = size_align {
                    let base_align = align.max(MIN_ALIGN);
                    let effective_align = if packing_limit == usize::MAX {
                        base_align
                    } else {
                        base_align.min(packing_limit)
                    };
                    let aligned = align_to(offset, effective_align);
                    offset = aligned + size;
                    Some(aligned)
                } else {
                    offset_known = false;
                    None
                }
            } else {
                None
            };

            let index_u32 = expect_u32_index(index_base.saturating_add(index), "field index");
            layouts.push(FieldLayout {
                name: field.name.clone(),
                ty,
                index: index_u32,
                offset: field_offset,
                span: None,
                mmio: None,
                display_name: field.display_name.clone(),
                is_required: field.is_required,
                is_nullable: field.ty.is_nullable(),
                is_readonly: field.is_readonly,
                view_of: field.view_of.clone(),
            });
        }

        let align = if known_any {
            Some(struct_align.max(MIN_ALIGN))
        } else if fields.is_empty() {
            Some(MIN_ALIGN)
        } else {
            None
        };

        let size = if offset_known {
            align.map(|align| align_to(offset, align))
        } else {
            None
        };

        (layouts, size, align)
    }

    pub(crate) fn ensure_type_expr_accessible(
        &mut self,
        ty: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
        usage: &str,
        span: Option<Span>,
    ) {
        if let Some(descriptor) = sequence_descriptor(ty) {
            self.ensure_type_expr_accessible(
                descriptor.element,
                namespace,
                context_type,
                usage,
                span,
            );
        } else {
            match self.resolve_type_for_expr(ty, namespace, context_type) {
                ImportResolution::Found(resolved) => {
                    self.ensure_type_accessible_resolved(
                        &resolved,
                        namespace,
                        context_type,
                        usage,
                        span,
                    );
                }
                ImportResolution::Ambiguous(candidates) => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "{usage} resolves to multiple types: {}",
                            candidates.join(", ")
                        ),
                        span,
                    });
                }
                ImportResolution::NotFound => {
                    self.ensure_type_accessible_resolved(
                        &ty.name.replace('.', "::"),
                        namespace,
                        context_type,
                        usage,
                        span,
                    );
                }
            }
        }
    }

    pub(crate) fn ty_from_type_expr(
        &mut self,
        expr: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> Ty {
        let mut alias_stack = Vec::new();
        let expanded = self
            .try_expand_alias(expr, namespace, context_type, &mut alias_stack)
            .unwrap_or_else(|| expr.clone());
        let ty = Ty::from_type_expr(&expanded);
        self.ensure_ty_layout(&ty);
        ty
    }

    pub(super) fn ensure_ty_layout(&mut self, ty: &Ty) {
        match ty {
            Ty::Tuple(tuple) => {
                self.type_layouts.ensure_tuple_layout(tuple);
                for element in &tuple.elements {
                    self.ensure_ty_layout(element);
                }
            }
            Ty::Array(array) => {
                self.ensure_ty_layout(array.element.as_ref());
                self.ensure_array_layout(array);
            }
            Ty::Vec(vec) => {
                self.ensure_ty_layout(vec.element.as_ref());
                self.ensure_vec_layout(vec);
            }
            Ty::Span(span) => {
                self.ensure_ty_layout(span.element.as_ref());
                self.ensure_span_layout(span);
            }
            Ty::ReadOnlySpan(span) => {
                self.ensure_ty_layout(span.element.as_ref());
                self.ensure_readonly_span_layout(span);
            }
            Ty::Nullable(inner) => {
                self.ensure_ty_layout(inner);
                self.type_layouts.ensure_nullable_layout(inner);
            }
            _ => {}
        }
    }

    fn ensure_array_layout(&mut self, array: &ArrayTy) {
        let name = Ty::Array(array.clone()).canonical_name();
        if self.type_layouts.types.contains_key(&name) {
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

        // Arrays are backed by the Chic runtime Vec representation today.
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
            auto_traits: auto_traits::unknown_set(),
            overrides: auto_traits::default_override(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.type_layouts
            .types
            .insert(name, TypeLayout::Struct(layout));
    }

    fn ensure_vec_layout(&mut self, vec: &VecTy) {
        let name = Ty::Vec(vec.clone()).canonical_name();
        if self.type_layouts.types.contains_key(&name) {
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
            auto_traits: auto_traits::unknown_set(),
            overrides: auto_traits::default_override(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.type_layouts
            .types
            .insert(name, TypeLayout::Struct(layout));
    }

    fn ensure_span_layout(&mut self, span: &SpanTy) {
        let name = Ty::Span(span.clone()).canonical_name();
        if self.type_layouts.types.contains_key(&name) {
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
            auto_traits: auto_traits::unknown_set(),
            overrides: auto_traits::default_override(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.type_layouts
            .types
            .insert(name, TypeLayout::Struct(layout));
    }

    fn ensure_readonly_span_layout(&mut self, span: &ReadOnlySpanTy) {
        let name = Ty::ReadOnlySpan(span.clone()).canonical_name();
        if self.type_layouts.types.contains_key(&name) {
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
            auto_traits: auto_traits::unknown_set(),
            overrides: auto_traits::default_override(),
            mmio: None,
            dispose: None,
            class: None,
        };

        self.type_layouts
            .types
            .insert(name, TypeLayout::Struct(layout));
    }

    pub(crate) fn lower_struct(&mut self, strct: &StructDecl, namespace: Option<&str>) {
        let struct_ns = qualify(namespace, &strct.name);
        let struct_layout =
            self.type_layouts
                .types
                .get(&struct_ns)
                .and_then(|layout| match layout {
                    TypeLayout::Struct(layout) => Some(layout.clone()),
                    _ => None,
                });
        for method in &strct.methods {
            let method_name = format!("{struct_ns}::{}", method.name);
            self.collect_exports_for(&method_name, &method.attributes);
            self.collect_link_library(method.link_library.as_deref());
            self.check_signature(
                &method.signature,
                namespace,
                Some(struct_ns.as_str()),
                &method_name,
            );
            let perf_diags = self.record_perf_attributes(&method_name, &method.attributes, None);
            self.diagnostics.extend(perf_diags);
            let lowered = lower_function(
                method,
                &method_name,
                FunctionKind::Method,
                Some(struct_ns.as_str()),
                self.current_package.as_deref(),
                strct.generics.as_ref(),
                &mut self.type_layouts,
                &self.type_visibilities,
                &self.primitive_registry,
                self.default_arguments.clone(),
                &self.function_packages,
                &self.operator_registry,
                &mut self.string_interner,
                &self.symbol_index,
                &self.import_resolver,
                &self.static_registry,
                &self.class_bases,
                &self.class_virtual_slots,
                &self.trait_decls,
                self.generic_specializations.clone(),
            );
            let _ = self.record_lowered_function(lowered);
        }
        for property in &strct.properties {
            self.lower_property(struct_ns.as_str(), property, strct.generics.as_ref());
        }
        let mut ctor_index = 0usize;
        for ctor in &strct.constructors {
            if let Some(layout) = struct_layout.as_ref() {
                let ctor_name = format!("{struct_ns}::init#{ctor_index}");
                let perf_diags = self.record_perf_attributes(&ctor_name, &ctor.attributes, None);
                self.diagnostics.extend(perf_diags);
                let lowered = lower_constructor(
                    struct_ns.as_str(),
                    ctor,
                    &ctor_name,
                    Some(struct_ns.as_str()),
                    self.current_package.as_deref(),
                    strct.generics.as_ref(),
                    &mut self.type_layouts,
                    &self.type_visibilities,
                    &self.primitive_registry,
                    self.default_arguments.clone(),
                    &self.function_packages,
                    &self.operator_registry,
                    &mut self.string_interner,
                    &self.symbol_index,
                    &self.import_resolver,
                    &self.static_registry,
                    &self.class_bases,
                    &self.class_virtual_slots,
                    &self.trait_decls,
                    self.generic_specializations.clone(),
                    layout,
                );
                let _ = self.record_lowered_function(lowered);
            } else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "missing layout for struct `{}` while lowering constructor",
                        struct_ns
                    ),
                    span: ctor.span,
                });
            }
            ctor_index += 1;
        }
    }

    pub(crate) fn infer_list_layout(fields: &[FieldLayout]) -> Option<ListLayout> {
        let mut length_field = None;
        let mut element_field = None;
        for field in fields {
            let lower = field.name.to_ascii_lowercase();
            if length_field.is_none() && matches!(lower.as_str(), "length" | "count" | "len") {
                length_field = Some(field.index);
            } else if element_field.is_none()
                && matches!(lower.as_str(), "data" | "items" | "elements" | "ptr")
            {
                element_field = Some(field.index);
            }
        }
        if length_field.is_some() || element_field.is_some() {
            return Some(ListLayout {
                element_index: element_field,
                length_index: length_field,
                span: None,
            });
        }
        None
    }

    pub(crate) fn type_size_and_align(
        &self,
        ty: &Ty,
        namespace: Option<&str>,
    ) -> Option<(usize, usize)> {
        type_size_and_align_for_ty(
            ty,
            &self.type_layouts,
            Some(&self.import_resolver),
            namespace,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::FieldDecl;
    use crate::frontend::ast::Visibility;

    #[test]
    fn inline_cross_struct_layout_sets_flag() {
        let mut lowering = ModuleLowering::default();
        let strct = StructDecl {
            visibility: Visibility::Public,
            name: "Opt".into(),
            fields: vec![FieldDecl {
                visibility: Visibility::Public,
                name: "Value".into(),
                ty: TypeExpr::simple("int"),
                initializer: None,
                mmio: None,
                doc: None,
                is_required: false,
                display_name: None,
                attributes: Vec::new(),
                is_readonly: false,
                is_static: false,
                view_of: None,
            }],
            properties: Vec::new(),
            constructors: Vec::new(),
            consts: Vec::new(),
            methods: Vec::new(),
            nested_types: Vec::new(),
            bases: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            mmio: None,
            doc: None,
            generics: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: Some(InlineAttr::Cross),
            is_record: false,
            record_positional_fields: Vec::new(),
        };
        lowering.register_struct_layout(&strct, Some("Sample"));
        let layout = lowering
            .type_layouts
            .layout_for_name("Sample::Opt")
            .and_then(|entry| match entry {
                TypeLayout::Struct(data) => Some(data.clone()),
                _ => None,
            })
            .expect("layout missing");
        assert!(
            layout.allow_cross_inline,
            "inline(cross) should mark layout as cross-inlineable"
        );
    }
}
