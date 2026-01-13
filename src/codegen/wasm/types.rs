use crate::mir::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ValueType {
    I32,
    I64,
    F32,
    F64,
}

impl ValueType {
    pub(crate) fn to_byte(self) -> u8 {
        match self {
            ValueType::I32 => 0x7F,
            ValueType::I64 => 0x7E,
            ValueType::F32 => 0x7D,
            ValueType::F64 => 0x7C,
        }
    }
}

pub(crate) fn map_type(ty: &Ty) -> ValueType {
    match ty {
        Ty::Unit | Ty::Unknown => ValueType::I32,
        Ty::Array(_) | Ty::Vec(_) | Ty::Span(_) | Ty::ReadOnlySpan(_) => ValueType::I32,
        Ty::Vector(_) => panic!(
            "[TYPE0704] WASM backend does not yet support SIMD vectors; enable wasm_simd128 or use the LLVM backend until scalar fallback is implemented",
        ),
        Ty::Rc(_) | Ty::Arc(_) => ValueType::I32,
        Ty::String => ValueType::I32,
        Ty::Str => ValueType::I64,
        Ty::Tuple(_) => ValueType::I32,
        Ty::Fn(_) => ValueType::I32,
        Ty::Pointer(_) | Ty::Ref(_) => ValueType::I32,
        Ty::Nullable(_) => ValueType::I32,
        Ty::TraitObject(_) => ValueType::I32,
        Ty::Named(name) => map_named_type(name.as_str()),
    }
}

fn map_named_type(name: &str) -> ValueType {
    let lowered = name.to_ascii_lowercase();
    match lowered.as_str() {
        // 32-bit signed/unsigned
        "int"
        | "uint"
        | "i32"
        | "u32"
        | "int32"
        | "uint32"
        | "intptr"
        | "uintptr"
        | "system::int32"
        | "std::int32"
        | "system::uint32"
        | "std::uint32"
        | "std::numeric::int32"
        | "std::numeric::uint32"
        | "std::numeric::intptr"
        | "std::numeric::uintptr" => ValueType::I32,
        // 16/8-bit widen to i32 per wasm32 scalar lowering
        "short"
        | "ushort"
        | "int16"
        | "uint16"
        | "char"
        | "byte"
        | "sbyte"
        | "int8"
        | "uint8"
        | "system::int16"
        | "std::int16"
        | "system::uint16"
        | "std::uint16"
        | "system::sbyte"
        | "std::sbyte"
        | "system::byte"
        | "std::byte"
        | "std::numeric::int16"
        | "std::numeric::uint16"
        | "std::numeric::int8"
        | "std::numeric::uint8" => ValueType::I32,
        // native sized ints
        "nint" | "nuint" | "isize" | "usize" => ValueType::I32,
        // 64-bit signed/unsigned
        "long"
        | "ulong"
        | "i64"
        | "u64"
        | "int64"
        | "uint64"
        | "system::int64"
        | "std::int64"
        | "system::uint64"
        | "std::uint64"
        | "std::numeric::int64"
        | "std::numeric::uint64" => ValueType::I64,
        // floating-point scalars
        "float16" | "half" | "f16" => ValueType::F32,
        "float"
        | "f32"
        | "float32"
        | "system::single"
        | "std::single"
        | "std::numeric::float32" => ValueType::F32,
        "double"
        | "f64"
        | "float64"
        | "system::double"
        | "std::double"
        | "std::numeric::float64" => ValueType::F64,
        "float128" | "quad" | "f128" | "std::numeric::float128" => ValueType::F64,
        _ => ValueType::I32,
    }
}

#[cfg(test)]
mod tests {
    use super::{ValueType, map_type};
    use crate::mir::{ArrayTy, Ty, VecTy};

    #[test]
    fn value_type_to_byte_matches_spec() {
        assert_eq!(ValueType::I32.to_byte(), 0x7F);
        assert_eq!(ValueType::I64.to_byte(), 0x7E);
        assert_eq!(ValueType::F32.to_byte(), 0x7D);
        assert_eq!(ValueType::F64.to_byte(), 0x7C);
    }

    #[test]
    fn map_type_handles_builtin_scalars_and_fallback() {
        assert_eq!(map_type(&Ty::Unit), ValueType::I32);
        assert_eq!(map_type(&Ty::Unknown), ValueType::I32);
        assert_eq!(map_type(&Ty::named("i64")), ValueType::I64);
        assert_eq!(map_type(&Ty::named("f16")), ValueType::F32);
        assert_eq!(map_type(&Ty::named("f32")), ValueType::F32);
        assert_eq!(map_type(&Ty::named("f64")), ValueType::F64);
        assert_eq!(map_type(&Ty::named("f128")), ValueType::F64);
        assert_eq!(map_type(&Ty::named("nint")), ValueType::I32);
        assert_eq!(map_type(&Ty::named("nuint")), ValueType::I32);
        assert_eq!(map_type(&Ty::named("isize")), ValueType::I32);
        assert_eq!(map_type(&Ty::named("usize")), ValueType::I32);
        // Unknown named types default to i32 to match wasm32 pointer representation.
        assert_eq!(map_type(&Ty::named("MyStruct")), ValueType::I32);
        let array = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
        assert_eq!(map_type(&array), ValueType::I32);
        let vec = Ty::Vec(VecTy::new(Box::new(Ty::named("int"))));
        assert_eq!(map_type(&vec), ValueType::I32);
        let ptr = Ty::Pointer(Box::new(crate::mir::PointerTy::new(Ty::named("int"), true)));
        assert_eq!(map_type(&ptr), ValueType::I32);
        let reference = Ty::Ref(Box::new(crate::mir::RefTy {
            element: Ty::named("string"),
            readonly: true,
        }));
        assert_eq!(map_type(&reference), ValueType::I32);
    }

    #[test]
    #[should_panic(expected = "SIMD vectors")]
    fn map_type_panics_on_vector_until_wasm_supports_simd() {
        let vector = Ty::Vector(crate::mir::VectorTy {
            element: Box::new(Ty::named("float")),
            lanes: 4,
        });
        let _ = map_type(&vector);
    }
}
