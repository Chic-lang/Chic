use crate::codegen::llvm::types::map_type_owned;
use crate::error::Error;
use crate::mir::{ConstOperand, ConstValue, Operand, Ty};
use crate::target::TargetArch;

use super::super::builder::FunctionEmitter;

#[derive(Clone, Debug)]
pub(crate) enum InterpolationOperandKind {
    Str,
    String,
    Bool { llvm_ty: String },
    Char { llvm_ty: String },
    SignedInt { bits: u32, llvm_ty: String },
    UnsignedInt { bits: u32, llvm_ty: String },
    Float { bits: u32, llvm_ty: String },
}

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn classify_interpolation_operand(
        &self,
        operand: &Operand,
    ) -> Result<InterpolationOperandKind, Error> {
        match operand {
            Operand::Const(constant) => self.classify_const_operand(constant, operand),
            Operand::Copy(place) | Operand::Move(place) => {
                let ty = self
                    .function
                    .body
                    .locals
                    .get(place.local.0)
                    .map(|decl| decl.ty.clone())
                    .unwrap_or(Ty::Unknown);
                self.classify_ty(&ty, operand)
            }
            Operand::Pending(pending) => Err(Error::Codegen(format!(
                "pending operand `{}` cannot be used in string interpolation",
                pending.repr
            ))),
            Operand::Mmio(_) => Err(Error::Codegen(
                "MMIO operands are not supported in string interpolation".into(),
            )),
            Operand::Borrow(_) => Err(Error::Codegen(
                "borrowed values are not yet supported in string interpolation".into(),
            )),
        }
    }

    pub(crate) fn classify_const_operand(
        &self,
        constant: &ConstOperand,
        operand: &Operand,
    ) -> Result<InterpolationOperandKind, Error> {
        match &constant.value {
            ConstValue::Str { .. } | ConstValue::RawStr(_) => Ok(InterpolationOperandKind::Str),
            ConstValue::Bool(_) => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("boolean constant missing type".into()))?;
                Ok(InterpolationOperandKind::Bool { llvm_ty })
            }
            ConstValue::Char(_) => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("char constant missing type".into()))?;
                Ok(InterpolationOperandKind::Char { llvm_ty })
            }
            ConstValue::Int(_) | ConstValue::Int32(_) => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("integer constant missing type".into()))?;
                let bits = self.parse_integer_bits(&llvm_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine width for integer constant of type `{llvm_ty}`"
                    ))
                })?;
                Ok(InterpolationOperandKind::SignedInt { bits, llvm_ty })
            }
            ConstValue::UInt(_) => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("integer constant missing type".into()))?;
                let bits = self.parse_integer_bits(&llvm_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine width for integer constant of type `{llvm_ty}`"
                    ))
                })?;
                Ok(InterpolationOperandKind::UnsignedInt { bits, llvm_ty })
            }
            ConstValue::Enum { .. } => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("enum constant missing type".into()))?;
                let bits = self.parse_integer_bits(&llvm_ty).ok_or_else(|| {
                    Error::Codegen(format!(
                        "cannot determine width for enum constant of type `{llvm_ty}`"
                    ))
                })?;
                Ok(InterpolationOperandKind::SignedInt { bits, llvm_ty })
            }
            ConstValue::Null => Err(Error::Codegen(
                "`null` constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Float(_) => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("floating constant missing type".into()))?;
                let bits = match llvm_ty.as_str() {
                    "half" => 16,
                    "float" => 32,
                    "double" => 64,
                    "fp128" => 128,
                    other => {
                        return Err(Error::Codegen(format!(
                            "unsupported float constant type `{other}`"
                        )));
                    }
                };
                Ok(InterpolationOperandKind::Float { bits, llvm_ty })
            }
            ConstValue::Decimal(_) => Err(Error::Codegen(
                "decimal constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Struct { .. } => Err(Error::Codegen(
                "struct constants are not yet supported in string interpolation".into(),
            )),
            ConstValue::Symbol(_) | ConstValue::Unit | ConstValue::Unknown => Err(Error::Codegen(
                "constant cannot be interpolated into a string".into(),
            )),
        }
    }

    pub(crate) fn classify_ty(
        &self,
        ty: &Ty,
        operand: &Operand,
    ) -> Result<InterpolationOperandKind, Error> {
        match ty {
            Ty::String => Ok(InterpolationOperandKind::String),
            Ty::Str => Ok(InterpolationOperandKind::Str),
            Ty::Nullable(inner) => self.classify_ty(inner, operand),
            Ty::Named(name) => self.classify_named_type(name, operand),
            Ty::Unknown => {
                let llvm_ty = self
                    .operand_type(operand)?
                    .ok_or_else(|| Error::Codegen("unable to infer interpolation type".into()))?;
                self.classify_llvm_type(&llvm_ty, None)
            }
            _ => Err(Error::Codegen(format!(
                "interpolated expression uses unsupported type `{}`",
                ty.canonical_name()
            ))),
        }
    }

    pub(crate) fn classify_named_type(
        &self,
        name: &str,
        _operand: &Operand,
    ) -> Result<InterpolationOperandKind, Error> {
        let short = Self::short_type_name(name);
        if short.eq_ignore_ascii_case("string") {
            return Ok(InterpolationOperandKind::String);
        }
        if short.eq_ignore_ascii_case("str") {
            return Ok(InterpolationOperandKind::Str);
        }

        let ty = Ty::named(name.to_string());
        let llvm_ty = map_type_owned(&ty, Some(self.type_layouts))?.ok_or_else(|| {
            Error::Codegen(format!("type `{name}` is not supported for interpolation"))
        })?;

        let lower = short.to_ascii_lowercase();
        match lower.as_str() {
            "bool" | "boolean" => Ok(InterpolationOperandKind::Bool { llvm_ty }),
            "char" => Ok(InterpolationOperandKind::Char { llvm_ty }),
            "sbyte" | "int8" => Ok(InterpolationOperandKind::SignedInt { bits: 8, llvm_ty }),
            "short" | "int16" => Ok(InterpolationOperandKind::SignedInt { bits: 16, llvm_ty }),
            "int" | "int32" => Ok(InterpolationOperandKind::SignedInt { bits: 32, llvm_ty }),
            "long" | "int64" => Ok(InterpolationOperandKind::SignedInt { bits: 64, llvm_ty }),
            "int128" => Ok(InterpolationOperandKind::SignedInt { bits: 128, llvm_ty }),
            "nint" | "isize" => Ok(InterpolationOperandKind::SignedInt {
                bits: self.pointer_width_bits(),
                llvm_ty,
            }),
            "byte" | "uint8" => Ok(InterpolationOperandKind::UnsignedInt { bits: 8, llvm_ty }),
            "ushort" | "uint16" => Ok(InterpolationOperandKind::UnsignedInt { bits: 16, llvm_ty }),
            "uint" | "uint32" => Ok(InterpolationOperandKind::UnsignedInt { bits: 32, llvm_ty }),
            "ulong" | "uint64" => Ok(InterpolationOperandKind::UnsignedInt { bits: 64, llvm_ty }),
            "uint128" => Ok(InterpolationOperandKind::UnsignedInt { bits: 128, llvm_ty }),
            "nuint" | "usize" => Ok(InterpolationOperandKind::UnsignedInt {
                bits: self.pointer_width_bits(),
                llvm_ty,
            }),
            "float16" | "half" | "f16" => Ok(InterpolationOperandKind::Float { bits: 16, llvm_ty }),
            "float" => Ok(InterpolationOperandKind::Float { bits: 32, llvm_ty }),
            "double" | "float64" => Ok(InterpolationOperandKind::Float { bits: 64, llvm_ty }),
            "float128" | "quad" | "f128" => {
                Ok(InterpolationOperandKind::Float { bits: 128, llvm_ty })
            }
            _ => self.classify_llvm_type(&llvm_ty, Some(Self::is_signed_hint(lower.as_str()))),
        }
    }

    pub(crate) fn classify_llvm_type(
        &self,
        llvm_ty: &str,
        sign_hint: Option<bool>,
    ) -> Result<InterpolationOperandKind, Error> {
        if llvm_ty == crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE {
            return Ok(InterpolationOperandKind::Str);
        }
        if llvm_ty == "half" {
            return Ok(InterpolationOperandKind::Float {
                bits: 16,
                llvm_ty: llvm_ty.to_string(),
            });
        }
        if llvm_ty == "float" {
            return Ok(InterpolationOperandKind::Float {
                bits: 32,
                llvm_ty: llvm_ty.to_string(),
            });
        }
        if llvm_ty == "double" {
            return Ok(InterpolationOperandKind::Float {
                bits: 64,
                llvm_ty: llvm_ty.to_string(),
            });
        }
        if llvm_ty == "fp128" {
            return Ok(InterpolationOperandKind::Float {
                bits: 128,
                llvm_ty: llvm_ty.to_string(),
            });
        }
        if let Some(bits) = self.parse_integer_bits(llvm_ty) {
            let signed = sign_hint.unwrap_or(true);
            if signed {
                Ok(InterpolationOperandKind::SignedInt {
                    bits,
                    llvm_ty: llvm_ty.to_string(),
                })
            } else {
                Ok(InterpolationOperandKind::UnsignedInt {
                    bits,
                    llvm_ty: llvm_ty.to_string(),
                })
            }
        } else {
            Err(Error::Codegen(format!(
                "LLVM type `{llvm_ty}` is not supported in string interpolation"
            )))
        }
    }

    pub(crate) fn parse_integer_bits(&self, llvm_ty: &str) -> Option<u32> {
        llvm_ty
            .strip_prefix('i')
            .and_then(|rest| rest.parse::<u32>().ok())
    }

    pub(crate) fn pointer_width_bits(&self) -> u32 {
        match self.arch {
            TargetArch::X86_64 | TargetArch::Aarch64 => 64,
        }
    }

    pub(crate) fn short_type_name(name: &str) -> &str {
        name.rsplit("::").next().unwrap_or(name)
    }

    pub(crate) fn is_signed_hint(name: &str) -> bool {
        match name {
            "byte" | "uint8" | "ushort" | "uint16" | "uint" | "uint32" | "ulong" | "uint64"
            | "uint128" | "usize" | "nuint" => false,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::isa::CpuIsaTier;
    use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
    use crate::codegen::llvm::types::map_type_owned;
    use crate::mir::{
        ClassVTable, FloatValue, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
        MmioAccess, MmioEndianness, PendingOperand, Place, RegionVar, StaticVar, StrId,
        TraitVTable, TypeLayoutTable, ValueCategory,
    };
    use crate::mmio::AddressSpaceId;
    use std::collections::{BTreeSet, HashMap, HashSet};

    fn with_emitter<F>(locals: Vec<Ty>, mut f: F)
    where
        F: FnMut(&mut FunctionEmitter<'_>, &TypeLayoutTable),
    {
        let mut body = MirBody::new(0, None);
        for ty in locals {
            body.locals
                .push(LocalDecl::new(None, ty, false, None, LocalKind::Local));
        }
        let function = MirFunction {
            name: "test".to_string(),
            kind: FunctionKind::Function,
            signature: FnSig::empty(),
            body,
            is_async: false,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        let signatures = HashMap::new();
        let mut externals: BTreeSet<&'static str> = BTreeSet::new();
        let vtable_symbols = HashSet::new();
        let trait_vtables: Vec<TraitVTable> = Vec::new();
        let class_vtables: Vec<ClassVTable> = Vec::new();
        let statics: Vec<StaticVar> = Vec::new();
        let str_literals: HashMap<StrId, crate::codegen::llvm::emitter::literals::StrLiteralInfo> =
            HashMap::new();
        let type_layouts = TypeLayoutTable::default();
        let mut metadata = MetadataRegistry::new();
        let target = crate::target::Target::parse("aarch64-unknown-linux-gnu").expect("target");
        let mut emitter = FunctionEmitter::new(
            &function,
            &signatures,
            &mut externals,
            &vtable_symbols,
            &trait_vtables,
            &class_vtables,
            CpuIsaTier::Baseline,
            &[CpuIsaTier::Baseline],
            TargetArch::Aarch64,
            &target,
            &statics,
            &str_literals,
            &type_layouts,
            &mut metadata,
            None,
        );
        if !function.body.locals.is_empty() {
            let local_tys: Vec<Option<String>> = function
                .body
                .locals
                .iter()
                .map(|local| {
                    map_type_owned(&local.ty, Some(&type_layouts))
                        .ok()
                        .flatten()
                })
                .collect();
            emitter.set_local_types_for_tests(local_tys);
        }
        f(&mut emitter, &type_layouts);
    }

    #[test]
    fn classifies_boolean_constant() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));

            let kind = emitter.classify_interpolation_operand(&operand).unwrap();

            match kind {
                InterpolationOperandKind::Bool { llvm_ty } => assert_eq!(llvm_ty, "i8"),
                other => panic!("expected bool classification, got {other:?}"),
            }
        });
    }

    #[test]
    fn errors_on_null_constant() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Null));

            let err = emitter
                .classify_interpolation_operand(&operand)
                .expect_err("null should not be allowed");
            assert!(
                err.to_string()
                    .contains("not yet supported in string interpolation"),
                "unexpected error: {err}"
            );
        });
    }

    #[test]
    fn classifies_copy_operand_via_local_type() {
        with_emitter(vec![Ty::named("long")], |emitter, _| {
            let place = Place {
                local: crate::mir::LocalId(0),
                projection: Vec::new(),
            };
            let operand = Operand::Copy(place);

            let kind = emitter.classify_interpolation_operand(&operand).unwrap();
            match kind {
                InterpolationOperandKind::SignedInt { bits, llvm_ty } => {
                    assert_eq!(bits, 64);
                    assert_eq!(llvm_ty, "i64");
                }
                other => panic!("expected signed int classification, got {other:?}"),
            }
        });
    }

    #[test]
    fn rejects_pending_operands() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: "pending".into(),
                span: None,
                info: None,
            });

            let err = emitter
                .classify_interpolation_operand(&operand)
                .expect_err("pending operands should be rejected");
            assert!(err.to_string().contains("pending operand"));
        });
    }

    #[test]
    fn named_types_cover_unsigned_and_strings() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int(0)));

            let uint_kind = emitter
                .classify_named_type("uint", &operand)
                .expect("uint should be supported");
            match uint_kind {
                InterpolationOperandKind::UnsignedInt { bits, .. } => assert_eq!(bits, 32),
                other => panic!("expected unsigned classification, got {other:?}"),
            }

            let str_kind = emitter.classify_named_type("string", &operand).unwrap();
            matches!(str_kind, InterpolationOperandKind::String);
        });
    }

    #[test]
    fn llvm_type_errors_on_unknown() {
        with_emitter(Vec::new(), |emitter, _| {
            let err = emitter
                .classify_llvm_type("ptr", None)
                .expect_err("unknown types should be rejected");
            assert!(err.to_string().contains("LLVM type `ptr` is not supported"));
        });
    }

    #[test]
    fn enum_constant_requires_type_information() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Enum {
                type_name: "Sample".into(),
                variant: "A".into(),
                discriminant: 0,
            }));

            let err = emitter
                .classify_interpolation_operand(&operand)
                .expect_err("enum constants without type should error");
            assert!(
                err.to_string().contains("enum constant missing type"),
                "unexpected error {err}"
            );
        });
    }

    #[test]
    fn parse_integer_bits_handles_non_integer() {
        with_emitter(Vec::new(), |emitter, _| {
            assert_eq!(emitter.parse_integer_bits("i32"), Some(32));
            assert_eq!(emitter.parse_integer_bits("double"), None);
        });
    }

    #[test]
    fn sign_hint_distinguishes_unsigned() {
        assert!(!FunctionEmitter::is_signed_hint("uint64"));
        assert!(FunctionEmitter::is_signed_hint("int32"));
    }

    #[test]
    fn unsigned_and_float_constants_classify() {
        with_emitter(Vec::new(), |emitter, _| {
            let unsigned = Operand::Const(ConstOperand::new(ConstValue::UInt(10)));
            let kind = emitter
                .classify_interpolation_operand(&unsigned)
                .expect("uint should classify");
            matches!(kind, InterpolationOperandKind::UnsignedInt { bits: 32, .. });

            let float = Operand::Const(ConstOperand::new(ConstValue::Float(FloatValue::from_f64(
                1.5,
            ))));
            let float_kind = emitter
                .classify_interpolation_operand(&float)
                .expect("float should classify");
            match float_kind {
                InterpolationOperandKind::Float { bits, llvm_ty } => {
                    assert_eq!(bits, 64);
                    assert_eq!(llvm_ty, "double");
                }
                other => panic!("expected float classification, got {other:?}"),
            }
        });
    }

    #[test]
    fn rejects_symbol_and_decimal_constants() {
        with_emitter(Vec::new(), |emitter, _| {
            let symbol = Operand::Const(ConstOperand::new(ConstValue::Symbol("sym".into())));
            let err = emitter
                .classify_interpolation_operand(&symbol)
                .expect_err("symbols should be rejected");
            assert!(err.to_string().contains("cannot be interpolated"));

            let decimal = Operand::Const(ConstOperand::new(ConstValue::Decimal(
                crate::decimal::Decimal128::zero(),
            )));
            let err = emitter
                .classify_interpolation_operand(&decimal)
                .expect_err("decimals should be rejected");
            assert!(err.to_string().contains("not yet supported"));
        });
    }

    #[test]
    fn classify_ty_handles_nullable_and_unknown() {
        with_emitter(Vec::new(), |emitter, _| {
            let nullable = Ty::Nullable(Box::new(Ty::String));
            let kind = emitter
                .classify_ty(
                    &nullable,
                    &Operand::Const(ConstOperand::new(ConstValue::Str {
                        id: StrId::new(0),
                        value: "x".into(),
                    })),
                )
                .expect("nullable string should classify");
            matches!(
                kind,
                InterpolationOperandKind::Str | InterpolationOperandKind::String
            );

            let unknown = Ty::Unknown;
            let fallback = emitter
                .classify_ty(
                    &unknown,
                    &Operand::Const(ConstOperand::new(ConstValue::UInt(3))),
                )
                .expect("unknown should classify via operand type");
            matches!(
                fallback,
                InterpolationOperandKind::SignedInt { bits: 32, .. }
            );
        });
    }

    #[test]
    fn classify_llvm_type_respects_sign_hint() {
        with_emitter(Vec::new(), |emitter, _| {
            let unsigned = emitter
                .classify_llvm_type("i16", Some(false))
                .expect("unsigned hint should be honoured");
            match unsigned {
                InterpolationOperandKind::UnsignedInt { bits, .. } => assert_eq!(bits, 16),
                other => panic!("expected unsigned int, got {other:?}"),
            }
        });
    }

    #[test]
    fn borrow_and_mmio_operands_rejected() {
        with_emitter(Vec::new(), |emitter, _| {
            let mmio = Operand::Mmio(crate::mir::MmioOperand {
                base_address: 0,
                offset: 0,
                width_bits: 32,
                access: MmioAccess::ReadWrite,
                endianness: MmioEndianness::Little,
                address_space: AddressSpaceId::from_optional(None),
                requires_unsafe: false,
                ty: Ty::named("uint32"),
                name: None,
            });
            let err = emitter
                .classify_interpolation_operand(&mmio)
                .expect_err("mmio operands should be rejected");
            assert!(err.to_string().contains("MMIO operands"));

            let borrow = Operand::Borrow(crate::mir::BorrowOperand {
                place: Place {
                    local: crate::mir::LocalId(0),
                    projection: Vec::new(),
                },
                kind: crate::mir::BorrowKind::Shared,
                region: RegionVar(0),
                span: None,
            });
            let err = emitter
                .classify_interpolation_operand(&borrow)
                .expect_err("borrowed operands should be rejected");
            assert!(
                err.to_string()
                    .contains("borrowed values are not yet supported")
            );
        });
    }

    #[test]
    fn classify_named_type_errors_when_unmapped() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int(0)));
            let err = emitter
                .classify_named_type("Custom::Unknown", &operand)
                .expect_err("unmapped types should be rejected");
            assert!(
                err.to_string().contains("supported"),
                "unexpected error {err}"
            );
        });
    }

    #[test]
    fn classifies_str_and_raw_str_constants() {
        with_emitter(Vec::new(), |emitter, _| {
            let str_operand = Operand::Const(ConstOperand::new(ConstValue::Str {
                id: StrId::new(1),
                value: "hello".into(),
            }));
            let raw_operand = Operand::Const(ConstOperand::new(ConstValue::RawStr("raw".into())));

            matches!(
                emitter
                    .classify_interpolation_operand(&str_operand)
                    .unwrap(),
                InterpolationOperandKind::Str
            );
            matches!(
                emitter
                    .classify_interpolation_operand(&raw_operand)
                    .unwrap(),
                InterpolationOperandKind::Str
            );
        });
    }

    #[test]
    fn classifies_char_constant_with_type() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Char('a' as u16)));

            let kind = emitter
                .classify_interpolation_operand(&operand)
                .expect("char should classify");
            match kind {
                InterpolationOperandKind::Char { llvm_ty } => assert_eq!(llvm_ty, "i16"),
                other => panic!("expected char classification, got {other:?}"),
            }
        });
    }

    #[test]
    fn classifies_integer_constants_with_inferred_width() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Int32(12)));

            let kind = emitter
                .classify_interpolation_operand(&operand)
                .expect("int constants should classify");
            match kind {
                InterpolationOperandKind::SignedInt { bits, llvm_ty } => {
                    assert_eq!(bits, 32);
                    assert_eq!(llvm_ty, "i32");
                }
                other => panic!("expected signed int classification, got {other:?}"),
            }
        });
    }

    #[test]
    fn rejects_struct_constants() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Struct {
                type_name: "Point".into(),
                fields: Vec::new(),
            }));

            let err = emitter
                .classify_interpolation_operand(&operand)
                .expect_err("struct constants are not supported");
            assert!(
                err.to_string()
                    .contains("struct constants are not yet supported")
            );
        });
    }

    #[test]
    fn classify_named_types_cover_boolean_pointer_and_float_cases() {
        with_emitter(Vec::new(), |emitter, _| {
            let operand = Operand::Const(ConstOperand::new(ConstValue::Bool(true)));

            let bool_kind = emitter.classify_named_type("bool", &operand).unwrap();
            matches!(bool_kind, InterpolationOperandKind::Bool { llvm_ty } if llvm_ty == "i8");

            let char_kind = emitter.classify_named_type("char", &operand).unwrap();
            matches!(char_kind, InterpolationOperandKind::Char { llvm_ty } if llvm_ty == "i16");

            let isize_kind = emitter.classify_named_type("isize", &operand).unwrap();
            matches!(isize_kind, InterpolationOperandKind::SignedInt { bits, .. } if bits == 64);

            let usize_kind = emitter.classify_named_type("usize", &operand).unwrap();
            matches!(usize_kind, InterpolationOperandKind::UnsignedInt { bits, .. } if bits == 64);

            let float_kind = emitter.classify_named_type("float", &operand).unwrap();
            matches!(float_kind, InterpolationOperandKind::Float { bits: 32, .. });

            let double_kind = emitter.classify_named_type("double", &operand).unwrap();
            matches!(
                double_kind,
                InterpolationOperandKind::Float { bits: 64, .. }
            );

            let half_kind = emitter.classify_named_type("half", &operand).unwrap();
            matches!(half_kind, InterpolationOperandKind::Float { bits: 16, .. });

            let quad_kind = emitter.classify_named_type("float128", &operand).unwrap();
            matches!(quad_kind, InterpolationOperandKind::Float { bits: 128, .. });
        });
    }

    #[test]
    fn classify_ty_rejects_unsupported_composites() {
        use crate::mir::TupleTy;

        with_emitter(Vec::new(), |emitter, _| {
            let tuple_ty = Ty::Tuple(TupleTy {
                elements: vec![Ty::String],
                element_names: vec![None],
            });
            let operand = Operand::Const(ConstOperand::new(ConstValue::Unit));

            let err = emitter
                .classify_ty(&tuple_ty, &operand)
                .expect_err("tuple types should be rejected");
            assert!(
                err.to_string()
                    .contains("interpolated expression uses unsupported type")
            );
        });
    }

    #[test]
    fn classify_llvm_type_handles_string_repr() {
        with_emitter(Vec::new(), |emitter, _| {
            let llvm_str = crate::codegen::llvm::emitter::literals::LLVM_STR_TYPE;

            let kind = emitter
                .classify_llvm_type(llvm_str, None)
                .expect("llvm str type should be supported");
            matches!(kind, InterpolationOperandKind::Str);
        });
    }

    #[test]
    fn short_type_name_trims_prefixes() {
        let name = FunctionEmitter::short_type_name("System::Collections::Generic::List");
        assert_eq!(name, "List");
    }

    #[test]
    fn float_literal_metadata_sets_width() {
        with_emitter(Vec::new(), |emitter, _| {
            use crate::syntax::numeric::{
                IntegerWidth, NumericLiteralMetadata, NumericLiteralType,
            };

            let literal = NumericLiteralMetadata {
                literal_type: NumericLiteralType::Float32,
                suffix_text: Some("f32".into()),
                explicit_suffix: true,
            };
            let operand = Operand::Const(ConstOperand::with_literal(
                ConstValue::Float(FloatValue::from_f64(1.0)),
                Some(literal),
            ));

            let kind = emitter
                .classify_interpolation_operand(&operand)
                .expect("float32 should classify");
            matches!(kind, InterpolationOperandKind::Float { bits: 32, llvm_ty } if llvm_ty == "float");

            let half_literal = NumericLiteralMetadata {
                literal_type: NumericLiteralType::Float16,
                suffix_text: Some("f16".into()),
                explicit_suffix: true,
            };
            let half_operand = Operand::Const(ConstOperand::with_literal(
                ConstValue::Float(FloatValue::from_f16(1.0)),
                Some(half_literal),
            ));
            let half_kind = emitter
                .classify_interpolation_operand(&half_operand)
                .expect("float16 should classify");
            matches!(half_kind, InterpolationOperandKind::Float { bits: 16, llvm_ty } if llvm_ty == "half");

            let quad_literal = NumericLiteralMetadata {
                literal_type: NumericLiteralType::Float128,
                suffix_text: Some("f128".into()),
                explicit_suffix: true,
            };
            let quad_operand = Operand::Const(ConstOperand::with_literal(
                ConstValue::Float(FloatValue::from_f64_as(1.0, crate::mir::FloatWidth::F128)),
                Some(quad_literal),
            ));
            let quad_kind = emitter
                .classify_interpolation_operand(&quad_operand)
                .expect("float128 should classify");
            matches!(quad_kind, InterpolationOperandKind::Float { bits: 128, llvm_ty } if llvm_ty == "fp128");

            let signed_meta = NumericLiteralMetadata {
                literal_type: NumericLiteralType::Signed(IntegerWidth::W128),
                suffix_text: Some("i128".into()),
                explicit_suffix: true,
            };
            let int_operand = Operand::Const(ConstOperand::with_literal(
                ConstValue::Int(5),
                Some(signed_meta),
            ));
            let int_kind = emitter
                .classify_interpolation_operand(&int_operand)
                .expect("i128 literal should classify");
            matches!(
                int_kind,
                InterpolationOperandKind::SignedInt { bits: 128, .. }
            );
        });
    }
}
