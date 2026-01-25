use std::borrow::Cow;

use crate::drop_glue::drop_type_identity;
use crate::mir::TypeLayoutTable;

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

    // Once a name resolves to a primitive descriptor (including aliases like `u64`/`i32` and
    // wrapper spellings like `Std::UInt64`), its identity must be stable across the compiler,
    // runtime glue tables, and user code. Canonicalize to the primitive spelling.
    Cow::Borrowed(desc.primitive_name.as_str())
}

pub(crate) fn type_identity_for_name(type_layouts: &TypeLayoutTable, name: &str) -> u64 {
    let seed = type_identity_seed_for_name(type_layouts, name);
    drop_type_identity(seed.as_ref())
}
