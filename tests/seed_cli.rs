use std::fs;

use assert_cmd::Command;
use chic::perf::{PerfRun, PerfSnapshot};
use chic::run_log::{RUN_LOG_VERSION, RngEvent, RngEventKind, RngStreamLog, RunLog};
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct SeedOutput {
    stream: u64,
    seed: String,
}

#[test]
fn chic_seed_reads_runlog_json() {
    let tmp = tempdir().expect("tempdir");
    let log = RunLog {
        version: RUN_LOG_VERSION.to_string(),
        rng_streams: vec![RngStreamLog {
            id: 1,
            seed: 0xAA,
            events: vec![RngEvent {
                index: 0,
                op: RngEventKind::Next { bits: 64 },
            }],
        }],
    };
    let path = tmp.path().join("runlog.json");
    fs::write(&path, serde_json::to_string(&log).unwrap()).expect("write runlog");

    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.args([
        "seed",
        "--from-run",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Vec<SeedOutput> =
        serde_json::from_slice(&output).expect("decode seed output as json");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].stream, 1);
    assert_eq!(parsed[0].seed, "0x000000000000000000000000000000aa");
}

#[test]
fn chic_seed_reads_perf_snapshot_run_log() {
    let tmp = tempdir().expect("tempdir");
    let log = RunLog {
        version: RUN_LOG_VERSION.to_string(),
        rng_streams: vec![RngStreamLog {
            id: 7,
            seed: 0xBEEF,
            events: vec![],
        }],
    };
    let snapshot = PerfSnapshot {
        version: "1".into(),
        target: "wasm32-unknown-unknown".into(),
        runs: vec![PerfRun {
            profile: "debug".into(),
            metrics: Vec::new(),
            run_log: Some(log),
        }],
        summary: None,
    };
    let path = tmp.path().join("perf.json");
    fs::write(&path, serde_json::to_string(&snapshot).unwrap()).expect("write perf snapshot");

    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.args([
        "seed",
        "--from-run",
        path.to_str().expect("utf8 path"),
        "--profile",
        "debug",
        "--json",
    ]);
    let output = cmd.assert().success().get_output().stdout.clone();
    let parsed: Vec<SeedOutput> =
        serde_json::from_slice(&output).expect("decode seed output as json");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].stream, 7);
    assert_eq!(parsed[0].seed, "0x0000000000000000000000000000beef");
}
