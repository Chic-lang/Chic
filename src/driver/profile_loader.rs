#![allow(dead_code)]

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ScheduleProfile {
    pub graph: String,
    pub hash: String,
    pub params: serde_json::Value,
}

/// Load a schedule profile and verify the hash matches.
pub fn load_profile(path: &Path, expected_hash: &str) -> Result<ScheduleProfile, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let profile: ScheduleProfile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    if profile.hash != expected_hash {
        return Err(format!(
            "profile hash mismatch: expected {expected_hash}, found {}",
            profile.hash
        ));
    }
    Ok(profile)
}
