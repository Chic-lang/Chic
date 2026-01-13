use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const METRICS_DIR: &str = "coverage/metrics";
const SNAPSHOT_FILE: &str = "pass_metrics.jsonl";
const BUDGET_FILE: &str = "pass_budgets.json";

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut update = false;
    for arg in args {
        match arg.as_str() {
            "--update" => update = true,
            other => return Err(format!("unknown budget flag `{other}`").into()),
        }
    }

    let root = repo_root();
    let metrics_dir = root.join(METRICS_DIR);
    fs::create_dir_all(&metrics_dir)?;
    let snapshot_path = metrics_dir.join(SNAPSHOT_FILE);
    if snapshot_path.exists() {
        fs::remove_file(&snapshot_path)?;
    }

    run_budget_harness(&root, &snapshot_path)?;

    let snapshots = read_snapshots(&snapshot_path)?;
    if snapshots.is_empty() {
        return Err("no pass metrics were recorded; ensure lowering_budget test ran".into());
    }
    let aggregates = aggregate_snapshots(&snapshots);
    let budget_path = metrics_dir.join(BUDGET_FILE);
    if update {
        write_budgets(&budget_path, &aggregates)?;
        println!("Pass budgets updated in {}/{}", METRICS_DIR, BUDGET_FILE);
        return Ok(());
    }

    verify_budgets(&budget_path, &aggregates)
}

fn run_budget_harness(root: &Path, snapshot_path: &Path) -> Result<(), Box<dyn Error>> {
    let path_str = snapshot_path
        .to_str()
        .ok_or_else(|| "snapshot path contains invalid UTF-8".to_string())?;
    let status = Command::new("cargo")
        .current_dir(root)
        .env("CHIC_PASS_BUDGET_OUT", path_str)
        .args(["test", "--package", "chic", "--test", "lowering_budget"])
        .status()?;
    if !status.success() {
        return Err(format!(
            "`cargo test --package chic --test lowering_budget` failed with status {status:?}"
        )
        .into());
    }
    Ok(())
}

fn read_snapshots(path: &Path) -> Result<Vec<PassSnapshot>, Box<dyn Error>> {
    let raw = fs::read_to_string(path)?;
    let mut snapshots = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let snapshot: PassSnapshot = serde_json::from_str(line)?;
        snapshots.push(snapshot);
    }
    Ok(snapshots)
}

fn aggregate_snapshots(snapshots: &[PassSnapshot]) -> BTreeMap<Key, AggregatedStages> {
    let mut map = BTreeMap::new();
    for snapshot in snapshots {
        let mut features = snapshot.features.clone();
        features.sort();
        let key = Key {
            profile: snapshot.profile.clone(),
            features,
        };
        let entry = map
            .entry(key)
            .or_insert_with(|| AggregatedStages::new(&snapshot.stages));
        entry.accumulate(&snapshot.stages);
    }
    map
}

fn write_budgets(
    path: &Path,
    aggregates: &BTreeMap<Key, AggregatedStages>,
) -> Result<(), Box<dyn Error>> {
    let entries: Vec<BudgetEntry> = aggregates
        .iter()
        .map(|(key, stages)| BudgetEntry {
            profile: key.profile.clone(),
            features: key.features.clone(),
            stages: stages
                .order
                .iter()
                .map(|name| StageBudget {
                    name: name.clone(),
                    max_count: *stages.counts.get(name).unwrap_or(&0),
                })
                .collect(),
        })
        .collect();
    let file = BudgetFile { entries };
    let pretty = serde_json::to_string_pretty(&file)?;
    fs::write(path, pretty)?;
    Ok(())
}

fn verify_budgets(
    path: &Path,
    aggregates: &BTreeMap<Key, AggregatedStages>,
) -> Result<(), Box<dyn Error>> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read {}: {err}. Run `cargo xtask budget --update` to initialise budgets.",
            path.display()
        )
    })?;
    let file: BudgetFile = serde_json::from_str(&raw)?;
    let mut remaining = aggregates.clone();
    let mut errors = Vec::new();

    for entry in file.entries {
        let mut features = entry.features.clone();
        features.sort();
        let key = Key {
            profile: entry.profile.clone(),
            features,
        };
        let actual = match remaining.remove(&key) {
            Some(value) => value,
            None => {
                errors.push(format!(
                    "no pass metrics recorded for profile `{}` with features {:?}",
                    entry.profile, entry.features
                ));
                continue;
            }
        };
        let expected_names: BTreeSet<_> = entry
            .stages
            .iter()
            .map(|stage| stage.name.as_str())
            .collect();
        let observed_names: BTreeSet<_> = actual.counts.keys().map(|name| name.as_str()).collect();
        if expected_names != observed_names {
            let missing: Vec<_> = expected_names
                .difference(&observed_names)
                .cloned()
                .collect();
            let extra: Vec<_> = observed_names
                .difference(&expected_names)
                .cloned()
                .collect();
            if !missing.is_empty() {
                errors.push(format!(
                    "expected pass stages missing for profile `{}` features {:?}: {:?}",
                    entry.profile, entry.features, missing
                ));
            }
            if !extra.is_empty() {
                errors.push(format!(
                    "unexpected pass stages detected for profile `{}` features {:?}: {:?}",
                    entry.profile, entry.features, extra
                ));
            }
            continue;
        }
        for stage in &entry.stages {
            let observed = actual.counts.get(&stage.name).copied().unwrap_or(0);
            if observed != stage.max_count {
                errors.push(format!(
                    "stage `{}` (profile `{}` features {:?}) ran {} time(s), budget allows {} time(s)",
                    stage.name, entry.profile, entry.features, observed, stage.max_count
                ));
            }
        }
    }

    for (key, _) in remaining {
        errors.push(format!(
            "metrics recorded for profile `{}` features {:?} but no matching budget entry exists",
            key.profile, key.features
        ));
    }

    if errors.is_empty() {
        println!(
            "Pass budgets verified against {}/{}",
            METRICS_DIR, BUDGET_FILE
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask dir has parent")
        .to_path_buf()
}

#[derive(Deserialize)]
struct PassSnapshot {
    profile: String,
    features: Vec<String>,
    stages: Vec<StageSample>,
}

#[derive(Deserialize)]
struct StageSample {
    name: String,
    count: u64,
}

#[derive(Default, Clone)]
struct AggregatedStages {
    order: Vec<String>,
    counts: BTreeMap<String, u64>,
}

impl AggregatedStages {
    fn new(stages: &[StageSample]) -> Self {
        Self {
            order: stages.iter().map(|stage| stage.name.clone()).collect(),
            counts: BTreeMap::new(),
        }
    }

    fn accumulate(&mut self, stages: &[StageSample]) {
        for stage in stages {
            if !self.counts.contains_key(&stage.name) {
                self.order.push(stage.name.clone());
            }
            *self.counts.entry(stage.name.clone()).or_default() += stage.count;
        }
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Clone)]
struct Key {
    profile: String,
    features: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct BudgetFile {
    entries: Vec<BudgetEntry>,
}

#[derive(Serialize, Deserialize)]
struct BudgetEntry {
    profile: String,
    features: Vec<String>,
    stages: Vec<StageBudget>,
}

#[derive(Serialize, Deserialize)]
struct StageBudget {
    name: String,
    max_count: u64,
}
