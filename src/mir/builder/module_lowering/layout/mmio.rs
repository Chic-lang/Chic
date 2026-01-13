//! MMIO struct layout computation helpers.

use super::super::super::{FieldLayout, MIN_ALIGN, StructDecl};
use super::super::driver::{LoweringDiagnostic, ModuleLowering, expect_u32_index};
use crate::frontend::ast::MmioStructAttr;
use crate::mir::layout::{MmioFieldLayout, MmioStructLayout};
use crate::mir::{MmioAccess, MmioEndianness};

impl ModuleLowering {
    // --- layout::mmio (planned extraction) ---
    // Depends on type accessibility checks in super::driver, shared field layout helpers, and backend MMIO metadata types.
    pub(crate) fn compute_mmio_struct_layout(
        &mut self,
        strct: &StructDecl,
        namespace: Option<&str>,
        context_type: &str,
        attr: &MmioStructAttr,
    ) -> (
        Vec<FieldLayout>,
        Option<usize>,
        Option<usize>,
        MmioStructLayout,
    ) {
        let mut layouts = Vec::with_capacity(strct.fields.len());
        let mut max_end: u64 = 0;
        let mut max_align: u64 = MIN_ALIGN as u64;
        let mut ranges: Vec<(u64, u64, String)> = Vec::new();
        let mut any_field = false;

        for (index, field) in strct.fields.iter().enumerate() {
            self.ensure_type_expr_accessible(
                &field.ty,
                namespace,
                Some(context_type),
                &format!("field `{}`", field.name),
                None,
            );
            let ty = self.ty_from_type_expr(&field.ty, namespace, Some(context_type));
            let ty_display = ty.canonical_name();
            let index_u32 = expect_u32_index(index, "field index");

            let Some(spec) = field.mmio.as_ref() else {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "MMIO struct `{context_type}` field `{}` requires a `@register` annotation",
                        field.name
                    ),
                    span: None,
                });
                layouts.push(FieldLayout {
                    name: field.name.clone(),
                    ty,
                    index: index_u32,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: field.display_name.clone(),
                    is_required: field.is_required,
                    is_nullable: field.ty.is_nullable(),
                    is_readonly: field.is_readonly,
                    view_of: field.view_of.clone(),
                });
                continue;
            };

            let offset = u64::from(spec.offset);
            let width_bits = u64::from(spec.width_bits);
            if width_bits == 0 || width_bits % 8 != 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "register `{}` in `{context_type}` must specify a width that is a multiple of 8 bits",
                        field.name
                    ),
                    span: None,
                                    });
                layouts.push(FieldLayout {
                    name: field.name.clone(),
                    ty,
                    index: index_u32,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: field.display_name.clone(),
                    is_required: field.is_required,
                    is_nullable: field.ty.is_nullable(),
                    is_readonly: field.is_readonly,
                    view_of: field.view_of.clone(),
                });
                continue;
            }

            let width_bytes = width_bits / 8;
            let end = offset.saturating_add(width_bytes);
            max_end = max_end.max(end);
            max_align = max_align.max(width_bytes.max(1));
            any_field = true;

            for (start, finish, other) in &ranges {
                if offset < *finish && end > *start {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "MMIO registers `{}` and `{other}` in `{context_type}` overlap",
                            field.name
                        ),
                        span: None,
                    });
                    break;
                }
            }
            ranges.push((offset, end, field.name.clone()));

            if width_bytes > 0 && offset % width_bytes != 0 {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "register `{}` in `{context_type}` has offset {:#x} which is not aligned to its {}-byte width",
                        field.name, offset, width_bytes
                    ),
                    span: None,
                                    });
            }

            if let Some((size, _align)) = self.type_size_and_align(&ty, namespace) {
                let expected = usize::try_from(width_bytes).unwrap_or(size);
                if size != expected {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "type `{ty_display}` for register `{}` does not match declared width of {} bits",
                            field.name, spec.width_bits
                        ),
                        span: None,
                                            });
                }
            }

            let offset_usize = match usize::try_from(offset) {
                Ok(value) => Some(value),
                Err(_) => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "register `{}` in `{context_type}` has offset exceeding host addressable range",
                            field.name
                        ),
                        span: None,
                                            });
                    None
                }
            };

            layouts.push(FieldLayout {
                name: field.name.clone(),
                ty,
                index: index_u32,
                offset: offset_usize,
                span: None,
                mmio: Some(MmioFieldLayout {
                    offset: spec.offset,
                    width_bits: spec.width_bits,
                    access: match spec.access {
                        crate::frontend::ast::MmioAccess::ReadOnly => MmioAccess::ReadOnly,
                        crate::frontend::ast::MmioAccess::WriteOnly => MmioAccess::WriteOnly,
                        crate::frontend::ast::MmioAccess::ReadWrite => MmioAccess::ReadWrite,
                    },
                }),
                display_name: field.display_name.clone(),
                is_required: field.is_required,
                is_nullable: field.ty.is_nullable(),
                is_readonly: field.is_readonly,
                view_of: field.view_of.clone(),
            });
        }

        if !any_field {
            max_align = MIN_ALIGN as u64;
        }

        let computed_size = if any_field { Some(max_end) } else { Some(0) };

        let size = if let Some(explicit) = attr.size {
            if explicit < max_end {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "`@mmio(size = 0x{explicit:x})` for `{context_type}` is smaller than the largest register end offset (0x{max_end:x})"
                    ),
                    span: None,
                                    });
            }
            Some(explicit as usize)
        } else {
            match usize::try_from(max_end) {
                Ok(value) => Some(value),
                Err(_) => {
                    self.diagnostics.push(LoweringDiagnostic {
                        message: format!(
                            "total MMIO size ({max_end}) for `{context_type}` exceeds host addressable range"
                        ),
                        span: None,
                                            });
                    computed_size.map(|value| value as usize)
                }
            }
        };

        let align = match usize::try_from(max_align.max(MIN_ALIGN as u64)) {
            Ok(value) => Some(value),
            Err(_) => {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!(
                        "required MMIO alignment ({}) for `{context_type}` exceeds host addressable range",
                        max_align.max(MIN_ALIGN as u64)
                    ),
                    span: None,
                                    });
                Some(MIN_ALIGN)
            }
        };

        let mmio_layout = MmioStructLayout {
            base_address: attr.base_address,
            size: attr.size,
            address_space: attr.address_space.clone(),
            endianness: match attr.endianness {
                crate::frontend::ast::MmioEndianness::Little => MmioEndianness::Little,
                crate::frontend::ast::MmioEndianness::Big => MmioEndianness::Big,
            },
            requires_unsafe: attr.requires_unsafe,
        };

        (layouts, size, align, mmio_layout)
    }
}
