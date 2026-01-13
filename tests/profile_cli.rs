use std::fs;

use assert_cmd::Command;
use chic::perf::PerfSnapshot;
use tempfile::tempdir;

#[test]
fn chic_profile_generates_artefacts_and_flamegraph() {
    let tempdir = tempdir().expect("tempdir");
    let source = tempdir.path().join("profile_sample.cl");
    fs::write(
        &source,
        r#"namespace Exec {
    public int Helper(int value) {
        return value + 1;
    }
    public int Main() {
        var result = Helper(1);
        return result - 2;
    }
}"#,
    )
    .expect("write profile sample");
    let output = tempdir.path().join("profiling").join("perf.json");
    let output_str = output.to_string_lossy().to_string();

    let mut cmd = Command::cargo_bin("chic").expect("chic binary");
    cmd.args([
        "profile",
        source.to_str().expect("utf8 source"),
        "--backend",
        "wasm",
        "--profile-out",
        &output_str,
        "--profile-flamegraph",
    ]);
    cmd.env("CHIC_SKIP_STDLIB", "1");
    cmd.assert().success();

    let perf_body = fs::read_to_string(&output).expect("read perf.json");
    let snapshot: PerfSnapshot = serde_json::from_str(&perf_body).expect("decode perf.json");
    assert!(snapshot.summary.is_some(), "summary should be present");
    let run = snapshot
        .run_by_profile(Some("debug"))
        .expect("profiling run");
    assert!(
        !run.metrics.is_empty(),
        "profiling run should record at least one metric"
    );
    let summary_path = output.with_extension("summary.json");
    assert!(
        summary_path.exists(),
        "summary output should exist at {}",
        summary_path.display()
    );
    let folded_path = output.with_extension("folded");
    assert!(
        folded_path.exists(),
        "folded output should exist at {}",
        folded_path.display()
    );
    let svg_path = output.with_extension("svg");
    assert!(svg_path.exists(), "flamegraph should be rendered");
}
