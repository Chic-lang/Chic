use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::cli::CliError;
use crate::error::{Error, Result};
use crate::perf::{PerfRun, PerfSnapshot};

#[derive(Debug, Serialize)]
struct ReportEntry {
    trace_id: u64,
    label: String,
    cpu_us: f64,
    budget_cpu_us: Option<u64>,
    budget_over_us: Option<f64>,
    budget_over_pct: Option<f64>,
    baseline_cpu_us: Option<f64>,
    baseline_delta_us: Option<f64>,
    baseline_delta_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ReportSummary {
    profile: String,
    target: String,
    total: usize,
    over_budget: usize,
    regressions: usize,
}

#[derive(Debug, Serialize)]
struct PerfReportOutput {
    summary: ReportSummary,
    entries: Vec<ReportEntry>,
}

pub(in crate::cli::dispatch) fn run_perf_report(
    perf: &Path,
    baseline: Option<&Path>,
    profile: Option<&str>,
    json: bool,
    strict: bool,
    tolerance: f64,
) -> Result<()> {
    let snapshot = load_snapshot(perf)?;
    let run = select_run(&snapshot, profile)?;
    let baseline_snapshot = if let Some(path) = baseline {
        Some(load_snapshot(path)?)
    } else {
        None
    };
    let baseline_run = match baseline_snapshot.as_ref() {
        Some(snapshot) => Some(select_run(snapshot, profile)?),
        None => None,
    };

    let report = build_report(&snapshot.target, run, baseline_run, tolerance);

    if json {
        serde_json::to_writer_pretty(std::io::stdout(), &report).map_err(|err| {
            Error::Cli(CliError::new(format!(
                "failed to encode report as JSON: {err}"
            )))
        })?;
        println!();
    } else {
        print_summary(&report);
    }

    if strict && (report.summary.over_budget > 0 || report.summary.regressions > 0) {
        return Err(Error::Cli(CliError::new(
            "perf regressions detected; rerun with --json for details",
        )));
    }

    Ok(())
}

fn load_snapshot(path: &Path) -> Result<PerfSnapshot> {
    let data = fs::read_to_string(path).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read perf data {}: {err}",
            path.display()
        )))
    })?;
    serde_json::from_str(&data).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to decode perf data {}: {err}",
            path.display()
        )))
    })
}

fn select_run<'a>(snapshot: &'a PerfSnapshot, profile: Option<&str>) -> Result<&'a PerfRun> {
    snapshot.run_by_profile(profile).ok_or_else(|| {
        Error::Cli(CliError::new(format!(
            "perf snapshot for target {} is missing run data",
            snapshot.target
        )))
    })
}

fn build_report(
    target: &str,
    run: &PerfRun,
    baseline: Option<&PerfRun>,
    tolerance: f64,
) -> PerfReportOutput {
    let mut over_budget = 0usize;
    let mut regressions = 0usize;
    let mut entries = Vec::new();

    let baseline_metrics: HashMap<u64, f64> = baseline
        .into_iter()
        .flat_map(|run| {
            run.metrics
                .iter()
                .map(|metric| (metric.trace_id, metric.cpu_us))
        })
        .collect();

    for metric in &run.metrics {
        let mut entry = ReportEntry {
            trace_id: metric.trace_id,
            label: metric.label.clone(),
            cpu_us: metric.cpu_us,
            budget_cpu_us: metric.budget_cpu_us,
            budget_over_us: None,
            budget_over_pct: None,
            baseline_cpu_us: None,
            baseline_delta_us: None,
            baseline_delta_pct: None,
        };

        if let Some(budget) = metric.budget_cpu_us {
            let delta = metric.cpu_us - budget as f64;
            if delta > 0.0 {
                over_budget += 1;
            }
            entry.budget_over_us = Some(delta);
            entry.budget_over_pct = Some(delta / budget as f64 * 100.0);
        }

        if let Some(previous) = baseline_metrics.get(&metric.trace_id).copied() {
            let delta = metric.cpu_us - previous;
            let pct = if previous.abs() > f64::EPSILON {
                delta / previous * 100.0
            } else {
                0.0
            };
            entry.baseline_cpu_us = Some(previous);
            entry.baseline_delta_us = Some(delta);
            entry.baseline_delta_pct = Some(pct);
            if pct > tolerance {
                regressions += 1;
            }
        }

        entries.push(entry);
    }

    entries.sort_by(|lhs, rhs| {
        rhs.budget_over_pct
            .partial_cmp(&lhs.budget_over_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| rhs.cpu_us.total_cmp(&lhs.cpu_us))
    });

    let summary = ReportSummary {
        profile: run.profile.clone(),
        target: target.to_string(),
        total: entries.len(),
        over_budget,
        regressions,
    };

    PerfReportOutput { summary, entries }
}

fn print_summary(report: &PerfReportOutput) {
    let summary = &report.summary;
    println!(
        "perf report for profile '{}' on target {}",
        summary.profile, summary.target
    );
    println!(
        "  metrics: {} | over budget: {} | regressions vs baseline: {}",
        summary.total, summary.over_budget, summary.regressions
    );
    if report.entries.is_empty() {
        println!("  no trace metrics recorded");
        return;
    }
    println!("  top drift:");
    for entry in report.entries.iter().take(5) {
        let budget_note =
            entry
                .budget_over_us
                .zip(entry.budget_over_pct)
                .and_then(|(delta, pct)| {
                    if let Some(budget) = entry.budget_cpu_us {
                        Some(format!(
                            "budget {}us drift {delta:+.2}us ({pct:+.1}%)",
                            budget
                        ))
                    } else {
                        None
                    }
                });
        let baseline_note = entry
            .baseline_delta_us
            .zip(entry.baseline_delta_pct)
            .map(|(delta, pct)| format!("baseline {delta:+.2}us ({pct:+.1}%)"));
        let mut notes = Vec::new();
        if let Some(note) = budget_note {
            notes.push(note);
        }
        if let Some(note) = baseline_note {
            notes.push(note);
        }
        let note = if notes.is_empty() {
            String::from("no budget/baseline data")
        } else {
            notes.join(" | ")
        };
        println!(
            "    {:016x} {:<32} {:>8.2}us  {}",
            entry.trace_id, entry.label, entry.cpu_us, note
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::PerfMetric;

    fn sample_metric(trace_id: u64, cpu_us: f64, budget: Option<u64>) -> PerfMetric {
        PerfMetric {
            trace_id,
            mir_id: format!("func{trace_id}"),
            label: format!("trace{trace_id}"),
            cpu_us,
            budget_cpu_us: budget,
            budget_gpu_us: None,
            budget_mem_bytes: None,
        }
    }

    fn sample_run(metrics: Vec<PerfMetric>) -> PerfRun {
        PerfRun {
            profile: "debug".into(),
            metrics,
            run_log: None,
        }
    }

    #[test]
    fn reports_budget_and_baseline_drift() {
        let run = sample_run(vec![
            sample_metric(1, 12.5, Some(10)),
            sample_metric(2, 5.0, None),
        ]);
        let baseline = sample_run(vec![sample_metric(1, 10.0, Some(10))]);
        let report = build_report("host", &run, Some(&baseline), 1.0);
        assert_eq!(report.summary.total, 2);
        assert_eq!(report.summary.over_budget, 1);
        assert_eq!(report.summary.regressions, 1);
        let entry = report
            .entries
            .iter()
            .find(|entry| entry.trace_id == 1)
            .unwrap();
        assert!(entry.budget_over_us.unwrap() > 0.0);
        assert!(entry.baseline_delta_us.unwrap() > 0.0);
    }
}
