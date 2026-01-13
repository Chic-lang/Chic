use std::path::PathBuf;

use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.first().is_some_and(|arg| is_help_flag(arg)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("perf report".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let mut perf_path = PathBuf::from("perf.json");
    let mut baseline = None;
    let mut json = false;
    let mut profile = None;
    let mut strict = false;
    let mut tolerance = 5.0_f64;

    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--baseline" | "-b" => {
                let Some(path) = iter.next() else {
                    return Err(CliError::with_usage(
                        "--baseline requires a perf.json path argument",
                    ));
                };
                baseline = Some(PathBuf::from(path));
            }
            "--json" | "-j" => json = true,
            "--profile" | "-p" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "--profile requires a profile name (e.g. debug/release)",
                    ));
                };
                profile = Some(value);
            }
            "--tolerance" => {
                let Some(value) = iter.next() else {
                    return Err(CliError::with_usage(
                        "--tolerance requires a percentage value",
                    ));
                };
                tolerance = value.parse::<f64>().map_err(|_| {
                    CliError::with_usage(format!(
                        "unable to parse tolerance `{value}` (expected percentage)"
                    ))
                })?;
            }
            "--fail-on-regressions" => strict = true,
            other if !other.starts_with('-') => perf_path = PathBuf::from(other),
            other => {
                return Err(CliError::with_usage(format!(
                    "unrecognised perf report option `{other}`"
                )));
            }
        }
    }

    Ok(Cli {
        command: Command::PerfReport {
            perf_path,
            baseline,
            profile,
            json,
            strict,
            tolerance,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
