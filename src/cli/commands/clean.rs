use crate::logging::LogOptions;

use super::super::{Cli, CliError, Command};
use super::common::is_help_flag;

pub(super) fn parse(args: Vec<String>) -> Result<Cli, CliError> {
    if args.iter().any(|value| is_help_flag(value)) {
        return Ok(Cli {
            command: Command::Help {
                topic: Some("clean".into()),
            },
            log_options: LogOptions::from_env(),
            error_format: None,
        });
    }

    let mut input = None;
    let mut artifacts_path = None;
    let mut configuration: Option<String> = None;
    let mut all = false;
    let mut dry_run = false;

    let mut idx = 0;
    while idx < args.len() {
        let flag = &args[idx];
        idx += 1;

        match flag.as_str() {
            "--artifacts-path" => {
                let Some(value) = args.get(idx) else {
                    return Err(CliError::with_usage(
                        "expected directory after --artifacts-path",
                    ));
                };
                idx += 1;
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(CliError::with_usage(
                        "--artifacts-path requires a non-empty value",
                    ));
                }
                artifacts_path = Some(trimmed.into());
            }
            "-c" | "--configuration" => {
                let Some(value) = args.get(idx) else {
                    return Err(CliError::with_usage(
                        "expected value after -c/--configuration",
                    ));
                };
                idx += 1;
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(CliError::with_usage(
                        "--configuration requires a non-empty value",
                    ));
                }
                configuration = Some(trimmed.to_string());
            }
            "--all" => {
                all = true;
            }
            "--dry-run" => {
                dry_run = true;
            }
            other if other.starts_with('-') => {
                return Err(CliError::with_usage(format!(
                    "unsupported option '{other}' for command"
                )));
            }
            other => {
                if input.is_some() {
                    return Err(CliError::with_usage(
                        "chic clean accepts at most one project/directory argument",
                    ));
                }
                if other.trim().is_empty() {
                    return Err(CliError::with_usage("input path must not be empty"));
                }
                input = Some(other.into());
            }
        }
    }

    Ok(Cli {
        command: Command::Clean {
            input,
            artifacts_path,
            configuration: configuration.unwrap_or_else(|| "Debug".to_string()),
            all,
            dry_run,
        },
        log_options: LogOptions::from_env(),
        error_format: None,
    })
}
