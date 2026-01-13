use crate::mir::MmioEndianness;
use blake3;
use std::fmt;

/// Compact identifier for an MMIO address space.
///
/// Address spaces let drivers distinguish between different peripheral buses
/// (e.g. AHB vs APB) without duplicating address ranges. They are encoded into
/// runtime flags so simulated environments can keep their state separated.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressSpaceId(u16);

impl AddressSpaceId {
    /// Canonical identifier for the default address space.
    pub const DEFAULT: Self = Self(0);

    /// Creates an identifier from an optional address space name.
    #[must_use]
    pub fn from_optional(space: Option<&str>) -> Self {
        match space {
            Some(name) => Self::from_name(name),
            None => Self::DEFAULT,
        }
    }

    /// Creates a stable identifier from a named address space.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let digest = blake3::hash(name.as_bytes());
        // Use the first two bytes as a 16-bit little-endian value. Reserved 0
        // for the default space; pulse to 1 when a collision occurs.
        let value = u16::from_le_bytes([digest.as_bytes()[0], digest.as_bytes()[1]]);
        if value == 0 { Self(1) } else { Self(value) }
    }

    /// Returns the raw identifier used when encoding runtime flags.
    #[must_use]
    pub const fn to_raw(self) -> u16 {
        self.0
    }
}

impl fmt::Debug for AddressSpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AddressSpaceId({:#06x})", self.0)
    }
}

/// Error emitted when a register width does not match the supported range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidWidthError {
    pub width_bits: u16,
}

impl fmt::Display for InvalidWidthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid MMIO width {}; expected 8, 16, 32, or 64 bits",
            self.width_bits
        )
    }
}

impl std::error::Error for InvalidWidthError {}

const BIG_ENDIAN_FLAG: u32 = 0x1;
const ADDRESS_SPACE_SHIFT: u32 = 8;
const ADDRESS_SPACE_MASK: u32 = 0xFFFF << ADDRESS_SPACE_SHIFT;

/// Encodes endianness + address space into the compact flag representation
/// consumed by runtime shims.
#[must_use]
pub fn encode_flags(endianness: MmioEndianness, space: AddressSpaceId) -> i32 {
    let mut flags = u32::from(space.to_raw()) << ADDRESS_SPACE_SHIFT;
    if matches!(endianness, MmioEndianness::Big) {
        flags |= BIG_ENDIAN_FLAG;
    }
    flags as i32
}

/// Decodes runtime flags back into endianness + address space identifiers.
#[must_use]
pub fn decode_flags(flags: i32) -> (MmioEndianness, AddressSpaceId) {
    let raw = flags as u32;
    let endianness = if (raw & BIG_ENDIAN_FLAG) != 0 {
        MmioEndianness::Big
    } else {
        MmioEndianness::Little
    };
    let space_raw = ((raw & ADDRESS_SPACE_MASK) >> ADDRESS_SPACE_SHIFT) as u16;
    (endianness, AddressSpaceId(space_raw))
}

/// Ensures the requested register width is supported (8/16/32/64).
pub fn validate_width(width_bits: u16) -> Result<(), InvalidWidthError> {
    if width_bits == 0 || width_bits > 64 || width_bits % 8 != 0 {
        Err(InvalidWidthError { width_bits })
    } else {
        Ok(())
    }
}

/// Returns a masking value that confines a raw MMIO read/write to the requested
/// width.
pub fn mask_for_width(width_bits: u16) -> Result<u64, InvalidWidthError> {
    validate_width(width_bits)?;
    Ok(if width_bits == 64 {
        u64::MAX
    } else {
        (1u64 << width_bits) - 1
    })
}

/// Converts a stored MMIO value into the architectural endianness expected by
/// the overlay fields.
pub fn decode_value(
    stored: u64,
    width_bits: u16,
    endianness: MmioEndianness,
) -> Result<u64, InvalidWidthError> {
    let mask = mask_for_width(width_bits)?;
    let adjusted = match endianness {
        MmioEndianness::Little => stored,
        MmioEndianness::Big => swap_bytes(stored, width_bits),
    };
    Ok(adjusted & mask)
}

/// Normalises a value before persisting it in simulated MMIO storage.
pub fn encode_value(
    value: u64,
    width_bits: u16,
    endianness: MmioEndianness,
) -> Result<u64, InvalidWidthError> {
    let mask = mask_for_width(width_bits)?;
    let masked = value & mask;
    Ok(match endianness {
        MmioEndianness::Little => masked,
        MmioEndianness::Big => swap_bytes(masked, width_bits),
    })
}

/// Swaps the byte order for a given register width (assumes validated width).
fn swap_bytes(value: u64, width_bits: u16) -> u64 {
    let width_bytes = usize::from(width_bits / 8);
    let mut bytes = value.to_le_bytes();
    bytes[..width_bytes].reverse();
    let mut out = [0u8; 8];
    out[..width_bytes].copy_from_slice(&bytes[..width_bytes]);
    u64::from_le_bytes(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_space_default_stable() {
        assert_eq!(AddressSpaceId::DEFAULT.to_raw(), 0);
        let derived = AddressSpaceId::from_optional(None);
        assert_eq!(derived.to_raw(), 0);
    }

    #[test]
    fn address_space_from_name_not_zero() {
        let id = AddressSpaceId::from_name("ahb");
        assert_ne!(id.to_raw(), 0);
        let repeat = AddressSpaceId::from_name("ahb");
        assert_eq!(id.to_raw(), repeat.to_raw());
    }

    #[test]
    fn encode_decode_flags_round_trip() {
        let id = AddressSpaceId::from_name("apb");
        let flags = encode_flags(MmioEndianness::Big, id);
        let (decoded_endian, decoded_space) = decode_flags(flags);
        assert_eq!(decoded_endian, MmioEndianness::Big);
        assert_eq!(decoded_space.to_raw(), id.to_raw());
    }

    #[test]
    fn mask_for_width_validates() {
        assert!(mask_for_width(16).is_ok());
        assert!(mask_for_width(7).is_err());
    }

    #[test]
    fn encode_decode_value_round_trip() {
        let id = AddressSpaceId::from_name("mmio");
        assert_ne!(id.to_raw(), 0);
        let stored = encode_value(0x1234, 16, MmioEndianness::Big).unwrap();
        assert_eq!(stored, 0x3412);
        let decoded = decode_value(stored, 16, MmioEndianness::Big).unwrap();
        assert_eq!(decoded, 0x1234);
    }
}
