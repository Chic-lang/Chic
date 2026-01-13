use crate::mir::make_field;
use crate::mir::{
    AutoTraitOverride, AutoTraitSet, ListLayout, StructLayout, TupleTy, TypeLayout,
    TypeLayoutTable, TypeRepr,
};
use crate::mir::{ClassLayoutInfo, ClassLayoutKind, Ty};

pub(crate) fn sample_pair_layout() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let struct_layout = StructLayout {
        name: "Demo::Pair".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![
            make_field("X", Ty::named("int"), 0, 0),
            make_field("Y", Ty::named("int"), 1, 4),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(8),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    layouts
        .types
        .insert("Demo::Pair".into(), TypeLayout::Struct(struct_layout));
    layouts
}

pub(crate) fn sample_class_layout() -> TypeLayoutTable {
    let mut layouts = super::super::wasm_layouts();
    let class_layout = StructLayout {
        name: "Demo::Window".into(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![
            make_field("Width", Ty::named("int"), 0, 0),
            make_field("Height", Ty::named("int"), 1, 4),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(8),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases: Vec::new(),
            vtable_offset: Some(0),
        }),
    };
    layouts
        .types
        .insert("Demo::Window".into(), TypeLayout::Class(class_layout));
    layouts
}

pub(crate) fn register_array_layout(layouts: &mut TypeLayoutTable, element: Ty) -> Ty {
    let array_ty = Ty::Array(crate::mir::ArrayTy::new(Box::new(element), 1));
    let name = array_ty.canonical_name();
    if layouts.types.contains_key(&name) {
        return array_ty;
    }

    let word = crate::mir::pointer_size();
    let mut offset = 0usize;
    let fields = vec![
        make_field("ptr", Ty::named("byte*"), 0, offset),
        {
            offset += word;
            make_field("len", Ty::named("usize"), 1, offset)
        },
        {
            offset += word;
            make_field("cap", Ty::named("usize"), 2, offset)
        },
        {
            offset += word;
            make_field("elem_size", Ty::named("usize"), 3, offset)
        },
        {
            offset += word;
            make_field("elem_align", Ty::named("usize"), 4, offset)
        },
        {
            offset += word;
            make_field("drop_fn", Ty::named("byte*"), 5, offset)
        },
    ];
    offset += word;

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
        size: Some(offset),
        align: Some(word),
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
    array_ty
}

pub(crate) fn simple_tuple_ty() -> TupleTy {
    TupleTy::new(vec![Ty::named("int"), Ty::named("int")])
}
