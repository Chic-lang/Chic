use crate::mir::{
    ArrayTy, AutoTraitOverride, AutoTraitSet, EnumLayout, EnumVariantLayout, ListLayout,
    StructLayout, Ty, TypeLayout, TypeLayoutTable, TypeRepr, make_field,
};
use crate::target::Target;

pub(super) fn function_ir<'a>(ir: &'a str, symbol: &str) -> &'a str {
    let needle = format!("@{symbol}");
    let pos = ir
        .find(&needle)
        .unwrap_or_else(|| panic!("symbol {symbol} not present in IR"));
    let prefix = &ir[..pos];
    let def_start = prefix
        .rfind("define")
        .unwrap_or_else(|| panic!("missing define before {symbol}"));
    let rest = &ir[def_start..];
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("missing function terminator for {symbol}"))
        + def_start
        + 2;
    &ir[def_start..end]
}

pub(super) fn test_target() -> Target {
    Target::parse("x86_64-unknown-none").expect("parse target")
}

pub(super) fn apple_target() -> Target {
    Target::parse("arm64-apple-macosx15.0").expect("parse arm64 apple target")
}

pub(super) fn linux_target() -> Target {
    Target::parse("aarch64-unknown-linux-gnu").expect("parse arm64 linux target")
}

pub(super) fn flag_layouts() -> TypeLayoutTable {
    let mut layouts = TypeLayoutTable::default();
    layouts.types.insert(
        "Flags::Permissions".into(),
        TypeLayout::Enum(EnumLayout {
            name: "Flags::Permissions".into(),
            repr: TypeRepr::Default,
            packing: None,
            underlying: Ty::named("int"),
            underlying_info: Some(crate::mir::casts::IntInfo {
                bits: 32,
                signed: true,
            }),
            explicit_underlying: false,
            variants: vec![
                EnumVariantLayout {
                    name: "None".into(),
                    index: 0,
                    discriminant: 0,
                    fields: Vec::new(),
                    positional: Vec::new(),
                },
                EnumVariantLayout {
                    name: "Read".into(),
                    index: 1,
                    discriminant: 1,
                    fields: Vec::new(),
                    positional: Vec::new(),
                },
                EnumVariantLayout {
                    name: "Write".into(),
                    index: 2,
                    discriminant: 2,
                    fields: Vec::new(),
                    positional: Vec::new(),
                },
                EnumVariantLayout {
                    name: "Execute".into(),
                    index: 3,
                    discriminant: 4,
                    fields: Vec::new(),
                    positional: Vec::new(),
                },
                EnumVariantLayout {
                    name: "All".into(),
                    index: 4,
                    discriminant: 7,
                    fields: Vec::new(),
                    positional: Vec::new(),
                },
            ],
            size: Some(4),
            align: Some(4),
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            is_flags: true,
        }),
    );
    layouts
}

pub(super) fn ensure_array_layout(layouts: &mut TypeLayoutTable, element: Ty) -> Ty {
    let array_ty = Ty::Array(ArrayTy::new(Box::new(element), 1));
    let name = array_ty.canonical_name();
    if !layouts.types.contains_key(&name) {
        let word = std::mem::size_of::<usize>();
        let align = std::mem::align_of::<usize>();
        let mut offset = 0usize;
        let mut fields = Vec::new();
        fields.push(make_field("ptr", Ty::named("byte*"), 0, offset));
        offset += word;
        fields.push(make_field("len", Ty::named("usize"), 1, offset));
        offset += word;
        fields.push(make_field("cap", Ty::named("usize"), 2, offset));
        offset += word;
        fields.push(make_field("elem_size", Ty::named("usize"), 3, offset));
        offset += word;
        fields.push(make_field("elem_align", Ty::named("usize"), 4, offset));
        offset += word;
        fields.push(make_field("drop_fn", Ty::named("byte*"), 5, offset));
        offset += word;
        let size = if align <= 1 {
            offset
        } else {
            ((offset + align - 1) / align) * align
        };

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
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        };
        layouts.types.insert(name, TypeLayout::Struct(layout));
    }
    array_ty
}
