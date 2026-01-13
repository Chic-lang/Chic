mod helpers;
mod intrinsic;
mod runtime;
mod shared;
mod wrappers;

#[cfg(test)]
mod tests;

pub(super) use runtime::decimal_runtime_symbol;

pub(super) const DECIMAL_PARTS_TY: &str = "{ i32, i32, i32, i32 }";
pub(super) const DECIMAL_RUNTIME_RESULT_TY: &str = "{ i32, { i32, i32, i32, i32 } }";
pub(super) const DECIMAL_FLAG_VECTORIZE: u128 = 0x0000_0001;
pub(super) const DECIMAL_INTRINSIC_RESULT_CANONICAL: &str =
    "Std::Numeric::Decimal::DecimalIntrinsicResult";
pub(super) const DECIMAL_VECTORIZE_CANONICAL: &str = "Std::Numeric::Decimal::DecimalVectorizeHint";
