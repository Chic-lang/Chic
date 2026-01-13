use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::cli::CliError;
use crate::error::{Error, Result};
use crate::perf::PerfSnapshot;
use crate::run_log::{self, RunLog};

#[derive(Debug, Serialize)]
struct SeedEntry {
    stream: u64,
    seed: String,
}

pub(in crate::cli::dispatch) fn run_seed(
    run_path: &Path,
    profile: Option<&str>,
    json: bool,
) -> Result<()> {
    let log = load_run_log(run_path, profile)?;
    let mut entries: Vec<SeedEntry> = log
        .rng_streams
        .iter()
        .map(|stream| SeedEntry {
            stream: stream.id,
            seed: format!("0x{seed:032x}", seed = stream.seed),
        })
        .collect();
    entries.sort_by_key(|entry| entry.stream);

    if json {
        serde_json::to_writer_pretty(std::io::stdout(), &entries)
            .map_err(|err| Error::Cli(CliError::new(format!("failed to encode seeds: {err}"))))?;
        println!();
        return Ok(());
    }

    let profile_suffix = profile.map(|name| format!(" ({name})")).unwrap_or_default();
    println!("rng seeds from {}{}:", run_path.display(), profile_suffix);
    if entries.is_empty() {
        println!("  (no rng streams recorded)");
    } else {
        for entry in &entries {
            println!("  stream {:016x}: {}", entry.stream, entry.seed);
        }
    }
    Ok(())
}

fn load_run_log(path: &Path, profile: Option<&str>) -> Result<RunLog> {
    if let Ok(log) = run_log::load(path) {
        return Ok(log);
    }
    let body = fs::read_to_string(path).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to read run log {}: {err}",
            path.display()
        )))
    })?;
    let snapshot: PerfSnapshot = serde_json::from_str(&body).map_err(|err| {
        Error::Cli(CliError::new(format!(
            "failed to decode run log {}: {err}",
            path.display()
        )))
    })?;
    let run = snapshot.run_by_profile(profile).ok_or_else(|| {
        Error::Cli(CliError::new(format!(
            "perf snapshot for target {} is missing run data",
            snapshot.target
        )))
    })?;
    if let Some(log) = run.run_log.clone() {
        return Ok(log);
    }
    Err(Error::Cli(CliError::new(format!(
        "profiling run {} does not include RNG data",
        run.profile
    ))))
}
