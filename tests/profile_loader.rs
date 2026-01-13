use chic::driver::profile_loader::{ScheduleProfile, load_profile};
use std::path::PathBuf;

#[test]
fn profile_loader_rejects_hash_mismatch() {
    let path = PathBuf::from("profiling/profiles/sample.json");
    let err = load_profile(&path, "cafebabe").unwrap_err();
    assert!(
        err.contains("profile hash mismatch"),
        "expected hash mismatch diagnostic: {err}"
    );
}

#[test]
fn profile_loader_reads_profile() {
    let path = PathBuf::from("profiling/profiles/sample.json");
    let profile = load_profile(&path, "deadbeef").expect("load profile");
    assert_eq!(
        profile,
        ScheduleProfile {
            graph: "example".into(),
            hash: "deadbeef".into(),
            params: serde_json::json!({ "tile": [4,4], "unroll": 2 }),
        }
    );
}
