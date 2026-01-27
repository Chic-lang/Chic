use std::error::Error;
use std::fs;
use std::path::PathBuf;

const REQUIRED_PATTERNS: &[&str] = &[
    "RuntimeIntrinsics.chic_rt_mutex_create",
    "RuntimeIntrinsics.chic_rt_mutex_lock",
    "RuntimeIntrinsics.chic_rt_mutex_unlock",
    "RuntimeIntrinsics.chic_rt_rwlock_create",
    "RuntimeIntrinsics.chic_rt_rwlock_read_lock",
    "RuntimeIntrinsics.chic_rt_rwlock_write_lock",
    "RuntimeIntrinsics.chic_rt_condvar_wait",
    "RuntimeIntrinsics.chic_rt_once_try_begin",
];

pub fn run() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let std_sync_path = manifest_dir
        .parent()
        .expect("xtask lives inside workspace")
        .join("packages/std/src/std_sync.ch");

    let contents = fs::read_to_string(&std_sync_path)?;
    let mut failures = Vec::new();

    for pattern in REQUIRED_PATTERNS {
        if !contents.contains(pattern) {
            failures.push(format!("missing runtime call `{pattern}`"));
        }
    }

    if contents.contains("SyncBackoff") {
        failures.push("found legacy `SyncBackoff` helper; runtime-backed implementation should remove busy spins".to_string());
    }

    if failures.is_empty() {
        println!(
            "lint-sync: packages/std/src/std_sync.ch uses the runtime-backed synchronization primitives."
        );
        Ok(())
    } else {
        eprintln!("lint-sync: synchronization validation failed:");
        for failure in failures {
            eprintln!("  - {failure}");
        }
        Err("lint-sync failed".into())
    }
}
