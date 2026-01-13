use crate::codegen::llvm::emitter::function::builder::FunctionEmitter;
use crate::error::Error;
use crate::mir::{ConstValue, Operand, Place};

pub(super) enum DecimalWrapperSignature {
    Binary,
    BinaryWithRounding,
    BinaryWithOptions,
    Fma,
    FmaWithRounding,
    FmaWithOptions,
}

pub(super) struct DecimalWrapperSpec<'a> {
    pub canonical: &'a str,
    pub signature: DecimalWrapperSignature,
}

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_decimal_wrapper_call(
        &mut self,
        spec: &DecimalWrapperSpec<'_>,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        let method = spec
            .canonical
            .rsplit("::")
            .next()
            .ok_or_else(|| Error::Codegen("decimal wrapper missing method name".into()))?;
        match spec.signature {
            DecimalWrapperSignature::Binary => {
                self.emit_decimal_wrapper_binary(method, args, destination, target)
            }
            DecimalWrapperSignature::BinaryWithRounding => {
                self.emit_decimal_wrapper_binary_with_rounding(method, args, destination, target)
            }
            DecimalWrapperSignature::BinaryWithOptions => {
                self.emit_decimal_wrapper_binary_with_options(method, args, destination, target)
            }
            DecimalWrapperSignature::Fma => {
                self.emit_decimal_wrapper_fma(method, args, destination, target)
            }
            DecimalWrapperSignature::FmaWithRounding => {
                self.emit_decimal_wrapper_fma_with_rounding(method, args, destination, target)
            }
            DecimalWrapperSignature::FmaWithOptions => {
                self.emit_decimal_wrapper_fma_with_options(method, args, destination, target)
            }
        }
    }

    fn emit_decimal_wrapper_binary(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 2 {
            return Err(Error::Codegen(format!(
                "`{}` expects two decimal arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        let vectorized = method.contains("Vectorized");
        let rounding_const = ConstValue::Enum {
            type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
            variant: "TiesToEven".into(),
            discriminant: 0,
        };
        let variant = if vectorized {
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            )
        } else {
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            )
        };
        self.emit_decimal_intrinsic_fixed(
            op,
            &args[..2],
            None,
            Some(rounding_const),
            if vectorized {
                super::DECIMAL_FLAG_VECTORIZE
            } else {
                0
            },
            variant,
            vectorized,
            destination,
            target,
        )
    }

    fn emit_decimal_wrapper_binary_with_rounding(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 3 {
            return Err(Error::Codegen(format!(
                "`{}` expects decimal, decimal, and rounding arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        let vectorized = method.contains("Vectorized");
        let flags = if vectorized {
            super::DECIMAL_FLAG_VECTORIZE
        } else {
            0
        };
        self.emit_decimal_intrinsic_fixed(
            op,
            &args[..2],
            Some(&args[2]),
            None,
            flags,
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            ),
            vectorized,
            destination,
            target,
        )
    }

    fn emit_decimal_wrapper_binary_with_options(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 4 {
            return Err(Error::Codegen(format!(
                "`{}` expects decimal, decimal, rounding, and vectorize arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        self.emit_decimal_intrinsic_with_options(
            op,
            &args[..2],
            &args[2],
            &args[3],
            destination,
            target,
        )
    }

    fn emit_decimal_wrapper_fma(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 3 {
            return Err(Error::Codegen(format!(
                "`{}` expects three decimal arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        let vectorized = method.contains("Vectorized");
        let rounding_const = ConstValue::Enum {
            type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
            variant: "TiesToEven".into(),
            discriminant: 0,
        };
        let variant = if vectorized {
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            )
        } else {
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            )
        };
        self.emit_decimal_intrinsic_fixed(
            op,
            &args[..3],
            None,
            Some(rounding_const),
            if vectorized {
                super::DECIMAL_FLAG_VECTORIZE
            } else {
                0
            },
            variant,
            vectorized,
            destination,
            target,
        )
    }

    fn emit_decimal_wrapper_fma_with_rounding(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 4 {
            return Err(Error::Codegen(format!(
                "`{}` expects three decimal and rounding arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        let vectorized = method.contains("Vectorized");
        let flags = if vectorized {
            super::DECIMAL_FLAG_VECTORIZE
        } else {
            0
        };
        self.emit_decimal_intrinsic_fixed(
            op,
            &args[..3],
            Some(&args[3]),
            None,
            flags,
            (
                "Std::Numeric::Decimal::DecimalIntrinsicVariant",
                "Scalar",
                0,
            ),
            vectorized,
            destination,
            target,
        )
    }

    fn emit_decimal_wrapper_fma_with_options(
        &mut self,
        method: &str,
        args: &[Operand],
        destination: Option<&Place>,
        target: crate::mir::BlockId,
    ) -> Result<(), Error> {
        if args.len() < 5 {
            return Err(Error::Codegen(format!(
                "`{}` expects three decimal, rounding, and vectorize arguments",
                method
            )));
        }
        let op = decimal_op_from_method(method)
            .ok_or_else(|| Error::Codegen(format!("unsupported decimal intrinsic `{}`", method)))?;
        self.emit_decimal_intrinsic_with_options(
            op,
            &args[..3],
            &args[3],
            &args[4],
            destination,
            target,
        )
    }
}

pub(super) fn wrapper_spec<'a>(
    canonical_lower: &str,
    canonical: &'a str,
) -> Option<DecimalWrapperSpec<'a>> {
    let signature = match canonical_lower {
        "std::decimal::intrinsics::add"
        | "std::decimal::intrinsics::sub"
        | "std::decimal::intrinsics::mul"
        | "std::decimal::intrinsics::div"
        | "std::decimal::intrinsics::rem"
        | "std::decimal::intrinsics::addvectorized"
        | "std::decimal::intrinsics::subvectorized"
        | "std::decimal::intrinsics::mulvectorized"
        | "std::decimal::intrinsics::divvectorized"
        | "std::decimal::intrinsics::remvectorized" => DecimalWrapperSignature::Binary,
        "std::decimal::intrinsics::addwithoptions"
        | "std::decimal::intrinsics::subwithoptions"
        | "std::decimal::intrinsics::mulwithoptions"
        | "std::decimal::intrinsics::divwithoptions"
        | "std::decimal::intrinsics::remwithoptions" => DecimalWrapperSignature::BinaryWithOptions,
        "std::decimal::intrinsics::addvectorizedwithrounding"
        | "std::decimal::intrinsics::subvectorizedwithrounding"
        | "std::decimal::intrinsics::mulvectorizedwithrounding"
        | "std::decimal::intrinsics::divvectorizedwithrounding"
        | "std::decimal::intrinsics::remvectorizedwithrounding" => {
            DecimalWrapperSignature::BinaryWithRounding
        }
        "std::decimal::intrinsics::fma" | "std::decimal::intrinsics::fmavectorized" => {
            DecimalWrapperSignature::Fma
        }
        "std::decimal::intrinsics::fmawithoptions" => DecimalWrapperSignature::FmaWithOptions,
        "std::decimal::intrinsics::fmawithrounding" => DecimalWrapperSignature::FmaWithRounding,
        _ => return None,
    };
    Some(DecimalWrapperSpec {
        canonical,
        signature,
    })
}

pub(super) fn decimal_op_from_method(method: &str) -> Option<&'static str> {
    if method.starts_with("Add") {
        Some("add")
    } else if method.starts_with("Sub") {
        Some("sub")
    } else if method.starts_with("Mul") {
        Some("mul")
    } else if method.starts_with("Div") {
        Some("div")
    } else if method.starts_with("Rem") {
        Some("rem")
    } else if method.starts_with("Fma") {
        Some("fma")
    } else {
        None
    }
}
