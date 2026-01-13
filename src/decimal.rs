use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::convert::TryInto;
use std::fmt;
use std::str::FromStr;

/// Maximum supported decimal scale (matches Chic spec).
pub const DECIMAL_MAX_SCALE: u32 = 28;

/// Bit mask for decimal runtime flags.
pub const DECIMAL_FLAG_VECTORIZE: u32 = 0x0000_0001;

/// Error produced while parsing or manipulating decimal values.
#[derive(Debug)]
pub struct DecimalError {
    kind: DecimalErrorKind,
}

impl DecimalError {
    pub fn overflow() -> Self {
        Self {
            kind: DecimalErrorKind::Overflow,
        }
    }

    pub fn divide_by_zero() -> Self {
        Self {
            kind: DecimalErrorKind::DivideByZero,
        }
    }

    pub fn invalid_literal(literal: impl Into<String>) -> Self {
        Self {
            kind: DecimalErrorKind::InvalidLiteral(literal.into()),
        }
    }

    pub fn invalid_conversion(message: impl Into<String>) -> Self {
        Self {
            kind: DecimalErrorKind::InvalidConversion(message.into()),
        }
    }

    pub(crate) fn kind(&self) -> &DecimalErrorKind {
        &self.kind
    }
}

#[derive(Debug)]
pub(crate) enum DecimalErrorKind {
    Overflow,
    DivideByZero,
    InvalidLiteral(String),
    InvalidConversion(String),
}

impl fmt::Display for DecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DecimalErrorKind::Overflow => write!(f, "decimal overflow"),
            DecimalErrorKind::DivideByZero => write!(f, "division by zero"),
            DecimalErrorKind::InvalidLiteral(text) => {
                write!(f, "invalid decimal literal `{text}`")
            }
            DecimalErrorKind::InvalidConversion(text) => {
                write!(f, "{text}")
            }
        }
    }
}

impl std::error::Error for DecimalError {}

/// Supported rounding strategies for decimal intrinsics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalRoundingMode {
    TiesToEven,
    TowardZero,
    AwayFromZero,
    TowardPositive,
    TowardNegative,
}

impl DecimalRoundingMode {
    fn strategy(self) -> RoundingStrategy {
        match self {
            DecimalRoundingMode::TiesToEven => RoundingStrategy::MidpointNearestEven,
            DecimalRoundingMode::TowardZero => RoundingStrategy::ToZero,
            DecimalRoundingMode::AwayFromZero => RoundingStrategy::AwayFromZero,
            DecimalRoundingMode::TowardPositive => RoundingStrategy::ToPositiveInfinity,
            DecimalRoundingMode::TowardNegative => RoundingStrategy::ToNegativeInfinity,
        }
    }

    pub fn from_discriminant(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::TiesToEven),
            1 => Some(Self::TowardZero),
            2 => Some(Self::AwayFromZero),
            3 => Some(Self::TowardPositive),
            4 => Some(Self::TowardNegative),
            _ => None,
        }
    }

    pub fn as_discriminant(self) -> u32 {
        match self {
            Self::TiesToEven => 0,
            Self::TowardZero => 1,
            Self::AwayFromZero => 2,
            Self::TowardPositive => 3,
            Self::TowardNegative => 4,
        }
    }
}

/// Canonical Chic decimal representation (mirrors .NET layout).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decimal128(Decimal);

impl Decimal128 {
    pub fn new(decimal: Decimal) -> Result<Self, DecimalError> {
        if decimal.scale() > DECIMAL_MAX_SCALE {
            return Err(DecimalError::overflow());
        }
        Ok(Self(decimal.normalize()))
    }

    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    pub fn as_decimal(&self) -> &Decimal {
        &self.0
    }

    pub fn into_decimal(self) -> Decimal {
        self.0
    }

    pub fn from_bits(bits: [u32; 4]) -> Self {
        let scale = ((bits[3] >> 16) & 0xFF) as u32;
        let negative = (bits[3] & 0x8000_0000) != 0;
        let decimal = Decimal::from_parts(bits[0], bits[1], bits[2], negative, scale);
        Self(decimal)
    }

    pub fn to_bits(self) -> [u32; 4] {
        let bytes = self.0.serialize();
        let flags = u32::from_le_bytes(bytes[0..4].try_into().expect("slice size matches"));
        let lo = u32::from_le_bytes(bytes[4..8].try_into().expect("slice size matches"));
        let mid = u32::from_le_bytes(bytes[8..12].try_into().expect("slice size matches"));
        let hi = u32::from_le_bytes(bytes[12..16].try_into().expect("slice size matches"));
        [lo, mid, hi, flags]
    }

    pub fn to_encoding(self) -> u128 {
        let bits = self.to_bits();
        let mut value = 0u128;
        value |= bits[0] as u128;
        value |= (bits[1] as u128) << 32;
        value |= (bits[2] as u128) << 64;
        value |= (bits[3] as u128) << 96;
        value
    }

    pub fn from_encoding(value: u128) -> Self {
        let lo = (value & 0xFFFF_FFFF) as u32;
        let mid = ((value >> 32) & 0xFFFF_FFFF) as u32;
        let hi = ((value >> 64) & 0xFFFF_FFFF) as u32;
        let flags = ((value >> 96) & 0xFFFF_FFFF) as u32;
        Self::from_bits([lo, mid, hi, flags])
    }

    pub fn parse_literal(text: &str) -> Result<Self, DecimalError> {
        let cleaned: String = text
            .chars()
            .filter(|ch| *ch != '_')
            .collect::<String>()
            .trim()
            .to_string();
        if cleaned.is_empty() {
            return Err(DecimalError::invalid_literal(text));
        }
        if Self::fractional_digit_count(&cleaned) > DECIMAL_MAX_SCALE {
            return Err(DecimalError::overflow());
        }
        let decimal =
            Decimal::from_str(&cleaned).map_err(|_| DecimalError::invalid_literal(text))?;
        Self::new(decimal)
    }

    fn fractional_digit_count(text: &str) -> u32 {
        let mut count = 0u32;
        let mut seen_dot = false;
        for ch in text.chars() {
            if ch == 'e' || ch == 'E' {
                break;
            }
            if seen_dot {
                if ch.is_ascii_digit() {
                    count += 1;
                } else {
                    break;
                }
            } else if ch == '.' {
                seen_dot = true;
            }
        }
        count
    }

    pub fn add(self, rhs: Self, rounding: DecimalRoundingMode) -> Result<Self, DecimalError> {
        let value = self
            .0
            .checked_add(rhs.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(value, rounding)
    }

    pub fn sub(self, rhs: Self, rounding: DecimalRoundingMode) -> Result<Self, DecimalError> {
        let value = self
            .0
            .checked_sub(rhs.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(value, rounding)
    }

    pub fn mul(self, rhs: Self, rounding: DecimalRoundingMode) -> Result<Self, DecimalError> {
        let value = self
            .0
            .checked_mul(rhs.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(value, rounding)
    }

    pub fn div(self, rhs: Self, rounding: DecimalRoundingMode) -> Result<Self, DecimalError> {
        if rhs.0.is_zero() {
            return Err(DecimalError::divide_by_zero());
        }
        let value = self
            .0
            .checked_div(rhs.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(value, rounding)
    }

    pub fn rem(self, rhs: Self) -> Result<Self, DecimalError> {
        if rhs.0.is_zero() {
            return Err(DecimalError::divide_by_zero());
        }
        let value = self
            .0
            .checked_rem(rhs.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(value, DecimalRoundingMode::TiesToEven)
    }

    pub fn fma(
        self,
        multiplicand: Self,
        addend: Self,
        rounding: DecimalRoundingMode,
    ) -> Result<Self, DecimalError> {
        let product = self
            .0
            .checked_mul(multiplicand.0)
            .ok_or_else(DecimalError::overflow)?;
        let sum = product
            .checked_add(addend.0)
            .ok_or_else(DecimalError::overflow)?;
        Self::round(sum, rounding)
    }

    fn round(value: Decimal, rounding: DecimalRoundingMode) -> Result<Self, DecimalError> {
        let rounded = value.round_dp_with_strategy(DECIMAL_MAX_SCALE, rounding.strategy());
        Self::new(rounded)
    }

    pub fn from_i128(value: i128) -> Result<Self, DecimalError> {
        let decimal = Decimal::from_i128_with_scale(value, 0);
        Self::new(decimal)
    }

    pub fn from_u128(value: u128) -> Result<Self, DecimalError> {
        let signed = i128::try_from(value).map_err(|_| DecimalError::overflow())?;
        Self::from_i128(signed)
    }

    pub fn from_f64(value: f64) -> Result<Self, DecimalError> {
        let decimal = Decimal::from_f64(value).ok_or_else(|| {
            DecimalError::invalid_conversion(
                "cannot convert floating-point value to decimal without overflow",
            )
        })?;
        Self::new(decimal)
    }

    pub fn negate(self) -> Self {
        Self((-self.0).normalize())
    }

    pub fn to_f64(&self) -> Option<f64> {
        self.0.to_f64()
    }
}

impl From<Decimal> for Decimal128 {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn as_string(value: &Decimal128) -> String {
        value.into_decimal().to_string()
    }

    #[test]
    fn parse_literal_accepts_underscores() {
        let value = Decimal128::parse_literal("1_000.500_0").expect("literal should parse");
        assert_eq!(as_string(&value), "1000.5");
    }

    #[test]
    fn parse_literal_rejects_excessive_scale() {
        let result = Decimal128::parse_literal("0.1234567890123456789012345678901");
        assert!(result.is_err(), "scale above 28 digits must be rejected");
    }

    #[test]
    fn division_by_zero_reports_error() {
        let lhs = Decimal128::parse_literal("1").unwrap();
        let rhs = Decimal128::parse_literal("0").unwrap();
        let result = lhs.div(rhs, DecimalRoundingMode::TiesToEven);
        assert!(matches!(result, Err(err) if matches!(err.kind(), DecimalErrorKind::DivideByZero)));
    }

    #[test]
    fn remainder_by_zero_reports_error() {
        let lhs = Decimal128::parse_literal("5").unwrap();
        let rhs = Decimal128::parse_literal("0").unwrap();
        let result = lhs.rem(rhs);
        assert!(matches!(result, Err(err) if matches!(err.kind(), DecimalErrorKind::DivideByZero)));
    }

    #[test]
    fn add_preserves_precision() {
        let a = Decimal128::parse_literal("0.125").unwrap();
        let b = Decimal128::parse_literal("0.875").unwrap();
        let sum = a
            .add(b, DecimalRoundingMode::TiesToEven)
            .expect("addition should succeed");
        assert_eq!(as_string(&sum), "1");
    }

    #[test]
    fn fma_combines_multiply_and_add() {
        let lhs = Decimal128::parse_literal("2.5").unwrap();
        let multiplicand = Decimal128::parse_literal("4").unwrap();
        let addend = Decimal128::parse_literal("0.125").unwrap();
        let result = lhs
            .fma(multiplicand, addend, DecimalRoundingMode::TowardZero)
            .expect("fma should succeed");
        assert_eq!(as_string(&result), "10.125");
    }
}
