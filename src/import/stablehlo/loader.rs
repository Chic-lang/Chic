#![allow(dead_code)]

/// Pinned StableHLO version understood by the loader.
pub const STABLEHLO_VERSION: &str = "0.0.0-stub";

/// Stub StableHLO loader. Will parse StableHLO and map into Chic graphs/schedules.
pub fn load_stablehlo(_bytes: &[u8]) -> Result<(), String> {
    Err("stablehlo loader not implemented".into())
}
