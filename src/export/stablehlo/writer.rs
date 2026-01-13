#![allow(dead_code)]

/// Stub writer for StableHLO export.
pub fn write_stablehlo(_path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(_path, bytes).map_err(|err| err.to_string())
}
