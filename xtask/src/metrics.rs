use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File},
    io::Write as _,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::Instant,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const METRICS_DIR: &str = "coverage/metrics";
const JSON_FILE: &str = "compile_metrics.json";
const CSV_FILE: &str = "compile_metrics.csv";
const BASELINE_FILE: &str = "baseline.json";
const CLEAN_REGRESSION_PCT: f64 = 0.10;
const CLEAN_REGRESSION_ABS: f64 = 2.0;
const INCREMENTAL_REGRESSION_PCT: f64 = 0.25;
const INCREMENTAL_REGRESSION_ABS: f64 = 0.5;
const BENCH_REGRESSION_PCT: f64 = 0.10;
const BENCH_REGRESSION_ABS: f64 = 0.25;
const SIZE_REGRESSION_PCT: f64 = 0.05;
const SIZE_REGRESSION_ABS_BYTES: u64 = 512 * 1024;

pub struct MetricsOptions {
    pub bench: Option<String>,
    pub skip_tests: bool,
}

pub fn run(options: MetricsOptions) -> Result<(), Box<dyn Error>> {
    if let Some(name) = options.bench {
        run_bench(&name)
    } else {
        run_full(options.skip_tests)
    }
}

fn run_full(skip_tests: bool) -> Result<(), Box<dyn Error>> {
    let root = repo_root();
    let metrics_dir = root.join(METRICS_DIR);
    fs::create_dir_all(&metrics_dir)?;

    let mut runs = Vec::new();
    for workload in WORKLOADS {
        if skip_tests && workload.name == "test-all" {
            println!("==> Skipping {} (requested --skip-tests)", workload.name);
            continue;
        }
        run_cargo_clean(&root)?;
        println!("==> Running {} (clean build)", workload.name);
        runs.push(execute_workload(&root, workload, RunMode::Clean)?);
        println!("==> Running {} (incremental build)", workload.name);
        runs.push(execute_workload(&root, workload, RunMode::Incremental)?);
    }
    for bench in BENCH_WORKLOADS {
        run_cargo_clean(&root)?;
        println!("==> Running {} (bench)", bench.name);
        runs.push(execute_workload(&root, bench, RunMode::Bench)?);
    }

    let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let report = MetricsReport {
        generated_at,
        git_commit: read_git_commit(&root),
        host: HostInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu: detect_cpu_brand(),
        },
        runs,
    };

    write_report(&metrics_dir, JSON_FILE, CSV_FILE, &report)?;
    if std::env::var_os("CI").is_some() {
        println!("CI environment detected; skipping baseline regression comparison.");
    } else {
        check_against_baseline(&metrics_dir, BASELINE_FILE, &report)?;
    }

    println!(
        "Metrics written to {}/{{{}, {}}}",
        METRICS_DIR, JSON_FILE, CSV_FILE
    );

    Ok(())
}

fn run_bench(name: &str) -> Result<(), Box<dyn Error>> {
    let workload = bench_workload(name).ok_or_else(|| format!("unknown bench `{name}`"))?;
    let root = repo_root();
    let metrics_dir = root.join(METRICS_DIR).join("bench").join(name);
    fs::create_dir_all(&metrics_dir)?;

    run_cargo_clean(&root)?;
    println!("==> Running bench {}", workload.name);
    let run = execute_workload(&root, workload, RunMode::Bench)?;

    let generated_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
    let report = MetricsReport {
        generated_at,
        git_commit: read_git_commit(&root),
        host: HostInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu: detect_cpu_brand(),
        },
        runs: vec![run],
    };

    let json_name = format!("{name}_bench.json");
    let csv_name = format!("{name}_bench.csv");
    let baseline_name = format!("{name}_bench_baseline.json");
    write_report(&metrics_dir, &json_name, &csv_name, &report)?;
    if std::env::var_os("CI").is_some() {
        println!("CI environment detected; skipping baseline regression comparison.");
    } else {
        check_against_baseline(&metrics_dir, &baseline_name, &report)?;
    }

    println!(
        "Bench metrics written to {}/bench/{name}/{{{json_name}, {csv_name}}}",
        METRICS_DIR
    );

    Ok(())
}

fn run_cargo_clean(root: &Path) -> Result<(), Box<dyn Error>> {
    let status = Command::new("cargo")
        .current_dir(root)
        .arg("clean")
        .status()?;
    ensure_success(status, "cargo clean")
}

fn execute_workload(
    root: &Path,
    workload: &Workload,
    mode: RunMode,
) -> Result<RunRecord, Box<dyn Error>> {
    let mut command = Command::new("cargo");
    command
        .args(workload.args)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let start = Instant::now();
    let status = command.status()?;
    let duration = start.elapsed().as_secs_f64();
    ensure_success(status, &format!("cargo {}", workload.args.join(" ")))?;

    Ok(RunRecord {
        workload: workload.name.to_string(),
        command: workload.args.iter().map(|arg| arg.to_string()).collect(),
        mode,
        duration_seconds: duration,
        exit_status: status.code(),
        artifacts: collect_artifacts(root, workload.artifacts),
    })
}

fn collect_artifacts(root: &Path, artifacts: &[Artifact]) -> Vec<ArtifactRecord> {
    artifacts
        .iter()
        .map(|artifact| {
            let absolute = root.join(artifact.path);
            let size_bytes = fs::metadata(&absolute).ok().map(|meta| meta.len());
            ArtifactRecord {
                label: artifact.label.to_string(),
                path: artifact.path.to_string(),
                size_bytes,
            }
        })
        .collect()
}

fn write_report(
    dir: &Path,
    json_name: &str,
    csv_name: &str,
    report: &MetricsReport,
) -> Result<(), Box<dyn Error>> {
    write_json(dir, json_name, report)?;
    write_csv(dir, csv_name, &report.runs)?;
    Ok(())
}

fn write_json(dir: &Path, filename: &str, report: &MetricsReport) -> Result<(), Box<dyn Error>> {
    let file = File::create(dir.join(filename))?;
    serde_json::to_writer_pretty(file, report)?;
    Ok(())
}

fn write_csv(dir: &Path, filename: &str, runs: &[RunRecord]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(dir.join(filename))?;
    writeln!(
        file,
        "workload,mode,command,duration_seconds,exit_status,artifact_label,artifact_path,size_bytes"
    )?;

    for run in runs {
        let command = format!("cargo {}", run.command.join(" "));
        if run.artifacts.is_empty() {
            writeln!(
                file,
                "{},{},{},{:.3},{},,,",
                run.workload,
                run.mode.as_str(),
                escape_csv(&command),
                run.duration_seconds,
                run.exit_status
                    .map_or_else(|| "".to_string(), |code| code.to_string())
            )?;
        } else {
            for artifact in &run.artifacts {
                writeln!(
                    file,
                    "{},{},{},{:.3},{},{},{},{}",
                    run.workload,
                    run.mode.as_str(),
                    escape_csv(&command),
                    run.duration_seconds,
                    run.exit_status
                        .map_or_else(|| "".to_string(), |code| code.to_string()),
                    artifact.label,
                    artifact.path,
                    artifact
                        .size_bytes
                        .map_or_else(|| "".to_string(), |size| size.to_string())
                )?;
            }
        }
    }

    Ok(())
}

fn check_against_baseline(
    metrics_dir: &Path,
    baseline_name: &str,
    report: &MetricsReport,
) -> Result<(), Box<dyn Error>> {
    let baseline_path = metrics_dir.join(baseline_name);
    let Some(baseline) = read_baseline_report(&baseline_path)? else {
        println!(
            "No baseline at {}; skipping regression comparison.",
            baseline_path.display()
        );
        return Ok(());
    };

    let regressions = detect_regressions(report, &baseline);
    if regressions.is_empty() {
        Ok(())
    } else {
        for regression in &regressions {
            eprintln!("metric regression: {regression}");
        }
        Err("metrics regression detected".into())
    }
}

fn read_baseline_report(path: &Path) -> Result<Option<MetricsReport>, Box<dyn Error>> {
    if !path.exists() {
        return Ok(None);
    }
    let file = File::open(path)?;
    let report = serde_json::from_reader(file)?;
    Ok(Some(report))
}

fn detect_regressions(current: &MetricsReport, baseline: &MetricsReport) -> Vec<String> {
    use std::collections::HashMap;

    let mut baseline_runs = HashMap::new();
    for run in &baseline.runs {
        baseline_runs.insert(run_key(run), run);
    }

    let mut regressions = Vec::new();
    for run in &current.runs {
        let key = run_key(run);
        let Some(base_run) = baseline_runs.get(&key) else {
            regressions.push(format!("no baseline entry for workload {key}"));
            continue;
        };

        if let Some(msg) = compare_duration(run, base_run) {
            regressions.push(msg);
        }

        let mut base_artifacts = HashMap::new();
        for artifact in &base_run.artifacts {
            base_artifacts.insert(artifact.label.as_str(), artifact);
        }
        for artifact in &run.artifacts {
            if let Some(base_artifact) = base_artifacts.get(artifact.label.as_str()) {
                if let Some(msg) = compare_size(run, artifact, base_artifact) {
                    regressions.push(msg);
                }
            }
        }
    }

    regressions
}

fn compare_duration(current: &RunRecord, baseline: &RunRecord) -> Option<String> {
    if current.duration_seconds <= baseline.duration_seconds {
        return None;
    }
    let allowed = allowed_duration_delta(current.mode, baseline.duration_seconds);
    let delta = current.duration_seconds - baseline.duration_seconds;
    if delta > allowed {
        Some(format!(
            "{} ({}) duration regressed by {:.2}s (allowed {:.2}s, baseline {:.2}s, current {:.2}s)",
            current.workload,
            current.mode.as_str(),
            delta,
            allowed,
            baseline.duration_seconds,
            current.duration_seconds
        ))
    } else {
        None
    }
}

fn compare_size(
    run: &RunRecord,
    current: &ArtifactRecord,
    baseline: &ArtifactRecord,
) -> Option<String> {
    let Some(current_size) = current.size_bytes else {
        return None;
    };
    let Some(baseline_size) = baseline.size_bytes else {
        return None;
    };

    if current_size <= baseline_size {
        return None;
    }

    let allowed = allowed_size_delta(baseline_size);
    let delta = current_size - baseline_size;
    let delta_f64 = delta as f64;
    if delta_f64 > allowed {
        Some(format!(
            "{} ({}) artifact '{}' grew by {:.2} MiB (allowed {:.2} MiB, baseline {:.2} MiB, current {:.2} MiB)",
            run.workload,
            run.mode.as_str(),
            current.label,
            bytes_to_mib(delta_f64),
            bytes_to_mib(allowed),
            bytes_to_mib(baseline_size as f64),
            bytes_to_mib(current_size as f64),
        ))
    } else {
        None
    }
}

fn allowed_duration_delta(mode: RunMode, baseline_seconds: f64) -> f64 {
    let (pct, abs) = match mode {
        RunMode::Clean => (CLEAN_REGRESSION_PCT, CLEAN_REGRESSION_ABS),
        RunMode::Incremental => (INCREMENTAL_REGRESSION_PCT, INCREMENTAL_REGRESSION_ABS),
        RunMode::Bench => (BENCH_REGRESSION_PCT, BENCH_REGRESSION_ABS),
    };
    (baseline_seconds * pct).max(abs)
}

fn allowed_size_delta(baseline_bytes: u64) -> f64 {
    let pct_limit = (baseline_bytes as f64) * SIZE_REGRESSION_PCT;
    let abs_limit = SIZE_REGRESSION_ABS_BYTES as f64;
    pct_limit.max(abs_limit)
}

fn bytes_to_mib(bytes: f64) -> f64 {
    bytes / (1024.0 * 1024.0)
}

fn run_key(run: &RunRecord) -> String {
    format!("{}::{}", run.workload, run.mode.as_str())
}

fn escape_csv(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        let mut escaped = String::from("\"");
        escaped.push_str(&value.replace('"', "\"\""));
        escaped.push('"');
        escaped
    } else {
        value.to_string()
    }
}

fn ensure_success(status: ExitStatus, label: &str) -> Result<(), Box<dyn Error>> {
    if status.success() {
        Ok(())
    } else {
        let code = status
            .code()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "terminated by signal".to_string());
        Err(format!("{label} failed with status {code}").into())
    }
}

fn read_git_commit(root: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn detect_cpu_brand() -> Option<String> {
    Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is in workspace")
        .to_path_buf()
}

#[derive(Clone, Copy)]
struct Workload {
    name: &'static str,
    args: &'static [&'static str],
    artifacts: &'static [Artifact],
}

#[derive(Clone, Copy)]
struct Artifact {
    label: &'static str,
    path: &'static str,
}

#[derive(Serialize, Deserialize, Clone)]
struct MetricsReport {
    generated_at: String,
    git_commit: Option<String>,
    host: HostInfo,
    runs: Vec<RunRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
struct HostInfo {
    os: String,
    arch: String,
    cpu: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RunRecord {
    workload: String,
    command: Vec<String>,
    #[serde(with = "run_mode_serde")]
    mode: RunMode,
    duration_seconds: f64,
    exit_status: Option<i32>,
    artifacts: Vec<ArtifactRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ArtifactRecord {
    label: String,
    path: String,
    size_bytes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RunMode {
    Clean,
    Incremental,
    Bench,
}

impl RunMode {
    fn as_str(&self) -> &'static str {
        match self {
            RunMode::Clean => "clean",
            RunMode::Incremental => "incremental",
            RunMode::Bench => "bench",
        }
    }
}

mod run_mode_serde {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    use super::RunMode;

    pub fn serialize<S>(mode: &RunMode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(mode.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RunMode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "clean" => Ok(RunMode::Clean),
            "incremental" => Ok(RunMode::Incremental),
            "bench" => Ok(RunMode::Bench),
            other => Err(D::Error::custom(format!("unknown run mode '{other}'"))),
        }
    }
}

const ARTIFACT_CHIC_DEBUG: Artifact = Artifact {
    label: "chic-debug",
    path: "target/debug/chic",
};

const ARTIFACT_CHIC_RELEASE: Artifact = Artifact {
    label: "chic-release",
    path: "target/release/chic",
};

const ARTIFACT_XTASK_DEBUG: Artifact = Artifact {
    label: "xtask-debug",
    path: "target/debug/xtask",
};

const ARTIFACT_XTASK_RELEASE: Artifact = Artifact {
    label: "xtask-release",
    path: "target/release/xtask",
};

const WORKLOADS: &[Workload] = &[
    Workload {
        name: "build-dev",
        args: &["build", "--workspace", "--all-targets"],
        artifacts: &[ARTIFACT_CHIC_DEBUG, ARTIFACT_XTASK_DEBUG],
    },
    Workload {
        name: "build-release",
        args: &["build", "--workspace", "--all-targets", "--release"],
        artifacts: &[ARTIFACT_CHIC_RELEASE, ARTIFACT_XTASK_RELEASE],
    },
    Workload {
        name: "test-all",
        args: &["test", "--workspace", "--all-targets"],
        artifacts: &[ARTIFACT_CHIC_DEBUG, ARTIFACT_XTASK_DEBUG],
    },
];

const TRAITS_BENCH_WORKLOAD: Workload = Workload {
    name: "traits-bench",
    args: &["bench", "--bench", "traits"],
    artifacts: &[],
};

const DECIMAL_FAST_BENCH_WORKLOAD: Workload = Workload {
    name: "decimal-fast-bench",
    args: &[
        "bench",
        "--bench",
        "decimal_fast",
        "--no-run",
        "--features",
        "simd-test-hooks",
    ],
    artifacts: &[],
};

const NDARRAY_BENCH_WORKLOAD: Workload = Workload {
    name: "ndarray",
    args: &["bench", "--bench", "ndarray", "--no-run"],
    artifacts: &[],
};

const BENCH_WORKLOADS: &[Workload] = &[DECIMAL_FAST_BENCH_WORKLOAD, NDARRAY_BENCH_WORKLOAD];

fn bench_workload(name: &str) -> Option<&'static Workload> {
    match name {
        "traits" => Some(&TRAITS_BENCH_WORKLOAD),
        "decimal_fast" => Some(&DECIMAL_FAST_BENCH_WORKLOAD),
        "ndarray" => Some(&NDARRAY_BENCH_WORKLOAD),
        _ => None,
    }
}
