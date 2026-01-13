use super::arena::TypeChecker;
use super::diagnostics::codes;
use crate::mir::{TypeLayout, TypeRepr};

impl<'a> TypeChecker<'a> {
    pub(super) fn verify_layout_attributes(&mut self) {
        for (name, infos) in self.types.clone() {
            let Some(layout) = self.type_layouts.types.get(&name) else {
                continue;
            };
            for info in &infos {
                if info.repr_c && !layout_repr_is_c(layout) {
                    self.emit_error(
                        codes::LAYOUT_REPR_MISMATCH,
                        None,
                        format!("`@repr(c)` on `{name}` did not propagate to layout"),
                    );
                }

                if let Some(expected_pack) = info.packing {
                    if layout_packing(layout) != Some(expected_pack) {
                        self.emit_error(
                            codes::LAYOUT_PACK_MISMATCH,
                            None,
                            format!("`@repr(packed)` on `{name}` did not propagate to layout"),
                        );
                    }
                }

                if let Some(expected_align) = info.align {
                    if let Some(actual_align) = layout_align(layout) {
                        if actual_align < expected_align as usize {
                            self.emit_error(
                                codes::LAYOUT_ALIGN_MISMATCH,
                                None,
                                format!(
                                    "`@align({expected_align})` on `{name}` did not raise layout alignment"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn layout_repr_is_c(layout: &TypeLayout) -> bool {
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data.repr == TypeRepr::C,
        TypeLayout::Enum(data) => data.repr == TypeRepr::C,
        TypeLayout::Union(data) => data.repr == TypeRepr::C,
    }
}

fn layout_packing(layout: &TypeLayout) -> Option<u32> {
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data.packing,
        TypeLayout::Enum(data) => data.packing,
        TypeLayout::Union(data) => data.packing,
    }
}

fn layout_align(layout: &TypeLayout) -> Option<usize> {
    match layout {
        TypeLayout::Struct(data) | TypeLayout::Class(data) => data.align,
        TypeLayout::Enum(data) => data.align,
        TypeLayout::Union(data) => data.align,
    }
}
