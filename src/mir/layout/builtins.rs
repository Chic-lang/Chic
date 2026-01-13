use super::auto_traits::{AutoTraitOverride, AutoTraitSet};
use super::table::{
    ClassLayoutInfo, ClassLayoutKind, EnumLayout, EnumVariantLayout, MIN_ALIGN, StructLayout,
    TypeLayout, TypeLayoutTable, TypeRepr, align_to, make_field, pointer_align, pointer_size,
};
use crate::mir::casts::IntInfo;
use crate::mir::data::{PointerQualifiers, PointerTy, Ty};

impl TypeLayoutTable {
    pub(crate) fn insert_builtin_collections_layouts(&mut self) {
        const PAD7: &str = "Std::Runtime::Collections::InlinePadding7";
        const BYTES64: &str = "Std::Runtime::Collections::InlineBytes64";

        if !self.types.contains_key(PAD7) {
            let mut fields = Vec::new();
            let mut offset = 0usize;
            for idx in 0..7usize {
                let field_name = format!("b{idx}");
                fields.push(make_field(
                    &field_name,
                    Ty::named("byte"),
                    idx as u32,
                    offset,
                ));
                offset += 1;
            }
            self.types.insert(
                PAD7.into(),
                TypeLayout::Struct(StructLayout {
                    name: PAD7.into(),
                    repr: TypeRepr::C,
                    packing: None,
                    fields,
                    positional: Vec::new(),
                    list: None,
                    size: Some(7),
                    align: Some(1),
                    is_readonly: false,
                    is_intrinsic: true,
                    allow_cross_inline: true,
                    auto_traits: AutoTraitSet::all_unknown(),
                    overrides: AutoTraitOverride::default(),
                    mmio: None,
                    dispose: None,
                    class: None,
                }),
            );
        }

        if !self.types.contains_key(BYTES64) {
            self.types.insert(
                BYTES64.into(),
                TypeLayout::Struct(chic_inline_bytes_layout(BYTES64, 64)),
            );
        }
    }

    pub(crate) fn insert_builtin_string_layouts(&mut self) {
        if !self.types.contains_key("string") {
            self.types
                .insert("string".into(), TypeLayout::Struct(owned_string_layout()));
        }
        if !self.types.contains_key("str") {
            self.types
                .insert("str".into(), TypeLayout::Struct(borrowed_str_layout()));
        }
        const INLINE32: &str = "Std::Runtime::Native::StringInlineBytes32";
        const INLINE64: &str = "Std::Runtime::Native::StringInlineBytes64";
        const CHIC_STR: &str = "Std::Runtime::Native::ChicStr";
        const CHIC_STRING: &str = "Std::Runtime::Native::ChicString";
        const CHIC_STR_SHORT: &str = "ChicStr";
        const CHIC_STRING_SHORT: &str = "ChicString";

        if !self.types.contains_key(INLINE32) {
            self.types.insert(
                INLINE32.into(),
                TypeLayout::Struct(chic_inline_bytes_layout(INLINE32, 32)),
            );
        }
        if !self.types.contains_key(INLINE64) {
            self.types.insert(
                INLINE64.into(),
                TypeLayout::Struct(chic_inline_bytes_layout(INLINE64, 64)),
            );
        }
        if !self.types.contains_key(CHIC_STR) {
            self.types.insert(
                CHIC_STR.into(),
                TypeLayout::Struct(chic_str_layout(CHIC_STR)),
            );
        }
        if !self.types.contains_key(CHIC_STR_SHORT) {
            self.types.insert(
                CHIC_STR_SHORT.into(),
                TypeLayout::Struct(chic_str_layout(CHIC_STR_SHORT)),
            );
        }
        if !self.types.contains_key(CHIC_STRING) {
            self.types.insert(
                CHIC_STRING.into(),
                TypeLayout::Struct(chic_string_layout(CHIC_STRING)),
            );
        }
        if !self.types.contains_key(CHIC_STRING_SHORT) {
            self.types.insert(
                CHIC_STRING_SHORT.into(),
                TypeLayout::Struct(chic_string_layout(CHIC_STRING_SHORT)),
            );
        }
    }

    pub(crate) fn insert_builtin_shared_layouts(&mut self) {
        if !self.types.contains_key("Rc") {
            self.types
                .insert("Rc".into(), TypeLayout::Struct(shared_pointer_layout("Rc")));
        }
        if !self.types.contains_key("Arc") {
            self.types.insert(
                "Arc".into(),
                TypeLayout::Struct(shared_pointer_layout("Arc")),
            );
        }
    }

    pub(crate) fn insert_builtin_startup_layouts(&mut self) {
        const ENTRY: &str = "Std::Runtime::Startup::ChicStartupEntrySnapshot";
        const SUITE: &str = "Std::Runtime::Startup::ChicStartupTestSuiteSnapshot";
        const STARTUP: &str = "Std::Runtime::Startup::ChicStartupDescriptorSnapshot";
        const TESTCASE: &str = "Std::Runtime::Startup::ChicStartupTestCaseSnapshot";

        if !self.types.contains_key(ENTRY) {
            self.types
                .insert(ENTRY.into(), TypeLayout::Struct(entry_descriptor_layout()));
        }
        if !self.types.contains_key(SUITE) {
            self.types.insert(
                SUITE.into(),
                TypeLayout::Struct(test_suite_descriptor_layout()),
            );
        }
        if !self.types.contains_key(STARTUP) {
            self.types.insert(
                STARTUP.into(),
                TypeLayout::Struct(startup_descriptor_layout()),
            );
        }
        if !self.types.contains_key(TESTCASE) {
            self.types.insert(
                TESTCASE.into(),
                TypeLayout::Struct(testcase_descriptor_layout()),
            );
        }
    }

    pub(crate) fn insert_builtin_async_layouts(&mut self) {
        const HEADER: &str = "Std::Async::FutureHeader";
        const FUTURE: &str = "Std::Async::Future";
        const FUTURE_BOOL: &str = "Std::Async::Future<bool>";
        const FUTURE_INT: &str = "Std::Async::Future<int>";
        const TASK: &str = "Std::Async::Task";
        const TASK_BOOL: &str = "Std::Async::Task<bool>";
        const TASK_INT: &str = "Std::Async::Task<int>";
        const RUNTIME_CTX: &str = "Std::Async::RuntimeContext";

        if !self.types.contains_key(HEADER) {
            self.types
                .insert(HEADER.into(), TypeLayout::Struct(future_header_layout()));
        }
        if !self.types.contains_key(FUTURE) {
            self.types
                .insert(FUTURE.into(), TypeLayout::Struct(untyped_future_layout()));
        }
        if !self.types.contains_key(FUTURE_BOOL) {
            self.types
                .insert(FUTURE_BOOL.into(), TypeLayout::Struct(future_bool_layout()));
        }
        if !self.types.contains_key(FUTURE_INT) {
            self.types
                .insert(FUTURE_INT.into(), TypeLayout::Struct(future_int_layout()));
        }
        if !self.types.contains_key(TASK) {
            self.types
                .insert(TASK.into(), TypeLayout::Struct(task_layout()));
        }
        if !self.types.contains_key(TASK_BOOL) {
            self.types
                .insert(TASK_BOOL.into(), TypeLayout::Struct(task_bool_layout()));
        }
        if !self.types.contains_key(TASK_INT) {
            self.types
                .insert(TASK_INT.into(), TypeLayout::Struct(task_int_layout()));
        }
        if !self.types.contains_key(RUNTIME_CTX) {
            self.types.insert(
                RUNTIME_CTX.into(),
                TypeLayout::Struct(runtime_context_layout()),
            );
        }
    }

    pub(crate) fn insert_builtin_decimal_layouts(&mut self) {
        const DECIMAL: &str = "decimal";
        const SYSTEM_DECIMAL: &str = "System::Decimal";
        const STD_DECIMAL: &str = "Std::Numeric::Decimal";
        const STATUS: &str = "Std::Numeric::Decimal::DecimalStatus";
        const ROUNDING: &str = "Std::Numeric::Decimal::DecimalRoundingMode";
        const ROUNDING_ENCODING: &str = "Std::Numeric::Decimal::DecimalRoundingEncoding";
        const VARIANT: &str = "Std::Numeric::Decimal::DecimalIntrinsicVariant";
        const VECT_HINT: &str = "Std::Numeric::Decimal::DecimalVectorizeHint";
        const RESULT: &str = "Std::Numeric::Decimal::DecimalIntrinsicResult";
        const RUNTIME_CALL: &str = "Std::Numeric::Decimal::DecimalRuntimeCall";
        const CONST_PTR: &str = "Std::Numeric::Decimal::DecimalConstPtr";
        const MUT_PTR: &str = "Std::Numeric::Decimal::DecimalMutPtr";
        const NATIVE_CONST_PTR: &str = "Std::Runtime::Native::DecimalConstPtr";
        const NATIVE_MUT_PTR: &str = "Std::Runtime::Native::DecimalMutPtr";
        const NATIVE_PARTS: &str = "Std::Runtime::Native::Decimal128Parts";

        if !self.types.contains_key(STATUS) {
            self.types
                .insert(STATUS.into(), TypeLayout::Enum(decimal_status_layout()));
        }
        if !self.types.contains_key(ROUNDING) {
            self.types
                .insert(ROUNDING.into(), TypeLayout::Enum(decimal_rounding_layout()));
        }
        if !self.types.contains_key(VARIANT) {
            self.types.insert(
                VARIANT.into(),
                TypeLayout::Enum(decimal_intrinsic_variant_layout()),
            );
        }
        if !self.types.contains_key(VECT_HINT) {
            self.types.insert(
                VECT_HINT.into(),
                TypeLayout::Enum(decimal_vectorize_hint_layout()),
            );
        }
        if !self.types.contains_key(RESULT) {
            self.types.insert(
                RESULT.into(),
                TypeLayout::Struct(decimal_intrinsic_result_layout()),
            );
        }
        if !self.types.contains_key(RUNTIME_CALL) {
            self.types.insert(
                RUNTIME_CALL.into(),
                TypeLayout::Struct(decimal_runtime_call_layout()),
            );
        }
        if !self.types.contains_key(ROUNDING_ENCODING) {
            self.types.insert(
                ROUNDING_ENCODING.into(),
                TypeLayout::Struct(decimal_rounding_encoding_layout()),
            );
        }
        if !self.types.contains_key(CONST_PTR) {
            self.types.insert(
                CONST_PTR.into(),
                TypeLayout::Struct(decimal_pointer_layout(CONST_PTR)),
            );
        }
        if !self.types.contains_key(MUT_PTR) {
            self.types.insert(
                MUT_PTR.into(),
                TypeLayout::Struct(decimal_pointer_layout(MUT_PTR)),
            );
        }
        if !self.types.contains_key(NATIVE_PARTS) {
            self.types.insert(
                NATIVE_PARTS.into(),
                TypeLayout::Struct(native_decimal_parts_layout(NATIVE_PARTS)),
            );
        }
        if !self.types.contains_key(NATIVE_CONST_PTR) {
            self.types.insert(
                NATIVE_CONST_PTR.into(),
                TypeLayout::Struct(native_decimal_pointer_layout(NATIVE_CONST_PTR, false)),
            );
        }
        if !self.types.contains_key(NATIVE_MUT_PTR) {
            self.types.insert(
                NATIVE_MUT_PTR.into(),
                TypeLayout::Struct(native_decimal_pointer_layout(NATIVE_MUT_PTR, true)),
            );
        }
        if !self.types.contains_key(DECIMAL) {
            self.types.insert(
                DECIMAL.into(),
                TypeLayout::Struct(decimal_value_layout(DECIMAL)),
            );
        }
        if !self.types.contains_key(SYSTEM_DECIMAL) {
            self.types.insert(
                SYSTEM_DECIMAL.into(),
                TypeLayout::Struct(decimal_value_layout(SYSTEM_DECIMAL)),
            );
        }
        if !self.types.contains_key(STD_DECIMAL) {
            self.types.insert(
                STD_DECIMAL.into(),
                TypeLayout::Struct(decimal_value_layout(STD_DECIMAL)),
            );
        }
    }

    pub(crate) fn insert_builtin_span_layouts(&mut self) {
        const VALUE_MUT_PTR: &str = "Std::Runtime::Collections::ValueMutPtr";
        const VALUE_CONST_PTR: &str = "Std::Runtime::Collections::ValueConstPtr";
        const SPAN_PTR: &str = "Std::Span::SpanPtr";
        const READONLY_SPAN_PTR: &str = "Std::Span::ReadOnlySpanPtr";

        for (name, is_mut) in [(VALUE_MUT_PTR, true), (VALUE_CONST_PTR, false)] {
            if !self.types.contains_key(name) {
                self.types.insert(
                    (*name).into(),
                    TypeLayout::Struct(value_ptr_layout(name, is_mut)),
                );
            }
        }

        if !self.types.contains_key(SPAN_PTR) {
            self.types.insert(
                SPAN_PTR.into(),
                TypeLayout::Struct(span_ptr_layout(SPAN_PTR)),
            );
        }
        if !self.types.contains_key(READONLY_SPAN_PTR) {
            self.types.insert(
                READONLY_SPAN_PTR.into(),
                TypeLayout::Struct(span_ptr_layout(READONLY_SPAN_PTR)),
            );
        }
    }

    pub(crate) fn insert_builtin_memory_layouts(&mut self) {
        const REGION_HANDLE: &str = "Std::Memory::RegionHandle";
        const NATIVE_REGION_HANDLE: &str = "Std::Runtime::Native::RegionHandle";
        const REGION_HANDLE_SHORT: &str = "RegionHandle";
        const PINNED: &str = "Std::Memory::Pinned";
        const UNIFIED: &str = "Std::Memory::Unified";
        const PINNED_SHORT: &str = "Pinned";
        const UNIFIED_SHORT: &str = "Unified";

        for name in [REGION_HANDLE, NATIVE_REGION_HANDLE, REGION_HANDLE_SHORT] {
            self.types.insert(
                (*name).into(),
                TypeLayout::Struct(region_handle_layout(name)),
            );
        }

        for name in [PINNED, UNIFIED] {
            if !self.types.contains_key(name) {
                self.types.insert(
                    (*name).into(),
                    TypeLayout::Struct(memory_length_layout(name)),
                );
            }
        }
        for name in [PINNED_SHORT, UNIFIED_SHORT] {
            if !self.types.contains_key(name) {
                self.types.insert(
                    (*name).into(),
                    TypeLayout::Struct(memory_length_layout(name)),
                );
            }
        }
    }

    pub(crate) fn insert_builtin_accelerator_layouts(&mut self) {
        const STREAM: &str = "Std::Accelerator::Stream";
        const EVENT: &str = "Std::Accelerator::Event";
        const HOST: &str = "Std::Accelerator::Host";
        const PINNED_HOST: &str = "Std::Accelerator::PinnedHost";
        const GPU: &str = "Std::Accelerator::Gpu";
        const NPU: &str = "Std::Accelerator::Npu";
        const UNIFIED: &str = "Std::Accelerator::Unified";

        for (name, layout) in [
            (HOST, TypeLayout::Struct(memspace_layout(HOST))),
            (
                PINNED_HOST,
                TypeLayout::Struct(memspace_layout(PINNED_HOST)),
            ),
            (GPU, TypeLayout::Struct(memspace_layout(GPU))),
            (NPU, TypeLayout::Struct(memspace_layout(NPU))),
            (UNIFIED, TypeLayout::Struct(memspace_layout(UNIFIED))),
            (STREAM, TypeLayout::Struct(stream_layout(STREAM))),
            (EVENT, TypeLayout::Struct(event_layout(EVENT))),
        ] {
            if !self.types.contains_key(name) {
                self.types.insert((*name).into(), layout);
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn insert_builtin_meta_layouts(&mut self) {
        for prefix in ["Std::Meta", "Foundation::Meta"] {
            insert_meta_enums(self, prefix);
            insert_meta_structs(self, prefix);
            insert_meta_descriptor_lists(self, prefix);
            if prefix == "Std::Meta" {
                insert_meta_quotes(self, prefix);
                insert_meta_quote_lists(self, prefix);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::data::Ty;

    #[test]
    fn decimal_layout_uses_16_byte_alignment() {
        let table = TypeLayoutTable::default();
        let layout = table
            .layout_for_name("decimal")
            .expect("decimal layout should be registered");
        match layout {
            TypeLayout::Struct(struct_layout) => {
                assert_eq!(
                    struct_layout.size,
                    Some(16),
                    "decimal should occupy 16 bytes"
                );
                assert_eq!(
                    struct_layout.align,
                    Some(16),
                    "decimal should align to 16 bytes"
                );
            }
            other => panic!("unexpected decimal layout: {other:?}"),
        }
    }

    #[test]
    fn int128_layout_matches_primitive_width() {
        let table = TypeLayoutTable::default();
        let (size, align) = table
            .size_and_align_for_ty(&Ty::named("int128"))
            .expect("int128 primitive layout");
        assert_eq!(size, 16);
        assert_eq!(align, 16);
    }
}

fn memspace_layout(name: &str) -> StructLayout {
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: Vec::new(),
        positional: Vec::new(),
        list: None,
        size: Some(0),
        align: Some(MIN_ALIGN),
        is_readonly: true,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn memory_length_layout(name: &str) -> StructLayout {
    let mut fields = Vec::new();
    const FIELD_ALIGN: usize = 4;
    let mut offset = 0usize;
    fields.push(make_field("Length", Ty::named("uint"), 0, offset));
    offset += FIELD_ALIGN;

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, FIELD_ALIGN)),
        align: Some(FIELD_ALIGN),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn stream_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();
    const FIELD_ALIGN: usize = 4;
    fields.push(make_field("Id", Ty::named("uint"), 0, offset));
    offset += FIELD_ALIGN;
    fields.push(make_field("Device", Ty::named("uint"), 1, offset));
    offset += FIELD_ALIGN;
    offset = align_to(offset, FIELD_ALIGN);

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, FIELD_ALIGN)),
        align: Some(FIELD_ALIGN),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::thread_share_yes_copy_no(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn event_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();
    const FIELD_ALIGN: usize = 4;
    fields.push(make_field("Id", Ty::named("uint"), 0, offset));
    offset += FIELD_ALIGN;
    fields.push(make_field("Stream", Ty::named("uint"), 1, offset));
    offset += FIELD_ALIGN;
    fields.push(make_field("Device", Ty::named("uint"), 2, offset));
    offset += FIELD_ALIGN;
    offset = align_to(offset, FIELD_ALIGN);

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, FIELD_ALIGN)),
        align: Some(FIELD_ALIGN),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::thread_share_yes_copy_no(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn shared_pointer_layout(name: &str) -> StructLayout {
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![make_field("ptr", Ty::named("usize"), 0, 0)],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size()),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn owned_string_layout() -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field("ptr", Ty::named("byte*"), 0, offset));
    offset += pointer_size();
    offset = align_to(offset, pointer_align());

    fields.push(make_field("len", Ty::named("usize"), 1, offset));
    offset += pointer_size();
    fields.push(make_field("cap", Ty::named("usize"), 2, offset));
    offset += pointer_size();

    // Keep the std string layout identical to the native Chic string handle so
    // the runtime can pass values across the FFI boundary without reshaping.
    fields.push(make_field(
        "inline_data",
        Ty::named("Std::Runtime::Native::StringInlineBytes32"),
        3,
        offset,
    ));
    offset += 32;

    let align = pointer_align();
    StructLayout {
        name: "string".to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, align)),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn borrowed_str_layout() -> StructLayout {
    StructLayout {
        name: "str".to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![
            make_field("ptr", Ty::named("byte*"), 0, 0),
            make_field("len", Ty::named("usize"), 1, pointer_size()),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size() * 2),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn chic_inline_bytes_layout(name: &str, size: usize) -> StructLayout {
    let mut fields = Vec::with_capacity(size);
    for index in 0..size {
        let field_name = format!("b{index:02}");
        fields.push(make_field(
            &field_name,
            Ty::named("byte"),
            index as u32,
            index,
        ));
    }
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(MIN_ALIGN),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn chic_str_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();
    let ptr_ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
        Ty::named("byte"),
        false,
        PointerQualifiers {
            restrict: false,
            noalias: false,
            readonly: true,
            expose_address: true,
            alignment: None,
        },
    )));
    fields.push(make_field("ptr", ptr_ty, 0, offset));
    offset += pointer_size();
    offset = align_to(offset, pointer_align());
    fields.push(make_field("len", Ty::named("usize"), 1, offset));
    offset += pointer_size();

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, pointer_align())),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn chic_string_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    let ptr_ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
        Ty::named("byte"),
        true,
        PointerQualifiers {
            restrict: false,
            noalias: false,
            readonly: false,
            expose_address: true,
            alignment: None,
        },
    )));
    fields.push(make_field("ptr", ptr_ty, 0, offset));
    offset += pointer_size();
    offset = align_to(offset, pointer_align());

    fields.push(make_field("len", Ty::named("usize"), 1, offset));
    offset += pointer_size();
    fields.push(make_field("cap", Ty::named("usize"), 2, offset));
    offset += pointer_size();

    fields.push(make_field(
        "inline_data",
        Ty::named("Std::Runtime::Native::StringInlineBytes32"),
        3,
        offset,
    ));
    offset += 32;

    let align = pointer_align();
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(align_to(offset, align)),
        align: Some(align),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn entry_descriptor_layout() -> StructLayout {
    StructLayout {
        name: "Std::Runtime::Startup::ChicStartupEntrySnapshot".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Function", Ty::named("isize"), 0, 0),
            make_field("Flags", Ty::named("uint"), 1, pointer_size()),
            make_field("Reserved", Ty::named("uint"), 2, pointer_size() + 4),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size() * 2),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn decimal_enum_layout(name: &str, variants: &[(&str, i128)]) -> EnumLayout {
    EnumLayout {
        name: name.to_string(),
        repr: TypeRepr::Default,
        packing: None,
        underlying: Ty::named("int"),
        underlying_info: Some(IntInfo {
            bits: 32,
            signed: true,
        }),
        explicit_underlying: false,
        variants: variants
            .iter()
            .enumerate()
            .map(|(index, (variant, discriminant))| EnumVariantLayout {
                name: (*variant).to_string(),
                index: index as u32,
                discriminant: *discriminant,
                fields: Vec::new(),
                positional: Vec::new(),
            })
            .collect(),
        size: Some(4),
        align: Some(4),
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        is_flags: false,
    }
}

fn decimal_status_layout() -> EnumLayout {
    decimal_enum_layout(
        "Std::Numeric::Decimal::DecimalStatus",
        &[
            ("Success", 0),
            ("Overflow", 1),
            ("DivideByZero", 2),
            ("InvalidRounding", 3),
            ("InvalidFlags", 4),
            ("InvalidPointer", 5),
            ("InvalidOperand", 6),
        ],
    )
}

fn decimal_rounding_layout() -> EnumLayout {
    decimal_enum_layout(
        "Std::Numeric::Decimal::DecimalRoundingMode",
        &[
            ("TiesToEven", 0),
            ("TowardZero", 1),
            ("AwayFromZero", 2),
            ("TowardPositive", 3),
            ("TowardNegative", 4),
        ],
    )
}

fn decimal_intrinsic_variant_layout() -> EnumLayout {
    decimal_enum_layout(
        "Std::Numeric::Decimal::DecimalIntrinsicVariant",
        &[("Scalar", 0)],
    )
}

fn decimal_vectorize_hint_layout() -> EnumLayout {
    decimal_enum_layout(
        "Std::Numeric::Decimal::DecimalVectorizeHint",
        &[("None", 0), ("Decimal", 1)],
    )
}

fn decimal_intrinsic_result_layout() -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field(
        "Status",
        Ty::named("Std::Numeric::Decimal::DecimalStatus"),
        0,
        offset,
    ));
    offset += 4;
    offset = align_to(offset, 4);

    fields.push(make_field("Value", Ty::named("decimal"), 1, offset));
    offset += 16;
    offset = align_to(offset, 4);

    fields.push(make_field(
        "Variant",
        Ty::named("Std::Numeric::Decimal::DecimalIntrinsicVariant"),
        2,
        offset,
    ));
    offset += 4;

    let size = align_to(offset, 4);

    StructLayout {
        name: "Std::Numeric::Decimal::DecimalIntrinsicResult".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn decimal_runtime_call_layout() -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field(
        "Status",
        Ty::named("Std::Numeric::Decimal::DecimalStatus"),
        0,
        offset,
    ));
    offset += 4;
    offset = align_to(offset, 4);

    fields.push(make_field("Value", Ty::named("decimal"), 1, offset));
    offset += 16;

    let size = align_to(offset, 4);

    StructLayout {
        name: "Std::Numeric::Decimal::DecimalRuntimeCall".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn decimal_rounding_encoding_layout() -> StructLayout {
    let mut fields = Vec::new();
    fields.push(make_field("Value", Ty::named("uint"), 0, 0));

    StructLayout {
        name: "Std::Numeric::Decimal::DecimalRoundingEncoding".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(4),
        align: Some(4),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn decimal_pointer_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field("Pointer", Ty::named("usize"), 0, offset));
    offset += pointer_size();

    let size = align_to(offset, pointer_align());

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn span_ptr_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    let data_ty = if name.contains("ReadOnly") {
        Ty::named("Std::Runtime::Collections::ValueConstPtr")
    } else {
        Ty::named("Std::Runtime::Collections::ValueMutPtr")
    };
    let data_size = align_to(pointer_size() * 3, pointer_align());
    fields.push(make_field("Data", data_ty, 0, offset));
    offset += data_size;

    fields.push(make_field("Length", Ty::named("usize"), 1, offset));
    offset += pointer_size();

    fields.push(make_field("ElementSize", Ty::named("usize"), 2, offset));
    offset += pointer_size();

    fields.push(make_field(
        "ElementAlignment",
        Ty::named("usize"),
        3,
        offset,
    ));
    offset += pointer_size();

    let size = align_to(offset, pointer_align());

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn region_handle_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field("Pointer", Ty::named("usize"), 0, offset));
    offset += pointer_size();

    fields.push(make_field("Profile", Ty::named("ulong"), 1, offset));
    offset += pointer_size();

    fields.push(make_field("Generation", Ty::named("ulong"), 2, offset));
    offset += pointer_size();

    let size = align_to(offset, pointer_align());

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn value_ptr_layout(name: &str, is_mut: bool) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    let element_ty = Ty::Pointer(Box::new(PointerTy::with_qualifiers(
        Ty::named("byte"),
        is_mut,
        PointerQualifiers {
            restrict: false,
            noalias: false,
            readonly: !is_mut,
            expose_address: true,
            alignment: None,
        },
    )));
    fields.push(make_field("Pointer", element_ty, 0, offset));
    offset += pointer_size();
    fields.push(make_field("Size", Ty::named("usize"), 1, offset));
    offset += pointer_size();
    fields.push(make_field("Alignment", Ty::named("usize"), 2, offset));
    offset += pointer_size();

    let size = align_to(offset, pointer_align());
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn native_decimal_pointer_layout(name: &str, mutable: bool) -> StructLayout {
    let mut qualifiers = PointerQualifiers::default();
    qualifiers.expose_address = true;
    if !mutable {
        qualifiers.readonly = true;
    }
    let pointer_ty = Ty::Pointer(Box::new(PointerTy {
        element: Ty::named("Std::Runtime::Native::Decimal128Parts"),
        mutable,
        qualifiers,
    }));
    let fields = vec![make_field("Pointer", pointer_ty, 0, 0)];
    let size = align_to(pointer_size(), pointer_align());

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn native_decimal_parts_layout(name: &str) -> StructLayout {
    let mut offset = 0usize;
    let mut fields = Vec::new();

    fields.push(make_field("lo", Ty::named("int"), 0, offset));
    offset += 4;
    fields.push(make_field("mid", Ty::named("int"), 1, offset));
    offset += 4;
    fields.push(make_field("hi", Ty::named("int"), 2, offset));
    offset += 4;
    fields.push(make_field("flags", Ty::named("int"), 3, offset));
    offset += 4;

    let size = align_to(offset, 16);

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(16),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: true,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn decimal_value_layout(name: &str) -> StructLayout {
    let fields = vec![make_field("value", Ty::named("int128"), 0, 0)];
    let size = 16;

    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(16),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn test_suite_descriptor_layout() -> StructLayout {
    StructLayout {
        name: "Std::Runtime::Startup::ChicStartupTestSuiteSnapshot".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Cases", Ty::named("isize"), 0, 0),
            make_field("Length", Ty::named("usize"), 1, pointer_size()),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size() * 2),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn startup_descriptor_layout() -> StructLayout {
    StructLayout {
        name: "Std::Runtime::Startup::ChicStartupDescriptorSnapshot".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Version", Ty::named("uint"), 0, 0),
            make_field(
                "Entry",
                Ty::named("Std::Runtime::Startup::ChicStartupEntrySnapshot"),
                1,
                pointer_size(),
            ),
            make_field(
                "Tests",
                Ty::named("Std::Runtime::Startup::ChicStartupTestSuiteSnapshot"),
                2,
                pointer_size() * 3,
            ),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size() * 5),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn testcase_descriptor_layout() -> StructLayout {
    StructLayout {
        name: "Std::Runtime::Startup::ChicStartupTestCaseSnapshot".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Function", Ty::named("isize"), 0, 0),
            make_field("NamePointer", Ty::named("isize"), 1, pointer_size()),
            make_field("NameLength", Ty::named("usize"), 2, pointer_size() * 2),
            make_field("Flags", Ty::named("uint"), 3, pointer_size() * 3),
            make_field("Reserved", Ty::named("uint"), 4, pointer_size() * 3 + 4),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size() * 4),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_yes(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn future_header_layout() -> StructLayout {
    let size = future_header_size();
    StructLayout {
        name: "Std::Async::FutureHeader".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("StatePointer", Ty::named("isize"), 0, 0),
            make_field("VTablePointer", Ty::named("isize"), 1, pointer_size()),
            make_field("ExecutorContext", Ty::named("isize"), 2, pointer_size() * 2),
            make_field("Flags", Ty::named("uint"), 3, pointer_size() * 3),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn untyped_future_layout() -> StructLayout {
    StructLayout {
        name: "Std::Async::Future".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![make_field(
            "Header",
            Ty::named("Std::Async::FutureHeader"),
            0,
            0,
        )],
        positional: Vec::new(),
        list: None,
        size: Some(future_header_size()),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn future_bool_layout() -> StructLayout {
    future_with_result_layout(
        "Std::Async::Future<bool>",
        Ty::named("bool"),
        std::mem::size_of::<bool>(),
        std::mem::align_of::<bool>(),
    )
}

fn future_int_layout() -> StructLayout {
    future_with_result_layout(
        "Std::Async::Future<int>",
        Ty::named("int"),
        std::mem::size_of::<i32>(),
        std::mem::align_of::<i32>(),
    )
}

fn future_with_result_layout(
    name: &str,
    result_ty: Ty,
    result_size: usize,
    result_align: usize,
) -> StructLayout {
    let header_size = future_header_size();
    let completed_offset = header_size;
    let result_offset = align_to(completed_offset + 1, result_align.max(1));
    let total_size = align_to(result_offset + result_size, pointer_align());
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Header", Ty::named("Std::Async::FutureHeader"), 0, 0),
            make_field("Completed", Ty::named("bool"), 1, completed_offset),
            make_field("Result", result_ty, 2, result_offset),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(total_size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn task_layout() -> StructLayout {
    let size = task_base_size();
    StructLayout {
        name: "Std::Async::Task".to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Header", Ty::named("Std::Async::FutureHeader"), 0, 0),
            make_field("Flags", Ty::named("uint"), 1, future_header_size()),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases: Vec::new(),
            vtable_offset: Some(0),
        }),
    }
}

fn task_bool_layout() -> StructLayout {
    task_with_inner_future_layout(
        "Std::Async::Task<bool>",
        "Std::Async::Future<bool>",
        future_bool_size(),
    )
}

fn task_int_layout() -> StructLayout {
    task_with_inner_future_layout(
        "Std::Async::Task<int>",
        "Std::Async::Future<int>",
        future_int_size(),
    )
}

fn task_with_inner_future_layout(
    name: &str,
    inner_future_ty: &str,
    inner_future_size: usize,
) -> StructLayout {
    let base_size = task_base_size();
    let inner_offset = align_to(base_size, pointer_align());
    let total_size = align_to(inner_offset + inner_future_size, pointer_align());
    StructLayout {
        name: name.to_string(),
        repr: TypeRepr::C,
        packing: None,
        fields: vec![
            make_field("Header", Ty::named("Std::Async::FutureHeader"), 0, 0),
            make_field("Flags", Ty::named("uint"), 1, future_header_size()),
            make_field("InnerFuture", Ty::named(inner_future_ty), 2, inner_offset),
        ],
        positional: Vec::new(),
        list: None,
        size: Some(total_size),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: Some(ClassLayoutInfo {
            kind: ClassLayoutKind::Class,
            bases: vec!["Std::Async::Task".into()],
            vtable_offset: Some(0),
        }),
    }
}

fn runtime_context_layout() -> StructLayout {
    StructLayout {
        name: "Std::Async::RuntimeContext".to_string(),
        repr: TypeRepr::Default,
        packing: None,
        fields: vec![make_field("Inner", Ty::named("isize"), 0, 0)],
        positional: Vec::new(),
        list: None,
        size: Some(pointer_size()),
        align: Some(pointer_align()),
        is_readonly: false,
        is_intrinsic: true,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    }
}

fn future_header_size() -> usize {
    align_to(pointer_size() * 3 + 4, pointer_align())
}

fn future_bool_size() -> usize {
    future_with_result_size(std::mem::size_of::<bool>(), std::mem::align_of::<bool>())
}

fn future_int_size() -> usize {
    future_with_result_size(std::mem::size_of::<i32>(), std::mem::align_of::<i32>())
}

fn future_with_result_size(result_size: usize, result_align: usize) -> usize {
    let header_size = future_header_size();
    let completed_offset = header_size + 1;
    let result_offset = align_to(completed_offset, result_align.max(1));
    align_to(result_offset + result_size, pointer_align())
}

fn task_base_size() -> usize {
    align_to(future_header_size() + 4, pointer_align())
}

#[allow(dead_code)]
fn insert_meta_enums(table: &mut TypeLayoutTable, prefix: &str) {
    let enums = [
        (
            "TypeKind",
            &[
                ("Struct", 0),
                ("Record", 1),
                ("Class", 2),
                ("Enum", 3),
                ("Interface", 4),
                ("Union", 5),
                ("Extension", 6),
                ("Trait", 7),
                ("Delegate", 8),
                ("Impl", 9),
                ("Function", 10),
                ("Const", 11),
                ("Static", 12),
            ][..],
        ),
        (
            "MemberKind",
            &[
                ("Field", 0),
                ("Property", 1),
                ("Method", 2),
                ("Constructor", 3),
                ("Const", 4),
                ("EnumVariant", 5),
                ("UnionField", 6),
                ("UnionView", 7),
                ("AssociatedType", 8),
                ("ExtensionMethod", 9),
                ("Parameter", 10),
                ("Return", 11),
                ("TraitMethod", 12),
            ][..],
        ),
        (
            "VisibilityDescriptor",
            &[
                ("Public", 0),
                ("Internal", 1),
                ("Protected", 2),
                ("Private", 3),
                ("ProtectedInternal", 4),
                ("PrivateProtected", 5),
            ][..],
        ),
        (
            "ParameterModifier",
            &[("In", 0), ("Ref", 1), ("Out", 2), ("Value", 3)][..],
        ),
    ];

    for (name, variants) in enums {
        let full = format!("{prefix}::{name}");
        if table.types.contains_key(&full) {
            continue;
        }
        table.types.insert(
            full.clone(),
            TypeLayout::Enum(decimal_enum_layout(full.as_str(), variants)),
        );
    }
}

#[allow(dead_code)]
fn insert_meta_structs(table: &mut TypeLayoutTable, prefix: &str) {
    insert_layout_descriptor_layout(table, prefix);
    insert_field_layout_descriptor_layout(table, prefix);
    insert_descriptor_list_layout(table, prefix, &format!("{prefix}::FieldLayoutDescriptor"));
    insert_type_layout_descriptor_layout(table, prefix);
    insert_parameter_descriptor_layout(table, prefix);
    insert_member_descriptor_layout(table, prefix);
    insert_type_descriptor_layout(table, prefix);
}

#[allow(dead_code)]
fn insert_meta_descriptor_lists(table: &mut TypeLayoutTable, prefix: &str) {
    for element in [
        "string",
        &format!("{prefix}::ParameterDescriptor"),
        &format!("{prefix}::MemberDescriptor"),
        &format!("{prefix}::FieldLayoutDescriptor"),
    ] {
        insert_descriptor_list_layout(table, prefix, element);
    }
}

#[allow(dead_code)]
fn insert_meta_quotes(table: &mut TypeLayoutTable, prefix: &str) {
    let quote_enum = format!("{prefix}::QuoteNodeKind");
    if !table.types.contains_key(&quote_enum) {
        table.types.insert(
            quote_enum.clone(),
            TypeLayout::Enum(decimal_enum_layout(
                &quote_enum,
                &[
                    ("Literal", 0),
                    ("Identifier", 1),
                    ("Unary", 2),
                    ("Binary", 3),
                    ("Conditional", 4),
                    ("Cast", 5),
                    ("Lambda", 6),
                    ("Tuple", 7),
                    ("Assign", 8),
                    ("Member", 9),
                    ("Call", 10),
                    ("Argument", 11),
                    ("Ref", 12),
                    ("New", 13),
                    ("Index", 14),
                    ("Await", 15),
                    ("TryPropagate", 16),
                    ("Throw", 17),
                    ("SizeOf", 18),
                    ("AlignOf", 19),
                    ("NameOf", 20),
                    ("InterpolatedString", 21),
                    ("Quote", 22),
                    ("Pattern", 23),
                    ("Unknown", 24),
                ],
            )),
        );
    }

    insert_quote_span_layout(table, prefix);
    insert_quote_hygiene_layout(table, prefix);
    insert_quote_node_layout(table, prefix);
    insert_quote_interpolation_layout(table, prefix);
    insert_quote_layout(table, prefix);
}

#[allow(dead_code)]
fn insert_meta_quote_lists(table: &mut TypeLayoutTable, prefix: &str) {
    for element in [
        "string",
        &format!("{prefix}::QuoteNode"),
        &format!("{prefix}::QuoteInterpolation"),
    ] {
        insert_descriptor_list_layout(table, prefix, element);
    }
}

#[allow(dead_code)]
fn insert_descriptor_list_layout(table: &mut TypeLayoutTable, prefix: &str, element: &str) {
    let name = format!("{prefix}::DescriptorList<{element}>");
    if table.types.contains_key(&name) {
        return;
    }

    let head_ty = Ty::named(element);
    let (head_size, head_align) = table
        .size_and_align_for_ty(&head_ty)
        .unwrap_or((pointer_size(), pointer_align()));

    let list_ty = Ty::named(name.as_str());
    let tail_ty = Ty::Nullable(Box::new(list_ty));

    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    let flag_offset = offset;
    offset += 1;
    struct_align = struct_align.max(MIN_ALIGN);

    let head_align = head_align.max(MIN_ALIGN);
    let head_offset = align_to(offset, head_align);
    offset = head_offset.saturating_add(head_size);
    struct_align = struct_align.max(head_align);

    let (tail_size, tail_align) = table
        .size_and_align_for_ty(&tail_ty)
        .unwrap_or((pointer_size(), pointer_align()));
    let tail_align = tail_align.max(MIN_ALIGN);
    let tail_offset = align_to(offset, tail_align);
    offset = tail_offset.saturating_add(tail_size);
    struct_align = struct_align.max(tail_align);

    let size = align_to(offset, struct_align.max(MIN_ALIGN));

    let fields = vec![
        make_field("IsEmpty", Ty::named("bool"), 0, flag_offset),
        make_field("Head", head_ty, 1, head_offset),
        make_field("Tail", tail_ty, 2, tail_offset),
    ];

    let layout_name = name.clone();
    table.types.insert(
        name,
        TypeLayout::Struct(StructLayout {
            name: layout_name,
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_parameter_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::ParameterDescriptor");
    if table.types.contains_key(&name) {
        return;
    }

    let mut fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in [
        ("Name", Ty::named("string")),
        ("TypeName", Ty::named("string")),
        (
            "Modifier",
            Ty::Nullable(Box::new(Ty::named(&format!("{prefix}::ParameterModifier")))),
        ),
        ("Nullable", Ty::Nullable(Box::new(Ty::named("bool")))),
    ]
    .into_iter()
    .enumerate()
    {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_member_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::MemberDescriptor");
    if table.types.contains_key(&name) {
        return;
    }

    let base_fields: Vec<(&str, Ty)> = vec![
        ("Name", Ty::named("string")),
        ("Kind", Ty::named(&format!("{prefix}::MemberKind"))),
        ("TypeName", Ty::Nullable(Box::new(Ty::named("string")))),
        ("ReturnType", Ty::Nullable(Box::new(Ty::named("string")))),
        (
            "Parameters",
            Ty::named(&format!(
                "{prefix}::DescriptorList<{prefix}::ParameterDescriptor>"
            )),
        ),
        (
            "Visibility",
            Ty::Nullable(Box::new(Ty::named(&format!(
                "{prefix}::VisibilityDescriptor"
            )))),
        ),
        (
            "Attributes",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        (
            "Flags",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        ("Doc", Ty::Nullable(Box::new(Ty::named("string")))),
    ];

    let mut fields: Vec<(&str, Ty)> = base_fields.clone();

    if prefix == "Std::Meta" {
        fields.extend_from_slice(&[
            ("IsStatic", Ty::Nullable(Box::new(Ty::named("bool")))),
            ("IsAsync", Ty::Nullable(Box::new(Ty::named("bool")))),
            ("IsConstexpr", Ty::Nullable(Box::new(Ty::named("bool")))),
            ("IsUnsafe", Ty::Nullable(Box::new(Ty::named("bool")))),
            (
                "Throws",
                Ty::named(&format!("{prefix}::DescriptorList<string>")),
            ),
            (
                "Members",
                Ty::named(&format!(
                    "{prefix}::DescriptorList<{prefix}::MemberDescriptor>"
                )),
            ),
        ]);
    } else {
        fields.push((
            "Members",
            Ty::named(&format!(
                "{prefix}::DescriptorList<{prefix}::MemberDescriptor>"
            )),
        ));
    }

    let mut struct_fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in fields.into_iter().enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_layout_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::LayoutDescriptor");
    if table.types.contains_key(&name) {
        return;
    }

    let mut fields: Vec<(&str, Ty)> = vec![
        ("ReprC", Ty::named("bool")),
        ("Pack", Ty::Nullable(Box::new(Ty::named("uint")))),
        ("Align", Ty::Nullable(Box::new(Ty::named("uint")))),
    ];

    let mut struct_fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in fields.drain(..).enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_field_layout_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::FieldLayoutDescriptor");
    if table.types.contains_key(&name) {
        return;
    }

    let mut fields: Vec<(&str, Ty)> = vec![
        ("Name", Ty::named("string")),
        ("Offset", Ty::Nullable(Box::new(Ty::named("ulong")))),
        ("TypeName", Ty::Nullable(Box::new(Ty::named("string")))),
        ("Readonly", Ty::Nullable(Box::new(Ty::named("bool")))),
    ];

    let mut struct_fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in fields.drain(..).enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_type_layout_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::TypeLayoutDescriptor");
    if table.types.contains_key(&name) {
        return;
    }

    let mut fields: Vec<(&str, Ty)> = vec![
        ("Size", Ty::Nullable(Box::new(Ty::named("ulong")))),
        ("Align", Ty::Nullable(Box::new(Ty::named("uint")))),
        (
            "Fields",
            Ty::named(&format!(
                "{prefix}::DescriptorList<{prefix}::FieldLayoutDescriptor>"
            )),
        ),
    ];

    let mut struct_fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in fields.drain(..).enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_type_descriptor_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::TypeDescriptor");
    if table.types.contains_key(&name) {
        return;
    }
    insert_layout_descriptor_layout(table, prefix);
    insert_field_layout_descriptor_layout(table, prefix);
    insert_descriptor_list_layout(table, prefix, &format!("{prefix}::FieldLayoutDescriptor"));
    insert_type_layout_descriptor_layout(table, prefix);

    let fields: Vec<(&str, Ty)> = vec![
        ("Namespace", Ty::Nullable(Box::new(Ty::named("string")))),
        ("Name", Ty::named("string")),
        ("FullName", Ty::Nullable(Box::new(Ty::named("string")))),
        ("Kind", Ty::named(&format!("{prefix}::TypeKind"))),
        (
            "Visibility",
            Ty::named(&format!("{prefix}::VisibilityDescriptor")),
        ),
        ("IsGeneric", Ty::Nullable(Box::new(Ty::named("bool")))),
        (
            "Generics",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        (
            "Bases",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        (
            "Attributes",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        (
            "UnderlyingType",
            Ty::Nullable(Box::new(Ty::named("string"))),
        ),
        (
            "Members",
            Ty::named(&format!(
                "{prefix}::DescriptorList<{prefix}::MemberDescriptor>"
            )),
        ),
        ("Doc", Ty::Nullable(Box::new(Ty::named("string")))),
        (
            "ExtensionTarget",
            Ty::Nullable(Box::new(Ty::named("string"))),
        ),
        ("ImplTarget", Ty::Nullable(Box::new(Ty::named("string")))),
        ("ImplTrait", Ty::Nullable(Box::new(Ty::named("string")))),
        ("ConstType", Ty::Nullable(Box::new(Ty::named("string")))),
        (
            "Layout",
            Ty::Nullable(Box::new(Ty::named(&format!("{prefix}::LayoutDescriptor")))),
        ),
        (
            "TypeLayout",
            Ty::Nullable(Box::new(Ty::named(&format!(
                "{prefix}::TypeLayoutDescriptor"
            )))),
        ),
    ];

    let mut struct_fields = Vec::new();
    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;

    for (index, (field_name, ty)) in fields.into_iter().enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_quote_span_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::QuoteSpan");
    if table.types.contains_key(&name) {
        return;
    }

    let mut offset = 0usize;
    let mut fields = Vec::new();
    let mut struct_align = MIN_ALIGN;
    for (index, field_name) in ["Start", "End"].into_iter().enumerate() {
        let ty = Ty::named("ulong");
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        fields.push(make_field(
            field_name,
            ty.clone(),
            index as u32,
            field_offset,
        ));
    }
    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_quote_hygiene_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::QuoteHygiene");
    if table.types.contains_key(&name) {
        return;
    }

    let mut offset = 0usize;
    let mut fields = Vec::new();
    let mut struct_align = MIN_ALIGN;
    for (index, field_name) in ["Anchor", "Seed"].into_iter().enumerate() {
        let ty = Ty::named("ulong");
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        fields.push(make_field(
            field_name,
            ty.clone(),
            index as u32,
            field_offset,
        ));
    }
    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_quote_node_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::QuoteNode");
    if table.types.contains_key(&name) {
        return;
    }

    let fields: [(&str, Ty); 3] = [
        ("Kind", Ty::named(&format!("{prefix}::QuoteNodeKind"))),
        ("Value", Ty::Nullable(Box::new(Ty::named("string")))),
        (
            "Children",
            Ty::named(&format!("{prefix}::DescriptorList<{prefix}::QuoteNode>")),
        ),
    ];

    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;
    let mut struct_fields = Vec::new();

    for (index, (field_name, ty)) in fields.into_iter().enumerate() {
        if matches!(ty, Ty::Nullable(_)) {
            let _ = table.size_and_align_for_ty(&ty);
            table.ensure_nullable_layout(match ty {
                Ty::Nullable(ref inner) => inner,
                _ => unreachable!(),
            });
        }
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_quote_interpolation_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::QuoteInterpolation");
    if table.types.contains_key(&name) {
        return;
    }

    let fields: [(&str, Ty); 3] = [
        ("Placeholder", Ty::named("string")),
        ("Value", Ty::named(&format!("{prefix}::Quote"))),
        ("Span", Ty::named(&format!("{prefix}::QuoteSpan"))),
    ];

    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;
    let mut struct_fields = Vec::new();

    for (index, (field_name, ty)) in fields.into_iter().enumerate() {
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

#[allow(dead_code)]
fn insert_quote_layout(table: &mut TypeLayoutTable, prefix: &str) {
    let name = format!("{prefix}::Quote");
    if table.types.contains_key(&name) {
        return;
    }

    let fields: [(&str, Ty); 7] = [
        ("Source", Ty::named("string")),
        ("Sanitized", Ty::named("string")),
        ("Span", Ty::named(&format!("{prefix}::QuoteSpan"))),
        ("Hygiene", Ty::named(&format!("{prefix}::QuoteHygiene"))),
        (
            "Captures",
            Ty::named(&format!("{prefix}::DescriptorList<string>")),
        ),
        (
            "Interpolations",
            Ty::named(&format!(
                "{prefix}::DescriptorList<{prefix}::QuoteInterpolation>"
            )),
        ),
        ("Root", Ty::named(&format!("{prefix}::QuoteNode"))),
    ];

    let mut offset = 0usize;
    let mut struct_align = MIN_ALIGN;
    let mut struct_fields = Vec::new();

    for (index, (field_name, ty)) in fields.into_iter().enumerate() {
        let (size, align) = table
            .size_and_align_for_ty(&ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let effective = align.max(MIN_ALIGN);
        let field_offset = align_to(offset, effective);
        offset = field_offset.saturating_add(size);
        struct_align = struct_align.max(effective);
        struct_fields.push(make_field(field_name, ty, index as u32, field_offset));
    }

    let size = align_to(offset, struct_align);

    table.types.insert(
        name.clone(),
        TypeLayout::Struct(StructLayout {
            name,
            repr: TypeRepr::Default,
            packing: None,
            fields: struct_fields,
            positional: Vec::new(),
            list: None,
            size: Some(size),
            align: Some(struct_align),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}
