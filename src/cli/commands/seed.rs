use std::path::PathBuf;

use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|arg| is_help_flag(arg)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("seed".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let mut run_path: Option<PathBuf> = None;
    let mut profile = None;
    let mut json = false;

    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--from-run" => {
                let Some(path) = iter.next() else {
                    return Err(CliError::with_usage(
                        "--from-run requires a perf.json/runlog path",
                    ));
                };
                run_path = Some(PathBuf::from(path));
            }
            "--profile" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "--profile requires a profile name (e.g. debug/release)",
                    ));
                };
                profile = Some(value);
            }
            "--json" | "-j" => json = true,
            other if !other.starts_with('-') && run_path.is_none() => {
                run_path = Some(PathBuf::from(other));
            }
            other => {
                return Err(CliError::with_usage(format!(
                    "unrecognised seed option `{other}`"
                )));
            }
        }
    }

    let run_path = run_path.unwrap_or_else(|| PathBuf::from("perf.json"));

    Ok(Cli {
        command: Command::Seed {
            run_path,
            profile,
            json,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
