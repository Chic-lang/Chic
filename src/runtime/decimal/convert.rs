#![cfg_attr(chic_native_runtime, allow(dead_code))]

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decimal128Parts {
    pub lo: u32,
    pub mid: u32,
    pub hi: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalConstPtr {
    pub ptr: *const Decimal128Parts,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalMutPtr {
    pub ptr: *mut Decimal128Parts,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalRoundingAbi {
    pub value: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecimalRuntimeResult {
    pub status: super::abi::DecimalRuntimeStatus,
    pub _padding: [u8; 12],
    pub value: Decimal128Parts,
}
