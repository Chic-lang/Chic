use super::*;

#[cfg(test)]
use crate::mmio::AddressSpaceId;

impl<'a> Executor<'a> {
    fn read_mmio(
        &self,
        address: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<u64, WasmExecutionError> {
        let width = Self::width_from_bits(width_bits)?;
        let (endianness, space) = decode_flags(flags);
        let key = (space, address);
        let stored = *self.mmio.get(&key).unwrap_or(&0);
        decode_value(stored, width, endianness).map_err(Self::mmio_width_error)
    }

    fn write_mmio(
        &mut self,
        address: u64,
        value: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<(), WasmExecutionError> {
        let width = Self::width_from_bits(width_bits)?;
        let (endianness, space) = decode_flags(flags);
        let key = (space, address);
        let stored = encode_value(value, width, endianness).map_err(Self::mmio_width_error)?;
        self.mmio.insert(key, stored);
        Ok(())
    }

    fn width_from_bits(width_bits: u32) -> Result<u16, WasmExecutionError> {
        if width_bits == 0 || width_bits > 64 || width_bits % 8 != 0 {
            return Err(Self::invalid_width(width_bits));
        }
        u16::try_from(width_bits).map_err(|_| Self::invalid_width(width_bits))
    }

    fn invalid_width(width_bits: u32) -> WasmExecutionError {
        WasmExecutionError {
            message: format!("invalid MMIO width {width_bits}; expected 8, 16, 32, or 64 bits"),
        }
    }

    fn mmio_width_error(err: InvalidWidthError) -> WasmExecutionError {
        WasmExecutionError {
            message: err.to_string(),
        }
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn set_mmio_value(&mut self, address: u64, value: u64) {
        self.mmio.insert((AddressSpaceId::DEFAULT, address), value);
    }

    #[cfg(test)]
    pub fn test_mmio_write(
        &mut self,
        address: u64,
        value: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<(), WasmExecutionError> {
        self.write_mmio(address, value, width_bits, flags)
    }

    #[cfg(test)]
    pub fn test_mmio_read(
        &self,
        address: u64,
        width_bits: u32,
        flags: i32,
    ) -> Result<u64, WasmExecutionError> {
        self.read_mmio(address, width_bits, flags)
    }

}
