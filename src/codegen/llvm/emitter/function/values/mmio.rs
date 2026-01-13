use std::fmt::Write;

use crate::error::Error;
use crate::mir::MmioOperand;
use crate::mmio::encode_flags;

use super::super::builder::FunctionEmitter;
use super::value_ref::ValueRef;

impl<'a> FunctionEmitter<'a> {
    pub(crate) fn emit_mmio_operand(
        &mut self,
        spec: &MmioOperand,
        expected: Option<&str>,
    ) -> Result<ValueRef, Error> {
        let address = self.mmio_address(spec)?;
        self.externals.insert("chic_rt.mmio_read");
        let call_tmp = self.new_temp();
        let flags = self.mmio_flags(spec);
        writeln!(
            &mut self.builder,
            "  {call_tmp} = call i64 @chic_rt.mmio_read(i64 {address}, i32 {}, i32 {})",
            spec.width_bits, flags
        )
        .ok();

        if let Some(exp) = expected {
            match exp {
                "i64" => Ok(ValueRef::new(call_tmp, "i64")),
                "i32" => {
                    let trunc = self.new_temp();
                    writeln!(&mut self.builder, "  {trunc} = trunc i64 {call_tmp} to i32").ok();
                    Ok(ValueRef::new(trunc, "i32"))
                }
                other => {
                    let value = ValueRef::new(call_tmp, "i64");
                    self.bitcast_value(&value, "i64", other)
                }
            }
        } else if spec.width_bits <= 32 {
            let trunc = self.new_temp();
            writeln!(&mut self.builder, "  {trunc} = trunc i64 {call_tmp} to i32").ok();
            Ok(ValueRef::new(trunc, "i32"))
        } else {
            Ok(ValueRef::new(call_tmp, "i64"))
        }
    }

    pub(crate) fn emit_mmio_store(
        &mut self,
        spec: &MmioOperand,
        value: &crate::mir::Operand,
    ) -> Result<(), Error> {
        let address = self.mmio_address(spec)?;
        self.externals.insert("chic_rt.mmio_write");
        let expected = if spec.width_bits <= 32 {
            Some("i32")
        } else {
            Some("i64")
        };
        let value_ref = self.emit_operand(value, expected)?;
        let value_repr = if spec.width_bits <= 32 {
            let ext = self.new_temp();
            writeln!(
                &mut self.builder,
                "  {ext} = zext i32 {} to i64",
                value_ref.repr()
            )
            .ok();
            ext
        } else {
            value_ref.repr().to_string()
        };

        let flags = self.mmio_flags(spec);
        writeln!(
            &mut self.builder,
            "  call void @chic_rt.mmio_write(i64 {address}, i64 {value_repr}, i32 {}, i32 {})",
            spec.width_bits, flags
        )
        .ok();
        Ok(())
    }

    pub(crate) fn mmio_address(&self, spec: &MmioOperand) -> Result<i64, Error> {
        let absolute = spec
            .base_address
            .checked_add(u64::from(spec.offset))
            .ok_or_else(|| {
                Error::Codegen(format!(
                    "MMIO address for register {} overflows 64-bit range",
                    spec.name.as_deref().unwrap_or("<unknown>")
                ))
            })?;
        i64::try_from(absolute).map_err(|_| {
            Error::Codegen(format!(
                "MMIO address for register {} exceeds i64 range",
                spec.name.as_deref().unwrap_or("<unknown>")
            ))
        })
    }

    pub(crate) fn mmio_flags(&self, spec: &MmioOperand) -> i32 {
        encode_flags(spec.endianness, spec.address_space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::isa::CpuIsaTier;
    use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
    use crate::codegen::llvm::signatures::LlvmFunctionSignature;
    use crate::mir::{ConstOperand, ConstValue, MmioOperand, StrId};
    use crate::mir::{
        FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction, MmioAccess,
        MmioEndianness, Operand, StaticVar, Ty, TypeLayoutTable,
    };
    use crate::mmio::AddressSpaceId;
    use crate::target::TargetArch;
    use std::collections::{BTreeSet, HashMap, HashSet};

    fn with_emitter<F, R>(f: F) -> (R, BTreeSet<&'static str>)
    where
        F: FnOnce(&mut FunctionEmitter<'_>) -> R,
    {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            None,
            Ty::String,
            false,
            None,
            LocalKind::Local,
        ));
        let function = MirFunction {
            name: "mmio".into(),
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
        let signatures: HashMap<String, LlvmFunctionSignature> = HashMap::new();
        let mut externals: BTreeSet<&'static str> = BTreeSet::new();
        let vtable_symbols: HashSet<String> = HashSet::new();
        let trait_vtables = Vec::new();
        let class_vtables = Vec::new();
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
        emitter.local_ptrs = vec![Some("%loc0".into())];
        emitter.set_local_types_for_tests(vec![Some(
            crate::codegen::llvm::types::map_type_owned(&Ty::String, Some(&type_layouts))
                .ok()
                .flatten()
                .unwrap(),
        )]);

        let result = f(&mut emitter);
        (result, externals)
    }

    fn sample_spec(width_bits: u16) -> MmioOperand {
        MmioOperand {
            base_address: 0x1000,
            offset: 4,
            width_bits,
            access: MmioAccess::ReadWrite,
            endianness: MmioEndianness::Little,
            address_space: AddressSpaceId::from_optional(None),
            requires_unsafe: false,
            ty: Ty::named("uint32"),
            name: Some("REG".into()),
        }
    }

    #[test]
    fn reads_mmio_and_truncates_for_narrow_width() {
        let ((value, ir), externals) = with_emitter(|emitter| {
            let spec = sample_spec(16);
            let value = emitter
                .emit_mmio_operand(&spec, None)
                .expect("mmio read should work");
            (value, emitter.ir().to_string())
        });

        assert!(ir.contains("mmio_read"));
        assert!(ir.contains("trunc i64"));
        assert_eq!(value.repr(), "%t1");
        assert!(externals.contains("chic_rt.mmio_read"));
    }

    #[test]
    fn reads_mmio_with_expected_type_and_bitcasts() {
        let ((value, ir), externals) = with_emitter(|emitter| {
            let mut spec = sample_spec(64);
            spec.endianness = MmioEndianness::Big;
            let value = emitter
                .emit_mmio_operand(&spec, Some("ptr"))
                .expect("mmio read should work");
            (value, emitter.ir().to_string())
        });

        assert!(ir.contains("mmio_read"));
        assert!(ir.contains("bitcast i64"));
        assert!(value.repr().starts_with("%t"));
        assert!(externals.contains("chic_rt.mmio_read"));
    }

    #[test]
    fn stores_mmio_with_zero_extension_for_narrow_width() {
        let (ir, externals) = with_emitter(|emitter| {
            let spec = sample_spec(32);
            let value = Operand::Const(ConstOperand::new(ConstValue::UInt(5)));
            emitter
                .emit_mmio_store(&spec, &value)
                .expect("mmio store should work");
            emitter.ir().to_string()
        });

        assert!(ir.contains("mmio_write"));
        assert!(ir.contains("zext i32"));
        assert!(externals.contains("chic_rt.mmio_write"));
    }

    #[test]
    fn stores_mmio_without_extension_for_wide_width() {
        let (ir, externals) = with_emitter(|emitter| {
            let spec = sample_spec(64);
            let value = Operand::Const(ConstOperand::new(ConstValue::Int(9)));
            emitter
                .emit_mmio_store(&spec, &value)
                .expect("mmio store should work");
            emitter.ir().to_string()
        });

        assert!(ir.contains("mmio_write"));
        assert!(!ir.contains("zext i32"));
        assert!(externals.contains("chic_rt.mmio_write"));
    }

    #[test]
    fn mmio_address_overflow_errors() {
        let (result, _) = with_emitter(|emitter| {
            let mut spec = sample_spec(32);
            spec.base_address = u64::MAX;
            spec.offset = 2;
            emitter.mmio_address(&spec)
        });
        let err = result.expect_err("overflow should error");
        assert!(
            err.to_string()
                .contains("MMIO address for register REG overflows")
        );
    }

    #[test]
    fn mmio_address_rejects_i64_overflow() {
        let (result, _) = with_emitter(|emitter| {
            let mut spec = sample_spec(32);
            spec.base_address = (i64::MAX as u64) + 1;
            spec.offset = 0;
            emitter.mmio_address(&spec)
        });
        let err = result.expect_err("i64 overflow should error");
        assert!(err.to_string().contains("exceeds i64 range"));
    }

    #[test]
    fn mmio_flags_encode_endianness_and_space() {
        let (flags, _) = with_emitter(|emitter| {
            let mut spec = sample_spec(32);
            spec.endianness = MmioEndianness::Big;
            spec.address_space = AddressSpaceId::from_name("apb");
            emitter.mmio_flags(&spec)
        });

        let expected = encode_flags(MmioEndianness::Big, AddressSpaceId::from_name("apb"));
        assert_eq!(flags, expected);
    }
}
