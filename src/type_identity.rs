use std::borrow::Cow;

use crate::drop_glue::drop_type_identity;
use crate::mir::{TypeLayout, TypeLayoutTable};

pub(crate) fn type_identity_seed_for_name<'a>(
    type_layouts: &'a TypeLayoutTable,
    name: &'a str,
) -> Cow<'a, str> {
    let Some(primitive_id) = type_layouts.primitive_registry.lookup_by_name(name) else {
        return Cow::Borrowed(name);
    };
    let Some(desc) = type_layouts.primitive_registry.descriptor(primitive_id) else {
        return Cow::Borrowed(name);
    };

    let is_intrinsic = type_layouts
        .layout_for_name(name)
        .is_some_and(|layout| match layout {
            TypeLayout::Struct(info) | TypeLayout::Class(info) => info.is_intrinsic,
            TypeLayout::Enum(_) | TypeLayout::Union(_) => false,
        });

    let is_primitive_spelling =
        !name.contains("::") && !name.contains('.') && name == desc.primitive_name;

    if is_intrinsic || is_primitive_spelling {
        return Cow::Borrowed(desc.primitive_name.as_str());
    }

    Cow::Borrowed(name)
}

pub(crate) fn type_identity_for_name(type_layouts: &TypeLayoutTable, name: &str) -> u64 {
    let seed = type_identity_seed_for_name(type_layouts, name);
    drop_type_identity(seed.as_ref())
}
