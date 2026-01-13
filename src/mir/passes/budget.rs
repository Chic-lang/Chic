use crate::mir::builder::PassStageMetric;
use serde::Serialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn record_pass_metrics(stages: &[PassStageMetric]) {
    if stages.is_empty() {
        return;
    }
    let path = match env::var("CHIC_PASS_BUDGET_OUT") {
        Ok(value) if !value.is_empty() => value,
        _ => return,
    };
    let snapshot = PassSnapshot {
        profile: build_profile(),
        target: format!("{}-{}", env::consts::OS, env::consts::ARCH),
        features: active_features(),
        stages,
    };
    if let Err(err) = append_snapshot(&path, &snapshot) {
        eprintln!("warning: failed to write pass budget snapshot: {err}");
    }
}

fn append_snapshot(path: &str, snapshot: &PassSnapshot<'_>) -> std::io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, snapshot)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn active_features() -> Vec<String> {
    let mut features = Vec::new();
    if cfg!(feature = "runtime-simd") {
        features.push("runtime-simd".to_string());
    }
    if cfg!(feature = "simd-test-hooks") {
        features.push("simd-test-hooks".to_string());
    }
    features.sort();
    features
}

#[derive(Serialize)]
struct PassSnapshot<'a> {
    profile: &'static str,
    target: String,
    features: Vec<String>,
    stages: &'a [PassStageMetric],
}

const fn build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
